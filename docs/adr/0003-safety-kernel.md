# 0003. Safety kernel

- Status: accepted
- Date: 2026-11-19

## Context

Castellan will be able to delete files and, later, change system settings. The plan requires one kernel that every mutation passes through, with rules a lower layer cannot loosen.

## Decision

Policy order, strongest first: compiled rules, machine policy, user config, CLI flags. A lower layer may only tighten. The effective sweep age is the maximum of the absolute floor (3600 seconds), the action, the machine policy, the user preference, and the CLI value.

Lockdown on either the machine policy or the user preference denies every mutation. Autopilot may run only kinds listed in machine policy, and only kinds that are both Safe and marked listable. MCP and Autopilot cannot run Moderate or Sensitive actions. MCP may run Safe actions only when `mcp_allow_safe_actions` is true. The default is false.

Unknown TOML fields are rejected.

The action catalog is a closed enum. The variants that exist today are `Sweep` of user temp or Windows temp, and `FlushDns`. There is no free-form path and no command string. Neither sweep nor DNS flush is reversible. Temp deletion is permanent; the ledger can record it, and undo cannot bring the bytes back.

Path rules:

- Normalize by stripping `\\?\` and `\\?\UNC\`, resolving `.` and `..`, and lowercasing.
- `is_within` compares whole components. `D:\Temp` does not contain `D:\Tempest`.
- UNC paths and volume roots are denied.
- Protected: the profile directory itself, Documents and the other protected user folders supplied by the caller, Program Files, ProgramData, Castellan’s own directories, System Volume Information, `$Recycle.Bin`, and everything under the Windows directory except `Windows\Temp`.
- The allow-root itself is never deleted.
- Names containing `\`, `/`, `:`, NUL, `.`, or `..` are not joined.
- Both created and modified times must be older than the minimum age. Age applies to files. An empty directory may be removed after its children are processed.
- In-use files (exclusive open failed) are skipped.
- Reparse points are not traversed. Elevated sweeps skip the link. Non-elevated sweeps may delete the link itself and count 0 bytes.
- Elevated sweeps skip `link_count > 1`. A caller that has not read `NumberOfLinks` passes `u32::MAX` and the delete is skipped.
- Autopilot sanity caps default to 50_000_000_000 bytes and 500_000 files. Crossing a cap aborts the rest of the batch when no person is present.

Debug mutation gate: a sandbox is accepted only when it is a strict descendant of the process temp directory. A debug build without the explicit flag, and without that sandbox, refuses. The test suite never sets the flag. In this build the CLI clean command does not read the flag at all, and it never deletes.

## Consequences

A feature that needs a new mutation adds an `Action` variant, a safety review, and tests that show the refusal cases. It does not shell out.

The live Windows deleter is intentionally unwired until handle-based delete (`open` without following reparse, final-path check, delete by handle) is reviewed. The in-memory walker is the tested decision path.
