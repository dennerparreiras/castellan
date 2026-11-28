//! Prints still frames of the keep as plain text, for design review.
//!
//! `cargo run -p castellan-tui --example gallery -- 100 30`
//!
//! The frames use a made-up witness. Nothing here reads the machine.

use castellan_core::{Band, Pillar, PillarScore, ScoreReport, Severity};
use castellan_tui::{
    Attention, Drive, Facts, KeepWitness, Key, Memory, Model, Msg, Pulse, Session, SweepReading,
    SweepSize, Tab, update, view,
};
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

fn sample(ascii: bool) -> Model {
    let mut model = Model::open(Session {
        persona: "Lighting the torches.".to_owned(),
        hostname: "KEEP-SAMPLE".to_owned(),
        clock: "14:02".to_owned(),
        witness: Some(KeepWitness {
            score: ScoreReport {
                keep_score: Some(64),
                band: Some(Band::Sound),
                confidence_millis: 220,
                pillars: vec![
                    pillar(Pillar::Storage, Some(64)),
                    pillar(Pillar::Stability, None),
                    pillar(Pillar::Performance, None),
                    pillar(Pillar::Security, None),
                    pillar(Pillar::Network, None),
                ],
                lines: Vec::new(),
            },
            show_band: false,
            attention: vec![Attention {
                id: "STORAGE-001".to_owned(),
                severity: Severity::Warning,
                title: "System drive is filling".to_owned(),
                detail: "27.8 GB free of 464.7 GB".to_owned(),
                advice: "Clearing old temporary files can win back space. Sweep shows how much."
                    .to_owned(),
                room: Some(Tab::Sweep),
            }],
            persona: "One thing needs your attention: the system drive is 94% full.".to_owned(),
        }),
        plain: false,
        ascii,
        reduce_motion: true,
        facts: Facts {
            drives: vec![
                Drive {
                    label: "C:".to_owned(),
                    total_bytes: 498_960_000_000,
                    free_bytes: 29_850_000_000,
                    system: true,
                },
                Drive {
                    label: "B:".to_owned(),
                    total_bytes: 1_000_200_000_000,
                    free_bytes: 412_000_000_000,
                    system: false,
                },
            ],
            memory: Some(MEMORY),
            uptime_secs: Some(9 * 86_400 + 4 * 3_600),
            logical_cpus: Some(16),
            dev_build: true,
        },
        sweep: Some([
            SweepReading::Measured(SweepSize {
                files: 3_412,
                bytes: 1_288_490_188,
                aged_files: 2_107,
                aged_bytes: 944_892_805,
                truncated: false,
            }),
            SweepReading::Unavailable(
                "Windows lets only administrators read part of this folder, so it was not measured."
                    .to_owned(),
            ),
        ]),
    });
    for cpu in [12u8, 18, 9, 31, 45, 22, 14, 60, 38, 17, 11, 25, 19, 8, 23] {
        update(
            &mut model,
            Msg::Pulse(Pulse {
                cpu_percent: Some(cpu),
                memory: Some(MEMORY),
                uptime_secs: None,
            }),
        );
    }
    model
}

const MEMORY: Memory = Memory {
    total_bytes: 34_359_738_368,
    available_bytes: 13_100_000_000,
};

fn draw(model: &Model, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let Ok(mut terminal) = Terminal::new(backend);
    if terminal.draw(|frame| view(model, frame)).is_err() {
        return String::new();
    }
    let buffer = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..buffer.area.height {
        let mut row = String::new();
        for x in 0..buffer.area.width {
            row.push_str(buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" "));
        }
        out.push_str(row.trim_end());
        out.push('\n');
    }
    out
}

fn main() {
    let mut args = std::env::args().skip(1);
    let width = args.next().and_then(|v| v.parse().ok()).unwrap_or(100u16);
    let height = args.next().and_then(|v| v.parse().ok()).unwrap_or(30u16);
    let ascii = args.next().as_deref() == Some("ascii");
    let only = args.next();
    let mut shots: Vec<(String, Model)> = Vec::new();
    for digit in 1..=9u8 {
        let mut model = sample(ascii);
        update(&mut model, Msg::Key(Key::Digit(digit)));
        shots.push((format!("room {digit}"), model));
    }
    let mut settings = sample(ascii);
    update(&mut settings, Msg::Key(Key::Char('s')));
    shots.push(("settings".to_owned(), settings));
    let mut help = sample(ascii);
    update(&mut help, Msg::Key(Key::Char('?')));
    shots.push(("help".to_owned(), help));
    let mut palette = sample(ascii);
    update(&mut palette, Msg::Key(Key::Char(':')));
    shots.push(("palette".to_owned(), palette));
    let mut toast = sample(ascii);
    update(&mut toast, Msg::Key(Key::Digit(2)));
    update(&mut toast, Msg::Key(Key::Enter));
    shots.push(("toast".to_owned(), toast));
    let mut splash = sample(ascii);
    splash.splash = true;
    splash.splash_ms = 900;
    shots.push(("splash".to_owned(), splash));
    shots.push(("empty".to_owned(), Model::fresh("Lighting the torches.")));
    for (name, model) in shots {
        if only.as_deref().is_some_and(|want| !name.contains(want)) {
            continue;
        }
        println!("===== {name} ({width}x{height}) =====");
        print!("{}", draw(&model, width, height));
    }
}
