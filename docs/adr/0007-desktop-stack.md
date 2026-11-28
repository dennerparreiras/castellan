# 0007. Desktop stack: Tauri 2, React, TypeScript

- Status: accepted
- Date: 2026-11-28

## Context

The desktop master plan (`CASTELLAN_GUI_MASTER_PLAN.md`) asks for a graphical Castellan that is more striking than the terminal UI, with animation, a brand, and every feature working. The backend is already a Rust workspace with a safety kernel that every frontend must share.

Options weighed: Electron, Slint, egui with wgpu, WinUI 3 through windows-rs, Flutter, and Tauri.

## Decision

- **Shell:** Tauri 2 (stable 2.12 line). Tauri 3 is still alpha and is not used.
- **Frontend:** React 19 and TypeScript 6 in strict mode, built by Vite 8, tested with Vitest 5 and Testing Library. TypeScript 7 is not used yet because typescript-eslint supports only versions below 6.1.
- **Layout:** `apps/desktop/` holds the frontend; `apps/desktop/src-tauri/` holds the `castellan-desktop` crate, a member of the Cargo workspace. This is the standard Tauri layout; the plan's `apps/desktop/ui/` folder is not used.
- **Node:** pinned to 24.22.3 LTS in `apps/desktop/.npmrc` with `use-node-version`. pnpm fetches that runtime into its own store, the same way `rust-toolchain.toml` pins Rust. The system Node is not used or changed. pnpm itself comes from `packageManager` through corepack.
- **Plugins:** `single-instance` and `window-state` only, both driven from Rust. No plugin is exposed to the frontend.
- **Licenses:** Tauri pulls in BSD-3-Clause crates (brotli) and five MPL-2.0 crates (cssparser, cssparser-macros, selectors, dtoa-short, option-ext). BSD-2-Clause, BSD-3-Clause, and ISC join the allow list, as the master plan already permits. MPL-2.0 is allowed only for those five crates by name in `deny.toml`. MPL-2.0 is file-level copyleft; the crates are used unmodified and do not change Castellan's MIT license.

Rejected: Electron (a Node runtime in the process and a much larger surface), Slint (smaller ceiling for effects), egui (weak accessibility), WinUI 3 from Rust (immature tooling), Flutter (a second runtime and language).

## Consequences

The desktop app reuses the service and the safety kernel unchanged. The window depends on WebView2, which Windows 11 ships and Windows 10 can install. The frontend gate (`pnpm check`) joins `cargo xtask ci`, so contributors need pnpm 10. The owner accepted the MPL-2.0 exceptions on 2026-11-28.
