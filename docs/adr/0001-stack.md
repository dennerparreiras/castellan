# 0001. Stack

- Status: accepted
- Date: 2026-11-19

## Context

Castellan has to be one native Windows binary, with a terminal UI, a CLI, and a later MCP server on stdio. The plan fixes Rust as the language and names the libraries. Versions were confirmed against crates.io on 2026-11-19 rather than taken from memory.

## Decision

- Rust 1.100.0, edition 2024, pinned in `rust-toolchain.toml` with rustfmt and clippy. Host target for development: `x86_64-pc-windows-msvc`.
- Workspace resolver 3. Every package has `publish = false` until the maintainer asks to publish.
- Libraries, exact versions locked in `Cargo.lock`:
  - `thiserror` 2.0.21
  - `serde` 1.0.229, `serde_json` 1.0.154
  - `proptest` 1.12.0
  - `clap` 4.6.8 (derive)
  - `ratatui` 0.30.2, `crossterm` 0.29.0
  - `rusqlite` 0.43.0 with `bundled`
  - `toml` 1.2.1
  - `blake3` 1.8.7
  - `windows` 0.62.2
  - `assert_cmd` 2.2.2 and `assert_fs` 1.1.4 as dev dependencies
- Release profile: opt-level 3, fat LTO, one codegen unit, `panic = abort`, symbols stripped. A `release-with-debug` profile keeps symbols.
- Every MSVC Windows build, including tests, uses `+crt-static` and Control Flow Guard (`.cargo/config.toml`).
- `unsafe_code` is forbidden in the workspace. `castellan-winapi` allows it and does not inherit the workspace forbid, because Cargo cannot override a workspace lint.
- The binary crate is `castellan-cli`. The binary name is `castellan`. See ADR 0005.
- MIT license, copyright 2026 Denner Parreiras.

Not adopted yet, because nothing in this slice calls them: `clap_complete`, `jiff`, `tracing`. They can return when a real caller exists. Adding them now only to match an early list would trip unused-dependency warnings.

## Consequences

The MSRV is 1.100.0. CI installs that toolchain on purpose. `set_var` is unsafe in edition 2024, so tests inject flags instead of mutating the process environment.
