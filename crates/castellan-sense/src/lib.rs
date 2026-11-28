//! Read-only collectors.
//!
//! Every function here only reads the machine. Nothing opens a file for writing,
//! changes a setting, or starts a process. Mutations belong to the safety kernel.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::todo,
        clippy::unimplemented,
        clippy::dbg_macro,
    )
)]

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use castellan_core::live::{
    Drive, Facts, MeasuredTarget, Memory, Pulse, SweepReading, SweepSize, VolumeReading,
};

/// Upper bound on entries walked per folder, so a huge temp folder cannot stall the walk.
pub const SWEEP_MEASURE_LIMIT: u64 = 200_000;

/// Physical memory right now.
#[must_use]
pub fn memory() -> Option<Memory> {
    castellan_winapi::memory_status().ok().map(|memory| Memory {
        total_bytes: memory.total_bytes,
        available_bytes: memory.available_bytes,
    })
}

/// Seconds since Windows started.
#[must_use]
pub fn uptime_secs() -> u64 {
    castellan_winapi::uptime_secs()
}

/// Local time as `HH:MM`.
#[must_use]
pub fn clock() -> String {
    castellan_winapi::local_clock()
}

/// The computer name for headers. Falls back to a neutral label.
#[must_use]
pub fn computer_name() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .filter(|name| !name.is_empty() && name.len() <= 63 && !name.chars().any(char::is_control))
        .unwrap_or_else(|| "this keep".to_owned())
}

fn system_root() -> Option<PathBuf> {
    castellan_winapi::windows_directory()
        .ok()
        .and_then(|windows| windows.ancestors().last().map(Path::to_path_buf))
}

/// Free space on the volume that holds Windows.
pub fn system_volume() -> Result<VolumeReading, String> {
    let windows = castellan_winapi::windows_directory().map_err(|error| error.to_string())?;
    let space = castellan_winapi::disk_free_space(&windows).map_err(|error| error.to_string())?;
    let label = windows
        .components()
        .next()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .unwrap_or_else(|| "volume".to_owned());
    Ok(VolumeReading {
        label,
        total_bytes: space.total_bytes,
        free_bytes: space.free_bytes,
    })
}

/// Facts about this PC. Fixed local drives only; the Windows drive comes first.
#[must_use]
pub fn facts() -> Facts {
    let system = system_root();
    let mut drives = Vec::new();
    for root in castellan_winapi::fixed_drive_roots() {
        let Ok(space) = castellan_winapi::disk_free_space(&root) else {
            continue;
        };
        if space.total_bytes == 0 {
            continue;
        }
        let label = root.to_string_lossy().trim_end_matches('\\').to_owned();
        let is_system = system.as_ref().is_some_and(|system| {
            system
                .to_string_lossy()
                .eq_ignore_ascii_case(&root.to_string_lossy())
        });
        drives.push(Drive {
            label,
            total_bytes: space.total_bytes,
            free_bytes: space.free_bytes,
            system: is_system,
        });
    }
    drives.sort_by_key(|drive| !drive.system);
    Facts {
        drives,
        memory: memory(),
        uptime_secs: Some(uptime_secs()),
        logical_cpus: std::thread::available_parallelism()
            .ok()
            .and_then(|count| u32::try_from(count.get()).ok()),
        dev_build: cfg!(debug_assertions),
    }
}

/// Live readings, at most once a second.
pub struct PulseClock {
    last: Instant,
    cpu: Option<castellan_winapi::CpuTimes>,
    period: Duration,
}

impl PulseClock {
    /// Start the clock. The first reading is due after one period.
    #[must_use]
    pub fn start() -> Self {
        Self {
            last: Instant::now(),
            cpu: castellan_winapi::cpu_times().ok(),
            period: Duration::from_millis(1_000),
        }
    }

    /// A reading if a period has passed since the last one, otherwise `None`.
    pub fn poll(&mut self) -> Option<Pulse> {
        if self.last.elapsed() < self.period {
            return None;
        }
        Some(self.read_now())
    }

    /// A reading now. The processor share covers the time since the last reading.
    pub fn read_now(&mut self) -> Pulse {
        self.last = Instant::now();
        let now = castellan_winapi::cpu_times().ok();
        let cpu_percent = match (self.cpu, now) {
            (Some(before), Some(after)) => castellan_winapi::cpu_busy_percent(before, after),
            _ => None,
        };
        self.cpu = now;
        Pulse {
            cpu_percent,
            memory: memory(),
            uptime_secs: Some(uptime_secs()),
        }
    }
}

/// Where a measured target lives on this PC.
#[must_use]
pub fn target_root(target: MeasuredTarget) -> Option<PathBuf> {
    match target {
        MeasuredTarget::UserTemp => castellan_winapi::user_temp_dir().ok(),
        MeasuredTarget::WindowsTemp => castellan_winapi::windows_directory()
            .ok()
            .map(|windows| windows.join("Temp")),
    }
}

/// Measure one folder without following links and without opening anything for writing.
#[must_use]
pub fn measure(root: &Path, aged_before: SystemTime) -> SweepReading {
    match castellan_guard::live::measure_no_follow(root, aged_before, SWEEP_MEASURE_LIMIT) {
        Ok(measure) => SweepReading::Measured(SweepSize {
            files: measure.files,
            bytes: measure.bytes,
            aged_files: measure.aged_files,
            aged_bytes: measure.aged_bytes,
            truncated: measure.truncated,
        }),
        Err(error) if error.code == 5 => SweepReading::Unavailable(
            "Windows lets only administrators read this folder, so it was not measured. \
             Castellan does not ask for administrator rights just to look."
                .to_owned(),
        ),
        Err(error) => SweepReading::Unavailable(format!(
            "This folder could not be read ({error}), so it was not measured."
        )),
    }
}

/// Measure every target on a worker thread. Each result arrives as it finishes.
#[must_use]
pub fn measure_targets(min_age_secs: u64) -> Receiver<(MeasuredTarget, SweepReading)> {
    let (sender, receiver) = channel();
    let _ = std::thread::Builder::new()
        .name("castellan-sweep-measure".to_owned())
        .spawn(move || {
            let cutoff = SystemTime::now()
                .checked_sub(Duration::from_secs(min_age_secs))
                .unwrap_or(UNIX_EPOCH);
            for target in MeasuredTarget::ALL {
                let reading = match target_root(target) {
                    Some(root) => measure(&root, cutoff),
                    None => SweepReading::Unavailable(
                        "Windows did not say where this folder is, so it was not measured."
                            .to_owned(),
                    ),
                };
                if sender.send((target, reading)).is_err() {
                    return;
                }
            }
        });
    receiver
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn facts_put_the_windows_drive_first() {
        let facts = facts();
        let first = facts.drives.first().expect("a fixed drive");
        assert!(first.system, "{facts:?}");
        assert!(facts.memory.is_some());
        assert!(facts.logical_cpus.unwrap_or(0) > 0);
    }

    #[test]
    fn the_system_volume_has_space_numbers() {
        let volume = system_volume().expect("volume");
        assert!(volume.total_bytes > 0);
        assert!(volume.free_bytes <= volume.total_bytes);
        assert!(volume.label.ends_with(':'), "{}", volume.label);
    }

    #[test]
    fn a_pulse_reads_processor_load_after_some_time() {
        let mut clock = PulseClock::start();
        assert!(clock.poll().is_none());
        std::thread::sleep(Duration::from_millis(150));
        let pulse = clock.read_now();
        assert!(pulse.memory.is_some());
        assert!(pulse.uptime_secs.unwrap_or(0) > 0);
        assert!(pulse.cpu_percent.is_some());
    }

    #[test]
    fn measuring_a_missing_folder_says_so() {
        let missing =
            std::env::temp_dir().join(format!("castellan-missing-{}", std::process::id()));
        assert!(matches!(
            measure(&missing, SystemTime::now()),
            SweepReading::Unavailable(_)
        ));
    }
}
