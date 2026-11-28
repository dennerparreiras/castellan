# Keybindings

The footer always shows the keys that work on the current screen.

| Key | Action |
| --- | --- |
| `1` through `9` | Overview, Sweep, Vitals, Network, Shield, Boost, Game, Fix, Ledger |
| Click a tab | Open that room |
| `Tab`, `l`, or Right | Next room. On the settings age row, raise the hours |
| `Shift+Tab`, `h`, or Left | Previous room. On the settings age row, lower the hours |
| Mouse wheel | Next or previous room, when no window and no settings page is open |
| `j` `k` or Up Down | Move in a list, or between settings rows |
| Click a row | Select a sweep target or a setting |
| `Enter` | Open what is selected. On a finding, open the room that helps. In Sweep, nothing is deleted |
| Space | Same as `Enter`. In settings, switch the row on or off |
| `s` | Open or close settings |
| `:` or `Ctrl+P` | Go to a room by typing part of its name |
| `?` | Help, with a short glossary |
| `Esc` | Close help, go to, settings, or a message. On the main screen, quit |
| `q` or `Ctrl+C` | Quit, from any screen, including the opening screen |

Any key other than `q` skips the opening screen.

Settings are saved to `preferences.toml` in your local app data folder. Sign-in cleanup is saved but not active: it does not register a scheduled task and does not delete files. The minimum file age cannot go below one hour.

`CASTELLAN_NO_ANIM=1` turns animations off. `CASTELLAN_ASCII=1` draws with plain characters. `NO_COLOR` paints no colors. These environment flags force the mode on; they do not force it off.
