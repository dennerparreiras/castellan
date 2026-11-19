# Agents working on Castellan

This file is the short guide for coding agents. Humans should read [`docs/dev/setup.md`](docs/dev/setup.md) too.

## Workspace

The git root is this directory (`castellan/`), not the parent folder. Run Cargo from here. The parent folder is not a Cargo workspace and must not become one.

```powershell
cargo xtask ci
```

That is the local gate: format, clippy (`-D warnings`), tests, architecture scan. Do not claim it passed unless you saw the success output.

## The machine you are on

The development PC belongs to the maintainer. Profile data may live on a drive other than `C:`.

- Do not delete, flush, repair, install, register a scheduled task, or write the registry on the real system.
- Do not set `CASTELLAN_ALLOW_REAL_SYSTEM`.
- Tests that need elevation, a service restart, or a reboot belong in CI, not on this PC.
- Mutable operations take an injectable root. Tests use a temp directory.
- Do not copy the maintainer's personal scripts into the repo. Do not commit secrets, minisign keys, or real machine dumps.

## Code rules

- Product language is English. pt-BR is a translation, not the source language.
- `unsafe` exists only in `castellan-winapi`, and each block has a `// SAFETY:` comment.
- No `cmd /c`, no `cmd.exe`, no `Win32_Product`, no plugin or "run this string" API.
- Do not follow reparse points while deleting. Do not disable Defender, firewall, UAC, SmartScreen, VBS/HVCI, or Windows Update.
- Do not ship `todo!()` or `unimplemented!()` for a feature the docs call done.
- `publish = false` stays until the maintainer says otherwise. Do not publish to crates.io or winget.
- Missing product decisions go in `docs/adr/`. Owner-only blockers go in `docs/dev/STATUS.md`. Do not stop the rest of the work for those.

## Crates

| Crate | Role |
| --- | --- |
| `castellan-core` | Types, actions, policy, score. No I/O. |
| `castellan-winapi` | The only `unsafe` crate. |
| `castellan-guard` | Path rules and the sweep decision. |
| `castellan-store` | Append-only ledger. |
| `castellan-engine` | Measurements to findings. |
| `castellan-tui` | Terminal UI state and drawing. |
| `castellan-cli` | Binary `castellan`. |
| `xtask` | `cargo xtask ci`. |

One owner per crate when several people work at once. The orchestrator owns `castellan-core` and the merge to `main`.
