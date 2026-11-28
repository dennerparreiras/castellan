# 0011. Installer and WebView2

- Status: proposed (decided in G11)
- Date: 2026-11-28

## Context

The desktop app needs WebView2. Windows 11 ships it; Windows 10 may not have it. The plan wants one installer for the GUI, the CLI, and the scheduled task, plus winget, Scoop, and a PowerShell one-liner.

## Decision

Bundling stays off in G0 (`bundle.active = false`). G11 chooses between the MSI (WiX) and NSIS targets of the Tauri bundler, and between the WebView2 bootstrapper (small, downloads the runtime with consent) and the offline installer (large, no network). The choice is recorded here with installer sizes and install times measured on a clean Windows 10 and Windows 11 VM.

Constraints already fixed: per-machine installs go to `%ProgramFiles%\Castellan` with administrator-only write access; per-user installs go to `%LOCALAPPDATA%\Programs\Castellan` and never register an elevated task; nothing installs silently from the app itself.

## Consequences

Until G11 the desktop app runs from `cargo` or `pnpm tauri dev`, and nothing is installed on any machine.
