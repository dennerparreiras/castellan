# Architecture

```mermaid
flowchart LR
  cli[castellan-cli]
  tui[castellan-tui]
  engine[castellan-engine]
  core[castellan-core]
  guard[castellan-guard]
  winapi[castellan-winapi]
  store[castellan-store]
  cli --> tui
  cli --> engine
  cli --> guard
  engine --> core
  guard --> core
  guard --> winapi
  cli --> winapi
  store --> core
```

`castellan-core` has no I/O. `castellan-winapi` is the only crate allowed to use `unsafe`. The store is not called by the CLI yet. See [crates.md](crates.md) and [ADR 0002](../adr/0002-architecture.md).
