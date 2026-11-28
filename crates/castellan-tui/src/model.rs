//! UI state and the pure update. Drawing lives in [`crate::view`].

use std::collections::VecDeque;

use castellan_core::policy::ABSOLUTE_MIN_AGE_SECS;
use castellan_core::{Pillar, ScoreReport, Severity, UserPreferences};

use crate::theme::Theme;

/// Rooms in the keep. The order is the number row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    /// Keep Score and whatever was actually measured.
    Overview,
    /// Sweep targets. Nothing is deleted from this screen in this build.
    Sweep,
    /// Hardware and system health. No probes yet.
    Vitals,
    /// Network. No probes yet.
    Network,
    /// Security posture. Nothing here disables a protection.
    Shield,
    /// Startup and power. Nothing is changed.
    Boost,
    /// Game mode. Nothing is applied.
    Game,
    /// Repair bench. Empty in this build.
    Fix,
    /// Ledger. No rows until a mutation exists.
    Ledger,
}

impl Tab {
    /// Left to right, matching keys 1 through 9.
    pub const ALL: [Self; 9] = [
        Self::Overview,
        Self::Sweep,
        Self::Vitals,
        Self::Network,
        Self::Shield,
        Self::Boost,
        Self::Game,
        Self::Fix,
        Self::Ledger,
    ];

    /// Short label for the tab row.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Sweep => "Sweep",
            Self::Vitals => "Vitals",
            Self::Network => "Network",
            Self::Shield => "Shield",
            Self::Boost => "Boost",
            Self::Game => "Game",
            Self::Fix => "Fix",
            Self::Ledger => "Ledger",
        }
    }

    /// Next room, wrapping after the ledger.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Overview => Self::Sweep,
            Self::Sweep => Self::Vitals,
            Self::Vitals => Self::Network,
            Self::Network => Self::Shield,
            Self::Shield => Self::Boost,
            Self::Boost => Self::Game,
            Self::Game => Self::Fix,
            Self::Fix => Self::Ledger,
            Self::Ledger => Self::Overview,
        }
    }

    /// Previous room.
    #[must_use]
    pub const fn prev(self) -> Self {
        match self {
            Self::Overview => Self::Ledger,
            Self::Sweep => Self::Overview,
            Self::Vitals => Self::Sweep,
            Self::Network => Self::Vitals,
            Self::Shield => Self::Network,
            Self::Boost => Self::Shield,
            Self::Game => Self::Boost,
            Self::Fix => Self::Game,
            Self::Ledger => Self::Fix,
        }
    }

    /// `1` is Overview. `9` is Ledger.
    #[must_use]
    pub const fn number(self) -> u8 {
        match self {
            Self::Overview => 1,
            Self::Sweep => 2,
            Self::Vitals => 3,
            Self::Network => 4,
            Self::Shield => 5,
            Self::Boost => 6,
            Self::Game => 7,
            Self::Fix => 8,
            Self::Ledger => 9,
        }
    }

    /// `1` is Overview.
    #[must_use]
    pub const fn from_digit(digit: u8) -> Option<Self> {
        match digit {
            1 => Some(Self::Overview),
            2 => Some(Self::Sweep),
            3 => Some(Self::Vitals),
            4 => Some(Self::Network),
            5 => Some(Self::Shield),
            6 => Some(Self::Boost),
            7 => Some(Self::Game),
            8 => Some(Self::Fix),
            9 => Some(Self::Ledger),
            _ => None,
        }
    }
}

/// One measured thing worth showing on the overview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attention {
    /// Finding id, such as `STORAGE-001`.
    pub id: String,
    /// How serious it is.
    pub severity: Severity,
    /// One line, already in English.
    pub title: String,
    /// Evidence the person can check.
    pub detail: String,
    /// What the person can do about it, in one sentence.
    pub advice: String,
    /// Room that Enter opens for this finding, if one helps.
    pub room: Option<Tab>,
}

/// One fixed local drive, as measured.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drive {
    /// Short name, such as `C:`.
    pub label: String,
    /// Size of the volume.
    pub total_bytes: u64,
    /// Bytes free for this user.
    pub free_bytes: u64,
    /// Windows lives on this drive.
    pub system: bool,
}

/// Physical memory, as measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Memory {
    /// Installed memory Windows can use.
    pub total_bytes: u64,
    /// Memory that is free or can be reclaimed right now.
    pub available_bytes: u64,
}

/// Plain facts about this PC. A field that could not be read stays empty.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// Fixed local drives.
    pub drives: Vec<Drive>,
    /// Physical memory.
    pub memory: Option<Memory>,
    /// Seconds since Windows started.
    pub uptime_secs: Option<u64>,
    /// Logical processors.
    pub logical_cpus: Option<u32>,
    /// This binary is a development build.
    pub dev_build: bool,
}

/// A fresh live reading. Sent about once a second while the keep is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pulse {
    /// Busy share of all processors since the previous reading, 0 to 100.
    pub cpu_percent: Option<u8>,
    /// Physical memory right now.
    pub memory: Option<Memory>,
    /// Seconds since Windows started.
    pub uptime_secs: Option<u64>,
}

/// Size of one sweep target from a read-only walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SweepSize {
    /// Files seen.
    pub files: u64,
    /// Their size.
    pub bytes: u64,
    /// Files old enough for a sweep to consider.
    pub aged_files: u64,
    /// Their size.
    pub aged_bytes: u64,
    /// The walk stopped early, so the numbers are a floor.
    pub truncated: bool,
}

/// What is known about one sweep target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SweepReading {
    /// The read-only walk is still running.
    Measuring,
    /// The walk finished.
    Measured(SweepSize),
    /// The folder could not be read. The text says why, in plain words.
    Unavailable(String),
}

/// Rows in the sweep room. The last one is a target Castellan refuses.
pub const SWEEP_ROWS: u8 = 3;

/// What the engine actually returned. Absent fields stay absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeepWitness {
    /// Score. Unassessed pillars have no number.
    pub score: ScoreReport,
    /// When false, the band is hidden even if the report computed one.
    pub show_band: bool,
    /// Findings worth a card. Empty when nothing failed.
    pub attention: Vec<Attention>,
    /// Persona line for this reading.
    pub persona: String,
}

/// How the shell opened the keep.
#[derive(Clone, Debug, Default)]
pub struct Session {
    /// Line shown before a witness exists.
    pub persona: String,
    /// Machine name for the header. Not a path.
    pub hostname: String,
    /// Local time `HH:MM`.
    pub clock: String,
    /// Inspection, if one finished.
    pub witness: Option<KeepWitness>,
    /// `NO_COLOR` was set.
    pub plain: bool,
    /// Draw with ASCII instead of box glyphs.
    pub ascii: bool,
    /// Skip the splash and the gauge ease.
    pub reduce_motion: bool,
    /// Facts read when the keep opened.
    pub facts: Facts,
    /// User temp, then Windows temp. `None` when nothing was measured.
    pub sweep: Option<[SweepReading; 2]>,
}

/// Keys the keep understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    /// Leave.
    Quit,
    /// Next room.
    Tab,
    /// Previous room.
    BackTab,
    /// Help overlay.
    Help,
    /// Close the top overlay.
    Esc,
    /// Open the selection.
    Enter,
    /// Reserved for selection. No deletion is bound to it.
    Space,
    /// Move up.
    Up,
    /// Move down.
    Down,
    /// Palette.
    Palette,
    /// Room number, 1 through 9.
    Digit(u8),
    /// A typed character, used by the palette.
    Char(char),
    /// Erase one palette character.
    Backspace,
}

/// Messages for [`update`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Msg {
    /// A key that was pressed.
    Key(Key),
    /// Time passed. Animations advance. Input is never blocked on this.
    Tick {
        /// Milliseconds since the previous tick.
        dt_ms: u32,
    },
    /// A new clock reading.
    Clock(String),
    /// Choose a row in the open list. A row past the end is ignored.
    Select(u8),
    /// The preferences file was written.
    PrefsStored,
    /// The preferences file could not be written.
    PrefsFailed(String),
    /// A fresh live reading.
    Pulse(Pulse),
    /// A sweep target finished its read-only walk. `0` is user temp, `1` is Windows temp.
    Sweep(u8, SweepReading),
}

/// Gauge fill that eases toward the measured score. Unassessed pillars stay at 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Gauges {
    storage: u16,
    stability: u16,
    performance: u16,
    security: u16,
    network: u16,
}

impl Gauges {
    const fn zero() -> Self {
        Self {
            storage: 0,
            stability: 0,
            performance: 0,
            security: 0,
            network: 0,
        }
    }

    fn get(self, pillar: Pillar) -> u16 {
        match pillar {
            Pillar::Storage => self.storage,
            Pillar::Stability => self.stability,
            Pillar::Performance => self.performance,
            Pillar::Security => self.security,
            Pillar::Network => self.network,
        }
    }

    fn set(&mut self, pillar: Pillar, value: u16) {
        match pillar {
            Pillar::Storage => self.storage = value,
            Pillar::Stability => self.stability = value,
            Pillar::Performance => self.performance = value,
            Pillar::Security => self.security = value,
            Pillar::Network => self.network = value,
        }
    }
}

/// Everything the view is allowed to draw.
#[derive(Clone, Debug)]
pub struct Model {
    /// Current room.
    pub tab: Tab,
    /// Colors.
    pub theme: Theme,
    /// ASCII glyphs.
    pub ascii: bool,
    /// Skip decorative motion.
    pub reduce_motion: bool,
    /// Machine name.
    pub hostname: String,
    /// `HH:MM`.
    pub clock: String,
    /// Persona before a witness, and the splash line's fallback.
    pub persona: String,
    /// Measured keep, if the walk finished.
    pub witness: Option<KeepWitness>,
    /// Help overlay.
    pub help: bool,
    /// Command palette.
    pub palette: bool,
    /// Palette query.
    pub palette_query: String,
    /// Selected palette row.
    pub palette_cursor: u8,
    /// Row inside the current room.
    pub cursor: u8,
    /// Splash is still on screen.
    pub splash: bool,
    /// How long the splash has been up.
    pub splash_ms: u32,
    /// Time since open, used by the torch. It keeps moving after the splash.
    pub(crate) elapsed_ms: u32,
    /// Eased overall number. `None` until a score exists.
    pub shown_score: Option<u16>,
    /// Toast text and milliseconds left.
    pub toast: Option<(String, u32)>,
    /// The person asked to leave.
    pub quit: bool,
    /// Milliseconds left in the room-change sweep.
    pub(crate) enter_ms: u32,
    /// Saved wishes and display choices. Sign-in flags do not register a task.
    pub preferences: UserPreferences,
    /// The settings page is open.
    pub settings: bool,
    /// Row inside settings.
    pub(crate) settings_cursor: u8,
    /// What the last save did. Empty until a toggle is stored.
    pub settings_note: String,
    /// True after a toggle until the shell writes the file.
    pub prefs_dirty: bool,
    /// Facts about this PC. Live readings update the memory and the uptime.
    pub facts: Facts,
    /// User temp, then Windows temp.
    pub sweep: [SweepReading; 2],
    /// Latest processor load, 0 to 100.
    pub cpu_percent: Option<u8>,
    /// Recent processor load, oldest first.
    pub(crate) cpu_history: VecDeque<u8>,
    /// Recent memory use, oldest first.
    pub(crate) memory_history: VecDeque<u8>,
    gauges: Gauges,
}

/// Live samples kept for the sparklines.
const HISTORY: usize = 240;

fn unread() -> SweepReading {
    SweepReading::Unavailable("This folder has not been measured.".to_owned())
}

impl Model {
    /// Open a session. Motion starts at the splash unless it was asked not to.
    #[must_use]
    pub fn open(session: Session) -> Self {
        let theme = if session.plain {
            Theme::plain()
        } else {
            Theme::keep()
        };
        let mut model = Self {
            tab: Tab::Overview,
            theme,
            ascii: session.ascii,
            reduce_motion: session.reduce_motion,
            hostname: session.hostname,
            clock: session.clock,
            persona: session.persona,
            witness: session.witness,
            help: false,
            palette: false,
            palette_query: String::new(),
            palette_cursor: 0,
            cursor: 0,
            splash: !session.reduce_motion,
            splash_ms: 0,
            elapsed_ms: 0,
            shown_score: None,
            toast: None,
            quit: false,
            enter_ms: 0,
            preferences: UserPreferences::compiled_default(),
            settings: false,
            settings_cursor: 0,
            settings_note: String::new(),
            prefs_dirty: false,
            facts: session.facts,
            sweep: session.sweep.unwrap_or_else(|| [unread(), unread()]),
            cpu_percent: None,
            cpu_history: VecDeque::new(),
            memory_history: VecDeque::new(),
            gauges: Gauges::zero(),
        };
        if session.reduce_motion {
            model.snap_motion();
        }
        model
    }

    /// A still overview with no inspection. Tests and the pre-walk frame use this.
    #[must_use]
    pub fn fresh(persona: impl Into<String>) -> Self {
        Self::open(Session {
            persona: persona.into(),
            hostname: "THIS-KEEP".to_owned(),
            clock: "--:--".to_owned(),
            witness: None,
            plain: false,
            ascii: false,
            reduce_motion: true,
            facts: Facts::default(),
            sweep: None,
        })
    }

    /// Milliseconds to wait before the next frame.
    ///
    /// Fast only while something is easing. An idle keep redraws four times a
    /// second, which is enough for the clock and the live readings.
    #[must_use]
    pub fn pace_ms(&self) -> u64 {
        if self.animating() { 33 } else { 250 }
    }

    fn animating(&self) -> bool {
        if self.reduce_motion {
            return false;
        }
        if self.splash || self.enter_ms > 0 || self.shown_score != self.score_target() {
            return true;
        }
        Pillar::all()
            .into_iter()
            .any(|pillar| self.gauges.get(pillar) != target_for(self.witness.as_ref(), pillar))
    }

    /// Milliseconds since the keep opened. Drives the slow header pulse.
    pub(crate) fn elapsed(&self) -> u32 {
        self.elapsed_ms
    }

    /// Share of the room-change ease still to run, 0 to 1000.
    pub(crate) fn entering(&self) -> u32 {
        self.enter_ms.saturating_mul(1_000) / ENTER_MS
    }

    fn score_target(&self) -> Option<u16> {
        self.witness
            .as_ref()
            .and_then(|witness| witness.score.keep_score)
            .map(u16::from)
    }

    fn snap_motion(&mut self) {
        self.shown_score = self.score_target();
        for pillar in Pillar::all() {
            let target = target_for(self.witness.as_ref(), pillar);
            self.gauges.set(pillar, target);
        }
    }

    pub(crate) fn gauge(&self, pillar: Pillar) -> u16 {
        self.gauges.get(pillar)
    }
}

fn target_for(witness: Option<&KeepWitness>, pillar: Pillar) -> u16 {
    let Some(witness) = witness else {
        return 0;
    };
    witness
        .score
        .pillars
        .iter()
        .find(|pillar_score| pillar_score.pillar == pillar)
        .and_then(|pillar_score| pillar_score.score)
        .map(u16::from)
        .unwrap_or(0)
}

/// Where a palette row goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pick {
    /// One of the nine rooms.
    Room(Tab),
    /// The settings page.
    Settings,
}

/// Palette entries. The query matches the label.
pub(crate) const PALETTE: &[(&str, Pick)] = &[
    ("overview", Pick::Room(Tab::Overview)),
    ("sweep", Pick::Room(Tab::Sweep)),
    ("vitals", Pick::Room(Tab::Vitals)),
    ("network", Pick::Room(Tab::Network)),
    ("shield", Pick::Room(Tab::Shield)),
    ("boost", Pick::Room(Tab::Boost)),
    ("game", Pick::Room(Tab::Game)),
    ("fix", Pick::Room(Tab::Fix)),
    ("ledger", Pick::Room(Tab::Ledger)),
    ("settings", Pick::Settings),
];

pub(crate) fn palette_hits(query: &str) -> Vec<(&'static str, Pick)> {
    let needle = query.trim().to_ascii_lowercase();
    PALETTE
        .iter()
        .copied()
        .filter(|(label, _)| needle.is_empty() || label.contains(&needle))
        .collect()
}

/// Apply one message. This function does not draw and does not touch the machine.
pub fn update(model: &mut Model, msg: Msg) {
    match msg {
        Msg::Clock(clock) => model.clock = clock,
        Msg::Tick { dt_ms } => advance(model, dt_ms),
        Msg::Key(key) => on_key(model, key),
        Msg::Select(index) => {
            if model.settings {
                if index < SETTING_ROWS {
                    model.settings_cursor = index;
                }
            } else if u16::from(index) < room_len(model) {
                model.cursor = index;
            }
        }
        Msg::PrefsStored => {
            model.prefs_dirty = false;
            model.settings_note = "Saved to preferences.toml.".to_owned();
        }
        Msg::PrefsFailed(error) => {
            model.prefs_dirty = false;
            model.settings_note = format!("Not recorded. {error}");
            model.toast = Some((format!("Preferences were not saved. {error}"), 4_000));
        }
        Msg::Pulse(pulse) => on_pulse(model, pulse),
        Msg::Sweep(index, reading) => {
            if let Some(slot) = model.sweep.get_mut(usize::from(index)) {
                *slot = reading;
            }
        }
    }
}

fn on_pulse(model: &mut Model, pulse: Pulse) {
    if let Some(cpu) = pulse.cpu_percent {
        model.cpu_percent = Some(cpu.min(100));
        push_sample(&mut model.cpu_history, cpu.min(100));
    }
    if let Some(memory) = pulse.memory {
        model.facts.memory = Some(memory);
        let used = memory.total_bytes.saturating_sub(memory.available_bytes);
        let share = used
            .saturating_mul(100)
            .checked_div(memory.total_bytes)
            .map(|value| u8::try_from(value.min(100)).unwrap_or(100))
            .unwrap_or(0);
        push_sample(&mut model.memory_history, share);
    }
    if pulse.uptime_secs.is_some() {
        model.facts.uptime_secs = pulse.uptime_secs;
    }
}

fn push_sample(history: &mut VecDeque<u8>, value: u8) {
    if history.len() >= HISTORY {
        history.pop_front();
    }
    history.push_back(value);
}

fn advance(model: &mut Model, dt_ms: u32) {
    model.elapsed_ms = model.elapsed_ms.saturating_add(dt_ms);
    model.enter_ms = model.enter_ms.saturating_sub(dt_ms);
    if model.splash {
        model.splash_ms = model.splash_ms.saturating_add(dt_ms);
        if model.splash_ms >= 1_200 {
            model.splash = false;
        }
    }
    if let Some((_, left)) = model.toast.as_mut() {
        *left = left.saturating_sub(dt_ms);
        if *left == 0 {
            model.toast = None;
        }
    }
    if model.reduce_motion {
        model.snap_motion();
        return;
    }
    // Hold the numbers at zero while the splash covers them, so the climb is visible.
    if model.splash {
        return;
    }
    model.shown_score = ease_option(model.shown_score, model.score_target(), dt_ms);
    for pillar in Pillar::all() {
        let next = ease_u16(
            model.gauges.get(pillar),
            target_for(model.witness.as_ref(), pillar),
            dt_ms,
        );
        model.gauges.set(pillar, next);
    }
}

fn ease_option(shown: Option<u16>, target: Option<u16>, dt_ms: u32) -> Option<u16> {
    target.map(|target| ease_u16(shown.unwrap_or(0), target, dt_ms))
}

fn ease_u16(shown: u16, target: u16, dt_ms: u32) -> u16 {
    if shown == target {
        return shown;
    }
    let ticks = usize::try_from((dt_ms / 40).clamp(1, 30)).unwrap_or(1);
    let mut value = shown;
    for _ in 0..ticks {
        if value == target {
            break;
        }
        let step = value.abs_diff(target) / 3;
        let step = step.max(1);
        value = if value < target {
            value.saturating_add(step).min(target)
        } else {
            value.saturating_sub(step).max(target)
        };
    }
    value
}

fn leaves(key: Key) -> bool {
    matches!(key, Key::Quit | Key::Char('q') | Key::Char('Q'))
}

fn on_key(model: &mut Model, key: Key) {
    // `q` leaves from the splash, the palette, help, and a toast. It used to be
    // swallowed as an unused character, so the keep would not close.
    if leaves(key) {
        model.quit = true;
        return;
    }
    if model.splash {
        model.splash = false;
        return;
    }
    if model.palette {
        on_palette_key(model, key);
        return;
    }
    if model.help {
        match key {
            Key::Esc | Key::Help | Key::Char('?') => model.help = false,
            _ => {}
        }
        return;
    }
    if model.settings {
        on_settings_key(model, key);
        return;
    }
    match key {
        Key::Help | Key::Char('?') => model.help = true,
        Key::Palette | Key::Char(':') => open_palette(model),
        Key::Esc => {
            if model.toast.is_some() {
                model.toast = None;
            } else {
                model.quit = true;
            }
        }
        Key::Tab | Key::Char('l') | Key::Char('L') => {
            let tab = model.tab.next();
            set_tab(model, tab);
        }
        Key::BackTab | Key::Char('h') | Key::Char('H') => {
            let tab = model.tab.prev();
            set_tab(model, tab);
        }
        Key::Digit(digit) => {
            if let Some(tab) = Tab::from_digit(digit) {
                set_tab(model, tab);
            }
        }
        Key::Char(ch @ '1'..='9') => {
            let digit = u8::try_from(u32::from(ch).saturating_sub(u32::from('0'))).unwrap_or(0);
            if let Some(tab) = Tab::from_digit(digit) {
                set_tab(model, tab);
            }
        }
        Key::Up | Key::Char('k') | Key::Char('K') => {
            model.cursor = model.cursor.saturating_sub(1);
        }
        Key::Down | Key::Char('j') | Key::Char('J') => move_down(model),
        Key::Enter | Key::Space => on_open(model),
        Key::Char('s') | Key::Char('S') => open_settings(model),
        Key::Quit | Key::Char(_) | Key::Backspace => {}
    }
}

pub(crate) const SETTING_ROWS: u8 = 6;
const ENTER_MS: u32 = 180;
const AGE_ROW: u8 = 5;
const HOUR_SECS: u64 = 60 * 60;
const MAX_AGE_SECS: u64 = 30 * 24 * 60 * 60;

fn open_settings(model: &mut Model) {
    model.settings = true;
    model.settings_cursor = 0;
}

fn on_settings_key(model: &mut Model, key: Key) {
    match key {
        Key::Esc => model.settings = false,
        Key::Help | Key::Char('?') => {
            model.settings = false;
            model.help = true;
        }
        Key::Char('s') | Key::Char('S') => model.settings = false,
        Key::Up | Key::Char('k') | Key::Char('K') => {
            model.settings_cursor = model.settings_cursor.saturating_sub(1);
        }
        Key::Down | Key::Char('j') | Key::Char('J') => {
            if model.settings_cursor + 1 < SETTING_ROWS {
                model.settings_cursor += 1;
            }
        }
        Key::Enter | Key::Space => toggle_setting(model),
        Key::Tab | Key::Char('l') | Key::Char('L') => {
            if model.settings_cursor == AGE_ROW {
                nudge_age(model, true);
            } else {
                let tab = model.tab.next();
                set_tab(model, tab);
            }
        }
        Key::BackTab | Key::Char('h') | Key::Char('H') => {
            if model.settings_cursor == AGE_ROW {
                nudge_age(model, false);
            } else {
                let tab = model.tab.prev();
                set_tab(model, tab);
            }
        }
        Key::Digit(digit) => {
            if let Some(tab) = Tab::from_digit(digit) {
                set_tab(model, tab);
            }
        }
        Key::Char(ch @ '1'..='9') => {
            let digit = u8::try_from(u32::from(ch).saturating_sub(u32::from('0'))).unwrap_or(0);
            if let Some(tab) = Tab::from_digit(digit) {
                set_tab(model, tab);
            }
        }
        Key::Quit | Key::Char(_) | Key::Backspace | Key::Palette => {}
    }
}

fn toggle_setting(model: &mut Model) {
    match model.settings_cursor {
        0 => model.preferences.user_temp_at_sign_in = !model.preferences.user_temp_at_sign_in,
        1 => model.preferences.windows_temp_at_sign_in = !model.preferences.windows_temp_at_sign_in,
        2 => {
            model.preferences.reduce_motion = !model.preferences.reduce_motion;
            model.reduce_motion = model.preferences.reduce_motion;
            if model.reduce_motion {
                model.snap_motion();
            }
        }
        3 => {
            model.preferences.ascii_art = !model.preferences.ascii_art;
            model.ascii = model.preferences.ascii_art;
        }
        4 => model.preferences.lockdown = !model.preferences.lockdown,
        _ => return,
    }
    model.prefs_dirty = true;
}

fn nudge_age(model: &mut Model, up: bool) {
    let current = model.preferences.min_age_secs;
    let next = if up {
        current.saturating_add(HOUR_SECS).min(MAX_AGE_SECS)
    } else {
        current.saturating_sub(HOUR_SECS).max(ABSOLUTE_MIN_AGE_SECS)
    };
    if next != current {
        model.preferences.min_age_secs = next;
        model.prefs_dirty = true;
    }
}

fn open_palette(model: &mut Model) {
    model.palette = true;
    model.palette_query.clear();
    model.palette_cursor = 0;
}

fn set_tab(model: &mut Model, tab: Tab) {
    if model.tab != tab && !model.reduce_motion {
        model.enter_ms = ENTER_MS;
    }
    model.tab = tab;
    model.cursor = 0;
    model.settings = false;
}

fn move_down(model: &mut Model) {
    let max = room_len(model).saturating_sub(1);
    if u16::from(model.cursor) < max {
        model.cursor = model.cursor.saturating_add(1);
    }
}

fn on_palette_key(model: &mut Model, key: Key) {
    match key {
        Key::Esc => model.palette = false,
        Key::Backspace => {
            model.palette_query.pop();
            model.palette_cursor = 0;
        }
        Key::Char(ch) => {
            if model.palette_query.len() < 32 {
                model.palette_query.push(ch);
                model.palette_cursor = 0;
            }
        }
        Key::Up => model.palette_cursor = model.palette_cursor.saturating_sub(1),
        Key::Down => {
            let len = palette_hits(&model.palette_query).len();
            if usize::from(model.palette_cursor).saturating_add(1) < len {
                model.palette_cursor = model.palette_cursor.saturating_add(1);
            }
        }
        Key::Enter => {
            let hits = palette_hits(&model.palette_query);
            if let Some((_, pick)) = hits.get(usize::from(model.palette_cursor)).copied() {
                model.palette = false;
                match pick {
                    Pick::Room(tab) => set_tab(model, tab),
                    Pick::Settings => open_settings(model),
                }
            }
        }
        _ => {}
    }
}

fn room_len(model: &Model) -> u16 {
    match model.tab {
        Tab::Sweep => u16::from(SWEEP_ROWS),
        Tab::Overview => u16::try_from(
            model
                .witness
                .as_ref()
                .map(|witness| witness.attention.len())
                .unwrap_or(0),
        )
        .unwrap_or(0),
        _ => 1,
    }
}

fn on_open(model: &mut Model) {
    let text = match model.tab {
        Tab::Overview => {
            let room = model
                .witness
                .as_ref()
                .and_then(|witness| witness.attention.get(usize::from(model.cursor)))
                .and_then(|item| item.room);
            if let Some(room) = room {
                set_tab(model, room);
                return;
            }
            "There is nothing to open here."
        }
        Tab::Sweep if model.cursor >= 2 => {
            "Prefetch is never cleaned. It is what makes Windows and your apps start faster."
        }
        Tab::Sweep => {
            "Preview only. The cleaner is not connected in this build, so nothing was deleted."
        }
        Tab::Vitals => "These readings are live. There is nothing to run here.",
        Tab::Ledger => "The ledger has no entries yet, because nothing has been changed.",
        _ => "This room is not built yet. Nothing runs from here.",
    };
    model.toast = Some((text.to_owned(), 3_200));
}
