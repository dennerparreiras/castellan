# 0006. Prefetch stays denied

- Status: accepted
- Date: 2026-11-19

## Context

Older maintenance scripts deleted `%SystemRoot%\Prefetch`. Windows uses those files to speed the next boot and the next launch of a program. Removing them makes the following boot slower, and they are rebuilt only after the programs run again. The plan allowed a hidden advanced toggle. A toggle that is off by default still leaves a foot-gun in the catalog.

Prefetch lives under the Windows directory. The deny-list already allows only `Windows\Temp` inside that tree.

## Decision

Prefetch is not an action and not a sweep target. The system-root rule denies it. There is no advanced switch.

`flushdns` can stay in the catalog as `FlushDns` because it does not discard boot data. `registerdns` is not in the default set. A boot-time winsock reset is not an action. `wuauclt` is not an action. Blind `del` of a path string is not an action. The Recent folder is not a default target.

## Consequences

A test asserts that a Prefetch path is `DenyHit::SystemRoot`. Adding Prefetch later requires a new ADR and a cited measurement that the benefit outweighs the slower boot. That bar is intentionally high.
