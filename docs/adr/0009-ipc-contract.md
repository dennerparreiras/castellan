# 0009. IPC contract and generated TypeScript types

- Status: accepted
- Date: 2026-11-28

## Context

The desktop frontend talks to Rust over Tauri IPC. Hand-written TypeScript types drift from the Rust types silently. The plan suggests `tauri-specta`; at the time of G0 it is still a release candidate (2.0.0-rc.29).

## Decision

- Types are generated with **ts-rs 12** (stable). `castellan-core` derives `ts_rs::TS` behind a `ts` feature, so the core has no TypeScript dependency in normal builds.
- A unit test in `castellan-desktop` (`ipc::bindings`) writes every IPC type into `apps/desktop/src/ipc/bindings.ts` and fails when the committed file is stale. Refresh with `CASTELLAN_UPDATE_BINDINGS=1 cargo test -p castellan-desktop bindings`.
- 64-bit integers are exported as `number`. Byte counts stay exact in JSON up to 8 PiB.
- JSON field names are camelCase for every IPC type, the score types included.
- `CONTRACT_VERSION` (1) is exported to TypeScript. The window checks it first and refuses to run against a different backend.
- Commands in v1: `contract_version`, `about`, `overview`, `facts`. All read-only, none take arguments. Events: `castellan:pulse` (about once a second) and `castellan:sweep` (each temp folder as it finishes measuring).
- Every command is listed in `build.rs`, which generates an `allow-<command>` permission for it. The capability file grants each one by name. A test checks that `build.rs`, `invoke_handler`, and the capability list agree.
- Errors cross the boundary as `{ code, message }` with a stable `CST-E…` code.

## Consequences

A Rust type change breaks the bindings test until the TypeScript file is regenerated, so the frontend compiler sees the change. Commands that change the system (G3) will take only a plan id and a one-time nonce issued by the service.
