//! `castellan` binary. Business rules live in the library crates.

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
use std::io::{self, Write};
use std::process::ExitCode;

use castellan_cli::{Io, execute};

fn main() -> ExitCode {
    let _ = castellan_winapi::lock_dll_search();
    if cfg!(debug_assertions) {
        let _ = writeln!(io::stderr(), "DEV BUILD — real-system actions disabled");
    }
    let args: Vec<OsString> = std::env::args_os().collect();
    if args.len() == 1 && io::IsTerminal::is_terminal(&io::stdout()) {
        if let Err(error) = castellan_cli::run_tui() {
            let _ = writeln!(io::stderr(), "castellan: {error}");
            return ExitCode::from(1);
        }
        return ExitCode::from(0);
    }
    let code = match execute(args, &mut Stdio) {
        Ok(exit) => exit.code,
        Err(error) => {
            let _ = writeln!(io::stderr(), "castellan: {error}");
            1
        }
    };
    ExitCode::from(u8::try_from(code).unwrap_or(1))
}

struct Stdio;

impl Io for Stdio {
    fn stdout(&mut self, text: &str) {
        let _ = writeln!(io::stdout(), "{text}");
    }

    fn stderr(&mut self, text: &str) {
        let _ = writeln!(io::stderr(), "{text}");
    }
}
