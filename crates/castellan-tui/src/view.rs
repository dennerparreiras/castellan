//! The frame around every room: header, tabs, footer, settings, and overlays.
//!
//! The view reads the model and does not change it.

use castellan_core::{Band, Severity};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use crate::art::{self, flame};
use crate::model::{Key, Model, Msg, SETTING_ROWS, SWEEP_ROWS, Tab, palette_hits};
use crate::rooms;
use crate::ui::{
    blank, bullet, card, center, fit, fit_right, heading, inner, nth, para, pick, wrap,
};

const MIN_W: u16 = 80;
const MIN_H: u16 = 24;
/// Past this width the content stops stretching and sits in the middle.
const MAX_BODY_W: u16 = 150;

/// Draw one frame.
pub fn view(model: &Model, frame: &mut Frame<'_>) {
    let area = frame.area();
    let theme = model.theme;
    frame.render_widget(
        Block::default().style(Style::default().fg(theme.text).bg(theme.background)),
        area,
    );
    if area.width < MIN_W || area.height < MIN_H {
        frame.render_widget(too_small(model, area), area);
        return;
    }
    if model.splash {
        frame.render_widget(splash(model, area), area);
        return;
    }
    let parts = parts(area);
    draw_header(model, frame, parts.header);
    if model.settings {
        draw_settings(model, frame, parts.body);
    } else {
        rooms::draw(model, frame, parts.body);
    }
    frame.render_widget(footer(model, parts.footer.width), parts.footer);
    if let Some((text, _)) = &model.toast {
        draw_toast(model, frame, parts.body, text);
    }
    if model.help {
        draw_help(model, frame, area);
    }
    if model.palette {
        draw_palette(model, frame, area);
    }
}

/// The three bands of the screen. Header and footer share the body's columns.
pub(crate) struct Parts {
    pub(crate) header: Rect,
    pub(crate) body: Rect,
    pub(crate) footer: Rect,
}

pub(crate) fn parts(area: Rect) -> Parts {
    let width = area.width.saturating_sub(2).min(MAX_BODY_W);
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    let column = |row: Rect| Rect {
        x,
        width,
        y: row.y,
        height: row.height,
    };
    Parts {
        header: column(nth(&rows, 0)),
        body: column(nth(&rows, 1)),
        footer: column(nth(&rows, 2)),
    }
}

fn too_small(model: &Model, area: Rect) -> Paragraph<'static> {
    let theme = model.theme;
    let lines = vec![
        blank(),
        Line::from(Span::styled(
            format!("{} CASTELLAN", pick(model, "▟█▙", "[#]")),
            theme.bold(theme.accent),
        )),
        blank(),
        Line::from("This window is too small to draw the keep."),
        Line::from(format!(
            "It needs at least 80x24. It is {}x{} now.",
            area.width, area.height
        )),
        blank(),
        Line::from(Span::styled(
            "Make the window larger, or press q to quit.",
            theme.fg(theme.muted),
        )),
    ];
    Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::default().fg(theme.text).bg(theme.background))
}

// ---------------------------------------------------------------- splash

fn splash(model: &Model, area: Rect) -> Paragraph<'static> {
    let theme = model.theme;
    let tower = art::castle(model.ascii);
    let revealed = splash_rows(model.splash_ms, tower.len());
    let first_lit = tower.len().saturating_sub(revealed);
    let body = tower.len() + 8;
    let pad = usize::from(area.height).saturating_sub(body) / 2;
    let mut lines: Vec<Line<'static>> = vec![blank(); pad];
    for (index, line) in tower.iter().enumerate() {
        let shown = if index == 0 {
            crown_flame(line, model.ascii, model.splash_ms / 90)
        } else {
            line.clone()
        };
        let color = if index <= 1 {
            theme.warn
        } else if index < first_lit {
            theme.faint
        } else if index == first_lit {
            theme.accent
        } else {
            theme.muted
        };
        lines.push(Line::from(Span::styled(shown, theme.fg(color))));
    }
    lines.push(blank());
    lines.push(wordmark(model));
    lines.push(Line::from(Span::styled(
        "The keeper of your machine.",
        theme.fg(theme.muted),
    )));
    lines.push(blank());
    lines.push(Line::from(Span::styled(
        splash_status(model),
        theme.bold(theme.steel),
    )));
    lines.push(Line::from(Span::styled(
        "Crafted by Denner Parreiras",
        theme.fg(theme.muted),
    )));
    lines.push(blank());
    lines.push(Line::from(vec![
        Span::styled("any key", theme.bold(theme.accent)),
        Span::styled(" enters     ", theme.fg(theme.muted)),
        Span::styled("q", theme.bold(theme.accent)),
        Span::styled(" quits", theme.fg(theme.muted)),
    ]));
    Paragraph::new(lines)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme.text).bg(theme.background))
}

fn crown_flame(line: &str, ascii: bool, phase: u32) -> String {
    let fire = flame(ascii, phase);
    let mut chars: Vec<char> = line.chars().collect();
    let start = chars.len().saturating_sub(3) / 2;
    for (offset, ch) in fire.chars().enumerate() {
        if let Some(slot) = chars.get_mut(start + offset) {
            *slot = ch;
        }
    }
    chars.into_iter().collect()
}

fn splash_rows(ms: u32, len: usize) -> usize {
    let steps = u64::try_from(len).unwrap_or(1).max(1);
    let shown = (u64::from(ms) * steps / 900).max(1);
    usize::try_from(shown).unwrap_or(len).min(len)
}

fn splash_status(model: &Model) -> &'static str {
    if model.witness.is_some() {
        "System drive read. The other areas are not inspected yet."
    } else {
        "Lighting the torches."
    }
}

fn wordmark(model: &Model) -> Line<'static> {
    let theme = model.theme;
    let text = "C A S T E L L A N";
    let len = u32::try_from(text.chars().count()).unwrap_or(17);
    let phase = usize::try_from((model.splash_ms / 45) % (len + 6)).unwrap_or(0);
    let spans: Vec<Span<'static>> = text
        .chars()
        .enumerate()
        .map(|(index, ch)| {
            let hot = index.abs_diff(phase) <= 1;
            let color = if hot { theme.text } else { theme.accent };
            Span::styled(ch.to_string(), theme.bold(color))
        })
        .collect();
    Line::from(spans)
}

// ---------------------------------------------------------------- header

/// Where each tab sits on the tab row, as `(tab, first column, width)`.
/// Columns are relative to the header. The width includes the gap after the label.
pub(crate) fn tab_cells(width: u16) -> (Vec<(Tab, u16, u16)>, u16) {
    let base: u16 = Tab::ALL
        .iter()
        .map(|tab| u16::try_from(tab.label().chars().count()).unwrap_or(8) + 3)
        .sum::<u16>()
        + 1;
    let gap = (width.saturating_sub(base) / 9).min(3);
    let mut cells = Vec::with_capacity(Tab::ALL.len());
    let mut x = 0u16;
    for tab in Tab::ALL {
        let label = u16::try_from(tab.label().chars().count()).unwrap_or(8) + 3;
        cells.push((tab, x, label + gap));
        x += label + gap;
    }
    (cells, gap)
}

fn draw_header(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let lines = vec![
        title_line(model, area.width),
        tab_line(model, area.width),
        rule_line(model, area.width),
    ];
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(model.theme.background)),
        area,
    );
}

fn title_line(model: &Model, width: u16) -> Line<'static> {
    let theme = model.theme;
    let width = usize::from(width);
    let brand = format!("{} CASTELLAN", pick(model, "▟█▙", "[#]"));
    let tagline = "   the keeper of your machine";
    let divider = pick(model, "  │  ", "  |  ");
    let host: String = model.hostname.chars().take(20).collect();
    let score = match model.shown_score {
        Some(value) => value.to_string(),
        None => "--".to_owned(),
    };
    let heart = pick(model, "♥", "*");
    let status = status_word(model);
    let right_len = host.chars().count()
        + divider.chars().count() * 2
        + 2
        + score.chars().count()
        + 1
        + status.chars().count()
        + model.clock.chars().count();
    let with_tagline = brand.chars().count() + tagline.chars().count() + right_len + 2 <= width;
    let left_len = brand.chars().count()
        + if with_tagline {
            tagline.chars().count()
        } else {
            0
        };
    let gap = width.saturating_sub(left_len + right_len);
    let mut heart_style = theme.bold(score_color(model));
    if !model.reduce_motion && (model.elapsed() / 700) % 2 == 1 {
        heart_style = heart_style.add_modifier(Modifier::DIM);
    }
    let mut spans = vec![Span::styled(brand, theme.bold(theme.accent))];
    if with_tagline {
        spans.push(Span::styled(tagline, theme.fg(theme.muted)));
    }
    spans.extend([
        Span::raw(" ".repeat(gap)),
        Span::styled(host, theme.fg(theme.text)),
        Span::styled(divider, theme.fg(theme.faint)),
        Span::styled(format!("{heart} "), heart_style),
        Span::styled(score, theme.bold(score_color(model))),
        Span::styled(format!(" {status}"), theme.fg(theme.muted)),
        Span::styled(divider, theme.fg(theme.faint)),
        Span::styled(model.clock.clone(), theme.fg(theme.muted)),
    ]);
    Line::from(spans)
}

fn tab_line(model: &Model, width: u16) -> Line<'static> {
    let theme = model.theme;
    let (cells, gap) = tab_cells(width);
    let selected = theme.selected();
    let mut spans = Vec::new();
    let mut after_active = false;
    for (tab, _, _) in cells {
        let active = !model.settings && tab == model.tab;
        let lead = if active || (after_active && gap == 0) {
            selected
        } else {
            Style::default()
        };
        spans.push(Span::styled(" ", lead));
        if active {
            spans.push(Span::styled(
                format!("{} {}", tab.number(), tab.label()),
                selected,
            ));
        } else {
            spans.push(Span::styled(
                tab.number().to_string(),
                theme.bold(theme.accent),
            ));
            spans.push(Span::styled(
                format!(" {}", tab.label()),
                theme.fg(theme.muted),
            ));
        }
        if gap > 0 {
            let trail = if active { selected } else { Style::default() };
            spans.push(Span::styled(" ", trail));
            spans.push(Span::raw(" ".repeat(usize::from(gap - 1))));
        }
        after_active = active;
    }
    if after_active && gap == 0 {
        spans.push(Span::styled(" ", selected));
    }
    Line::from(spans)
}

fn rule_line(model: &Model, width: u16) -> Line<'static> {
    let theme = model.theme;
    let width = usize::from(width);
    let thin = pick(model, "─", "-");
    let thick = pick(model, "━", "=");
    let (cells, gap) = tab_cells(u16::try_from(width).unwrap_or(u16::MAX));
    let active = cells
        .into_iter()
        .find(|(tab, _, _)| !model.settings && *tab == model.tab);
    let Some((_, start, len)) = active else {
        return Line::from(Span::styled(thin.repeat(width), theme.fg(theme.border)));
    };
    // The mark covers the label and one cell on each side, like the tab itself.
    let full = usize::from(len.saturating_sub(gap)) + 1;
    let shrink = full * usize::try_from(model.entering()).unwrap_or(0) / 2_000;
    let from = (usize::from(start) + shrink).min(width);
    let to = (usize::from(start) + full)
        .saturating_sub(shrink)
        .clamp(from, width);
    Line::from(vec![
        Span::styled(thin.repeat(from), theme.fg(theme.border)),
        Span::styled(thick.repeat(to - from), theme.fg(theme.accent)),
        Span::styled(thin.repeat(width - to), theme.fg(theme.border)),
    ])
}

/// One word for the state of the score: a band, `partial`, or `no reading`.
pub(crate) fn status_word(model: &Model) -> String {
    let Some(witness) = model.witness.as_ref() else {
        return "no reading".to_owned();
    };
    if witness.show_band {
        witness
            .score
            .band
            .map(Band::label)
            .unwrap_or("partial")
            .to_lowercase()
    } else if witness.score.keep_score.is_some() {
        "partial".to_owned()
    } else {
        "no reading".to_owned()
    }
}

pub(crate) fn score_color(model: &Model) -> ratatui::style::Color {
    match model.shown_score {
        Some(value) => model
            .theme
            .score(u8::try_from(value.min(100)).unwrap_or(100)),
        None => model.theme.muted,
    }
}

pub(crate) fn severity_color(model: &Model, severity: Severity) -> ratatui::style::Color {
    let theme = model.theme;
    match severity {
        Severity::Info => theme.info,
        Severity::Notice => theme.steel,
        Severity::Warning => theme.warn,
        Severity::Critical => theme.critical,
    }
}

// ---------------------------------------------------------------- footer

fn footer(model: &Model, width: u16) -> Paragraph<'static> {
    let theme = model.theme;
    let width = usize::from(width);
    let arrows = pick(model, "← →", "<- ->");
    let updown = pick(model, "↑ ↓", "j k");
    // (key, label, priority). Lower priorities are dropped first when it is tight.
    let mut hints: Vec<(&str, &str, u8)> = Vec::new();
    if model.palette {
        hints.extend([
            ("type", "filter", 9),
            (updown, "move", 8),
            ("enter", "go", 9),
            ("esc", "close", 9),
        ]);
    } else if model.help {
        hints.extend([("esc", "close help", 9), ("q", "quit", 8)]);
    } else if model.settings {
        hints.push((updown, "move", 9));
        if model.settings_cursor + 1 == SETTING_ROWS {
            hints.push((arrows, "change hours", 9));
        } else {
            hints.push(("space", "switch on or off", 9));
        }
        hints.extend([("esc", "back", 9), ("?", "help", 5), ("q", "quit", 6)]);
    } else {
        hints.extend([("1-9", "rooms", 9), (arrows, "next room", 7)]);
        let rows = match model.tab {
            Tab::Sweep => usize::from(SWEEP_ROWS),
            Tab::Overview => model
                .witness
                .as_ref()
                .map_or(0, |witness| witness.attention.len()),
            _ => 0,
        };
        if rows > 1 {
            hints.push((updown, "move", 8));
        }
        if rows > 0 {
            hints.push(("enter", "open", 8));
        }
        hints.extend([
            ("s", "settings", 6),
            (":", "go to", 3),
            ("?", "help", 7),
            ("q", "quit", 9),
        ]);
    }
    let status = if model.facts.dev_build {
        "dev build · changes nothing"
    } else {
        "preview · changes nothing"
    };
    let status = if model.ascii {
        status.replace('·', "-")
    } else {
        status.to_owned()
    };
    let cost = |hints: &[(&str, &str, u8)]| -> usize {
        hints
            .iter()
            .map(|(key, label, _)| key.chars().count() + label.chars().count() + 4)
            .sum()
    };
    let mut show_status = true;
    while cost(&hints)
        + if show_status {
            status.chars().count()
        } else {
            0
        }
        > width
    {
        if show_status {
            show_status = false;
            continue;
        }
        let Some(weakest) = hints
            .iter()
            .enumerate()
            .min_by_key(|(_, hint)| hint.2)
            .map(|(index, _)| index)
        else {
            break;
        };
        hints.remove(weakest);
    }
    let used = cost(&hints);
    let mut spans = Vec::new();
    for (key, label, _) in hints {
        spans.push(Span::styled(key.to_owned(), theme.bold(theme.accent)));
        spans.push(Span::styled(format!(" {label}   "), theme.fg(theme.muted)));
    }
    if show_status {
        spans.push(Span::styled(
            fit_right(&status, width.saturating_sub(used)),
            theme.fg(theme.faint),
        ));
    }
    Paragraph::new(Line::from(spans)).style(Style::default().bg(theme.background))
}

// ---------------------------------------------------------------- settings

/// One visual line of the settings list.
#[derive(Clone, Copy)]
enum SettingLine {
    Heading(&'static str),
    Row(u8),
    Gap,
}

const SETTING_LINES: [SettingLine; 11] = [
    SettingLine::Heading("Sign-in cleanup · saved, not active yet"),
    SettingLine::Row(0),
    SettingLine::Row(1),
    SettingLine::Gap,
    SettingLine::Heading("Display"),
    SettingLine::Row(2),
    SettingLine::Row(3),
    SettingLine::Gap,
    SettingLine::Heading("Safety"),
    SettingLine::Row(4),
    SettingLine::Row(5),
];

struct Setting {
    label: &'static str,
    value: String,
    on: Option<bool>,
    what: &'static str,
    now: &'static str,
}

fn settings(model: &Model) -> [Setting; 6] {
    let prefs = &model.preferences;
    let switch = |on: bool| (if on { "on" } else { "off" }.to_owned(), Some(on));
    let wish = "Not active yet. This build saves your choice, but it does not register \
                a task and it does not delete anything.";
    let (user, user_on) = switch(prefs.user_temp_at_sign_in);
    let (windows, windows_on) = switch(prefs.windows_temp_at_sign_in);
    let (motion, motion_on) = switch(!prefs.reduce_motion);
    let (ascii, ascii_on) = switch(prefs.ascii_art);
    let (lock, lock_on) = switch(prefs.lockdown);
    [
        Setting {
            label: "Clean user temp at sign-in",
            value: user,
            on: user_on,
            what: "Asks Castellan to clear old files from your temporary folder every time \
                   you sign in to Windows.",
            now: wish,
        },
        Setting {
            label: "Clean Windows temp at sign-in",
            value: windows,
            on: windows_on,
            what: "The same, for the temporary folder that belongs to Windows. It will need \
                   administrator approval.",
            now: wish,
        },
        Setting {
            label: "Animations",
            value: motion,
            on: motion_on,
            what: "The opening screen and the easing of numbers and meters. Turn it off for \
                   a still screen.",
            now: "Applies right away.",
        },
        Setting {
            label: "Plain ASCII drawing",
            value: ascii,
            on: ascii_on,
            what: "Draws with basic characters only. Use it if boxes or bars look broken in \
                   your terminal.",
            now: "Applies right away.",
        },
        Setting {
            label: "Read-only lock",
            value: lock,
            on: lock_on,
            what: "A kill switch. When it is on, Castellan refuses every change to the \
                   system, whatever else is set.",
            now: "This build is already read-only, so the lock changes nothing yet.",
        },
        Setting {
            label: "Minimum file age for a sweep",
            value: format!("{} h", prefs.min_age_secs / 3_600),
            on: None,
            what: "A temporary file must be at least this old before a sweep may touch it. \
                   This protects installers that are still running.",
            now: "Change it with the left and right arrows. It can never go below 1 hour.",
        },
    ]
}

fn setting_columns(area: Rect) -> (Rect, Rect) {
    let detail = (area.width * 2 / 5).clamp(30, 52);
    let cols = Layout::horizontal([
        Constraint::Min(40),
        Constraint::Length(1),
        Constraint::Length(detail),
    ])
    .split(area);
    (nth(&cols, 0), nth(&cols, 2))
}

fn draw_settings(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let theme = model.theme;
    let (list_area, detail_area) = setting_columns(area);
    let rows = settings(model);
    let (width, _) = inner(list_area);
    let mut lines = Vec::new();
    for line in SETTING_LINES {
        match line {
            SettingLine::Gap => lines.push(blank()),
            SettingLine::Heading(text) => {
                let text = if model.ascii {
                    text.replace('·', "-")
                } else {
                    text.to_owned()
                };
                lines.push(heading(model, &text));
            }
            SettingLine::Row(index) => {
                if let Some(row) = rows.get(usize::from(index)) {
                    lines.push(setting_row(
                        model,
                        row,
                        width,
                        index == model.settings_cursor,
                    ));
                }
            }
        }
    }
    lines.push(blank());
    lines.extend(para(
        "Choices are saved to preferences.toml in your local app data folder, and they are loaded the next time Castellan opens.",
        width,
        0,
        theme.fg(theme.muted),
    ));
    if !model.settings_note.is_empty() {
        lines.push(blank());
        lines.extend(bullet(
            pick(model, "✓", "+"),
            theme.bold(theme.ok),
            &model.settings_note,
            width,
            theme.fg(theme.muted),
        ));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(card(model, "Settings", true))
            .style(theme.ink()),
        list_area,
    );

    let (width, _) = inner(detail_area);
    let mut lines = Vec::new();
    if let Some(row) = rows.get(usize::from(model.settings_cursor)) {
        lines.extend(para(row.label, width, 0, theme.bold(theme.text)));
        lines.push(blank());
        lines.push(heading(model, "What it does"));
        lines.extend(para(row.what, width, 0, theme.fg(theme.text)));
        lines.push(blank());
        lines.push(heading(model, "Right now"));
        lines.extend(para(row.now, width, 0, theme.fg(theme.muted)));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(card(model, "About this setting", false))
            .style(theme.ink()),
        detail_area,
    );
}

fn setting_row(model: &Model, row: &Setting, width: usize, selected: bool) -> Line<'static> {
    let theme = model.theme;
    let base = crate::ui::row_style(model, selected);
    let marker = if selected {
        pick(model, "▸ ", "> ")
    } else {
        "  "
    };
    let (value, value_style) = match row.on {
        Some(true) => (
            format!("{} {}", pick(model, "●", "[x]"), row.value),
            theme.bold(theme.ok),
        ),
        Some(false) => (
            format!("{} {}", pick(model, "○", "[ ]"), row.value),
            theme.fg(theme.muted),
        ),
        None => (
            format!(
                "{} {} {}",
                pick(model, "‹", "<"),
                row.value,
                pick(model, "›", ">")
            ),
            theme.bold(theme.steel),
        ),
    };
    let value = fit_right(&value, 9);
    let label_width = width.saturating_sub(2 + value.chars().count());
    let label_style = if selected {
        theme.bold(theme.text)
    } else {
        theme.fg(theme.text)
    };
    Line::from(vec![
        Span::styled(marker, theme.bold(theme.accent).patch(base)),
        Span::styled(fit(row.label, label_width), label_style.patch(base)),
        Span::styled(value, value_style.patch(base)),
    ])
}

// ---------------------------------------------------------------- overlays

fn overlay(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width.saturating_sub(4)).max(20);
    let height = height.min(area.height.saturating_sub(2)).max(5);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

fn draw_toast(model: &Model, frame: &mut Frame<'_>, body: Rect, text: &str) {
    let theme = model.theme;
    let longest = u16::try_from(text.chars().count()).unwrap_or(u16::MAX);
    let width = longest
        .saturating_add(4)
        .clamp(24, 64)
        .min(body.width.saturating_sub(2));
    let rows = wrap(text, usize::from(width.saturating_sub(4)));
    let height = u16::try_from(rows.len()).unwrap_or(3).saturating_add(2);
    let area = Rect {
        x: body.x + body.width.saturating_sub(width + 2),
        y: body.y + body.height.saturating_sub(height + 1),
        width,
        height: height.min(body.height.saturating_sub(1)),
    };
    let lines: Vec<Line<'static>> = rows
        .into_iter()
        .map(|row| Line::from(Span::styled(row, theme.fg(theme.text))))
        .collect();
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(card(model, "Castellan", true))
            .style(Style::default().fg(theme.text).bg(theme.raised)),
        area,
    );
}

fn key_line(model: &Model, key: &str, text: &str) -> Line<'static> {
    let theme = model.theme;
    Line::from(vec![
        Span::styled(fit(key, 15), theme.bold(theme.accent)),
        Span::styled(text.to_owned(), theme.fg(theme.text)),
    ])
}

fn word_line(model: &Model, word: &str, text: &str) -> Line<'static> {
    let theme = model.theme;
    Line::from(vec![
        Span::styled(fit(word, 15), theme.bold(theme.text)),
        Span::styled(text.to_owned(), theme.fg(theme.muted)),
    ])
}

fn draw_help(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let arrows = pick(model, "← →  Tab", "<- ->  Tab");
    let updown = pick(model, "↑ ↓  j k", "Up Down  j k");
    let lines = vec![
        heading(model, "Move around"),
        key_line(model, "1 to 9", "open a room"),
        key_line(model, arrows, "next or previous room"),
        key_line(model, updown, "move inside a list"),
        key_line(model, "Enter", "open what is selected"),
        key_line(model, ":  Ctrl+P", "go to a room by typing its name"),
        blank(),
        heading(model, "Anywhere"),
        key_line(model, "s", "settings"),
        key_line(model, "?", "this help"),
        key_line(model, "Esc", "close a window, or go back"),
        key_line(model, "q", "quit"),
        blank(),
        heading(model, "Words used here"),
        word_line(model, "Keep Score", "health of this PC from 0 to 100"),
        word_line(model, "Area", "one of five parts of the score"),
        word_line(model, "Partial", "some areas are not checked yet"),
        word_line(model, "Finding", "one measured problem and its proof"),
        word_line(model, "Sweep", "clearing old temporary files"),
    ];
    let height = u16::try_from(lines.len()).unwrap_or(20) + 2;
    let area = overlay(area, 62, height);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(card(model, "Help", true))
            .style(model.theme.ink()),
        area,
    );
}

fn draw_palette(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let theme = model.theme;
    let area = overlay(area, 44, 16);
    let (width, _) = inner(area);
    let hits = palette_hits(&model.palette_query);
    let mut lines = vec![
        Line::from(vec![
            Span::styled(pick(model, "› ", "> "), theme.bold(theme.accent)),
            Span::styled(model.palette_query.clone(), theme.bold(theme.text)),
            Span::styled(pick(model, "▏", "_"), theme.fg(theme.accent)),
        ]),
        Line::from(Span::styled(
            pick(model, "─", "-").repeat(width),
            theme.fg(theme.border),
        )),
    ];
    if hits.is_empty() {
        lines.push(Line::from(Span::styled(
            "No room has that name.",
            theme.fg(theme.muted),
        )));
    }
    for (index, (label, _)) in hits.iter().enumerate() {
        let mut name: Vec<char> = label.chars().collect();
        if let Some(first) = name.first_mut() {
            *first = first.to_ascii_uppercase();
        }
        let name: String = name.into_iter().collect();
        let text = fit(&format!("  {name}"), width);
        let style = if index == usize::from(model.palette_cursor) {
            theme.selected()
        } else {
            theme.fg(theme.text)
        };
        lines.push(Line::from(Span::styled(text, style)));
    }
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .block(card(model, "Go to", true))
            .style(theme.ink()),
        area,
    );
}

// ---------------------------------------------------------------- pointer

/// Where a mouse click lands. `column` and `row` are terminal cells.
///
/// A click on a tab opens that room. A click on a sweep target or a settings
/// row selects it.
#[must_use]
pub fn pointer(model: &Model, width: u16, height: u16, column: u16, row: u16) -> Option<Msg> {
    if width < MIN_W || height < MIN_H {
        return None;
    }
    if model.splash {
        return Some(Msg::Key(Key::Enter));
    }
    if model.help || model.palette {
        return None;
    }
    let parts = parts(Rect::new(0, 0, width, height));
    if row == parts.header.y + 1 {
        let at = column.checked_sub(parts.header.x)?;
        let (cells, _) = tab_cells(parts.header.width);
        return cells
            .into_iter()
            .find(|(_, start, len)| at >= *start && at < start + len)
            .map(|(tab, _, _)| Msg::Key(Key::Digit(tab.number())));
    }
    let line = usize::from(row.checked_sub(parts.body.y + 1)?);
    if model.settings {
        let (list, _) = setting_columns(parts.body);
        if column < list.x || column >= list.x + list.width {
            return None;
        }
        return match SETTING_LINES.get(line) {
            Some(SettingLine::Row(index)) => Some(Msg::Select(*index)),
            _ => None,
        };
    }
    if model.tab == Tab::Sweep {
        let list = rooms::sweep_list(parts.body);
        if column < list.x || column >= list.x + list.width {
            return None;
        }
        let index = u8::try_from(line / 3).unwrap_or(u8::MAX);
        if index < SWEEP_ROWS && line % 3 != 2 {
            return Some(Msg::Select(index));
        }
    }
    None
}

/// Centered emblem lines for a room, or nothing in ASCII mode.
pub(crate) fn emblem_lines(model: &Model, tab: Tab, width: usize) -> Vec<Line<'static>> {
    art::emblem(tab, model.ascii)
        .iter()
        .map(|row| {
            Line::from(Span::styled(
                center(row, width),
                model.theme.fg(model.theme.accent),
            ))
        })
        .collect()
}
