# Setup

Install Rust 1.100.0 with rustup. The repository pin in `rust-toolchain.toml` selects it, including rustfmt and clippy.

```powershell
Set-Location <path-to>\castellan
cargo xtask ci
```

Run Cargo from the `castellan` directory. The parent folder is not a workspace.

The desktop frontend needs pnpm 10. Enable it with `corepack enable` (pnpm's version comes from `packageManager` in `apps/desktop/package.json`). pnpm then downloads the Node version pinned in `apps/desktop/.npmrc` into its own store; your system Node is not used. `cargo xtask ci` runs the frontend gate too. To open the desktop window while developing:

```powershell
Set-Location <path-to>\castellan\apps\desktop
pnpm install
pnpm tauri dev
```

Do not install Castellan onto the system, do not register a scheduled task, and do not set `CASTELLAN_ALLOW_REAL_SYSTEM` while developing.
