//! The nine rooms. Each one fills the body area that [`crate::view`] hands it.
//!
//! Every number on these screens was measured. A room that cannot measure
//! anything yet says so and explains what it will do.

use castellan_core::{Pillar, Severity, format_bytes};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::model::{Model, SWEEP_ROWS, SweepReading, Tab};
use crate::ui::{
    big_number, blank, bullet, card, center, chart, fit, fit_right, heading, inner, meter, nth,
    para, percent, pick, row_style, span_text, spark, thousands,
};
use crate::view::{emblem_lines, score_color, severity_color, status_word};

/// Draw the current room into `area`.
pub(crate) fn draw(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    match model.tab {
        Tab::Overview => overview(model, frame, area),
        Tab::Sweep => sweep(model, frame, area),
        Tab::Vitals => vitals(model, frame, area),
        tab => plan_room(model, frame, area, tab),
    }
}

fn show(
    model: &Model,
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    lit: bool,
    lines: Vec<Line<'static>>,
) {
    frame.render_widget(
        Paragraph::new(lines)
            .block(card(model, title, lit))
            .style(model.theme.ink()),
        area,
    );
}

fn pillar_name(pillar: Pillar) -> &'static str {
    match pillar {
        Pillar::Storage => "Storage",
        Pillar::Stability => "Stability",
        Pillar::Performance => "Performance",
        Pillar::Security => "Security",
        Pillar::Network => "Network",
    }
}

fn pillar_score(model: &Model, pillar: Pillar) -> Option<u8> {
    model
        .witness
        .as_ref()?
        .score
        .pillars
        .iter()
        .find(|item| item.pillar == pillar)
        .and_then(|item| item.score)
}

// ---------------------------------------------------------------- overview

fn overview(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let left = if area.width >= 120 { 40 } else { 36 };
    let cols = Layout::horizontal([
        Constraint::Length(left),
        Constraint::Length(1),
        Constraint::Min(30),
    ])
    .split(area);
    score_card(model, frame, nth(&cols, 0));
    let right = nth(&cols, 2);
    // The attention card takes what its text needs. The live card gets the rest.
    let (width, _) = inner(right);
    let floor = 8u16.min(right.height / 2);
    let most = usize::from(right.height.saturating_sub(floor + 2));
    let wanted = attention_lines(model, width, most).len() + 2;
    let wanted = u16::try_from(wanted).unwrap_or(u16::MAX);
    let attention_h = wanted.clamp(6, right.height.saturating_sub(floor));
    let rows =
        Layout::vertical([Constraint::Length(attention_h), Constraint::Min(floor)]).split(right);
    attention_card(model, frame, nth(&rows, 0));
    now_card(model, frame, nth(&rows, 1));
}

fn score_card(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let theme = model.theme;
    let (width, height) = inner(area);
    let number = match model.shown_score {
        Some(value) => value.to_string(),
        None => "--".to_owned(),
    };
    let color = score_color(model);
    let mut top = vec![blank()];
    for row in big_number(model, &number) {
        top.push(Line::from(Span::styled(
            center(&row, width),
            theme.bold(color),
        )));
    }
    let checked = Pillar::all()
        .into_iter()
        .filter(|pillar| pillar_score(model, *pillar).is_some())
        .count();
    let dot = pick(model, " · ", " - ");
    let status = if model.witness.is_some() {
        format!("{}{dot}{checked} of 5 areas checked", status_word(model))
    } else {
        "no reading yet".to_owned()
    };
    top.push(Line::from(Span::styled(
        center(&status, width),
        theme.fg(theme.muted),
    )));
    let explain = if model.witness.is_none() {
        "Health of this PC from 0 to 100. No area could be read yet, so there is no number."
    } else if checked < 5 {
        "Health of this PC from 0 to 100. Only part of it has been checked, so read this as a first look, not a verdict."
    } else {
        "Health of this PC from 0 to 100, built from the five areas below."
    };
    let explain = para(explain, width, 0, theme.fg(theme.muted));

    let mut areas = vec![heading(model, "Areas")];
    let name_w = 12usize;
    let value_w = 4usize;
    for pillar in Pillar::all() {
        let name = Span::styled(fit(pillar_name(pillar), name_w), theme.fg(theme.text));
        match pillar_score(model, pillar) {
            Some(value) => {
                let shown = model.gauge(pillar).min(100);
                let mut spans = vec![name];
                spans.extend(meter(
                    model,
                    u32::from(shown) * 10,
                    width.saturating_sub(name_w + value_w),
                    theme.score(value),
                ));
                spans.push(Span::styled(
                    fit_right(&value.to_string(), value_w),
                    theme.bold(theme.score(value)),
                ));
                areas.push(Line::from(spans));
            }
            None => areas.push(Line::from(vec![
                name,
                Span::styled(
                    format!("{} not checked yet", pick(model, "·", "-")),
                    theme.fg(theme.faint),
                ),
            ])),
        }
    }

    let mut lines = top;
    let fixed = lines.len() + areas.len() + 1;
    if fixed + explain.len() < height {
        lines.push(blank());
        lines.extend(explain);
    }
    lines.push(blank());
    lines.extend(areas);
    let legend = bands_legend(model, width);
    if lines.len() + legend.len() < height {
        let spare = height - lines.len() - legend.len();
        lines.extend(std::iter::repeat_n(blank(), spare));
        lines.extend(legend);
    }
    show(model, frame, area, "Keep Score", true, lines);
}

/// What the score ranges mean, colored like the score itself.
fn bands_legend(model: &Model, width: usize) -> Vec<Line<'static>> {
    let theme = model.theme;
    let mark = pick(model, "■", "#");
    let item = |range: &str, name: &str, value: u8| {
        vec![
            Span::styled(format!("{mark} "), theme.fg(theme.score(value))),
            Span::styled(format!("{range:<7}"), theme.bold(theme.text)),
            Span::styled(name.to_owned(), theme.fg(theme.muted)),
        ]
    };
    let rows = [
        item("90-100", "Fortified", 95),
        item("75-89", "Sound", 80),
        item("50-74", "Needs attention", 60),
        item("0-49", "At risk", 20),
    ];
    let mut lines = vec![heading(model, "What the number means")];
    if width >= 36 {
        let mut iter = rows.into_iter();
        while let Some(mut left) = iter.next() {
            let used: usize = left.iter().map(Span::width).sum();
            left.push(Span::raw(" ".repeat(18usize.saturating_sub(used))));
            if let Some(right) = iter.next() {
                left.extend(right);
            }
            lines.push(Line::from(left));
        }
    } else {
        lines.extend(rows.into_iter().map(Line::from));
    }
    lines
}

fn severity_mark(model: &Model, severity: Severity) -> &'static str {
    match severity {
        Severity::Info => pick(model, "●", "i"),
        Severity::Notice => pick(model, "●", "+"),
        Severity::Warning => pick(model, "▲", "!"),
        Severity::Critical => pick(model, "◆", "X"),
    }
}

fn attention_card(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let (width, height) = inner(area);
    let lines = attention_lines(model, width, height);
    show(model, frame, area, "Needs your attention", false, lines);
}

fn attention_lines(model: &Model, width: usize, height: usize) -> Vec<Line<'static>> {
    let theme = model.theme;
    let mut lines = Vec::new();
    match model.witness.as_ref() {
        None => {
            lines.extend(bullet(
                pick(model, "○", "o"),
                theme.fg(theme.muted),
                &model.persona,
                width,
                theme.fg(theme.text),
            ));
        }
        Some(witness) if witness.attention.is_empty() => {
            lines.extend(bullet(
                pick(model, "✓", "+"),
                theme.bold(theme.ok),
                "Nothing that was measured needs a repair.",
                width,
                theme.bold(theme.text),
            ));
            lines.extend(para(&witness.persona, width, 2, theme.fg(theme.muted)));
        }
        Some(witness) => {
            for (index, item) in witness.attention.iter().enumerate() {
                let selected = index == usize::from(model.cursor);
                let color = severity_color(model, item.severity);
                let base = row_style(model, selected);
                let marker = if selected {
                    pick(model, "▸", ">")
                } else {
                    " "
                };
                let title = fit(&item.title, width.saturating_sub(4));
                lines.push(Line::from(vec![
                    Span::styled(format!("{marker} "), theme.bold(theme.accent).patch(base)),
                    Span::styled(
                        format!("{} ", severity_mark(model, item.severity)),
                        theme.bold(color).patch(base),
                    ),
                    Span::styled(title, theme.bold(theme.text).patch(base)),
                ]));
                lines.extend(para(&item.detail, width, 4, theme.bold(color)));
                lines.extend(para(&item.advice, width, 4, theme.fg(theme.muted)));
                if selected && let Some(room) = item.room {
                    lines.push(Line::from(vec![
                        Span::raw("    "),
                        Span::styled("Enter", theme.bold(theme.accent)),
                        Span::styled(format!(" opens {}", room.label()), theme.fg(theme.muted)),
                    ]));
                }
                lines.push(blank());
            }
        }
    }
    let dark: Vec<&str> = Pillar::all()
        .into_iter()
        .filter(|pillar| pillar_score(model, *pillar).is_none())
        .map(pillar_name)
        .collect();
    if !dark.is_empty() {
        let names = match dark.as_slice() {
            [one] => (*one).to_owned(),
            [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
            [] => String::new(),
        };
        let verb = if dark.len() == 1 { "has" } else { "have" };
        let text = format!(
            "{names} {verb} no reading yet. Those checks are not built, so nothing is reported for them."
        );
        let block = para(&text, width, 0, theme.fg(theme.muted));
        if lines.len() + block.len() + 2 <= height {
            if !lines.last().is_some_and(|line| line.width() == 0) {
                lines.push(blank());
            }
            lines.push(heading(model, "Not checked yet"));
            lines.extend(block);
        }
    }
    while lines.last().is_some_and(|line| line.width() == 0) {
        lines.pop();
    }
    lines
}

/// One labeled reading: `label  meter  value  detail`.
fn reading(
    model: &Model,
    label: &str,
    share: Option<u8>,
    value: &str,
    detail: Vec<Span<'static>>,
    width: usize,
) -> Line<'static> {
    let theme = model.theme;
    let label_w = 10usize;
    let value_w = 6usize;
    let mut spans = vec![Span::styled(fit(label, label_w), theme.fg(theme.text))];
    let Some(share) = share else {
        spans.push(Span::styled(value.to_owned(), theme.fg(theme.faint)));
        return Line::from(spans);
    };
    let rest = width.saturating_sub(label_w + value_w);
    let bar_w = (rest / 2).clamp(8, 26);
    spans.extend(meter(
        model,
        u32::from(share) * 10,
        bar_w,
        theme.usage(share),
    ));
    spans.push(Span::styled(
        fit_right(value, value_w),
        theme.bold(theme.usage(share)),
    ));
    let detail_w = rest.saturating_sub(bar_w + 2);
    if detail_w >= 6 {
        spans.push(Span::raw("  "));
        let mut left = detail_w;
        for span in detail {
            let text = fit(&span.content, left.min(span.width()));
            left = left.saturating_sub(text.chars().count());
            spans.push(Span::styled(text, span.style));
        }
    }
    Line::from(spans)
}

fn now_card(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let theme = model.theme;
    let (width, height) = inner(area);
    let detail_w = width.saturating_sub(16 + (width.saturating_sub(16) / 2).clamp(8, 26) + 2);
    let mut lines = Vec::new();
    match model.cpu_percent {
        Some(cpu) => {
            let line = spark(model, model.cpu_history.iter(), detail_w);
            let detail = vec![Span::styled(line, theme.fg(theme.steel))];
            lines.push(reading(
                model,
                "Processor",
                Some(cpu),
                &format!("{cpu}%"),
                detail,
                width,
            ));
        }
        None => lines.push(reading(
            model,
            "Processor",
            None,
            "measuring…",
            Vec::new(),
            width,
        )),
    }
    match model.facts.memory {
        Some(memory) => {
            let used = memory.total_bytes.saturating_sub(memory.available_bytes);
            let share = percent(used, memory.total_bytes);
            let detail = vec![Span::styled(
                format!(
                    "{} of {}",
                    format_bytes(used),
                    format_bytes(memory.total_bytes)
                ),
                theme.fg(theme.muted),
            )];
            lines.push(reading(
                model,
                "Memory",
                Some(share),
                &format!("{share}%"),
                detail,
                width,
            ));
        }
        None => lines.push(reading(
            model,
            "Memory",
            None,
            "not read",
            Vec::new(),
            width,
        )),
    }
    // The Windows drive first, then as many others as fit above the uptime and hint.
    let reserved = 1 + if height >= 6 { 2 } else { 0 };
    let mut drives: Vec<_> = model.facts.drives.iter().collect();
    drives.sort_by_key(|drive| !drive.system);
    let room = height.saturating_sub(lines.len() + reserved).max(1);
    for drive in drives.into_iter().take(room) {
        let used = drive.total_bytes.saturating_sub(drive.free_bytes);
        let share = percent(used, drive.total_bytes);
        let label = format!("Drive {}", drive.label);
        let mut detail = vec![Span::styled(
            format!("{} free", format_bytes(drive.free_bytes)),
            theme.fg(theme.muted),
        )];
        if drive.system {
            detail.push(Span::styled("  Windows", theme.fg(theme.accent)));
        }
        lines.push(reading(
            model,
            &label,
            Some(share),
            &format!("{share}%"),
            detail,
            width,
        ));
    }
    if let Some(uptime) = model.facts.uptime_secs {
        let when = span_text(uptime);
        let tail = if 10 + when.chars().count() + 22 <= width {
            " since Windows started"
        } else {
            " since start"
        };
        lines.push(Line::from(vec![
            Span::styled(fit("Running", 10), theme.fg(theme.text)),
            Span::styled(when, theme.bold(theme.steel)),
            Span::styled(tail, theme.fg(theme.muted)),
        ]));
    }
    if lines.len() + 2 <= height {
        let hint = if width >= 48 {
            " opens Vitals, with live graphs and every drive."
        } else {
            " opens Vitals for more."
        };
        lines.push(blank());
        lines.push(Line::from(vec![
            Span::styled("3", theme.bold(theme.accent)),
            Span::styled(hint, theme.fg(theme.muted)),
        ]));
    }
    show(model, frame, area, "This PC right now", false, lines);
}

// ---------------------------------------------------------------- sweep

/// The list column of the sweep room. The pointer uses it too.
pub(crate) fn sweep_list(body: Rect) -> Rect {
    let width = if body.width >= 110 { 36 } else { 30 };
    let cols = Layout::horizontal([Constraint::Length(width), Constraint::Min(20)]).split(body);
    nth(&cols, 0)
}

struct Target {
    name: &'static str,
    what: &'static str,
}

const TARGETS: [Target; 3] = [
    Target {
        name: "User temp",
        what: "Programs keep working files here while they run, and many forget to remove them afterwards. Old files in this folder are safe to delete.",
    },
    Target {
        name: "Windows temp",
        what: "Windows, drivers, and installers keep their working files here. Cleaning it needs administrator approval.",
    },
    Target {
        name: "Prefetch",
        what: "Windows writes down how your apps start and uses those notes to start them faster next time. Deleting them only makes the next starts slower, and Windows writes them again anyway.",
    },
];

fn reading_summary(model: &Model, reading: &SweepReading) -> (String, String, Color) {
    let theme = model.theme;
    match reading {
        SweepReading::Measuring => (String::new(), "measuring…".to_owned(), theme.muted),
        SweepReading::Unavailable(_) => (String::new(), "could not be read".to_owned(), theme.warn),
        SweepReading::Measured(size) => {
            let plus = if size.truncated { "+" } else { "" };
            (
                format!("{}{plus}", format_bytes(size.bytes)),
                format!("{} old enough", format_bytes(size.aged_bytes)),
                theme.steel,
            )
        }
    }
}

fn sweep(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let theme = model.theme;
    let list_area = sweep_list(area);
    let detail_area = Rect {
        x: list_area.x + list_area.width + 1,
        width: area.width.saturating_sub(list_area.width + 1),
        ..area
    };
    let (width, height) = inner(list_area);
    let mut lines = Vec::new();
    for index in 0..SWEEP_ROWS {
        let selected = index == model.cursor;
        let base = row_style(model, selected);
        let marker = if selected {
            pick(model, "▸ ", "> ")
        } else {
            "  "
        };
        let target = TARGETS.get(usize::from(index));
        let name = target.map_or("", |target| target.name);
        let (size, sub, color) = match model.sweep.get(usize::from(index)) {
            Some(reading) => reading_summary(model, reading),
            None => (
                "never".to_owned(),
                "refused, see why".to_owned(),
                theme.muted,
            ),
        };
        let size_w = size.chars().count() + 1;
        lines.push(Line::from(vec![
            Span::styled(marker, theme.bold(theme.accent).patch(base)),
            Span::styled(
                fit(name, width.saturating_sub(2 + size_w)),
                theme.bold(theme.text).patch(base),
            ),
            Span::styled(fit_right(&size, size_w), theme.bold(color).patch(base)),
        ]));
        lines.push(Line::from(Span::styled(
            fit(&format!("  {sub}"), width),
            theme.fg(theme.muted).patch(base),
        )));
        lines.push(blank());
    }
    let measured: Vec<_> = model
        .sweep
        .iter()
        .filter_map(|reading| match reading {
            SweepReading::Measured(size) => Some(size.aged_bytes),
            _ => None,
        })
        .collect();
    if !measured.is_empty() && lines.len() + 4 <= height {
        let total: u64 = measured.iter().sum();
        lines.push(Line::from(Span::styled(
            pick(model, "─", "-").repeat(width),
            theme.fg(theme.border),
        )));
        lines.push(Line::from(Span::styled(
            "A sweep could free about",
            theme.fg(theme.muted),
        )));
        lines.push(Line::from(Span::styled(
            format_bytes(total),
            theme.bold(theme.ok),
        )));
    }
    show(model, frame, list_area, "What can be cleaned", true, lines);
    sweep_detail(model, frame, detail_area);
}

fn sweep_detail(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let theme = model.theme;
    let (width, height) = inner(area);
    let index = usize::from(model.cursor);
    let Some(target) = TARGETS.get(index) else {
        return;
    };
    let hours = model.preferences.min_age_secs / 3_600;
    let mut lines = vec![Line::from(Span::styled(
        target.name,
        theme.bold(theme.text),
    ))];
    lines.extend(para(target.what, width, 0, theme.fg(theme.muted)));
    lines.push(blank());
    match model.sweep.get(index) {
        None => {
            lines.push(heading(model, "Why Castellan never cleans it"));
            lines.extend(bullet(
                pick(model, "•", "*"),
                theme.fg(theme.accent),
                "Many cleaning scripts delete this folder at every start. That makes Windows slower, not faster.",
                width,
                theme.fg(theme.text),
            ));
            lines.extend(bullet(
                pick(model, "•", "*"),
                theme.fg(theme.accent),
                "Windows keeps the folder small on its own.",
                width,
                theme.fg(theme.text),
            ));
        }
        Some(SweepReading::Measuring) => {
            lines.push(heading(model, "Measured"));
            lines.push(Line::from(Span::styled(
                "Counting files. Nothing is opened for writing.",
                theme.fg(theme.muted),
            )));
        }
        Some(SweepReading::Unavailable(why)) => {
            lines.push(heading(model, "Measured"));
            lines.extend(para(why, width, 0, theme.fg(theme.warn)));
        }
        Some(SweepReading::Measured(size)) => {
            lines.push(heading(model, "Measured"));
            let label_w = 12usize;
            let row = |label: &str, bytes: u64, files: u64| {
                Line::from(vec![
                    Span::styled(fit(label, label_w), theme.fg(theme.text)),
                    Span::styled(fit(&format_bytes(bytes), 10), theme.bold(theme.text)),
                    Span::styled(
                        format!("in {} files", thousands(files)),
                        theme.fg(theme.muted),
                    ),
                ])
            };
            lines.push(row("Everything", size.bytes, size.files));
            lines.push(row("Old enough", size.aged_bytes, size.aged_files));
            let share = percent(size.aged_bytes, size.bytes);
            let mut bar = vec![Span::raw(" ".repeat(label_w))];
            bar.extend(meter(
                model,
                u32::from(share) * 10,
                width.saturating_sub(label_w + 6),
                theme.ok,
            ));
            bar.push(Span::styled(
                fit_right(&format!("{share}%"), 6),
                theme.fg(theme.muted),
            ));
            lines.push(Line::from(bar));
            if size.truncated {
                lines.extend(para(
                    "The count stopped early because the folder is very large. The real size is bigger.",
                    width,
                    0,
                    theme.fg(theme.warn),
                ));
            }
            lines.push(blank());
            lines.push(heading(model, "How a sweep decides"));
            for rule in [
                format!("Only files older than {hours} h, by both creation and last change."),
                "Files that another program has open are skipped.".to_owned(),
                "Links to other folders are never followed.".to_owned(),
                "Deleted temp files cannot be restored, which is why the rules are strict."
                    .to_owned(),
            ] {
                let block = bullet(
                    pick(model, "•", "*"),
                    theme.fg(theme.accent),
                    &rule,
                    width,
                    theme.fg(theme.text),
                );
                if lines.len() + block.len() > height {
                    break;
                }
                lines.extend(block);
            }
        }
    }
    let note = if index == 2 {
        "refused · always"
    } else {
        "preview · nothing is deleted"
    };
    let note = if model.ascii {
        note.replace('·', "-")
    } else {
        note.to_owned()
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                card(model, "Details", false)
                    .title_bottom(Span::styled(format!(" {note} "), theme.fg(theme.muted))),
            )
            .style(theme.ink()),
        area,
    );
}

// ---------------------------------------------------------------- vitals

fn vitals(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let rows =
        Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
    let top = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Length(1),
        Constraint::Min(10),
    ])
    .split(nth(&rows, 0));
    let bottom = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Length(1),
        Constraint::Min(10),
    ])
    .split(nth(&rows, 1));
    cpu_card(model, frame, nth(&top, 0));
    memory_card(model, frame, nth(&top, 2));
    drives_card(model, frame, nth(&bottom, 0));
    uptime_card(model, frame, nth(&bottom, 2));
}

/// Pin `tail` to the bottom of the card. A tail that does not fit whole is left
/// out, so a sentence is never cut in half.
fn fill_to(lines: &mut Vec<Line<'static>>, height: usize, tail: Vec<Line<'static>>) {
    if lines.len() + tail.len() + 1 > height {
        return;
    }
    let spare = height - lines.len() - tail.len();
    lines.extend(std::iter::repeat_n(blank(), spare));
    lines.extend(tail);
}

/// A big number on the left and a live chart beside it, then a caption row.
fn live_block(
    model: &Model,
    width: usize,
    value: u8,
    samples: &[u8],
    caption: &str,
) -> Vec<Line<'static>> {
    let theme = model.theme;
    let color = theme.usage(value);
    let number_w = 15usize;
    let chart_w = width.saturating_sub(number_w);
    let digits = big_number(model, &value.to_string());
    let bars = chart(model, samples, chart_w, 3);
    let mut lines: Vec<Line<'static>> = digits
        .into_iter()
        .zip(bars)
        .map(|(digit, bar)| {
            Line::from(vec![
                Span::styled(fit(&digit, number_w), theme.bold(color)),
                Span::styled(bar, theme.fg(theme.steel)),
            ])
        })
        .collect();
    let span = if samples.len() >= 2 {
        format!("last {} s", samples.len().min(chart_w))
    } else {
        String::new()
    };
    lines.push(Line::from(vec![
        Span::styled(fit(caption, number_w), theme.fg(theme.muted)),
        Span::styled(fit_right(&span, chart_w), theme.fg(theme.faint)),
    ]));
    lines
}

fn cpu_card(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let theme = model.theme;
    let (width, height) = inner(area);
    let mut lines = Vec::new();
    match model.cpu_percent {
        Some(cpu) => {
            let samples: Vec<u8> = model.cpu_history.iter().copied().collect();
            lines.extend(live_block(model, width, cpu, &samples, "% busy"));
        }
        None => lines.push(Line::from(Span::styled(
            "Measuring. The first reading takes a second.",
            theme.fg(theme.muted),
        ))),
    }
    let cpus = model.facts.logical_cpus.map_or_else(
        || "all processors".to_owned(),
        |count| format!("all {count} logical processors"),
    );
    let tail = para(
        &format!("Share of time {cpus} were working, read every second. Short spikes are normal."),
        width,
        0,
        theme.fg(theme.muted),
    );
    fill_to(&mut lines, height, tail);
    show(model, frame, area, "Processor", false, lines);
}

fn memory_card(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let theme = model.theme;
    let (width, height) = inner(area);
    let mut lines = Vec::new();
    match model.facts.memory {
        Some(memory) => {
            let used = memory.total_bytes.saturating_sub(memory.available_bytes);
            let share = percent(used, memory.total_bytes);
            let samples: Vec<u8> = model.memory_history.iter().copied().collect();
            lines.extend(live_block(model, width, share, &samples, "% in use"));
            let long = format!(
                " of {} in use, {} available",
                format_bytes(memory.total_bytes),
                format_bytes(memory.available_bytes)
            );
            let short = format!(" of {} in use", format_bytes(memory.total_bytes));
            let used_text = format_bytes(used);
            let rest = if used_text.chars().count() + long.chars().count() <= width {
                long
            } else {
                short
            };
            lines.push(Line::from(vec![
                Span::styled(used_text, theme.bold(theme.text)),
                Span::styled(rest, theme.fg(theme.muted)),
            ]));
        }
        None => lines.push(Line::from(Span::styled(
            "Memory could not be read.",
            theme.fg(theme.warn),
        ))),
    }
    let tail = para(
        "Windows uses spare memory as a cache, so a high number is normal. Staying near 100% is what slows a PC.",
        width,
        0,
        theme.fg(theme.muted),
    );
    fill_to(&mut lines, height, tail);
    show(model, frame, area, "Memory", false, lines);
}

fn drives_card(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let theme = model.theme;
    let (width, height) = inner(area);
    let mut lines = Vec::new();
    if model.facts.drives.is_empty() {
        lines.push(Line::from(Span::styled(
            "No fixed drive could be read.",
            theme.fg(theme.warn),
        )));
    }
    for drive in &model.facts.drives {
        if lines.len() + 2 > height {
            break;
        }
        let used = drive.total_bytes.saturating_sub(drive.free_bytes);
        let share = percent(used, drive.total_bytes);
        let label_color = if drive.system {
            theme.accent
        } else {
            theme.text
        };
        let mut head = vec![Span::styled(
            format!("{:<4}", drive.label),
            theme.bold(label_color),
        )];
        let free = format!(
            "{} free of {}",
            format_bytes(drive.free_bytes),
            format_bytes(drive.total_bytes)
        );
        let tag = "  Windows";
        let fits_tag = 4 + free.chars().count() + tag.chars().count() <= width;
        head.push(Span::styled(free, theme.fg(theme.muted)));
        if drive.system && fits_tag {
            head.push(Span::styled(tag, theme.fg(theme.accent)));
        }
        lines.push(Line::from(head));
        let mut bar = vec![Span::raw("    ")];
        bar.extend(meter(
            model,
            u32::from(share) * 10,
            width.saturating_sub(10),
            theme.usage(share),
        ));
        bar.push(Span::styled(
            fit_right(&format!("{share}%"), 6),
            theme.bold(theme.usage(share)),
        ));
        lines.push(Line::from(bar));
    }
    let tail = para(
        "Windows needs free space for updates and memory paging. Keep at least 15% free on the Windows drive.",
        width,
        0,
        theme.fg(theme.muted),
    );
    fill_to(&mut lines, height, tail);
    show(model, frame, area, "Drives", false, lines);
}

fn uptime_card(model: &Model, frame: &mut Frame<'_>, area: Rect) {
    let theme = model.theme;
    let (width, height) = inner(area);
    let mut lines = Vec::new();
    match model.facts.uptime_secs {
        Some(secs) => {
            lines.push(Line::from(Span::styled(
                span_text(secs),
                theme.bold(theme.steel),
            )));
            lines.push(Line::from(Span::styled(
                "since Windows last started",
                theme.fg(theme.muted),
            )));
            lines.push(blank());
            let advice = if secs > 7 * 86_400 {
                "That is more than a week. A restart finishes pending updates and starts every program fresh."
            } else {
                "A recent start. Nothing to do here."
            };
            lines.extend(para(advice, width, 0, theme.fg(theme.text)));
        }
        None => lines.push(Line::from(Span::styled("Not read.", theme.fg(theme.warn)))),
    }
    let tail = para(
        "With Fast Startup on, Shut down does not reset this count. Only Restart does.",
        width,
        0,
        theme.fg(theme.muted),
    );
    fill_to(&mut lines, height, tail);
    show(model, frame, area, "Time running", false, lines);
}

// ---------------------------------------------------------------- planned rooms

struct Plan {
    purpose: &'static str,
    will: &'static [&'static str],
    today: &'static str,
    promise: Option<&'static str>,
}

fn plan(tab: Tab) -> Plan {
    match tab {
        Tab::Network => Plan {
            purpose: "Find out why the internet feels slow, and whose fault it is.",
            will: &[
                "Measure delay, jitter, and lost packets to your router and to the internet.",
                "Tell a weak Wi-Fi signal apart from a slow provider.",
                "Compare DNS servers and spot one that was tampered with.",
                "Repair the network in small steps, the safest first.",
            ],
            today: "Not built yet. Nothing in this room runs or changes your PC.",
            promise: None,
        },
        Tab::Shield => Plan {
            purpose: "Check how well this PC is protected, and explain every gap.",
            will: &[
                "Read the state of antivirus, firewall, encryption, and Secure Boot.",
                "List everything that starts by itself, with who signed it.",
                "Offer fixes one by one, each with a way back.",
            ],
            today: "Not built yet. Nothing in this room runs or changes your PC.",
            promise: Some(
                "This room will never turn off Defender, the firewall, UAC, SmartScreen, or Windows Update.",
            ),
        },
        Tab::Boost => Plan {
            purpose: "Make startup and everyday use faster, with changes you can undo.",
            will: &[
                "Show which apps slow down sign-in, measured from Windows' own logs.",
                "Explain in plain words why the PC is slow right now.",
                "Pick a power plan that fits a desktop or a laptop.",
            ],
            today: "Not built yet. Nothing in this room runs or changes your PC.",
            promise: Some(
                "No memory cleaner and no registry cleaner. They do not make PCs faster.",
            ),
        },
        Tab::Game => Plan {
            purpose: "Get the PC ready for a game, and put everything back afterwards.",
            will: &[
                "Check that the monitor really runs at its highest refresh rate.",
                "Pause the apps you choose while a game runs.",
                "Tell you whether lag comes from your network or not.",
                "Undo every change when the game closes, even after a crash.",
            ],
            today: "Not built yet. Nothing in this room runs or changes your PC.",
            promise: None,
        },
        Tab::Fix => Plan {
            purpose: "Guided repairs for the problems Windows users hit most.",
            will: &[
                "Check and repair system files with DISM and SFC, with real progress.",
                "Restart sound, printing, Explorer, or Windows Update the right way.",
                "Explain each result instead of dumping a log.",
            ],
            today: "Not built yet. Nothing in this room runs or changes your PC.",
            promise: None,
        },
        Tab::Ledger => Plan {
            purpose: "A record of every change Castellan makes to this PC.",
            will: &[
                "One entry per change: what, when, and who asked for it.",
                "The old value, so the change can be undone.",
                "A chain of hashes, so a tampered entry shows.",
            ],
            today: "Empty, because this build has not changed anything.",
            promise: None,
        },
        Tab::Overview | Tab::Sweep | Tab::Vitals => Plan {
            purpose: "",
            will: &[],
            today: "",
            promise: None,
        },
    }
}

fn plan_room(model: &Model, frame: &mut Frame<'_>, area: Rect, tab: Tab) {
    let theme = model.theme;
    let width = area.width.min(78);
    let area = Rect {
        x: area.x + (area.width - width) / 2,
        width,
        ..area
    };
    let (text_w, height) = inner(area);
    let plan = plan(tab);
    let dot = pick(model, "•", "*");
    let mut body = vec![Line::from(Span::styled(
        tab.label(),
        theme.bold(theme.accent),
    ))];
    body.extend(para(plan.purpose, text_w, 0, theme.fg(theme.text)));
    body.push(blank());
    body.push(heading(model, "What this room will do"));
    for item in plan.will {
        body.extend(bullet(
            dot,
            theme.fg(theme.accent),
            item,
            text_w,
            theme.fg(theme.text),
        ));
    }
    if let Some(promise) = plan.promise {
        body.push(blank());
        body.extend(bullet(
            pick(model, "◆", "!"),
            theme.bold(theme.ok),
            promise,
            text_w,
            theme.fg(theme.text),
        ));
    }
    body.push(blank());
    body.push(heading(model, "Today"));
    body.extend(bullet(
        pick(model, "○", "o"),
        theme.fg(theme.muted),
        plan.today,
        text_w,
        theme.fg(theme.muted),
    ));
    let emblem = emblem_lines(model, tab, text_w);
    let mut lines = Vec::new();
    let total = |extra: usize| body.len() + extra;
    let with_emblem = !emblem.is_empty() && total(emblem.len() + 1) <= height;
    let used = if with_emblem {
        total(emblem.len() + 1)
    } else {
        total(0)
    };
    lines.extend(std::iter::repeat_n(
        blank(),
        height.saturating_sub(used) / 2,
    ));
    if with_emblem {
        lines.extend(emblem);
        lines.push(blank());
    }
    lines.extend(body);
    let title = if tab == Tab::Ledger {
        "Ledger"
    } else {
        "Coming later"
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(card(model, title, false))
            .style(theme.ink()),
        area,
    );
}
