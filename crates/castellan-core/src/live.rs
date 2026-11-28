//! Plain data that every frontend shows: facts about the PC, live readings,
//! sweep measurements, and the overview the service builds.
//!
//! These types carry no behavior and do no I/O. `castellan-sense` fills them,
//! `castellan-service` hands them out, and the GUI, CLI, TUI, and MCP draw them.
//! With the `ts` feature they also describe themselves to TypeScript, so the
//! desktop frontend uses the same shapes.
//!
//! Byte counts are `u64` in Rust and `number` in TypeScript (the exporter maps
//! large integers to `number`). JSON numbers are
//! exact up to 2^53 bytes (8 PiB), far above any local disk.

use serde::{Deserialize, Serialize};

use crate::model::Severity;
use crate::score::{Band, ScoreReport};

/// Physical memory, as measured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Memory {
    /// Installed memory Windows can use.
    pub total_bytes: u64,
    /// Memory that is free or can be reclaimed right now.
    pub available_bytes: u64,
}

/// One fixed local drive, as measured.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
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

/// Plain facts about this PC. A field that could not be read stays empty.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Facts {
    /// Fixed local drives. The Windows drive comes first.
    pub drives: Vec<Drive>,
    /// Physical memory.
    pub memory: Option<Memory>,
    /// Seconds since Windows started.
    pub uptime_secs: Option<u64>,
    /// Logical processors available to this process.
    pub logical_cpus: Option<u32>,
    /// This binary is a development build, which refuses real changes.
    pub dev_build: bool,
}

/// A fresh live reading, sent about once a second.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Pulse {
    /// Busy share of all processors since the previous reading, 0 to 100.
    pub cpu_percent: Option<u8>,
    /// Physical memory right now.
    pub memory: Option<Memory>,
    /// Seconds since Windows started.
    pub uptime_secs: Option<u64>,
}

/// Size of one sweep target from a read-only walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(tag = "state", content = "detail", rename_all = "camelCase")]
pub enum SweepReading {
    /// The read-only walk is still running.
    Measuring,
    /// The walk finished.
    Measured(SweepSize),
    /// The folder could not be read. The text says why, in plain words.
    Unavailable(String),
}

/// The read-only targets the sweep screens measure.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum MeasuredTarget {
    /// The signed-in user's temporary folder.
    UserTemp,
    /// The Windows temporary folder.
    WindowsTemp,
}

impl MeasuredTarget {
    /// Both targets, in display order.
    pub const ALL: [Self; 2] = [Self::UserTemp, Self::WindowsTemp];

    /// Position in [`Self::ALL`].
    #[must_use]
    pub const fn index(self) -> u8 {
        match self {
            Self::UserTemp => 0,
            Self::WindowsTemp => 1,
        }
    }
}

/// A screen a frontend can open. Rooms are shared by the GUI and the TUI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum Room {
    /// Keep Score and what needs attention.
    Overview,
    /// Cleaning.
    Sweep,
    /// Hardware and system health.
    Vitals,
    /// Network.
    Network,
    /// Security posture.
    Shield,
    /// Startup and performance.
    Boost,
    /// Game mode.
    Game,
    /// Guided repairs.
    Fix,
    /// Record of every change.
    Ledger,
}

/// A finding, worded for a person.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct FindingView {
    /// Stable id, such as `STORAGE-001`.
    pub id: String,
    /// How serious it is.
    pub severity: Severity,
    /// One line, in English.
    pub title: String,
    /// Evidence the person can check.
    pub detail: String,
    /// What the person can do about it, in one sentence.
    pub advice: String,
    /// Room that helps, if one does.
    pub room: Option<Room>,
}

/// Free space on the Windows volume, as read for the score.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct VolumeReading {
    /// Short label, such as `C:`.
    pub label: String,
    /// Capacity in bytes.
    pub total_bytes: u64,
    /// Free bytes.
    pub free_bytes: u64,
}

/// Everything the overview screens show, built from measurements.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    /// Score and its explanation. Unassessed pillars have no number.
    pub score: ScoreReport,
    /// Band safe to show. `None` while too little was inspected.
    pub display_band: Option<Band>,
    /// Findings that lower the score, most serious first.
    pub findings: Vec<FindingView>,
    /// One persona line for this reading.
    pub persona: String,
    /// The Windows volume the score read.
    pub system_volume: VolumeReading,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_readings_keep_a_stable_json_shape() {
        let measured = SweepReading::Measured(SweepSize {
            files: 2,
            bytes: 8,
            aged_files: 1,
            aged_bytes: 5,
            truncated: false,
        });
        let json = serde_json::to_string(&measured).expect("json");
        assert_eq!(
            json,
            r#"{"state":"measured","detail":{"files":2,"bytes":8,"agedFiles":1,"agedBytes":5,"truncated":false}}"#
        );
        let json = serde_json::to_string(&SweepReading::Measuring).expect("json");
        assert_eq!(json, r#"{"state":"measuring"}"#);
        let back: SweepReading =
            serde_json::from_str(r#"{"state":"unavailable","detail":"denied"}"#).expect("parse");
        assert_eq!(back, SweepReading::Unavailable("denied".to_owned()));
    }

    #[test]
    fn targets_know_their_position() {
        for (index, target) in MeasuredTarget::ALL.into_iter().enumerate() {
            assert_eq!(usize::from(target.index()), index);
        }
    }
}
