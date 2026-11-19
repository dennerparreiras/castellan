//! Turns measured facts into findings and a Keep Score.
//!
//! The engine does not delete files and does not open a socket. Callers pass
//! measurements in. That keeps the score testable without the live machine.

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

use castellan_core::{
    Band, Evidence, EvidenceValue, Finding, FindingId, LocalizedKey, Pillar, ScoreInput,
    ScoreReport, Severity, Status, score,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Engine error.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum EngineError {
    /// A measurement could not become a finding.
    #[error("{0}")]
    Finding(String),
}

/// Free space on one volume. `label` is a drive letter or mount, not a file path from a user.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeSpace {
    /// Short label such as `C:`.
    pub label: String,
    /// Capacity in bytes.
    pub total_bytes: u64,
    /// Free bytes.
    pub free_bytes: u64,
}

/// Result of the probes that exist in this build.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inspection {
    /// Findings.
    pub findings: Vec<Finding>,
    /// Score over the pillars those findings assessed.
    pub score: ScoreReport,
    /// Band safe to show. Suppressed when most of the keep was not inspected.
    pub display_band: Option<Band>,
}

/// Inspect one volume. Other pillars stay unassessed, and the display band says so.
pub fn inspect_volume(space: &VolumeSpace) -> Result<Inspection, EngineError> {
    let finding = volume_finding(space)?;
    let findings = vec![finding];
    let score = score(ScoreInput {
        findings: &findings,
        assessed: &[Pillar::Storage],
    });
    let display_band = castellan_core::display_band(&score);
    Ok(Inspection {
        findings,
        score,
        display_band,
    })
}

/// Build `STORAGE-001` from free space.
///
/// Bands are 15%, 10%, and 5% free, the thresholds in the product plan.
/// Microsoft documents that Windows needs free space for updates and Storage Sense:
/// <https://support.microsoft.com/en-us/windows/manage-drive-space-with-storage-sense-654f6ada-7bfc-45e5-966b-e24aded96ad5>
pub fn volume_finding(space: &VolumeSpace) -> Result<Finding, EngineError> {
    if space.total_bytes == 0 || space.free_bytes > space.total_bytes {
        return Err(EngineError::Finding(
            "volume sizes are inconsistent".to_owned(),
        ));
    }
    let free_bp = u16::try_from((space.free_bytes.saturating_mul(10_000)) / space.total_bytes)
        .unwrap_or(10_000);
    let used_percent = u8::try_from(100u64.saturating_sub(u64::from(free_bp) / 100)).unwrap_or(100);
    let (severity, status) = if free_bp >= 1_500 {
        (Severity::Info, Status::Pass)
    } else if free_bp >= 1_000 {
        (Severity::Notice, Status::Fail)
    } else if free_bp >= 500 {
        (Severity::Warning, Status::Fail)
    } else {
        (Severity::Critical, Status::Fail)
    };
    let evidence = vec![
        Evidence::new(
            key("evidence.free_bytes")?,
            EvidenceValue::Bytes(space.free_bytes),
        )
        .map_err(|error| EngineError::Finding(error.to_string()))?,
        Evidence::new(
            key("evidence.used_percent")?,
            EvidenceValue::Count(u64::from(used_percent)),
        )
        .map_err(|error| EngineError::Finding(error.to_string()))?,
    ];
    Finding::builder(FindingId::new("STORAGE-001").map_err(to_engine)?, Pillar::Storage)
        .severity(severity)
        .status(status)
        .title(key("finding.storage.free.title")?)
        .impact(key("finding.storage.free.impact")?)
        .evidence(evidence)
        .references(vec![
            "https://support.microsoft.com/en-us/windows/manage-drive-space-with-storage-sense-654f6ada-7bfc-45e5-966b-e24aded96ad5".to_owned(),
        ])
        .build()
        .map_err(to_engine)
}

fn key(value: &str) -> Result<LocalizedKey, EngineError> {
    LocalizedKey::new(value).map_err(to_engine)
}

fn to_engine(error: impl ToString) -> EngineError {
    EngineError::Finding(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_healthy_volume_passes_and_does_not_pretend_the_whole_keep_was_inspected() {
        let inspection = inspect_volume(&VolumeSpace {
            label: "D:".to_owned(),
            total_bytes: 1000,
            free_bytes: 400,
        })
        .expect("inspect");
        assert_eq!(inspection.findings[0].status(), Status::Pass);
        assert_eq!(inspection.display_band, None);
        assert!(inspection.score.confidence_millis < 500);
    }

    #[test]
    fn five_percent_free_is_critical() {
        let finding = volume_finding(&VolumeSpace {
            label: "D:".to_owned(),
            total_bytes: 1000,
            free_bytes: 40,
        })
        .expect("finding");
        assert_eq!(finding.severity(), Severity::Critical);
        assert_eq!(finding.status(), Status::Fail);
    }
}
