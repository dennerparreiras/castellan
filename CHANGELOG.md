# Changelog

All notable changes to Castellan are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Workspace, lint policy, `cargo xtask ci`, and the architecture scan.
- `castellan-core`: closed action catalog, policy precedence, and the integer Keep Score.
- `castellan-guard`: lexical path rules, an in-memory sweep walker, and a read-only live scan that does not follow reparse points.
- `castellan-winapi`: temp, Windows, and system directory lookups, disk free space, non-following open, and a DNS-flush export check that does not call the flush.
- `castellan-store`: append-only SQLite ledger with a BLAKE3 hash chain.
- `castellan-engine`: volume free-space finding `STORAGE-001`.
- `castellan` CLI (`status`, `about`, `doctor`, `sweep`) and a small terminal UI.
- Documentation skeleton, ADRs 0001–0006, and threat-model notes for this slice.

### Not in this version

Live deletion, DNS flush from the CLI, undo, vitals, network, shield, boost, game mode, repair playbooks, the elevated broker, MCP, the installer, and code signing. See `docs/dev/STATUS.md`.
