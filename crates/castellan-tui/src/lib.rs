//! Terminal UI. `update` is pure. `view` only draws the model it is given.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::todo,
        clippy::unimplemented,
        clippy::dbg_macro,
    )
)]

use castellan_core::ScoreReport;
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

/// Screens the overview build knows how to show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    /// Keep Score and the storage finding.
    Overview,
    /// What a sweep would touch. The data comes from the model, not from a fake scan.
    Sweep,
    /// Ledger entries the model already loaded.
    Ledger,
}

impl Tab {
    const ALL: [Self; 3] = [Self::Overview, Self::Sweep, Self::Ledger];

    fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Sweep => "Sweep",
            Self::Ledger => "Ledger",
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Overview => Self::Sweep,
            Self::Sweep => Self::Ledger,
            Self::Ledger => Self::Overview,
        }
    }
}

/// Palette. Tokens are semantic; the view does not invent one-off colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    /// Background.
    pub background: Color,
    /// Primary text.
    pub text: Color,
    /// Secondary text.
    pub muted: Color,
    /// Torch accent.
    pub accent: Color,
    /// Healthy.
    pub ok: Color,
    /// Warning.
    pub warn: Color,
    /// Critical.
    pub critical: Color,
}

impl Theme {
    /// Night fortress. The default.
    #[must_use]
    pub const fn keep() -> Self {
        Self {
            background: Color::Rgb(14, 17, 22),
            text: Color::Rgb(230, 225, 214),
            muted: Color::Rgb(139, 149, 165),
            accent: Color::Rgb(242, 165, 65),
            ok: Color::Rgb(123, 196, 127),
            warn: Color::Rgb(232, 184, 74),
            critical: Color::Rgb(229, 96, 77),
        }
    }
}

/// Messages the event loop feeds to [`update`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    /// A key that was actually pressed.
    Key(Key),
}

/// Keys the UI handles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// Quit.
    Quit,
    /// Next tab.
    Tab,
    /// Toggle help.
    Help,
    /// Close help or do nothing.
    Esc,
    /// Select a tab by number, starting at 1.
    Digit(u8),
}

/// UI state.
#[derive(Clone, Debug)]
pub struct Model {
    /// Current tab.
    pub tab: Tab,
    /// Score from the engine. `None` until an inspection finishes.
    pub score: Option<ScoreReport>,
    /// Persona or status line. Empty when persona is off.
    pub status_line: String,
    /// Help overlay.
    pub help: bool,
    /// User asked to quit.
    pub quit: bool,
    /// Colors.
    pub theme: Theme,
}

impl Model {
    /// Model before any inspection has returned.
    #[must_use]
    pub fn fresh(status_line: impl Into<String>) -> Self {
        Self {
            tab: Tab::Overview,
            score: None,
            status_line: status_line.into(),
            help: false,
            quit: false,
            theme: Theme::keep(),
        }
    }
}

/// Apply one message. Drawing is not done here.
pub fn update(model: &mut Model, msg: Msg) {
    let Msg::Key(key) = msg;
    match key {
        Key::Quit => model.quit = true,
        Key::Help => model.help = !model.help,
        Key::Esc => model.help = false,
        Key::Tab => model.tab = model.tab.next(),
        Key::Digit(1) => model.tab = Tab::Overview,
        Key::Digit(2) => model.tab = Tab::Sweep,
        Key::Digit(3) => model.tab = Tab::Ledger,
        Key::Digit(_) => {}
    }
}

/// Draw the model. Idle callers should not invoke this when nothing changed.
pub fn view(model: &Model, frame: &mut Frame<'_>) {
    let area = frame.area();
    if area.width < 40 || area.height < 10 {
        frame.render_widget(
            Paragraph::new("Enlarge the terminal to at least 40×10."),
            area,
        );
        return;
    }
    let theme = model.theme;
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);
    let header = chunks.first().copied().unwrap_or(Rect::default());
    let body = chunks.get(1).copied().unwrap_or(Rect::default());
    let footer = chunks.get(2).copied().unwrap_or(Rect::default());
    frame.render_widget(header_line(model), header);
    frame.render_widget(body_block(model), body);
    frame.render_widget(
        Paragraph::new("tab next   1-3 jump   ? help   q quit")
            .style(Style::default().fg(theme.muted)),
        footer,
    );
    if model.help {
        let help = Paragraph::new("tab switches screens. q leaves. This build inspects storage and explains the score. It does not invent findings.")
            .block(Block::default().borders(Borders::ALL).title(" Help "))
            .wrap(Wrap { trim: true })
            .style(Style::default().fg(theme.text).bg(theme.background));
        frame.render_widget(help, centered(body));
    }
}

fn header_line(model: &Model) -> Paragraph<'static> {
    let mark = "CASTELLAN";
    let score = match model.score.as_ref().and_then(|report| report.keep_score) {
        Some(value) => format!("{value}"),
        None => "--".to_owned(),
    };
    let tabs: Vec<Span<'static>> = Tab::ALL
        .into_iter()
        .map(|tab| {
            let style = if tab == model.tab {
                Style::default()
                    .fg(model.theme.background)
                    .bg(model.theme.accent)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(model.theme.muted)
            };
            Span::styled(format!(" {0} ", tab.label()), style)
        })
        .collect();
    let mut line = vec![Span::styled(
        format!(" {mark}  "),
        Style::default()
            .fg(model.theme.accent)
            .add_modifier(Modifier::BOLD),
    )];
    line.extend(tabs);
    line.push(Span::styled(
        format!("  Keep {score}"),
        Style::default().fg(model.theme.text),
    ));
    Paragraph::new(Line::from(line)).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(model.theme.muted)),
    )
}

fn body_block(model: &Model) -> Paragraph<'static> {
    let text = match model.tab {
        Tab::Overview => overview_text(model),
        Tab::Sweep => "Sweep deletes aged files in the user temp directory and Windows Temp.\nPrefetch is not cleaned. A debug build refuses the deletion.".to_owned(),
        Tab::Ledger => "The ledger records each mutation with a hash chain. Temp files are marked irreversible.".to_owned(),
    };
    Paragraph::new(text)
        .style(Style::default().fg(model.theme.text))
        .wrap(Wrap { trim: true })
        .block(Block::default().borders(Borders::NONE))
}

fn overview_text(model: &Model) -> String {
    let Some(report) = model.score.as_ref() else {
        return format!("Inspection has not finished.\n{}", model.status_line);
    };
    let mut lines = Vec::new();
    if let Some(score) = report.keep_score {
        let band = report
            .band
            .map(castellan_core::Band::label)
            .unwrap_or("not banded");
        lines.push(format!("Keep Score {score} ({band})"));
    } else {
        lines.push("Keep Score is not available yet.".to_owned());
    }
    lines.push(format!(
        "Confidence {}%",
        u32::from(report.confidence_millis) / 10
    ));
    for pillar in &report.pillars {
        let value = match pillar.score {
            Some(score) => score.to_string(),
            None => "not inspected".to_owned(),
        };
        lines.push(format!("{}: {value}", pillar.pillar.as_str()));
    }
    if !model.status_line.is_empty() {
        lines.push(model.status_line.clone());
    }
    lines.join("\n")
}

fn centered(area: Rect) -> Rect {
    let width = area.width.saturating_mul(3) / 4;
    let height = area.height.saturating_mul(1) / 2;
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn quit_and_tabs_are_pure() {
        let mut model = Model::fresh("Nothing to fix here. The walls hold.");
        update(&mut model, Msg::Key(Key::Digit(2)));
        assert_eq!(model.tab, Tab::Sweep);
        update(&mut model, Msg::Key(Key::Help));
        assert!(model.help);
        update(&mut model, Msg::Key(Key::Esc));
        assert!(!model.help);
        update(&mut model, Msg::Key(Key::Quit));
        assert!(model.quit);
    }

    #[test]
    fn overview_says_when_inspection_has_not_run() {
        let model = Model::fresh("Lighting the torches.");
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|frame| view(&model, frame)).expect("draw");
        let text = buffer_text(terminal.backend());
        assert!(text.contains("Inspection has not finished"));
        assert!(text.contains("CASTELLAN"));
        assert!(!text.contains("86"));
    }

    #[test]
    fn a_small_terminal_asks_for_space() {
        let model = Model::fresh("");
        let backend = TestBackend::new(20, 8);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal.draw(|frame| view(&model, frame)).expect("draw");
        let text = buffer_text(terminal.backend());
        assert!(text.contains("Enlarge the terminal"));
    }

    fn buffer_text(backend: &TestBackend) -> String {
        let mut out = String::new();
        let buffer = backend.buffer();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let symbol = buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" ");
                out.push_str(symbol);
            }
            out.push('\n');
        }
        out
    }
}
