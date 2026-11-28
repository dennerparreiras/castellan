# 0008. One facade for every frontend: castellan-service

- Status: accepted
- Date: 2026-11-28

## Context

Before G0, the CLI read the machine through `castellan-winapi`, called the engine, built the overview, and talked to the guard. A second frontend (the desktop app) and later the MCP server would each have repeated that, and each copy could drift or skip a safety check.

## Decision

- `castellan-sense` holds the read-only collectors: facts, live readings, the Windows volume, and sweep measurements. It never writes.
- `castellan-service` is the facade. It exposes `Castellan` with the overview, facts, a pulse clock, sweep measurements, preferences, and the real-change gate. Later milestones add planning and approval here, and nowhere else.
- The shared data shapes live in `castellan_core::live` (plain data, no I/O), so every frontend draws the same types.
- Frontends depend only on the service and core. The CLI may also use the TUI. `cargo xtask architecture` fails the build when a frontend names any other workspace crate in `[dependencies]`.
- Calls are blocking in G0. The desktop app runs them on Tauri's blocking pool. An async API can come later if a module needs it; that is an implementation detail behind the same facade.

## Consequences

The CLI and TUI behave as before; their tests moved with the code. Adding a frontend means adding a client of `Castellan`, not a new path to the machine. The overview's wording (titles, advice, the room that helps) is decided once, in the service.
