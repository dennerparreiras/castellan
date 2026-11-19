# 0002. Architecture

- Status: accepted
- Date: 2026-11-19

## Context

The product plan splits Castellan into crates so the safety kernel, the Win32 boundary, and the UI can change on different schedules. This repository starts with the crates that have real code. Empty crates for later milestones were not created.

## Decision

| Crate | Owns | Must not |
| --- | --- | --- |
| `castellan-core` | Types, the closed `Action` enum, policy, Keep Score, persona text | I/O, `unsafe` |
| `castellan-winapi` | Win32 calls behind a small API | Business policy |
| `castellan-guard` | Allow-root, deny-list, per-node decision, sweep walker | Shell execution |
| `castellan-store` | Append-only ledger | Deciding what may be deleted |
| `castellan-engine` | Measurements in, findings out | Deleting files, opening sockets |
| `castellan-tui` | TEA update and view | Mutations |
| `castellan-cli` | Argument parsing and process exit | Policy that disagrees with `guard` |
| `xtask` | The local CI gate and the architecture scan | Product behavior |

The architecture scan walks Rust sources and rejects `unsafe` outside `castellan-winapi`, the delete and DNS-flush symbols outside `castellan-guard` and `castellan-winapi`, and `Win32_Product`, `cmd /c`, and `cmd.exe` anywhere.

Paths come from Win32 (`GetTempPath2W`, `GetSystemWindowsDirectoryW`, `GetSystemDirectoryW`). The code does not assume the system drive is `C:`.

## Consequences

Later modules (broker, MCP, probes beyond volume free space) are new crates or new modules behind the same contracts. They do not get a back door around `castellan-guard`.

The public contracts in `castellan-core` are owned by the orchestrator. Other work treats them as frozen unless an ADR changes them.
