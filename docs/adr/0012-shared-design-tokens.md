# 0012. Design tokens shared by the GUI and the TUI

- Status: proposed (implemented in G1)
- Date: 2026-11-28

## Context

The TUI theme lives in `crates/castellan-tui/src/theme.rs` and the desktop draft palette in `apps/desktop/src/styles/tokens.css`. Two hand-kept copies drift.

## Decision

G1 adds `design/tokens.json` as the single source for color, spacing, radius, type scale, and motion. `cargo xtask tokens` generates `tokens.css` for the desktop app and the color constants of the TUI theme, and CI fails when either generated file is stale. Contrast pairs used for text are checked against WCAG AA by the same task.

## Consequences

Until G1, `tokens.css` is a hand-written draft that matches the master plan's table. The G0 values already use the same amethyst and plasma as the TUI.
