//! The facade. GUI, CLI, TUI, and MCP call this crate and nothing below it.
//!
//! Frontends ask questions and, later, ask for plans. They never build an
//! [`castellan_core::Action`] themselves and never reach the safety kernel or
//! the Win32 wrappers directly. The architecture check in `xtask` enforces that.
//!
//! Everything this build exposes is read-only, except writing the user's own
//! preferences file.

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
use std::sync::mpsc::Receiver;

use castellan_core::live::{
    Facts, FindingView, MeasuredTarget, Overview, Room, SweepReading, VolumeReading,
};
use castellan_core::{Language, PersonaMoment, Severity, UserPreferences, persona};
use castellan_engine::{VolumeSpace, inspect_volume};
use thiserror::Error;

pub use castellan_sense::PulseClock;

/// Version of the contract between the service and its frontends.
/// Bump it when a type in [`castellan_core::live`] changes shape.
pub const CONTRACT_VERSION: u32 = 1;

/// Why a service call failed. The text is safe to show to a person.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ServiceError {
    /// The machine could not be read.
    #[error("the system could not be read: {0}")]
    Read(String),
    /// A measurement could not be turned into a score.
    #[error("the measurement could not be scored: {0}")]
    Score(String),
    /// The preferences file could not be read or written.
    #[error("preferences: {0}")]
    Preferences(String),
}

/// Prepare the process before any library is loaded on demand.
///
/// Restricts later DLL loads to System32. Call it first in `main`.
pub fn harden_process() {
    let _ = castellan_winapi::lock_dll_search();
}

/// The Castellan service. Cheap to create; holds no open handles.
#[derive(Clone, Debug, Default)]
pub struct Castellan;

impl Castellan {
    /// Open the service.
    #[must_use]
    pub fn open() -> Self {
        Self
    }

    /// The overview every frontend starts on. Reads free space on the Windows volume.
    pub fn overview(&self) -> Result<Overview, ServiceError> {
        let volume = castellan_sense::system_volume().map_err(ServiceError::Read)?;
        overview_from(volume)
    }

    /// Facts about this PC: drives, memory, uptime, processors.
    #[must_use]
    pub fn facts(&self) -> Facts {
        castellan_sense::facts()
    }

    /// A clock that hands out live readings about once a second.
    #[must_use]
    pub fn pulse(&self) -> PulseClock {
        PulseClock::start()
    }

    /// Measure both temp folders on a worker thread, read-only.
    #[must_use]
    pub fn measure_sweep(&self, min_age_secs: u64) -> Receiver<(MeasuredTarget, SweepReading)> {
        castellan_sense::measure_targets(min_age_secs)
    }

    /// The computer name for headers.
    #[must_use]
    pub fn computer_name(&self) -> String {
        castellan_sense::computer_name()
    }

    /// Local time as `HH:MM`.
    #[must_use]
    pub fn clock(&self) -> String {
        castellan_sense::clock()
    }

    /// Whether this process may change the real system. Debug builds always say no.
    ///
    /// The live cleaner is not connected yet, so even a yes changes nothing today.
    #[must_use]
    pub fn real_changes_allowed(&self) -> bool {
        castellan_guard::mutation_allowed(cfg!(debug_assertions), false, false)
    }

    /// `{LOCALAPPDATA}\Castellan\preferences.toml`. Never a hard-coded drive.
    pub fn preferences_path(&self) -> Result<PathBuf, ServiceError> {
        let root = std::env::var_os("LOCALAPPDATA")
            .ok_or_else(|| ServiceError::Preferences("LOCALAPPDATA is not set".to_owned()))?;
        let root = PathBuf::from(root);
        if root.as_os_str().is_empty() || !root.is_absolute() {
            return Err(ServiceError::Preferences(
                "LOCALAPPDATA is not an absolute path".to_owned(),
            ));
        }
        Ok(root.join("Castellan").join("preferences.toml"))
    }

    /// Load the user's preferences, or the defaults when there is no file yet.
    pub fn load_preferences(&self) -> Result<UserPreferences, ServiceError> {
        load_preferences(&self.preferences_path()?)
    }

    /// Save the user's preferences.
    pub fn save_preferences(&self, prefs: &UserPreferences) -> Result<(), ServiceError> {
        save_preferences(&self.preferences_path()?, prefs)
    }
}

/// Read preferences from `path`. A missing file means the compiled defaults.
pub fn load_preferences(path: &Path) -> Result<UserPreferences, ServiceError> {
    match std::fs::read_to_string(path) {
        Ok(text) => UserPreferences::from_toml(&text)
            .map_err(|error| ServiceError::Preferences(error.to_string())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(UserPreferences::compiled_default())
        }
        Err(error) => Err(ServiceError::Preferences(error.to_string())),
    }
}

/// Write preferences to `path`, creating the folder if needed.
pub fn save_preferences(path: &Path, prefs: &UserPreferences) -> Result<(), ServiceError> {
    let text = prefs
        .to_toml()
        .map_err(|error| ServiceError::Preferences(error.to_string()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| ServiceError::Preferences(error.to_string()))?;
    }
    std::fs::write(path, text).map_err(|error| ServiceError::Preferences(error.to_string()))
}

/// Build the overview from one volume reading. Pure, so it is easy to test.
pub fn overview_from(volume: VolumeReading) -> Result<Overview, ServiceError> {
    let inspection = inspect_volume(&VolumeSpace {
        label: volume.label.clone(),
        total_bytes: volume.total_bytes,
        free_bytes: volume.free_bytes,
    })
    .map_err(|error| ServiceError::Score(error.to_string()))?;
    let detail = format!(
        "{} free of {}",
        castellan_core::format_bytes(volume.free_bytes),
        castellan_core::format_bytes(volume.total_bytes)
    );
    let mut findings: Vec<FindingView> = inspection
        .findings
        .iter()
        .filter(|finding| finding.deducts_score())
        .map(|finding| {
            let id = finding.id().as_str();
            FindingView {
                id: id.to_owned(),
                severity: finding.severity(),
                title: finding_title(id),
                detail: detail.clone(),
                advice: finding_advice(id),
                room: finding_room(id),
            }
        })
        .collect();
    findings.sort_by_key(|finding| std::cmp::Reverse(finding.severity));
    let deducts = !findings.is_empty();
    Ok(Overview {
        persona: status_line(volume.total_bytes, volume.free_bytes, deducts),
        score: inspection.score,
        display_band: inspection.display_band,
        findings,
        system_volume: volume,
    })
}

/// Exit status the CLI uses for an overview: 11 critical, 10 warning, 0 clean.
#[must_use]
pub fn exit_code(overview: &Overview) -> i32 {
    if overview
        .findings
        .iter()
        .any(|finding| finding.severity == Severity::Critical)
    {
        11
    } else if overview.findings.is_empty() {
        0
    } else {
        10
    }
}

/// One persona line for a volume reading.
#[must_use]
pub fn status_line(total_bytes: u64, free_bytes: u64, deducts: bool) -> String {
    if !deducts {
        return persona(Language::En, PersonaMoment::WallsHold);
    }
    let used = total_bytes.saturating_sub(free_bytes);
    let percent_used = used
        .saturating_mul(100)
        .checked_div(total_bytes)
        .map(|percent| u8::try_from(percent.min(100)).unwrap_or(100))
        .unwrap_or(0);
    persona(Language::En, PersonaMoment::VolumeFilling { percent_used })
}

fn finding_title(id: &str) -> String {
    match id {
        "STORAGE-001" => "System drive is filling".to_owned(),
        _ => "A measured finding needs a look".to_owned(),
    }
}

fn finding_advice(id: &str) -> String {
    match id {
        "STORAGE-001" => {
            "Clearing old temporary files can win back space. Sweep shows how much.".to_owned()
        }
        _ => "Open the room for this area to see the details.".to_owned(),
    }
}

fn finding_room(id: &str) -> Option<Room> {
    match id {
        "STORAGE-001" => Some(Room::Sweep),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB: u64 = 1024 * 1024 * 1024;

    #[test]
    fn a_full_volume_is_not_called_healthy() {
        let line = status_line(1000, 40, true);
        assert!(line.contains("96% full"));
        assert!(!line.contains("Nothing to fix"));
    }

    #[test]
    fn a_filling_volume_points_to_sweep() {
        let overview = overview_from(VolumeReading {
            label: "C:".to_owned(),
            total_bytes: 500 * GB,
            free_bytes: 20 * GB,
        })
        .expect("overview");
        let finding = overview.findings.first().expect("a finding");
        assert_eq!(finding.id, "STORAGE-001");
        assert_eq!(finding.room, Some(Room::Sweep));
        assert!(
            finding.detail.contains("20 GB free of 500 GB"),
            "{}",
            finding.detail
        );
        assert!(exit_code(&overview) >= 10);
        assert!(overview.score.keep_score.is_some());
        assert_eq!(
            overview.display_band, None,
            "one pillar is not enough for a band"
        );
    }

    #[test]
    fn a_roomy_volume_has_nothing_to_fix() {
        let overview = overview_from(VolumeReading {
            label: "C:".to_owned(),
            total_bytes: 500 * GB,
            free_bytes: 400 * GB,
        })
        .expect("overview");
        assert!(overview.findings.is_empty());
        assert_eq!(exit_code(&overview), 0);
        assert!(overview.persona.contains("walls hold"));
    }

    #[test]
    fn inconsistent_sizes_are_an_error_not_a_score() {
        let result = overview_from(VolumeReading {
            label: "C:".to_owned(),
            total_bytes: 0,
            free_bytes: 0,
        });
        assert!(matches!(result, Err(ServiceError::Score(_))));
    }

    #[test]
    fn preferences_round_trip_in_a_temp_dir() {
        let dir = std::env::temp_dir().join(format!("castellan-svc-prefs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("nested").join("preferences.toml");
        let mut prefs = UserPreferences::compiled_default();
        prefs.user_temp_at_sign_in = true;
        save_preferences(&path, &prefs).expect("save");
        let loaded = load_preferences(&path).expect("load");
        assert!(loaded.user_temp_at_sign_in);
        let fresh = load_preferences(&dir.join("missing.toml")).expect("missing");
        assert!(!fresh.user_temp_at_sign_in);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_real_overview_reads_this_pc() {
        let overview = Castellan::open().overview().expect("overview");
        assert!(overview.system_volume.total_bytes > 0);
    }

    #[test]
    fn a_debug_build_refuses_real_changes() {
        if cfg!(debug_assertions) {
            assert!(!Castellan::open().real_changes_allowed());
        }
    }
}
