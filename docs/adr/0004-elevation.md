# 0004. Elevation

- Status: accepted
- Date: 2026-11-19

## Context

Some maintenance needs an administrator token: Windows\Temp, later SFC, DISM, and scheduled tasks. A permanent service running as SYSTEM would enlarge the attack surface for a program that is idle most of the day.

## Decision

Castellan does not install a Windows service. Elevation is on demand, through a broker process that will exist in a later milestone. The scheduled Morning Round, also later, is a scheduled task that starts that broker for a fixed list of safe actions. It is not a general remote shell.

This build does not register a task, does not launch an elevated child, and does not embed an application manifest that requests `requireAdministrator`. When a manifest is added, it will be `asInvoker`.

`Sweep` of Windows temp is marked as needing elevation in `castellan-core`. The CLI cannot perform it yet.

The broker, when it exists, will:

- Accept a typed plan, not a command line.
- Refuse paths that were not opened by handle inside the allow-root.
- Refuse any action whose risk is above Safe unless a person completed the confirmation the policy requires.
- Speak over a private IPC channel. MCP stays on stdio and is never the elevated process.

## Consequences

Tests that need a real elevated token run on a disposable CI runner, not on the maintainer’s PC. Until the broker exists, those tests are not in the tree, and `docs/dev/STATUS.md` says so.
