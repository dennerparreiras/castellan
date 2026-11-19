# TUI architecture

The TUI is a small TEA loop. `Model` holds the tab, the help flag, the quit flag, and the persona line. `update` is pure. `view` draws with ratatui.

`castellan` starts the loop only when there are no arguments and stdout is a terminal. Tests call `view` with `TestBackend` and never open a console.

Tabs: Overview, Sweep, Ledger. The ledger tab does not read the SQLite file yet. It says the ledger is not loaded.
