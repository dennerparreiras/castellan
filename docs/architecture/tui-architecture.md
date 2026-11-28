# TUI architecture

The TUI is a TEA loop in `castellan-tui`.

| File | Role |
| --- | --- |
| `model.rs` | `Model`, `Msg`, and the pure `update`. Holds the room, eased gauges, live history, sweep readings, settings, and overlays. |
| `view.rs` | The frame: header, tabs, footer, settings, help, go to, toasts, and mouse hit-testing (`pointer`). |
| `rooms.rs` | The nine rooms. |
| `ui.rs` | Shared parts: cards, meters, charts, big digits, word wrap, number formatting. |
| `theme.rs` | Semantic color tokens. |
| `art.rs` | Splash castle and room emblems. |

`update` never touches the machine and never writes files. `view` reads only the model.

## Data flow

The CLI (`castellan-cli`) gathers everything and sends it in as messages. Every call it makes for the TUI is read-only:

- On open: free space on the Windows drive (the Keep Score witness), every fixed drive, memory, uptime, and the logical processor count.
- On a worker thread: a walk of user temp and Windows temp through `castellan_guard::live::measure_no_follow`. It skips reparse points, opens nothing for writing, and stops at 200,000 entries. Results arrive as `Msg::Sweep`.
- About once a second: processor load from two `GetSystemTimes` readings, memory, and uptime, as `Msg::Pulse`. The model keeps the last 240 samples for the charts.

The loop saves `{LOCALAPPDATA}\Castellan\preferences.toml` only after a setting changes.

## Redraw

`Model::pace_ms` is 33 ms while something is easing (splash, score, meters, tab underline) and 250 ms otherwise. An idle keep therefore redraws four times a second, enough for the clock and the live readings.

## Tests

`src/tests.rs` draws every room into a `TestBackend` at 80x24, 100x30, 120x40, and 200x60, in Unicode and ASCII. A CLI test draws Overview, Sweep, and Vitals from this PC's real readings. `cargo run -p castellan-tui --example gallery -- <width> <height>` prints every screen for design review.

Not in this build: other themes, slide transitions between rooms, hold-to-confirm, and a recorded GIF.
