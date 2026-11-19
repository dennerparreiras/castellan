# Safety kernel

The kernel is `castellan-guard` plus the policy function in `castellan-core`. Read [ADR 0003](../adr/0003-safety-kernel.md) for the decision. This page is how the code is shaped.

## Two questions, in order

1. May this action run at all? `castellan_core::decide` answers, given the action, the origin (TUI, CLI, MCP, Autopilot), the machine policy, the user preferences, and an optional CLI age.
2. May this node be deleted? `decide_node` answers from facts taken from a handle: final path, reparse bit, link count, times, exclusive open, elevation.

A walker that deletes a node `decide_node` did not approve is a bug. The architecture scan is the backstop for the dangerous Win32 symbols: they may be named only in `castellan-winapi` and `castellan-guard`.

## What the CLI does with that

`castellan sweep clean` calls `mutation_allowed(debug, false, false)` and then refuses. It never passes the real-system flag. On a debug build the gate is closed. On a release build the gate function would allow a non-sandbox root, and the command still refuses, because the live deleter is not connected. Exit code 20, unless `--dry-run`, which exits 0 and deletes nothing either.

## Handle rules for the deleter that is not wired yet

When it is connected, the only acceptable sequence is:

1. Open with `FILE_FLAG_OPEN_REPARSE_POINT` so the open does not follow the link.
2. Read the final path from the handle.
3. Read `NumberOfLinks`. If that read fails, use `u32::MAX`.
4. Call `decide_node`.
5. Delete with `SetFileInformationByHandle` on that same handle, and only if the decision was `Delete`.

Closing the handle and deleting by path is the bug the design exists to prevent.

## Ledger

`castellan-store` appends a row whose hash is BLAKE3 of the previous hash, a newline, and the payload JSON. The genesis previous hash is the string `genesis`. `verify` walks the chain. A test flips a byte and expects failure. The database in that test is under the process temp directory.

Temp-file deletion cannot be undone. The ledger is a record, not a recycle bin. This build does not write ledger rows from the CLI.

## DNS

`guard_flush_dns` loads `dnsapi.dll` by an absolute System32 path with `LOAD_LIBRARY_SEARCH_SYSTEM32`, then calls `DnsFlushResolverCache` through the returned function pointer. The unit test only checks that the export exists. It does not flush. The CLI does not call the flush.
