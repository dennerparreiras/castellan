//! Castellan Desktop: the Tauri shell around [`castellan_service`].
//!
//! This crate only translates between the service and the window. It holds no
//! business rules, never builds an action, and never reaches the safety kernel
//! or Win32 directly. The frontend is treated as untrusted: every command takes
//! closed types, and nothing here mutates the system.

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

pub mod ipc;

use std::time::Duration;

use castellan_service::Castellan;
use tauri::{Emitter, Manager};

/// Start the desktop app. Returns when the last window closes.
pub fn run() -> Result<(), String> {
    castellan_service::harden_process();
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // A second launch focuses the window that is already open.
            if let Some(window) = app.get_webview_window(ipc::MAIN_WINDOW) {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .manage(Castellan::open())
        .invoke_handler(tauri::generate_handler![
            ipc::contract_version,
            ipc::about,
            ipc::overview,
            ipc::facts,
        ])
        .on_page_load(|webview, _payload| {
            if webview.label() == ipc::MAIN_WINDOW {
                start_streams(webview.app_handle().clone());
            }
        })
        .run(tauri::generate_context!())
        .map_err(|error| error.to_string())
}

/// Live readings and sweep measurements, pushed to the window as events.
///
/// Started once per page load. A reload starts a fresh pair of workers; the old
/// ones stop when their next emit finds no listener worth keeping, which is
/// harmless because emits never fail loudly and the workers only read.
fn start_streams(app: tauri::AppHandle) {
    let service = app.state::<Castellan>().inner().clone();
    let min_age = service
        .load_preferences()
        .map(|prefs| prefs.min_age_secs)
        .unwrap_or(castellan_core::policy::DEFAULT_MIN_AGE_SECS);

    let sweep_app = app.clone();
    let readings = service.measure_sweep(min_age);
    let _ = std::thread::Builder::new()
        .name("castellan-desktop-sweep".to_owned())
        .spawn(move || {
            for (target, reading) in readings {
                let _ = sweep_app.emit(ipc::EVENT_SWEEP, ipc::SweepEvent { target, reading });
            }
        });

    let _ = std::thread::Builder::new()
        .name("castellan-desktop-pulse".to_owned())
        .spawn(move || {
            let mut clock = service.pulse();
            loop {
                std::thread::sleep(Duration::from_millis(1_000));
                let pulse = clock.read_now();
                if app.emit(ipc::EVENT_PULSE, pulse).is_err() {
                    return;
                }
            }
        });
}
