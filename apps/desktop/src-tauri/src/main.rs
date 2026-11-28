//! `Castellan.exe`, the desktop app.

// Release builds are a GUI program and open no console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::ExitCode;

fn main() -> ExitCode {
    match castellan_desktop::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("castellan desktop: {error}");
            ExitCode::FAILURE
        }
    }
}
