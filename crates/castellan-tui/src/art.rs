//! Character art for the keep.
//!
//! None of these pictures are measurements. ASCII mode draws the castle with
//! plain characters and leaves the room emblems out.

use crate::model::Tab;

const CASTLE_W: usize = 41;

/// Splash castle. Every line is [`CASTLE_W`] cells, so the tower does not wobble.
pub(crate) fn castle(ascii: bool) -> Vec<String> {
    if ascii {
        ascii_castle()
    } else {
        unicode_castle()
    }
}

fn unicode_castle() -> Vec<String> {
    vec![
        band("", "", "", ' '),
        band("", "▲", "", ' '),
        band("", "▄█▄", "", ' '),
        band("", "▄█████▄", "", ' '),
        band("", "▄█████████████▄", "", ' '),
        band("▄█████▄", "█ ▄█▄ ▄█▄ ▄█▄ █", "▄█████▄", ' '),
        band("█ ▄█▄ █", "█ █ █ █ █ █ █ █", "█ ▄█▄ █", ' '),
        band("███████", "█ ▀▀▀ ▀▀▀ ▀▀▀ █", "███████", '█'),
        band("███████", "███████████████", "███████", '█'),
        band("█ █ █ █", "╔═══════╗", "█ █ █ █", '█'),
        band("███████", "║       ║", "███████", '█'),
        band("▀▀▀▀▀▀▀", "╚═══════╝", "▀▀▀▀▀▀▀", '▀'),
    ]
}

fn ascii_castle() -> Vec<String> {
    vec![
        band("", "", "", ' '),
        band("", "^", "", ' '),
        band("", "/#\\", "", ' '),
        band("", "/#####\\", "", ' '),
        band("", "/###########\\", "", ' '),
        band("/#####\\", "| # # # # # |", "\\#####/", ' '),
        band("|# # #|", "| # # # # # |", "|# # #|", ' '),
        band("|#####|", "|###########|", "|#####|", '#'),
        band("|#####|", "|###########|", "|#####|", '#'),
        band("|# # #|", "| |=======| |", "|# # #|", '#'),
        band("|#####|", "| |       | |", "|#####|", '#'),
        band("=======", "===========", "=======", '='),
    ]
}

/// Three-cell torch for the top of the splash tower.
pub(crate) fn flame(ascii: bool, phase: u32) -> &'static str {
    if ascii {
        match phase % 4 {
            0 => " ^ ",
            1 => "^^ ",
            2 => "^^^",
            _ => " ^^",
        }
    } else {
        match phase % 4 {
            0 => " ▁ ",
            1 => " ▂▄",
            2 => "▄█▄",
            _ => " ▂ ",
        }
    }
}

/// A small emblem for a room. Every line of one emblem has the same width.
/// ASCII mode has no emblems.
pub(crate) fn emblem(tab: Tab, ascii: bool) -> &'static [&'static str] {
    if ascii {
        return &[];
    }
    match tab {
        Tab::Overview => &["    ▄█▄    ", " ▄▄█████▄▄ ", " █ ▄ █ ▄ █ ", " ███▀ ▀███ "],
        Tab::Sweep => &["       ╱   ", "      ╱    ", "   ▄▟█▙▄   ", "   ▐▌▐▌▐   "],
        Tab::Vitals => &["      ╭╮      ", "──╮╭──╯│ ╭────", "  ╰╯   ╰─╯    "],
        Tab::Network => &["●────●────●", "│    │    │", "●────●────●"],
        Tab::Shield => &[" ▄▄▄▄▄▄▄▄▄ ", " █   ◆   █ ", "  ▀▄   ▄▀  ", "    ▀▄▀    "],
        Tab::Boost => &["     ▲     ", "    ▟█▙    ", "   ▟███▙   ", "     █     "],
        Tab::Game => &["╭─────────╮", "│ ┼   ● ● │", "╰──╯   ╰──╯"],
        Tab::Fix => &["  ▄▄▄▄▄▄▄  ", "  ▀▀▀█▀▀▀  ", "     █     ", "     █     "],
        Tab::Ledger => &["╭─────────╮", "│▌ ═════  │", "│▌ ═══    │", "╰─────────╯"],
    }
}

fn band(left: &str, mid: &str, right: &str, fill: char) -> String {
    let used = left.chars().count() + mid.chars().count() + right.chars().count();
    let rest = CASTLE_W.saturating_sub(used);
    let gap_l = rest / 2;
    let gap_r = rest - gap_l;
    let mut out = String::new();
    out.push_str(left);
    out.extend(std::iter::repeat_n(fill, gap_l));
    out.push_str(mid);
    out.extend(std::iter::repeat_n(fill, gap_r));
    out.push_str(right);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn castle_lines_share_one_width_and_ascii_has_no_blocks() {
        for ascii in [true, false] {
            let picture = castle(ascii);
            assert_eq!(picture.len(), 12);
            for line in &picture {
                assert_eq!(line.chars().count(), CASTLE_W, "{line}");
                if ascii {
                    assert!(!line.contains('█'), "{line}");
                }
            }
        }
    }

    #[test]
    fn every_emblem_is_a_clean_rectangle() {
        for tab in Tab::ALL {
            assert!(emblem(tab, true).is_empty());
            let picture = emblem(tab, false);
            assert!(!picture.is_empty(), "{tab:?}");
            let width = picture
                .first()
                .map(|line| line.chars().count())
                .unwrap_or(0);
            for line in picture {
                assert_eq!(line.chars().count(), width, "{tab:?} {line}");
            }
        }
    }

    #[test]
    fn the_torch_is_always_three_cells() {
        for ascii in [true, false] {
            for phase in 0..8 {
                assert_eq!(flame(ascii, phase).chars().count(), 3);
            }
        }
    }
}
