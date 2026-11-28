# 0010. Window frame, Mica, and Snap Layouts

- Status: accepted for G0; to be revisited in G2
- Date: 2026-11-28

## Context

The plan wants a branded title bar and Mica, and it makes Windows 11 Snap Layouts (the menu on hover over maximize), drag, double-click to maximize, and `Win+arrow` non-negotiable. A frameless Tauri window loses Snap Layouts unless a helper restores the caption hit-testing.

## Decision

G0 keeps the **native title bar** and applies **Mica** through `windowEffects` in `tauri.conf.json`. Mica requires a transparent window, so the page paints a 78% dark veil over it to keep text contrast steady on any wallpaper. Windows 10 shows the veil without Mica.

In G2, a custom title bar may replace the native one only if Snap Layouts, drag, double-click, and keyboard snapping all keep working (for example with `tauri-plugin-decorum` or an equivalent). The G2 PR must show captures of each behavior.

## Consequences

Snap Layouts work from day one. The title bar is the system's, not the brand's, until G2 proves a replacement. Window size and position persist through the `window-state` plugin. A second launch focuses the existing window through `single-instance`.
