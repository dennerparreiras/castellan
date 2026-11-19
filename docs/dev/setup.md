# Setup

Install Rust 1.100.0 with rustup. The repository pin in `rust-toolchain.toml` selects it, including rustfmt and clippy.

```powershell
Set-Location <path-to>\castellan
cargo xtask ci
```

Run Cargo from the `castellan` directory. The parent folder is not a workspace.

Do not install Castellan onto the system, do not register a scheduled task, and do not set `CASTELLAN_ALLOW_REAL_SYSTEM` while developing.
