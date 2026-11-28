//! Command dispatch for the `castellan` binary.
//!
//! Debug builds refuse mutations unless `CASTELLAN_ALLOW_REAL_SYSTEM=1` is set,
//! and the test suite never sets it. Read-only commands may look at the machine.

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

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use castellan_core::{ActionKind, Language, PersonaMoment, UserPreferences, format_bytes, persona};
use castellan_engine::{VolumeSpace, inspect_volume};
use castellan_tui::{
    Attention, Drive, Facts, KeepWitness, Key, Memory, Model, Msg, Pulse, Session, SweepReading,
    SweepSize, Tab, pointer, update, view,
};
use clap::{Parser, Subcommand};
use serde::Serialize;

/// Process exit requested by a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Exit {
    /// Process status.
    pub code: i32,
}

/// Where command text goes. Tests capture it.
pub trait Io {
    /// Standard output.
    fn stdout(&mut self, text: &str);
    /// Standard error.
    fn stderr(&mut self, text: &str);
}

/// Run one invocation.
pub fn execute<I, T>(args: I, io: &mut T) -> Result<Exit, String>
where
    I: IntoIterator<Item = OsString>,
    T: Io,
{
    let cli = Cli::try_parse_from(args).map_err(|error| {
        // clap already formats a useful message. Keep the status it asked for.
        let _ = error;
        "unrecognized or incomplete command. Try `castellan --help`.".to_owned()
    })?;
    match cli.command {
        Some(Command::About) => {
            io.stdout(&about_text());
            Ok(Exit { code: 0 })
        }
        Some(Command::Doctor) => {
            io.stdout(&doctor_text());
            Ok(Exit { code: 0 })
        }
        Some(Command::Sweep(SweepCmd::Targets)) => {
            io.stdout(&targets_text());
            Ok(Exit { code: 0 })
        }
        Some(Command::Sweep(SweepCmd::Clean { dry_run })) => clean(dry_run, io),
        Some(Command::Status) | None => status(cli.json, io),
    }
}

fn clean<T: Io>(dry_run: bool, io: &mut T) -> Result<Exit, String> {
    // The live deleter is not connected. The gate stays closed in debug builds
    // because this command never passes the explicit real-system flag.
    let gate_open = castellan_guard::mutation_allowed(cfg!(debug_assertions), false, false);
    if dry_run {
        io.stdout(
            "Sweep clean dry-run. Nothing was deleted. The live deleter stays unwired in this build.",
        );
        return Ok(Exit { code: 0 });
    }
    if !gate_open {
        io.stdout(
            "Sweep clean did not delete anything. DEV BUILD — real-system actions disabled. The live deleter stays unwired in this build.",
        );
        return Ok(Exit { code: 20 });
    }
    io.stdout("Sweep clean did not delete anything. The live deleter stays unwired in this build.");
    Ok(Exit { code: 20 })
}

/// Open the terminal UI. The caller must be a real terminal; tests use [`view`] directly.
///
/// `q` leaves from every screen. A click on a numbered room jumps there. The wheel
/// moves between rooms. Mouse capture is turned off before the terminal is restored.
pub fn run_tui() -> Result<(), String> {
    let mut model = prepare_model();
    let readings = measure_sweep_in_background(model.preferences.min_age_secs);
    let mut terminal = ratatui::try_init().map_err(|error| error.to_string())?;
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
    let mut last = std::time::Instant::now();
    let mut pulse = PulseClock::start();
    let result = (|| -> Result<(), String> {
        loop {
            hold_env_overrides(&mut model);
            while let Ok(msg) = readings.try_recv() {
                update(&mut model, msg);
            }
            if let Some(reading) = pulse.poll() {
                update(&mut model, Msg::Pulse(reading));
            }
            terminal
                .draw(|frame| view(&model, frame))
                .map_err(|error| error.to_string())?;
            let waited = crossterm::event::poll(std::time::Duration::from_millis(model.pace_ms()))
                .map_err(|error| error.to_string())?;
            if waited {
                let event = crossterm::event::read().map_err(|error| error.to_string())?;
                dispatch_event(&mut model, &terminal, event);
            }
            let now = std::time::Instant::now();
            let elapsed = u32::try_from(now.saturating_duration_since(last).as_millis().min(1_000))
                .unwrap_or(1_000);
            last = now;
            update(&mut model, Msg::Tick { dt_ms: elapsed });
            update(&mut model, Msg::Clock(castellan_winapi::local_clock()));
            if model.prefs_dirty {
                store_preferences(&mut model);
            }
            if model.quit {
                break;
            }
        }
        Ok(())
    })();
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
    ratatui::restore();
    result
}

fn dispatch_event(
    model: &mut Model,
    terminal: &ratatui::DefaultTerminal,
    event: crossterm::event::Event,
) {
    match event {
        crossterm::event::Event::Key(key)
            if matches!(
                key.kind,
                crossterm::event::KeyEventKind::Press | crossterm::event::KeyEventKind::Repeat
            ) =>
        {
            if let Some(mapped) = map_key(key) {
                update(model, Msg::Key(mapped));
            }
        }
        crossterm::event::Event::Mouse(mouse) => on_mouse(model, terminal, mouse),
        _ => {}
    }
}

fn on_mouse(
    model: &mut Model,
    terminal: &ratatui::DefaultTerminal,
    mouse: crossterm::event::MouseEvent,
) {
    let size = terminal.size().unwrap_or(ratatui::layout::Size {
        width: 0,
        height: 0,
    });
    match mouse.kind {
        crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
        | crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Right) => {
            if let Some(msg) = pointer(model, size.width, size.height, mouse.column, mouse.row) {
                update(model, msg);
            }
        }
        crossterm::event::MouseEventKind::ScrollUp => scroll_room(model, Key::BackTab),
        crossterm::event::MouseEventKind::ScrollDown => scroll_room(model, Key::Tab),
        _ => {}
    }
}

fn scroll_room(model: &mut Model, key: Key) {
    if model.splash || model.help || model.palette || model.settings {
        return;
    }
    update(model, Msg::Key(key));
}

fn map_key(key: crossterm::event::KeyEvent) -> Option<Key> {
    let ctrl = key
        .modifiers
        .contains(crossterm::event::KeyModifiers::CONTROL);
    match key.code {
        crossterm::event::KeyCode::Char('c') if ctrl => Some(Key::Quit),
        crossterm::event::KeyCode::Char('q' | 'Q') if ctrl => Some(Key::Quit),
        crossterm::event::KeyCode::Char('p') if ctrl => Some(Key::Palette),
        crossterm::event::KeyCode::Char(_) if ctrl => None,
        crossterm::event::KeyCode::Esc => Some(Key::Esc),
        crossterm::event::KeyCode::Tab => Some(Key::Tab),
        crossterm::event::KeyCode::BackTab => Some(Key::BackTab),
        crossterm::event::KeyCode::Enter => Some(Key::Enter),
        crossterm::event::KeyCode::Backspace => Some(Key::Backspace),
        crossterm::event::KeyCode::Up => Some(Key::Up),
        crossterm::event::KeyCode::Down => Some(Key::Down),
        crossterm::event::KeyCode::Left => Some(Key::BackTab),
        crossterm::event::KeyCode::Right => Some(Key::Tab),
        crossterm::event::KeyCode::Char(' ') => Some(Key::Space),
        crossterm::event::KeyCode::Char('q' | 'Q') => Some(Key::Quit),
        crossterm::event::KeyCode::Char(ch) => Some(Key::Char(ch)),
        _ => None,
    }
}

/// Live readings, at most once a second. Every call it makes is read-only.
struct PulseClock {
    last: std::time::Instant,
    cpu: Option<castellan_winapi::CpuTimes>,
}

impl PulseClock {
    fn start() -> Self {
        Self {
            last: std::time::Instant::now(),
            cpu: castellan_winapi::cpu_times().ok(),
        }
    }

    fn poll(&mut self) -> Option<Pulse> {
        if self.last.elapsed() < std::time::Duration::from_millis(1_000) {
            return None;
        }
        self.last = std::time::Instant::now();
        let now = castellan_winapi::cpu_times().ok();
        let cpu_percent = match (self.cpu, now) {
            (Some(before), Some(after)) => castellan_winapi::cpu_busy_percent(before, after),
            _ => None,
        };
        self.cpu = now;
        Some(Pulse {
            cpu_percent,
            memory: read_memory(),
            uptime_secs: Some(castellan_winapi::uptime_secs()),
        })
    }
}

/// Upper bound on entries walked per folder, so a huge temp folder cannot stall the walk.
const SWEEP_MEASURE_LIMIT: u64 = 200_000;

/// Measure both temp folders on a worker thread. Nothing is opened for writing.
fn measure_sweep_in_background(min_age_secs: u64) -> std::sync::mpsc::Receiver<Msg> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let _ = std::thread::Builder::new()
        .name("castellan-sweep-measure".to_owned())
        .spawn(move || {
            let cutoff = std::time::SystemTime::now()
                .checked_sub(std::time::Duration::from_secs(min_age_secs))
                .unwrap_or(std::time::UNIX_EPOCH);
            let roots = [
                castellan_winapi::user_temp_dir().ok(),
                castellan_winapi::windows_directory()
                    .ok()
                    .map(|windows| windows.join("Temp")),
            ];
            for (index, root) in (0u8..).zip(roots) {
                let reading = match root {
                    Some(root) => measure_reading(&root, cutoff),
                    None => SweepReading::Unavailable(
                        "Windows did not say where this folder is, so it was not measured."
                            .to_owned(),
                    ),
                };
                if sender.send(Msg::Sweep(index, reading)).is_err() {
                    return;
                }
            }
        });
    receiver
}

fn measure_reading(root: &Path, cutoff: std::time::SystemTime) -> SweepReading {
    match castellan_guard::live::measure_no_follow(root, cutoff, SWEEP_MEASURE_LIMIT) {
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

fn read_memory() -> Option<Memory> {
    castellan_winapi::memory_status().ok().map(|memory| Memory {
        total_bytes: memory.total_bytes,
        available_bytes: memory.available_bytes,
    })
}

/// Read-only facts for the overview and Vitals.
fn read_facts() -> Facts {
    let system_root = castellan_winapi::windows_directory()
        .ok()
        .and_then(|windows| windows.ancestors().last().map(Path::to_path_buf));
    let mut drives = Vec::new();
    for root in castellan_winapi::fixed_drive_roots() {
        let Ok(space) = castellan_winapi::disk_free_space(&root) else {
            continue;
        };
        if space.total_bytes == 0 {
            continue;
        }
        let label = root.to_string_lossy().trim_end_matches('\\').to_owned();
        let system = system_root.as_ref().is_some_and(|system| {
            system
                .to_string_lossy()
                .eq_ignore_ascii_case(&root.to_string_lossy())
        });
        drives.push(Drive {
            label,
            total_bytes: space.total_bytes,
            free_bytes: space.free_bytes,
            system,
        });
    }
    drives.sort_by_key(|drive| !drive.system);
    Facts {
        drives,
        memory: read_memory(),
        uptime_secs: Some(castellan_winapi::uptime_secs()),
        logical_cpus: std::thread::available_parallelism()
            .ok()
            .and_then(|count| u32::try_from(count.get()).ok()),
        dev_build: cfg!(debug_assertions),
    }
}

fn prepare_model() -> Model {
    let mut session = open_session();
    let mut toast = None;
    let prefs = match preferences_path().and_then(|path| load_preferences(&path)) {
        Ok(prefs) => {
            session.ascii = std::env::var_os("CASTELLAN_ASCII").is_some() || prefs.ascii_art;
            session.reduce_motion =
                std::env::var_os("CASTELLAN_NO_ANIM").is_some() || prefs.reduce_motion;
            prefs
        }
        Err(error) => {
            toast = Some(format!("Preferences were not loaded. {error}"));
            UserPreferences::compiled_default()
        }
    };
    let mut model = Model::open(session);
    model.preferences = prefs;
    if let Some(text) = toast {
        model.toast = Some((text, 4_000));
    }
    model
}

fn hold_env_overrides(model: &mut Model) {
    if std::env::var_os("CASTELLAN_ASCII").is_some() {
        model.ascii = true;
    }
    if std::env::var_os("CASTELLAN_NO_ANIM").is_some() {
        model.reduce_motion = true;
    }
}

fn store_preferences(model: &mut Model) {
    let msg = match preferences_path() {
        Ok(path) => match save_preferences(&path, &model.preferences) {
            Ok(()) => Msg::PrefsStored,
            Err(error) => Msg::PrefsFailed(error),
        },
        Err(error) => Msg::PrefsFailed(error),
    };
    update(model, msg);
}

/// `{LOCALAPPDATA}\Castellan\preferences.toml`. Never a hardcoded drive.
fn preferences_path() -> Result<PathBuf, String> {
    let root =
        std::env::var_os("LOCALAPPDATA").ok_or_else(|| "LOCALAPPDATA is not set".to_owned())?;
    let root = PathBuf::from(root);
    if root.as_os_str().is_empty() || !root.is_absolute() {
        return Err("LOCALAPPDATA is not an absolute path".to_owned());
    }
    Ok(root.join("Castellan").join("preferences.toml"))
}

fn load_preferences(path: &Path) -> Result<UserPreferences, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => UserPreferences::from_toml(&text).map_err(|error| error.to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(UserPreferences::compiled_default())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn save_preferences(path: &Path, prefs: &UserPreferences) -> Result<(), String> {
    let text = prefs.to_toml().map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, text).map_err(|error| error.to_string())
}

fn open_session() -> Session {
    let (witness, persona) = match load_witness() {
        Ok(witness) => {
            let persona = witness.persona.clone();
            (Some(witness), persona)
        }
        Err(error) => (None, format!("The system drive could not be read. {error}")),
    };
    Session {
        persona,
        hostname: computer_name(),
        clock: castellan_winapi::local_clock(),
        witness,
        plain: std::env::var_os("NO_COLOR").is_some(),
        ascii: std::env::var_os("CASTELLAN_ASCII").is_some(),
        reduce_motion: std::env::var_os("CASTELLAN_NO_ANIM").is_some(),
        facts: read_facts(),
        sweep: Some([SweepReading::Measuring, SweepReading::Measuring]),
    }
}

fn computer_name() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .filter(|name| !name.is_empty() && name.len() <= 63 && !name.chars().any(char::is_control))
        .unwrap_or_else(|| "this keep".to_owned())
}

fn load_witness() -> Result<KeepWitness, String> {
    let windows = castellan_winapi::windows_directory().map_err(|error| error.to_string())?;
    let space = castellan_winapi::disk_free_space(&windows).map_err(|error| error.to_string())?;
    let label = windows
        .components()
        .next()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .unwrap_or_else(|| "volume".to_owned());
    let inspection = inspect_volume(&VolumeSpace {
        label,
        total_bytes: space.total_bytes,
        free_bytes: space.free_bytes,
    })
    .map_err(|error| error.to_string())?;
    let deducts = inspection
        .findings
        .iter()
        .any(|finding| finding.deducts_score());
    let persona_line = status_line(space.total_bytes, space.free_bytes, deducts);
    let detail = format!(
        "{} free of {}",
        format_bytes(space.free_bytes),
        format_bytes(space.total_bytes)
    );
    let mut attention = Vec::new();
    for finding in &inspection.findings {
        if !finding.deducts_score() {
            continue;
        }
        attention.push(Attention {
            id: finding.id().as_str().to_owned(),
            severity: finding.severity(),
            title: finding_title(finding.id().as_str()),
            detail: detail.clone(),
            advice: finding_advice(finding.id().as_str()),
            room: finding_room(finding.id().as_str()),
        });
    }
    Ok(KeepWitness {
        score: inspection.score,
        show_band: inspection.display_band.is_some(),
        attention,
        persona: persona_line,
    })
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

fn finding_room(id: &str) -> Option<Tab> {
    match id {
        "STORAGE-001" => Some(Tab::Sweep),
        _ => None,
    }
}

fn status<T: Io>(json: bool, io: &mut T) -> Result<Exit, String> {
    let windows = castellan_winapi::windows_directory().map_err(|error| error.to_string())?;
    let space = castellan_winapi::disk_free_space(&windows).map_err(|error| error.to_string())?;
    let label = windows
        .components()
        .next()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .unwrap_or_else(|| "volume".to_owned());
    let inspection = inspect_volume(&VolumeSpace {
        label,
        total_bytes: space.total_bytes,
        free_bytes: space.free_bytes,
    })
    .map_err(|error| error.to_string())?;
    if json {
        let payload = StatusJson {
            keep_score: inspection.score.keep_score,
            confidence_millis: inspection.score.confidence_millis,
            display_band: inspection.display_band.map(|band| band.label().to_owned()),
            storage_free_bytes: space.free_bytes,
            storage_total_bytes: space.total_bytes,
        };
        io.stdout(&serde_json::to_string(&payload).map_err(|error| error.to_string())?);
    } else {
        let deducts = inspection
            .findings
            .iter()
            .any(|finding| finding.deducts_score());
        let line = status_line(space.total_bytes, space.free_bytes, deducts);
        let score = inspection
            .score
            .keep_score
            .map(|value| value.to_string())
            .unwrap_or_else(|| "--".to_owned());
        io.stdout(&format!(
            "Storage free: {} of {} bytes. Partial Keep Score {score}. Confidence is low until the other pillars are inspected. {line}",
            space.free_bytes, space.total_bytes
        ));
    }
    let code = if inspection.findings.iter().any(|finding| {
        finding.severity() == castellan_core::Severity::Critical && finding.deducts_score()
    }) {
        11
    } else if inspection
        .findings
        .iter()
        .any(|finding| finding.deducts_score())
    {
        10
    } else {
        0
    };
    Ok(Exit { code })
}

fn about_text() -> String {
    format!(
        "Castellan {version}\nThe keeper of your machine.\nCrafted by Denner Parreiras · github.com/dennerparreiras\n{build}",
        version = env!("CARGO_PKG_VERSION"),
        build = if cfg!(debug_assertions) {
            "DEV BUILD — real-system actions disabled"
        } else {
            "release"
        }
    )
}

fn doctor_text() -> String {
    let mutations = if cfg!(debug_assertions) {
        "Mutations: debug build refuses real-system actions. The live deleter is not connected."
    } else {
        "Mutations: the live deleter is not connected, so sweep clean deletes nothing."
    };
    format!(
        "Castellan doctor\nScheduled task: not registered.\n{mutations}\n{}",
        config_line()
    )
}

fn config_line() -> String {
    match preferences_path() {
        Ok(path) if path.is_file() => {
            format!("Config: user preferences at {}", path.display())
        }
        Ok(_) => "Config: compiled defaults. No preferences file yet.".to_owned(),
        Err(_) => "Config: LOCALAPPDATA is not set, so no preferences file is used.".to_owned(),
    }
}

fn status_line(total_bytes: u64, free_bytes: u64, deducts: bool) -> String {
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

fn targets_text() -> String {
    format!(
        "{user}\n  Aged files in the user temporary directory. Risk: safe. Undo: not possible.\n{windows}\n  Aged files in Windows Temp. Risk: safe. Needs elevation. Undo: not possible.\nPrefetch is refused. Cleaning it slows the next startup.",
        user = ActionKind::SweepUserTemp.as_str(),
        windows = ActionKind::SweepWindowsTemp.as_str(),
    )
}

#[derive(Serialize)]
struct StatusJson {
    keep_score: Option<u8>,
    confidence_millis: u16,
    display_band: Option<String>,
    storage_free_bytes: u64,
    storage_total_bytes: u64,
}

#[derive(Parser)]
#[command(
    name = "castellan",
    version,
    about = "The keeper of your machine.",
    author = "Denner Parreiras <dennerparreiras@gmail.com>"
)]
struct Cli {
    /// Print machine-readable JSON. No persona.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Show the partial health summary.
    Status,
    /// Credits and version.
    About,
    /// Report what this build will and will not do. Does not change the system.
    Doctor,
    /// Cleanup commands.
    #[command(subcommand)]
    Sweep(SweepCmd),
}

#[derive(Subcommand)]
enum SweepCmd {
    /// List cleanup targets.
    Targets,
    /// Delete nothing. Debug builds always take this path for real roots.
    Clean {
        /// Show the refusal or the plan and do not delete.
        #[arg(long)]
        dry_run: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Buf {
        out: String,
        err: String,
    }

    impl Io for Buf {
        fn stdout(&mut self, text: &str) {
            self.out.push_str(text);
            self.out.push('\n');
        }
        fn stderr(&mut self, text: &str) {
            self.err.push_str(text);
        }
    }

    #[test]
    fn clean_in_a_debug_build_deletes_nothing() {
        let mut io = Buf::default();
        let exit =
            execute(["castellan", "sweep", "clean"].map(OsString::from), &mut io).expect("execute");
        assert_eq!(exit.code, 20);
        assert!(io.out.contains("did not delete"));
    }

    #[test]
    fn about_names_the_author() {
        let mut io = Buf::default();
        let exit = execute(["castellan", "about"].map(OsString::from), &mut io).expect("about");
        assert_eq!(exit.code, 0);
        assert!(io.out.contains("Denner Parreiras"));
    }

    #[test]
    fn status_json_is_structured() {
        let mut io = Buf::default();
        let exit = execute(
            ["castellan", "status", "--json"].map(OsString::from),
            &mut io,
        )
        .expect("status");
        assert!(exit.code == 0 || exit.code == 10 || exit.code == 11);
        let value: serde_json::Value = serde_json::from_str(io.out.trim()).expect("json");
        assert!(value.get("storage_total_bytes").is_some());
        assert!(value.get("confidence_millis").is_some());
    }

    #[test]
    fn a_full_volume_is_not_called_healthy() {
        let line = status_line(1000, 40, true);
        assert!(line.contains("96% full"));
        assert!(!line.contains("Nothing to fix"));
    }

    #[test]
    fn doctor_does_not_claim_a_task_or_a_deletion() {
        let mut io = Buf::default();
        let exit = execute(["castellan", "doctor"].map(OsString::from), &mut io).expect("doctor");
        assert_eq!(exit.code, 0);
        assert!(io.out.contains("Scheduled task: not registered"));
        assert!(io.out.contains("Config:"));
        assert!(!io.out.contains("was registered"));
    }

    /// Draws the keep from this PC's real, read-only readings. Run with
    /// `--nocapture` to see the frames.
    #[test]
    fn the_keep_draws_from_real_read_only_readings() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let mut session = open_session();
        session.reduce_motion = true;
        let mut model = Model::open(session);
        let cutoff = std::time::SystemTime::now() - std::time::Duration::from_secs(24 * 3_600);
        let temp = castellan_winapi::user_temp_dir().expect("temp");
        update(&mut model, Msg::Sweep(0, measure_reading(&temp, cutoff)));
        let windows_temp = castellan_winapi::windows_directory()
            .expect("windows")
            .join("Temp");
        update(
            &mut model,
            Msg::Sweep(1, measure_reading(&windows_temp, cutoff)),
        );
        let mut clock = PulseClock::start();
        clock.last -= std::time::Duration::from_secs(2);
        std::thread::sleep(std::time::Duration::from_millis(300));
        let pulse = clock.poll().expect("pulse");
        assert!(pulse.cpu_percent.is_some());
        update(&mut model, Msg::Pulse(pulse));
        assert!(!model.facts.drives.is_empty());
        assert!(model.facts.drives.iter().any(|drive| drive.system));
        for digit in [1u8, 2, 3] {
            update(&mut model, Msg::Key(Key::Digit(digit)));
            let mut terminal = Terminal::new(TestBackend::new(120, 36)).expect("terminal");
            terminal.draw(|frame| view(&model, frame)).expect("draw");
            let buffer = terminal.backend().buffer();
            let mut text = String::new();
            for y in 0..buffer.area.height {
                for x in 0..buffer.area.width {
                    text.push_str(buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" "));
                }
                text.push('\n');
            }
            assert!(text.contains("CASTELLAN"));
            println!("{text}");
        }
        assert!(temp.exists());
    }

    #[test]
    fn preferences_round_trip_in_a_temp_dir() {
        let dir = std::env::temp_dir().join(format!("castellan-prefs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("preferences.toml");
        let mut prefs = UserPreferences::compiled_default();
        prefs.user_temp_at_sign_in = true;
        prefs.reduce_motion = true;
        save_preferences(&path, &prefs).expect("save");
        let loaded = load_preferences(&path).expect("load");
        assert!(loaded.user_temp_at_sign_in);
        assert!(loaded.reduce_motion);
        assert!(!loaded.windows_temp_at_sign_in);
        let fresh = load_preferences(&dir.join("missing.toml")).expect("missing");
        assert!(!fresh.user_temp_at_sign_in);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
