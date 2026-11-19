# Status

Updated 2026-11-19. Product version 0.1.0. This is not v1.0.

## Milestone

| Milestone | State |
| --- | --- |
| M0 Foundation | Local gate is green. `cargo xtask ci` exited 0 on 2026-11-19 (fmt, clippy `-D warnings`, workspace tests, architecture scan). The public repository is https://github.com/dennerparreiras/castellan. Branch protection requires a pull request, linear history, and the CI checks, and it blocks force-pushes. A second reviewer is not required, because there is only one maintainer. |
| M1 Contracts | Partial. Core types, the action enum, policy, and the Keep Score have tests, including the golden score of 83. Coverage is not measured at 95%. Platform fakes beyond the in-memory sweep view are not built. |
| M2 Guard and store | Partial. Path rules, `decide_node`, the in-memory walker, the hash-chain ledger, and a read-only live scan exist. Handle-based live delete, undo, and mutation testing are not done. `cargo-mutants` has not been run. |
| M3 TUI | Partial. One theme, three tabs, help, a small-terminal message, and tests that the overview does not show a fake score. No animation engine, no snapshots, no GIF. |
| M4–M9 | Not started. No vitals, net, shield, boost, game mode, fix playbooks, broker, scheduled task, MCP server, installer, winget, Scoop bucket, or signed release. |

## What a person can run

- `castellan status` and `status --json`: free space on the Windows directory, partial score, no display band.
- `castellan about`, `castellan doctor`.
- `castellan sweep targets`.
- `castellan sweep clean`: always refuses. Exit 0 with `--dry-run`, otherwise 20.
- No arguments on a terminal: the small TUI. Pipes and tests take the CLI path.

## Owner-only blockers

These wait for Denner Parreiras. The rest of the work does not.

| Blocker | Recommendation |
| --- | --- |
| Authenticode certificate (SignPath or equivalent) | Do not sign until a certificate exists. Document verification as "not signed yet". |
| minisign key | Do not generate a private key in the repo or on the agent's behalf. Generate it on the maintainer's machine when the first release is cut. |
| crates.io publish of `castellan-cli` | Leave `publish = false`. The name `castellan` is someone else's crate. |
| winget-pkgs submission | Prepare the manifest in a later milestone and ask before opening the PR. |
| Scoop bucket repository | Ask before creating `dennerparreiras/scoop-bucket`. |
| Real-machine validation of a live sweep | Only after the handle-based deleter is reviewed, and only with an explicit request. Not during development. |
| MCP client validation | After the stdio server exists, sit with one real client. |

## Reflection (plan section 1.2)

1. **Correctness.** The plan asked for a public v1.0. This tree is the foundation and the safety-critical core of that product, with the UI and CLI telling the truth about what is missing. Shipping empty modules for M4–M9 would have violated the "do not pretend" rule. Prefetch is hard-denied (ADR 0006) instead of hidden behind a toggle. The product name stayed Castellan (ADR 0005).
2. **Evidence.** `cargo test --workspace` passed for core, engine, guard, store, tui, winapi, and the CLI after the proptest format-string fix and the two guard assertion fixes. `cargo xtask ci` is the acceptance command and is recorded again when it finishes. Win32 tests that ran are read-only (known folders, free space, DNS export presence). The live scan test creates and deletes one file under the process temp directory.
3. **Failure.** A profile on `B:` is covered by synthetic path tests, not by hard-coded `C:`. A small terminal is tested at 20×8. ARM64 is a CI build, not a local run. Disk-full, access-denied, and in-use are represented in the pure decision (`exclusive_open`, sanity caps) and are not exercised against a full volume. Unicode path components are preserved by the normalizer as lowercase Rust strings; a live non-ASCII temp name was not part of the scan test. Windows 10 versus 11 was not compared on hardware.
4. **Security.** No elevated process is spawned. The CLI cannot be talked into deleting, because clean ignores the environment flag and has no deleter. External TOML cannot add a field that becomes a command. Child names with separators are not joined. The live scan does not follow reparse points. Gap: link count is not read from a handle yet, so a future live caller must pass `u32::MAX` until it does. Gap: clap parse errors still exit 1 instead of 2.
5. **Reversibility.** Sweeps and DNS flush are defined as irreversible. Undo is not implemented. The ledger can detect tampering. Nothing in the CLI writes a ledger row, because nothing in the CLI mutates.
6. **Honesty.** No tweak catalog entries. No "optimizer". The volume bands are labeled as product thresholds plus a Microsoft page about free space, not as a myth about RAM. The TUI test asserts the overview does not contain a fake score.
7. **What a strict reviewer would say.** The mutation gate in the CLI is stricter than the plan (it never honors the env flag). That is deliberate until the handle deleter exists, and it is documented. Coverage, nextest, cargo-deny locally, and privileged E2E are not green because they are not wired into `xtask`. Calling this v1.0 would be false. The STATUS page is the correction.

## Agents

One orchestrator. No subagents were given crate ownership. The safety kernel stayed with the orchestrator.
