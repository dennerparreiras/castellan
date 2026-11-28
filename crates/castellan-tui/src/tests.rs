//! Frames drawn into a test backend. Nothing here touches the machine.

use super::*;
use castellan_core::{Band, Pillar, PillarScore, ScoreReport, Severity};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn pillar(pillar: Pillar, score: Option<u8>) -> PillarScore {
    PillarScore {
        pillar,
        score,
        confidence_millis: if score.is_some() { 1000 } else { 0 },
        veto_ceiling: None,
    }
}

fn partial_storage() -> ScoreReport {
    ScoreReport {
        keep_score: Some(88),
        band: Some(Band::Sound),
        confidence_millis: 220,
        pillars: vec![
            pillar(Pillar::Storage, Some(88)),
            pillar(Pillar::Stability, None),
            pillar(Pillar::Performance, None),
            pillar(Pillar::Security, None),
            pillar(Pillar::Network, None),
        ],
        lines: Vec::new(),
    }
}

const GB: u64 = 1024 * 1024 * 1024;

fn facts() -> Facts {
    Facts {
        drives: vec![Drive {
            label: "C:".to_owned(),
            total_bytes: 500 * GB,
            free_bytes: 30 * GB,
            system: true,
        }],
        memory: Some(Memory {
            total_bytes: 16 * GB,
            available_bytes: 6 * GB,
        }),
        uptime_secs: Some(3 * 86_400),
        logical_cpus: Some(8),
        dev_build: true,
    }
}

fn session(witness: Option<KeepWitness>, reduce_motion: bool) -> Session {
    Session {
        persona: "fallback".to_owned(),
        hostname: "KEEP-TEST".to_owned(),
        clock: "14:02".to_owned(),
        witness,
        reduce_motion,
        facts: facts(),
        sweep: Some([
            SweepReading::Measured(SweepSize {
                files: 1_200,
                bytes: 2 * GB,
                aged_files: 900,
                aged_bytes: GB,
                truncated: false,
            }),
            SweepReading::Unavailable(
                "Windows lets only administrators read this folder.".to_owned(),
            ),
        ]),
        ..Session::default()
    }
}

fn storage_witness(attention: Vec<Attention>) -> KeepWitness {
    KeepWitness {
        score: partial_storage(),
        show_band: false,
        attention,
        persona: "One thing needs your attention: the system drive is 94% full.".to_owned(),
    }
}

fn filling() -> Attention {
    Attention {
        id: "STORAGE-001".to_owned(),
        severity: Severity::Warning,
        title: "System drive is filling".to_owned(),
        detail: "27.8 GB free of 464.7 GB".to_owned(),
        advice: "Sweep shows how much space old temporary files take.".to_owned(),
        room: Some(Tab::Sweep),
    }
}

fn witnessed() -> Model {
    Model::open(session(Some(storage_witness(vec![filling()])), true))
}

fn draw(model: &Model, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|frame| view(model, frame)).expect("draw");
    let mut out = String::new();
    let buffer = terminal.backend().buffer();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let symbol = buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" ");
            out.push_str(symbol);
        }
        out.push('\n');
    }
    out
}

const SIZES: [(u16, u16); 4] = [(80, 24), (100, 30), (120, 40), (200, 60)];

#[test]
fn keys_move_through_the_keep_without_drawing() {
    let mut model = Model::fresh("Nothing to fix here. The walls hold.");
    update(&mut model, Msg::Key(Key::Digit(2)));
    assert_eq!(model.tab, Tab::Sweep);
    update(&mut model, Msg::Key(Key::Char('9')));
    assert_eq!(model.tab, Tab::Ledger);
    update(&mut model, Msg::Key(Key::Char('?')));
    assert!(model.help);
    update(&mut model, Msg::Key(Key::Esc));
    assert!(!model.help);
    update(&mut model, Msg::Key(Key::Char(':')));
    assert!(model.palette);
    update(&mut model, Msg::Key(Key::Char('h')));
    assert!(model.palette_query.contains('h'));
    update(&mut model, Msg::Key(Key::Esc));
    update(&mut model, Msg::Key(Key::Char('l')));
    assert_eq!(model.tab, Tab::Overview);
    update(&mut model, Msg::Key(Key::Quit));
    assert!(model.quit);
}

#[test]
fn a_score_eases_toward_the_measured_value() {
    let mut model = Model::open(session(Some(storage_witness(Vec::new())), false));
    assert!(model.splash);
    update(&mut model, Msg::Key(Key::Char('x')));
    assert!(!model.splash);
    assert!(!model.quit);
    assert_eq!(model.shown_score, None);
    assert!(model.pace_ms() < 100, "easing should draw fast");
    update(&mut model, Msg::Tick { dt_ms: 40 });
    assert_eq!(model.shown_score, Some(29));
    update(&mut model, Msg::Tick { dt_ms: 1_000 });
    assert_eq!(model.shown_score, Some(88));
    update(&mut model, Msg::Tick { dt_ms: 1_000 });
    assert!(model.pace_ms() >= 200, "an idle keep should redraw slowly");
}

#[test]
fn the_splash_names_the_keep_and_any_key_skips_it() {
    let mut model = Model::open(Session {
        persona: "Lighting the torches.".to_owned(),
        ..session(None, false)
    });
    let text = draw(&model, 80, 24);
    assert!(text.contains("C A S T E L L A N"), "{text}");
    assert!(text.contains("Lighting the torches."), "{text}");
    assert!(text.contains("Denner Parreiras"), "{text}");
    assert!(text.contains("q quits"), "{text}");
    update(&mut model, Msg::Tick { dt_ms: 1_199 });
    assert!(model.splash);
    update(&mut model, Msg::Tick { dt_ms: 1 });
    assert!(!model.splash);
}

#[test]
fn an_ascii_splash_has_no_block_glyphs() {
    let model = Model::open(Session {
        ascii: true,
        ..session(Some(storage_witness(Vec::new())), false)
    });
    let text = draw(&model, 80, 24);
    assert!(text.contains("System drive read"), "{text}");
    assert!(!text.contains('█'), "{text}");
}

#[test]
fn overview_before_a_reading_does_not_invent_a_score() {
    let model = Model::fresh("The system drive could not be read.");
    let text = draw(&model, 80, 24);
    assert!(text.contains("CASTELLAN"), "{text}");
    assert!(text.contains("no reading"), "{text}");
    assert!(text.contains("not checked yet"), "{text}");
    assert!(text.contains("could not be read"), "{text}");
    assert!(!text.contains("Sound"), "{text}");
}

#[test]
fn a_partial_score_stays_partial_even_when_the_band_was_computed() {
    for (width, height) in SIZES {
        let text = draw(&witnessed(), width, height);
        assert!(text.contains("88"), "{width}x{height}\n{text}");
        assert!(text.contains("partial"), "{width}x{height}\n{text}");
        assert!(text.contains("not checked yet"), "{width}x{height}\n{text}");
        assert!(
            text.contains("System drive is filling"),
            "{width}x{height}\n{text}"
        );
        assert!(text.contains("27.8 GB free"), "{width}x{height}\n{text}");
        // The legend explains the bands. The score itself never claims one.
        assert!(!text.contains("♥ 88 sound"), "{width}x{height}\n{text}");
    }
}

#[test]
fn every_room_names_every_tab_at_every_size() {
    for (width, height) in SIZES {
        for digit in 1..=9u8 {
            let mut model = witnessed();
            update(&mut model, Msg::Key(Key::Digit(digit)));
            let text = draw(&model, width, height);
            for tab in Tab::ALL {
                assert!(
                    text.contains(tab.label()),
                    "{width}x{height} room {digit}\n{text}"
                );
            }
            assert!(
                text.contains("quit"),
                "{width}x{height} room {digit}\n{text}"
            );
        }
    }
}

#[test]
fn ascii_mode_draws_every_room_without_box_or_block_glyphs() {
    for digit in 1..=9u8 {
        let mut model = Model::open(Session {
            ascii: true,
            ..session(Some(storage_witness(vec![filling()])), true)
        });
        update(&mut model, Msg::Key(Key::Digit(digit)));
        let text = draw(&model, 100, 30);
        for glyph in ['█', '━', '▸', '●', '╭', '▁'] {
            assert!(!text.contains(glyph), "room {digit} has {glyph}\n{text}");
        }
    }
}

#[test]
fn a_small_terminal_asks_for_80x24() {
    let model = Model::fresh("");
    let text = draw(&model, 20, 8);
    assert!(text.contains("80x24"), "{text}");
}

#[test]
fn q_leaves_from_the_keep_the_splash_and_the_palette() {
    let mut model = Model::fresh("Nothing to fix here. The walls hold.");
    update(&mut model, Msg::Key(Key::Char('q')));
    assert!(model.quit);

    let mut splash = Model::open(session(None, false));
    assert!(splash.splash);
    update(&mut splash, Msg::Key(Key::Char('Q')));
    assert!(splash.quit);

    let mut palette = Model::fresh("Nothing to fix here. The walls hold.");
    update(&mut palette, Msg::Key(Key::Char(':')));
    update(&mut palette, Msg::Key(Key::Char('q')));
    assert!(palette.quit);
}

#[test]
fn a_click_on_a_tab_opens_that_room() {
    let model = Model::fresh("");
    // At 80 columns the header is 78 wide and starts at column 1.
    assert_eq!(pointer(&model, 80, 24, 3, 1), Some(Msg::Key(Key::Digit(1))));
    assert_eq!(
        pointer(&model, 80, 24, 74, 1),
        Some(Msg::Key(Key::Digit(9)))
    );
    assert_eq!(pointer(&model, 80, 24, 3, 0), None);
}

#[test]
fn a_click_selects_a_sweep_target() {
    let mut model = witnessed();
    update(&mut model, Msg::Key(Key::Digit(2)));
    // The body starts at row 3 with the card border, so the first target is row 4.
    assert_eq!(pointer(&model, 100, 30, 5, 4), Some(Msg::Select(0)));
    assert_eq!(pointer(&model, 100, 30, 5, 7), Some(Msg::Select(1)));
    assert_eq!(pointer(&model, 100, 30, 5, 6), None);
}

#[test]
fn enter_on_a_finding_opens_its_room() {
    let mut model = witnessed();
    update(&mut model, Msg::Key(Key::Enter));
    assert_eq!(model.tab, Tab::Sweep);
}

#[test]
fn settings_saves_a_choice_and_says_it_is_not_active() {
    let mut model = Model::fresh("");
    update(&mut model, Msg::Key(Key::Char('s')));
    assert!(model.settings);
    update(&mut model, Msg::Key(Key::Space));
    assert!(model.preferences.user_temp_at_sign_in);
    assert!(model.prefs_dirty);
    let text = draw(&model, 100, 30);
    assert!(text.contains("NOT ACTIVE YET"), "{text}");
    assert!(text.contains("register a task"), "{text}");
    update(&mut model, Msg::Key(Key::Char('q')));
    assert!(model.quit);
}

#[test]
fn sweep_shows_measured_sizes_and_refuses_prefetch() {
    let mut model = witnessed();
    update(&mut model, Msg::Key(Key::Digit(2)));
    let text = draw(&model, 100, 30);
    assert!(text.contains("2 GB"), "{text}");
    assert!(text.contains("1,200 files"), "{text}");
    assert!(text.contains("could not be read"), "{text}");
    assert!(text.contains("Prefetch"), "{text}");
    update(&mut model, Msg::Key(Key::Enter));
    let text = draw(&model, 100, 30);
    assert!(text.contains("nothing was deleted"), "{text}");
    update(&mut model, Msg::Key(Key::Esc));
    update(&mut model, Msg::Key(Key::Down));
    update(&mut model, Msg::Key(Key::Down));
    let text = draw(&model, 100, 30);
    assert!(text.contains("NEVER CLEANS IT"), "{text}");
}

#[test]
fn vitals_draws_live_readings() {
    let mut model = witnessed();
    update(&mut model, Msg::Key(Key::Digit(3)));
    let before = draw(&model, 100, 30);
    assert!(before.contains("Measuring"), "{before}");
    for cpu in [10, 40, 70] {
        update(
            &mut model,
            Msg::Pulse(Pulse {
                cpu_percent: Some(cpu),
                memory: None,
                uptime_secs: Some(10 * 86_400),
            }),
        );
    }
    let text = draw(&model, 100, 30);
    assert_eq!(model.cpu_percent, Some(70));
    assert!(text.contains("% busy"), "{text}");
    assert!(text.contains("8 logical processors"), "{text}");
    assert!(text.contains("10 d 0 h"), "{text}");
    assert!(text.contains("more than a week"), "{text}");
    assert!(text.contains("30 GB free"), "{text}");
}
