//! Build script. Every command the frontend may call is listed here, so each one
//! gets its own `allow-*` permission and the capability file has to grant it by name.

fn main() {
    let manifest = tauri_build::AppManifest::new().commands(castellan_commands());
    let attributes = tauri_build::Attributes::new().app_manifest(manifest);
    if let Err(error) = tauri_build::try_build(attributes) {
        println!("cargo::error=tauri build step failed: {error}");
    }
}

/// Keep in sync with `invoke_handler` in `src/lib.rs`. The bindings test checks both.
const fn castellan_commands() -> &'static [&'static str] {
    &["contract_version", "about", "overview", "facts"]
}
