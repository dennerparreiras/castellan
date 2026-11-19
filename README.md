# Castellan

The keeper of your machine.

Castellan is a native Windows program for cleaning, diagnosing, and explaining the state of a PC. This repository is the **0.1.0 foundation**: the safety rules, the Keep Score, a read-only status command, and an honest terminal UI. It is not the finished 1.0 product. Features that are not built yet are listed below and in [`docs/dev/STATUS.md`](docs/dev/STATUS.md).

## What this build does

- `castellan status` reads free space on the Windows directory and prints a partial Keep Score. With `--json` it prints the same facts as JSON.
- `castellan about` and `castellan doctor` describe this build. Doctor does not claim a scheduled task or a live cleaner.
- `castellan sweep targets` lists the only sweep targets the safety kernel knows.
- `castellan sweep clean` refuses to delete anything. The live deleter is not connected.
- With no arguments, on a real terminal, it opens a small terminal UI. The overview does not invent a score.

## What this build does not do

It does not clean temp files on a real disk, flush DNS, change startup programs, tune Windows, repair system files, install itself, register a scheduled task, open a network port, or speak MCP. Those are later milestones. There is no telemetry.

## Build

Requires Rust 1.100.0 (the `rust-toolchain.toml` pin) on Windows.

```powershell
cargo xtask ci
```

That runs `cargo fmt --check`, clippy with warnings denied, the test suite, and an architecture scan. The scan rejects `unsafe` outside `castellan-winapi` and rejects shell execution anywhere in the Rust sources.

Debug builds print `DEV BUILD — real-system actions disabled` and the clean command still deletes nothing.

## Safety, in five lines

Mutations go through `castellan-guard`, or they do not happen. The action catalog is a closed enum: there is no "run this command" action. Paths are compared component by component after `..` is resolved, and reparse points are not traversed. A debug build will not mutate a real root. Deleting a temp file is irreversible; this build does not delete them.

## Documentation

Start at [`docs/README.md`](docs/README.md). The score formula is [`docs/reference/health-score.md`](docs/reference/health-score.md).

## Author

Crafted by [Denner Parreiras](https://github.com/dennerparreiras).

## License

MIT. Copyright (c) 2026 Denner Parreiras. See [LICENSE](LICENSE).
