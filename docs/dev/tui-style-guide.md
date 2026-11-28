# TUI style

## Palette

Theme Keep, the night fortress. Colors are semantic tokens in `crates/castellan-tui/src/theme.rs`.

| Role | Hex | Used for |
| --- | --- | --- |
| Background | `#0D1016` | Page |
| Surface | `#141921` | Cards |
| Raised | `#222936` | Selected row, toasts |
| Border | `#343E4E` | Card borders and rules |
| Faint | `#3A4456` | Empty meter track, "not checked yet" |
| Text | `#E6E1D6` | Body |
| Muted | `#8B95A5` | Explanations |
| Accent (amethyst) | `#B094FF` | Brand, keys, selection |
| Steel | `#62CCDA` | Headings, neutral values, charts |
| Ok | `#7EC882` | Healthy, "on" |
| Warn | `#ECBC50` | Warnings |
| Critical | `#EC6856` | Critical |
| Info | `#74ACE0` | Info |

Warning stays gold, so a full disk is not the same color as the brand. Scores use ok, steel, warn, and critical for the four bands. Usage shares (processor, memory, disk) use steel below 75%, warn up to 89%, and critical above.

`NO_COLOR` keeps the layout and paints nothing. Selection then uses reversed video.

## Layout

- The screen is a header (title, tabs, rule), a body, and a one-line footer.
- Past 150 columns the body stops growing and stays centered, so lines stay readable.
- Below 80x24 the keep asks for a larger window and explains why.
- Cards have a title and one column of padding. Section headings inside a card are small uppercase steel text.
- Text is wrapped by `ui::wrap`, never by the terminal. A sentence that does not fit whole is left out instead of being cut.

## Writing

- Plain words. Say what a number means and what to do about it, in one sentence each.
- Every number was measured. A missing reading says `not checked yet`, `measuring…`, or why it could not be read.
- Rooms that are not built say what they will do and that nothing in them runs today.
- A score with confidence under 500 is `partial` and does not show a band.

## Motion

The splash builds the castle for about a second; any key skips it. Scores and meters ease toward their value. The tab underline narrows into place when the room changes. The heart in the header pulses slowly. The loop draws every 33 ms only while something is easing and every 250 ms otherwise. The *Animations* setting or `CASTELLAN_NO_ANIM=1` turns all of it off.

## ASCII

*Plain ASCII drawing* or `CASTELLAN_ASCII=1` replaces boxes, bars, charts, and marks with plain characters and leaves the room emblems out. A test draws every room in ASCII and refuses block and box glyphs.

Keys are listed in `docs/guide/keybindings.md`.
