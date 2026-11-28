//! Keep, the night-fortress palette. Tokens are semantic.

use ratatui::style::{Color, Modifier, Style};

/// One Castellan palette.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    /// Page background.
    pub background: Color,
    /// Card surface.
    pub surface: Color,
    /// Raised surface, used behind the selected row.
    pub raised: Color,
    /// Card border and rules.
    pub border: Color,
    /// Parchment text.
    pub text: Color,
    /// Secondary text.
    pub muted: Color,
    /// Empty meter track and other marks that should almost disappear.
    pub faint: Color,
    /// Amethyst. The keep's mark and the selection.
    pub accent: Color,
    /// Cold steel. Section headings and neutral values.
    pub steel: Color,
    /// Healthy.
    pub ok: Color,
    /// Warning.
    pub warn: Color,
    /// Critical.
    pub critical: Color,
    /// Informational.
    pub info: Color,
    /// When true, colors are not painted. Hierarchy uses weight instead.
    pub plain: bool,
}

impl Theme {
    /// Night fortress. The default.
    #[must_use]
    pub const fn keep() -> Self {
        Self {
            background: Color::Rgb(13, 16, 22),
            surface: Color::Rgb(20, 25, 33),
            raised: Color::Rgb(34, 41, 54),
            border: Color::Rgb(52, 62, 78),
            text: Color::Rgb(230, 225, 214),
            muted: Color::Rgb(139, 149, 165),
            faint: Color::Rgb(58, 68, 86),
            accent: Color::Rgb(176, 148, 255),
            steel: Color::Rgb(98, 204, 218),
            ok: Color::Rgb(126, 200, 130),
            warn: Color::Rgb(236, 188, 80),
            critical: Color::Rgb(236, 104, 86),
            info: Color::Rgb(116, 172, 224),
            plain: false,
        }
    }

    /// Same structure, no painted color. Honors `NO_COLOR`.
    #[must_use]
    pub const fn plain() -> Self {
        Self {
            background: Color::Reset,
            surface: Color::Reset,
            raised: Color::Reset,
            border: Color::Reset,
            text: Color::Reset,
            muted: Color::Reset,
            faint: Color::Reset,
            accent: Color::Reset,
            steel: Color::Reset,
            ok: Color::Reset,
            warn: Color::Reset,
            critical: Color::Reset,
            info: Color::Reset,
            plain: true,
        }
    }

    /// The selected tab, row, or key. Reversed video when colors are off.
    #[must_use]
    pub fn selected(&self) -> Style {
        if self.plain {
            Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            Style::default()
                .fg(self.background)
                .bg(self.accent)
                .add_modifier(Modifier::BOLD)
        }
    }

    /// Body text on a card.
    #[must_use]
    pub fn ink(&self) -> Style {
        Style::default().fg(self.text).bg(self.surface)
    }

    /// Text in `color`, with no background of its own.
    #[must_use]
    pub fn fg(&self, color: Color) -> Style {
        Style::default().fg(color)
    }

    /// Bold text in `color`.
    #[must_use]
    pub fn bold(&self, color: Color) -> Style {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }

    /// Color for a health score, 0 to 100, where higher is better.
    #[must_use]
    pub const fn score(&self, value: u8) -> Color {
        match value {
            90..=100 => self.ok,
            75..=89 => self.steel,
            50..=74 => self.warn,
            _ => self.critical,
        }
    }

    /// Color for a usage share, 0 to 100, where higher is worse.
    #[must_use]
    pub const fn usage(&self, percent: u8) -> Color {
        match percent {
            0..=74 => self.steel,
            75..=89 => self.warn,
            _ => self.critical,
        }
    }
}
