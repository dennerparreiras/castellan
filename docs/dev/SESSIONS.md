# Sessions

The orchestrator keeps this table current (desktop master plan, section 3). One area, one worktree, one branch per session. Two sessions never write the same files. Contract changes (`castellan-core`, `castellan-service`, `apps/desktop/src-tauri/src/ipc.rs`) go through the orchestrator.

| Wave | Area | Owner | Branch | State |
| --- | --- | --- | --- | --- |
| W0 | Foundation: sense, service, desktop shell, IPC v1, frontend CI, ADRs 0007–0012 | orchestrator | `main` | done |
| W1 | (a) Brand and brand book | — | `feat/brand` | not started |
| W1 | (b) Design system and shared tokens | — | `feat/design-system` | not started |
| W1 | (c) Window shell: navigation, palette, tray, title bar, isolation | — | `feat/desktop-shell` | not started |
| W1 | (d) SQLite store, ledger, undo | — | `feat/store-ledger` | not started |
| W1 | (e) Elevated broker, IPC, scheduled tasks | — | `feat/broker` | not started |

## Brief checklist

Each session brief names: the goal; the sections of both plans to read; the files and crates it owns; the contract it consumes and produces; the feature-matrix rows it must turn green; the commands that prove it; what was already ruled out.
