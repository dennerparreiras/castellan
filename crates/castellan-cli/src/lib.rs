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

use castellan_core::{ActionKind, Language, PersonaMoment, persona};
use castellan_engine::{VolumeSpace, inspect_volume};
use castellan_tui::{Key, Model, Msg, update, view};
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
pub fn run_tui() -> Result<(), String> {
    let mut model = Model::fresh(persona(Language::En, PersonaMoment::DevBuild));
    let mut terminal = ratatui::try_init().map_err(|error| error.to_string())?;
    let result = (|| -> Result<(), String> {
        loop {
            terminal
                .draw(|frame| view(&model, frame))
                .map_err(|error| error.to_string())?;
            if crossterm::event::poll(std::time::Duration::from_millis(200))
                .map_err(|error| error.to_string())?
                && let crossterm::event::Event::Key(key) =
                    crossterm::event::read().map_err(|error| error.to_string())?
                && key.kind == crossterm::event::KeyEventKind::Press
            {
                update(&mut model, Msg::Key(map_key(key.code)));
            }
            if model.quit {
                break;
            }
        }
        Ok(())
    })();
    ratatui::restore();
    result
}

fn map_key(code: crossterm::event::KeyCode) -> Key {
    match code {
        crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Char('Q') => Key::Quit,
        crossterm::event::KeyCode::Tab => Key::Tab,
        crossterm::event::KeyCode::Char('?') => Key::Help,
        crossterm::event::KeyCode::Esc => Key::Esc,
        crossterm::event::KeyCode::Char('1') => Key::Digit(1),
        crossterm::event::KeyCode::Char('2') => Key::Digit(2),
        crossterm::event::KeyCode::Char('3') => Key::Digit(3),
        _ => Key::Digit(0),
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
        let line = persona(Language::En, PersonaMoment::WallsHold);
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
    "Castellan doctor\nInstall mode: development tree (not installed).\nScheduled task: not registered.\nReal mutations: disabled in debug builds.\nConfig: defaults, no policy file required yet."
        .to_owned()
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
    /// Check this development tree. Does not change the system.
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
}
