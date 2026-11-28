# TUI tour

Run `castellan` with no arguments in a terminal to open the keep. Everything on these screens is read from this PC, and nothing on them changes it.

## The frame

The top line shows the name of this PC, the Keep Score with its state (`partial` while only some areas are checked), and the clock. The second line lists the nine rooms. The open room is highlighted and underlined. Press its number, or click it, to go there.

The bottom line lists the keys that work on the current screen. On the right it says `dev build · changes nothing` or `preview · changes nothing`, so you always know nothing will be deleted.

## 1 Overview

- **Keep Score** shows the health of this PC from 0 to 100, how many of the five areas were checked, and a legend for the ranges: 90–100 Fortified, 75–89 Sound, 50–74 Needs attention, 0–49 At risk. Today only Storage is checked, so the score is marked `partial` and never shows a band.
- **Needs your attention** lists measured findings. Each one shows the proof (for example `27.8 GB free of 464.7 GB`) and what you can do. `Enter` opens the room that helps. Areas without a check are named under *Not checked yet*.
- **This PC right now** shows the processor load, memory in use, every fixed drive, and how long Windows has been running. These update every second.

## 2 Sweep

The list shows what can be cleaned: user temp, Windows temp, and Prefetch. Castellan walks each temp folder once when it opens, without following links and without opening anything for writing. It shows the total size and how much is old enough to sweep under the current minimum age.

The details card explains what the folder is, the measurement, and the rules a sweep follows. Prefetch is listed so the screen can explain why Castellan never cleans it. If Windows refuses to let you read a folder, the screen says so instead of guessing a size.

The cleaner is not connected in this build. `Enter` says so and deletes nothing.

## 3 Vitals

Four cards: processor load with a live chart, memory with a live chart, every fixed drive with its free space, and time since Windows started. Each card ends with one sentence about what the number means.

## 4 to 9

Network, Shield, Boost, Game, and Fix are not built yet. Each room says what it will do and states that nothing in it runs today. Shield promises it will never turn off Defender, the firewall, UAC, SmartScreen, or Windows Update. Boost promises no memory or registry cleaner. The Ledger explains what it will record and is empty because nothing has been changed.

## Settings, help, and go to

`s` opens settings. The list is grouped into sign-in cleanup, display, and safety. The card on the right explains the selected setting and what it does in this build. Sign-in cleanup is saved but not active: no task is registered and nothing is deleted.

`?` shows every key and a short glossary. `:` or `Ctrl+P` opens *Go to*, where you type part of a room name.

## Seeing every screen without a PC to inspect

```bash
cargo run -p castellan-tui --example gallery -- 100 30
```

The gallery prints every room at the size you give, with made-up readings. Add `ascii` as a third argument to see the plain drawing.
