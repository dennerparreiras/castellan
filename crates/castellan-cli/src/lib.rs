//! Command dispatch for the `castellan` binary.
//!
//! The CLI is a thin client of [`castellan_service`]. It never reaches the
//! safety kernel or the Win32 wrappers directly. Debug builds refuse real
//! changes, and the test suite never asks for them.

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

use castellan_core::{ActionKind, UserPreferences};
use castellan_service::{Castellan, exit_code};
use castellan_tui::{KeepWitness, Key, Model, Msg, Session, SweepReading, pointer, update, view};
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
    let service = Castellan::open();
    match cli.command {
        Some(Command::About) => {
            io.stdout(&about_text());
            Ok(Exit { code: 0 })
        }
        Some(Command::Doctor) => {
            io.stdout(&doctor_text(&service));
            Ok(Exit { code: 0 })
        }
        Some(Command::Sweep(SweepCmd::Targets)) => {
            io.stdout(&targets_text());
            Ok(Exit { code: 0 })
        }
        Some(Command::Sweep(SweepCmd::Clean { dry_run })) => clean(&service, dry_run, io),
        Some(Command::Status) | None => status(&service, cli.json, io),
    }
}

fn clean<T: Io>(service: &Castellan, dry_run: bool, io: &mut T) -> Result<Exit, String> {
    // The live deleter is not connected. The gate stays closed in debug builds
    // because this command never passes the explicit real-system flag.
    if dry_run {
        io.stdout(
            "Sweep clean dry-run. Nothing was deleted. The live deleter stays unwired in this build.",
        );
        return Ok(Exit { code: 0 });
    }
    if !service.real_changes_allowed() {
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
    let service = Castellan::open();
    let mut model = prepare_model(&service);
    let readings = service.measure_sweep(model.preferences.min_age_secs);
    let mut terminal = ratatui::try_init().map_err(|error| error.to_string())?;
    let _ = crossterm::execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
    let mut last = std::time::Instant::now();
    let mut pulse = service.pulse();
    let result = (|| -> Result<(), String> {
        loop {
            hold_env_overrides(&mut model);
            while let Ok((target, reading)) = readings.try_recv() {
                update(&mut model, Msg::Sweep(target.index(), reading));
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
            update(&mut model, Msg::Clock(service.clock()));
            if model.prefs_dirty {
                store_preferences(&service, &mut model);
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

fn prepare_model(service: &Castellan) -> Model {
    let mut session = open_session(service);
    let mut toast = None;
    let prefs = match service.load_preferences() {
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

fn store_preferences(service: &Castellan, model: &mut Model) {
    let msg = match service.save_preferences(&model.preferences) {
        Ok(()) => Msg::PrefsStored,
        Err(error) => Msg::PrefsFailed(error.to_string()),
    };
    update(model, msg);
}

fn open_session(service: &Castellan) -> Session {
    let (witness, persona) = match service.overview() {
        Ok(overview) => {
            let witness = KeepWitness::from_overview(&overview);
            let persona = witness.persona.clone();
            (Some(witness), persona)
        }
        Err(error) => (None, format!("The system drive could not be read. {error}")),
    };
    Session {
        persona,
        hostname: service.computer_name(),
        clock: service.clock(),
        witness,
        plain: std::env::var_os("NO_COLOR").is_some(),
        ascii: std::env::var_os("CASTELLAN_ASCII").is_some(),
        reduce_motion: std::env::var_os("CASTELLAN_NO_ANIM").is_some(),
        facts: service.facts(),
        sweep: Some([SweepReading::Measuring, SweepReading::Measuring]),
    }
}

fn status<T: Io>(service: &Castellan, json: bool, io: &mut T) -> Result<Exit, String> {
    let overview = service.overview().map_err(|error| error.to_string())?;
    let volume = &overview.system_volume;
    if json {
        let payload = StatusJson {
            keep_score: overview.score.keep_score,
            confidence_millis: overview.score.confidence_millis,
            display_band: overview.display_band.map(|band| band.label().to_owned()),
            storage_free_bytes: volume.free_bytes,
            storage_total_bytes: volume.total_bytes,
        };
        io.stdout(&serde_json::to_string(&payload).map_err(|error| error.to_string())?);
    } else {
        let score = overview
            .score
            .keep_score
            .map(|value| value.to_string())
            .unwrap_or_else(|| "--".to_owned());
        io.stdout(&format!(
            "Storage free: {} of {} bytes. Partial Keep Score {score}. Confidence is low until the other pillars are inspected. {}",
            volume.free_bytes, volume.total_bytes, overview.persona
        ));
    }
    Ok(Exit {
        code: exit_code(&overview),
    })
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

fn doctor_text(service: &Castellan) -> String {
    let mutations = if cfg!(debug_assertions) {
        "Mutations: debug build refuses real-system actions. The live deleter is not connected."
    } else {
        "Mutations: the live deleter is not connected, so sweep clean deletes nothing."
    };
    format!(
        "Castellan doctor\nScheduled task: not registered.\n{mutations}\n{}",
        config_line(service)
    )
}

fn config_line(service: &Castellan) -> String {
    match service.preferences_path() {
        Ok(path) if path.is_file() => {
            format!("Config: user preferences at {}", path.display())
        }
        Ok(_) => "Config: compiled defaults. No preferences file yet.".to_owned(),
        Err(_) => "Config: LOCALAPPDATA is not set, so no preferences file is used.".to_owned(),
    }
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
    fn doctor_does_not_claim_a_task_or_a_deletion() {
        let mut io = Buf::default();
        let exit = execute(["castellan", "doctor"].map(OsString::from), &mut io).expect("doctor");
        assert_eq!(exit.code, 0);
        assert!(io.out.contains("Scheduled task: not registered"));
        assert!(io.out.contains("Config:"));
        assert!(!io.out.contains("was registered"));
    }

    /// Draws the keep from this PC's real, read-only readings through the service.
    #[test]
    fn the_keep_draws_from_real_read_only_readings() {
        use ratatui::Terminal;
        use ratatui::backend::TestBackend;

        let service = Castellan::open();
        let mut session = open_session(&service);
        session.reduce_motion = true;
        let mut model = Model::open(session);
        for (target, reading) in service.measure_sweep(24 * 3_600) {
            update(&mut model, Msg::Sweep(target.index(), reading));
        }
        let mut clock = service.pulse();
        std::thread::sleep(std::time::Duration::from_millis(150));
        update(&mut model, Msg::Pulse(clock.read_now()));
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
        }
    }
}
