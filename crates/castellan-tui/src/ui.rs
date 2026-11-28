//! Small drawing parts shared by every room: cards, meters, sparklines, text fitting.
//!
//! Text is wrapped here, by hand, so a picture or an indented list is never
//! re-flowed by the terminal library.

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding};

use crate::model::Model;

/// `unicode` normally, `ascii` when the person asked for plain drawing.
pub(crate) fn pick(model: &Model, unicode: &'static str, ascii: &'static str) -> &'static str {
    if model.ascii { ascii } else { unicode }
}

pub(crate) fn nth(areas: &[Rect], index: usize) -> Rect {
    areas.get(index).copied().unwrap_or_default()
}

/// A titled card. `lit` draws the border in the accent color.
pub(crate) fn card(model: &Model, title: &str, lit: bool) -> Block<'static> {
    let theme = model.theme;
    let border = if model.ascii {
        BorderType::Plain
    } else {
        BorderType::Rounded
    };
    let edge = if lit { theme.accent } else { theme.border };
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(border)
        .border_style(Style::default().fg(edge))
        .padding(Padding::horizontal(1))
        .style(Style::default().bg(theme.surface));
    if !title.is_empty() {
        block = block.title(Span::styled(format!(" {title} "), theme.bold(theme.text)));
    }
    block
}

/// Columns and rows inside a card drawn by [`card`].
pub(crate) fn inner(area: Rect) -> (usize, usize) {
    (
        usize::from(area.width.saturating_sub(4)),
        usize::from(area.height.saturating_sub(2)),
    )
}

/// Word wrap. Words longer than the width are split.
pub(crate) fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut count = 0usize;
    for word in text.split_whitespace() {
        let mut word: Vec<char> = word.chars().collect();
        while word.len() > width {
            if count > 0 {
                lines.push(std::mem::take(&mut current));
                count = 0;
            }
            let rest = word.split_off(width);
            lines.push(word.into_iter().collect());
            word = rest;
        }
        let len = word.len();
        if count > 0 && count + 1 + len > width {
            lines.push(std::mem::take(&mut current));
            count = 0;
        }
        if count > 0 {
            current.push(' ');
            count += 1;
        }
        current.extend(word);
        count += len;
    }
    if count > 0 || lines.is_empty() {
        lines.push(current);
    }
    lines
}

/// Wrapped paragraph in one style, with an indent on every line.
pub(crate) fn para(text: &str, width: usize, indent: usize, style: Style) -> Vec<Line<'static>> {
    wrap(text, width.saturating_sub(indent))
        .into_iter()
        .map(|row| Line::from(Span::styled(format!("{}{row}", " ".repeat(indent)), style)))
        .collect()
}

/// A bullet whose wrapped lines hang under the text, not under the mark.
pub(crate) fn bullet(
    mark: &str,
    mark_style: Style,
    text: &str,
    width: usize,
    style: Style,
) -> Vec<Line<'static>> {
    let hang = mark.chars().count() + 1;
    let mut lines = Vec::new();
    for (index, row) in wrap(text, width.saturating_sub(hang))
        .into_iter()
        .enumerate()
    {
        if index == 0 {
            lines.push(Line::from(vec![
                Span::styled(format!("{mark} "), mark_style),
                Span::styled(row, style),
            ]));
        } else {
            lines.push(Line::from(Span::styled(
                format!("{}{row}", " ".repeat(hang)),
                style,
            )));
        }
    }
    lines
}

/// A small section heading inside a card.
pub(crate) fn heading(model: &Model, text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_uppercase(),
        model.theme.bold(model.theme.steel),
    ))
}

pub(crate) fn blank() -> Line<'static> {
    Line::from("")
}

/// Exactly `width` cells: padded on the right, or cut with an ellipsis.
pub(crate) fn fit(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        return format!("{text}{}", " ".repeat(width - count));
    }
    if width == 0 {
        return String::new();
    }
    let mut out: String = text.chars().take(width - 1).collect();
    out.push('…');
    out
}

/// Exactly `width` cells, text on the right.
pub(crate) fn fit_right(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count >= width {
        return text.chars().take(width).collect();
    }
    format!("{}{text}", " ".repeat(width - count))
}

/// Text centered in `width` cells. Longer text is returned unchanged.
pub(crate) fn center(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count >= width {
        return text.to_owned();
    }
    let left = (width - count) / 2;
    format!("{}{text}", " ".repeat(left))
}

/// A horizontal meter. `permille` is the filled share, 0 to 1000.
pub(crate) fn meter(
    model: &Model,
    permille: u32,
    width: usize,
    color: Color,
) -> Vec<Span<'static>> {
    let theme = model.theme;
    let width = width.max(1);
    let permille = usize::try_from(permille.min(1_000)).unwrap_or(1_000);
    let mut filled = permille * width / 1_000;
    if permille > 0 && filled == 0 {
        filled = 1;
    }
    let (on, off) = if model.ascii {
        ("#", "-")
    } else if theme.plain {
        ("━", "─")
    } else {
        ("━", "━")
    };
    vec![
        Span::styled(on.repeat(filled), Style::default().fg(color)),
        Span::styled(off.repeat(width - filled), Style::default().fg(theme.faint)),
    ]
}

/// The last `width` samples as a sparkline. Each sample is 0 to 100.
pub(crate) fn spark<'a>(
    model: &Model,
    samples: impl DoubleEndedIterator<Item = &'a u8> + ExactSizeIterator,
    width: usize,
) -> String {
    const UNICODE: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    const ASCII: [char; 8] = ['_', '_', '.', '-', '=', '+', '*', '#'];
    let levels = if model.ascii { ASCII } else { UNICODE };
    let skip = samples.len().saturating_sub(width);
    let mut out = " ".repeat(width.saturating_sub(samples.len()));
    for sample in samples.skip(skip) {
        let level = usize::from((*sample).min(100)) * 7 / 100;
        out.push(levels.get(level).copied().unwrap_or(' '));
    }
    out
}

/// The last `width` samples as a bar chart `rows` tall, top row first.
/// Each sample is 0 to 100. Columns without a sample stay blank.
pub(crate) fn chart(model: &Model, samples: &[u8], width: usize, rows: usize) -> Vec<String> {
    const EIGHTHS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let rows = rows.max(1);
    let shown = samples
        .get(samples.len().saturating_sub(width)..)
        .unwrap_or(&[]);
    let pad = width.saturating_sub(shown.len());
    (0..rows)
        .map(|row| {
            let floor = (rows - 1 - row) * 8;
            let mut line = " ".repeat(pad);
            for sample in shown {
                let level = usize::from((*sample).min(100)) * rows * 8 / 100;
                let fill = level.saturating_sub(floor).min(8);
                let ch = if model.ascii {
                    match fill {
                        0 => ' ',
                        1..=3 => '.',
                        _ => '#',
                    }
                } else {
                    EIGHTHS.get(fill).copied().unwrap_or(' ')
                };
                line.push(ch);
            }
            line
        })
        .collect()
}

/// A number drawn three rows tall. ASCII mode gets a plain row in the middle.
pub(crate) fn big_number(model: &Model, text: &str) -> [String; 3] {
    if model.ascii {
        return [String::new(), text.to_owned(), String::new()];
    }
    let mut rows = [String::new(), String::new(), String::new()];
    for ch in text.chars() {
        let glyph = digit_rows(ch);
        for (row, part) in rows.iter_mut().zip(glyph) {
            if !row.is_empty() {
                row.push(' ');
            }
            row.push_str(part);
        }
    }
    rows
}

fn digit_rows(ch: char) -> [&'static str; 3] {
    match ch {
        '0' => ["█▀▀█", "█  █", "█▄▄█"],
        '1' => [" ▀█ ", "  █ ", " ▄█▄"],
        '2' => ["▀▀▀█", "█▀▀▀", "█▄▄▄"],
        '3' => ["▀▀▀█", " ▀▀█", "▄▄▄█"],
        '4' => ["█  █", "▀▀▀█", "   █"],
        '5' => ["█▀▀▀", "▀▀▀█", "▄▄▄█"],
        '6' => ["█▀▀▀", "█▀▀█", "█▄▄█"],
        '7' => ["▀▀▀█", "   █", "   █"],
        '8' => ["█▀▀█", "█▀▀█", "█▄▄█"],
        '9' => ["█▀▀█", "▀▀▀█", "▄▄▄█"],
        '-' => ["    ", " ▀▀ ", "    "],
        _ => ["    ", "    ", "    "],
    }
}

/// `12345` as `12,345`.
pub(crate) fn thousands(value: u64) -> String {
    let digits = value.to_string();
    let len = digits.len();
    let mut out = String::with_capacity(len + len / 3);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (len - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// `93784` seconds as `1 d 2 h`. Short spans use minutes.
pub(crate) fn span_text(secs: u64) -> String {
    let days = secs / 86_400;
    let hours = (secs % 86_400) / 3_600;
    let minutes = (secs % 3_600) / 60;
    if days > 0 {
        format!("{days} d {hours} h")
    } else if hours > 0 {
        format!("{hours} h {minutes} min")
    } else {
        format!("{minutes} min")
    }
}

/// Share of `part` in `whole`, 0 to 100.
pub(crate) fn percent(part: u64, whole: u64) -> u8 {
    part.saturating_mul(100)
        .checked_div(whole)
        .map(|value| u8::try_from(value.min(100)).unwrap_or(100))
        .unwrap_or(0)
}

/// A highlighted row background that spans the card, even on short text.
pub(crate) fn row_style(model: &Model, selected: bool) -> Style {
    if selected && !model.theme.plain {
        Style::default().bg(model.theme.raised)
    } else if selected {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_keeps_words_whole_and_splits_only_giants() {
        assert_eq!(wrap("one two three", 7), vec!["one two", "three"]);
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"]);
        assert_eq!(wrap("", 10), vec![""]);
        for row in wrap("the quick brown fox jumps over the lazy dog", 9) {
            assert!(row.chars().count() <= 9, "{row}");
        }
    }

    #[test]
    fn numbers_read_like_numbers() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(1_234_567), "1,234,567");
        assert_eq!(span_text(59), "0 min");
        assert_eq!(span_text(3_700), "1 h 1 min");
        assert_eq!(span_text(93_784), "1 d 2 h");
        assert_eq!(percent(1, 0), 0);
        assert_eq!(percent(94, 100), 94);
    }

    #[test]
    fn every_big_digit_is_four_cells_wide() {
        for ch in "0123456789-".chars() {
            for row in digit_rows(ch) {
                assert_eq!(row.chars().count(), 4, "{ch} {row}");
            }
        }
    }

    #[test]
    fn fit_always_returns_the_asked_width() {
        assert_eq!(fit("ab", 4), "ab  ");
        assert_eq!(fit("abcdef", 4), "abc…");
        assert_eq!(fit_right("ab", 4), "  ab");
        assert_eq!(fit("abc", 0), "");
    }
}
