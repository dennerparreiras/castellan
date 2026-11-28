//! `cargo xtask ci` runs the local gate: format, clippy, tests, the architecture
//! check, and the desktop frontend gate. `cargo xtask ui` runs only the frontend.

use std::path::Path;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("ci") => {
            if ci().is_err() {
                ExitCode::from(1)
            } else {
                ExitCode::from(0)
            }
        }
        Some("ui") => {
            if ui().is_err() {
                ExitCode::from(1)
            } else {
                ExitCode::from(0)
            }
        }
        Some("architecture") => {
            if architecture().is_err() {
                ExitCode::from(1)
            } else {
                ExitCode::from(0)
            }
        }
        _ => {
            eprintln!("usage: cargo xtask <ci|ui|architecture>");
            ExitCode::from(2)
        }
    }
}

fn ci() -> Result<(), ()> {
    cargo(&["fmt", "--all", "--check"])?;
    cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ])?;
    cargo(&["test", "--workspace"])?;
    architecture()?;
    ui()?;
    Ok(())
}

/// The desktop frontend gate: format, lint, types, tests, build.
///
/// Uses the Node version pinned in `apps/desktop/.npmrc`, which pnpm fetches into
/// its own store. The system Node is not used or changed.
fn ui() -> Result<(), ()> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("apps")
        .join("desktop");
    let pnpm = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };
    for args in [&["install", "--frozen-lockfile"][..], &["check"][..]] {
        let status = Command::new(pnpm)
            .args(args)
            .current_dir(&dir)
            .status()
            .map_err(|error| {
                eprintln!(
                    "failed to start pnpm ({error}). Install pnpm 10 to build the desktop frontend."
                );
            })?;
        if !status.success() {
            eprintln!("pnpm {} failed in apps/desktop", args.join(" "));
            return Err(());
        }
    }
    Ok(())
}

fn cargo(args: &[&str]) -> Result<(), ()> {
    let status = Command::new("cargo").args(args).status().map_err(|error| {
        eprintln!("failed to start cargo: {error}");
    })?;
    if status.success() {
        Ok(())
    } else {
        eprintln!("cargo {} failed", args.join(" "));
        Err(())
    }
}

fn spell(parts: &[&str]) -> String {
    let mut out = String::new();
    for part in parts {
        out.push_str(part);
    }
    out
}

/// Crates a frontend may depend on. Everything else in the workspace is reached
/// through `castellan-service`.
const FRONTENDS: [(&str, &[&str]); 3] = [
    (
        "crates/castellan-cli",
        &["castellan-core", "castellan-service", "castellan-tui"],
    ),
    ("crates/castellan-tui", &["castellan-core"]),
    (
        "apps/desktop/src-tauri",
        &["castellan-core", "castellan-service"],
    ),
];

/// Workspace crates named in the `[dependencies]` table of a manifest.
fn workspace_dependencies(manifest: &str) -> Vec<String> {
    let mut in_deps = false;
    let mut names = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line == "[dependencies]";
            continue;
        }
        if !in_deps {
            continue;
        }
        let name = line.split(['=', '.', ' ']).next().unwrap_or_default();
        if name.starts_with("castellan-") {
            names.push(name.to_owned());
        }
    }
    names
}

fn frontend_boundaries(root: &Path, failures: &mut Vec<String>) {
    for (dir, allowed) in FRONTENDS {
        let path = root.join(dir).join("Cargo.toml");
        let Ok(manifest) = std::fs::read_to_string(&path) else {
            failures.push(format!("{dir}/Cargo.toml could not be read"));
            continue;
        };
        for name in workspace_dependencies(&manifest) {
            if !allowed.contains(&name.as_str()) {
                failures.push(format!(
                    "{dir} depends on {name}; frontends reach it through castellan-service"
                ));
            }
        }
    }
}

fn architecture() -> Result<(), ()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut failures = Vec::new();
    walk(&root, &mut failures);
    frontend_boundaries(&root, &mut failures);
    if failures.is_empty() {
        Ok(())
    } else {
        for failure in &failures {
            eprintln!("architecture: {failure}");
        }
        Err(())
    }
}

fn walk(dir: &Path, failures: &mut Vec<String>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if matches!(name.as_ref(), "target" | "node_modules" | "dist" | "gen")
            || name.starts_with('.')
        {
            continue;
        }
        if path.is_dir() {
            walk(&path, failures);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let path_text = path.display().to_string().replace('\\', "/");
        let in_winapi = path_text.contains("/castellan-winapi/");
        let in_guard = path_text.contains("/castellan-guard/");
        // Needles are assembled so this file does not contain the spellings it searches for.
        let unsafe_token = spell(&["uns", "afe "]);
        let delete_token = spell(&["guard_delete_", "opened"]);
        let flush_token = spell(&["guard_flush_", "dns"]);
        let wmi_token = spell(&["Win32_", "Product"]);
        let shell_token = spell(&["cmd /", "c"]);
        let shell_exe = spell(&["cmd.", "exe"]);
        if !in_winapi && text.contains(&unsafe_token) {
            failures.push(format!(
                "{path_text} contains {unsafe_token}outside castellan-winapi"
            ));
        }
        if !in_guard && !in_winapi && text.contains(&delete_token) {
            failures.push(format!(
                "{path_text} names {delete_token} outside the safety kernel"
            ));
        }
        if !in_guard && !in_winapi && text.contains(&flush_token) {
            failures.push(format!(
                "{path_text} names {flush_token} outside the safety kernel"
            ));
        }
        if text.contains(&wmi_token) || text.contains(&shell_token) || text.contains(&shell_exe) {
            failures.push(format!(
                "{path_text} contains a forbidden execution pattern"
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn architecture_check_passes_on_this_tree() {
        architecture().expect("architecture");
    }

    #[test]
    fn a_frontend_reaching_past_the_service_is_caught() {
        let manifest = "[package]\nname = \"x\"\n\n[dependencies]\ncastellan-core.workspace = true\ncastellan-winapi = { path = \"../w\" }\nserde = \"1\"\n\n[dev-dependencies]\ncastellan-guard.workspace = true\n";
        assert_eq!(
            workspace_dependencies(manifest),
            vec!["castellan-core".to_owned(), "castellan-winapi".to_owned()]
        );
    }
}
