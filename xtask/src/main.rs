//! `cargo xtask ci` runs the local gate: format, clippy, tests, and the architecture check.

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
        Some("architecture") => {
            if architecture().is_err() {
                ExitCode::from(1)
            } else {
                ExitCode::from(0)
            }
        }
        _ => {
            eprintln!("usage: cargo xtask ci");
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

fn architecture() -> Result<(), ()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut failures = Vec::new();
    walk(&root, &mut failures);
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
        if name == "target" || name.starts_with('.') {
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
}
