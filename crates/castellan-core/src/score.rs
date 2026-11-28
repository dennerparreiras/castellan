//! Keep Score. Integer arithmetic only, so the same findings always yield the same number.
//!
//! The formula is specified in `docs/reference/health-score.md`. Change the formula, the doc,
//! and the golden tests together.

use crate::model::{Finding, Pillar, Severity, Status};
use serde::{Deserialize, Serialize};

/// Band shown next to the Keep Score.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum Band {
    /// 90–100.
    Fortified,
    /// 75–89.
    Sound,
    /// 50–74.
    NeedsAttention,
    /// Below 50.
    AtRisk,
}

impl Band {
    /// Band for a score.
    #[must_use]
    pub const fn from_score(score: u8) -> Self {
        match score {
            90..=100 => Self::Fortified,
            75..=89 => Self::Sound,
            50..=74 => Self::NeedsAttention,
            _ => Self::AtRisk,
        }
    }

    /// English label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Fortified => "Fortified",
            Self::Sound => "Sound",
            Self::NeedsAttention => "Needs attention",
            Self::AtRisk => "At risk",
        }
    }
}

/// Pillar weights in thousandths. They sum to 1000.
const STORAGE_WEIGHT: u32 = 220;
const STABILITY_WEIGHT: u32 = 180;
const PERFORMANCE_WEIGHT: u32 = 180;
const SECURITY_WEIGHT: u32 = 240;
const NETWORK_WEIGHT: u32 = 180;

/// Once a pillar has already lost this many points, further notices and warnings hurt less.
const CURVE_KINK: u32 = 36;

/// Points removed for one failed finding, before the weight and the curve.
const fn base_points(severity: Severity) -> u32 {
    match severity {
        Severity::Info => 0,
        Severity::Notice => 2,
        Severity::Warning => 12,
        Severity::Critical => 40,
    }
}

fn curved_points(severity: Severity, penalty_so_far: u32) -> u32 {
    let base = base_points(severity);
    if severity == Severity::Critical || penalty_so_far < CURVE_KINK {
        return base;
    }
    match severity {
        Severity::Warning => base * 3 / 4,
        Severity::Notice => base / 2,
        Severity::Info | Severity::Critical => base,
    }
}

/// One line of the explanation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DeductionLine {
    /// Finding that caused the line.
    pub finding_id: String,
    /// Pillar.
    pub pillar: Pillar,
    /// Points actually removed by this finding after weight and curve.
    pub points: u32,
    /// Pillar ceiling applied because of this finding, if it was the tightest.
    pub veto_ceiling: Option<u8>,
}

/// Score of one pillar.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct PillarScore {
    /// Which pillar.
    pub pillar: Pillar,
    /// `None` when the pillar was not assessed.
    pub score: Option<u8>,
    /// Confidence in thousandths. Unassessed pillars are 0.
    pub confidence_millis: u16,
    /// Tightest veto ceiling applied, if any.
    pub veto_ceiling: Option<u8>,
}

/// The full Keep Score and the explanation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct ScoreReport {
    /// Overall score. `None` when nothing was assessed.
    pub keep_score: Option<u8>,
    /// Band for [`Self::keep_score`].
    pub band: Option<Band>,
    /// Confidence in the overall score, in thousandths. Unassessed weight counts as zero.
    pub confidence_millis: u16,
    /// One entry per pillar, in display order.
    pub pillars: Vec<PillarScore>,
    /// Deductions and vetoes, in the order they were applied.
    pub lines: Vec<DeductionLine>,
}

/// Inputs to the score. `assessed` lists pillars whose probes finished, even if they found nothing.
#[derive(Clone, Copy, Debug)]
pub struct ScoreInput<'a> {
    /// Findings produced by the probes.
    pub findings: &'a [Finding],
    /// Pillars that were actually run.
    pub assessed: &'a [Pillar],
}

/// Band to show a person. A thin inspection does not get to call the keep fortified.
#[must_use]
pub fn display_band(report: &ScoreReport) -> Option<Band> {
    if report.confidence_millis < 500 {
        None
    } else {
        report.band
    }
}

/// Compute the Keep Score.
#[must_use]
pub fn score(input: ScoreInput<'_>) -> ScoreReport {
    let mut failing: Vec<&Finding> = input
        .findings
        .iter()
        .filter(|finding| finding.deducts_score())
        .collect();
    failing.sort_by(|left, right| {
        right
            .severity()
            .rank()
            .cmp(&left.severity().rank())
            .then_with(|| left.id().as_str().cmp(right.id().as_str()))
    });

    let mut penalties = PillarPenalties::default();
    let mut lines = Vec::new();

    for finding in failing {
        let penalty_so_far = penalties.get(finding.pillar());
        let curved = curved_points(finding.severity(), penalty_so_far);
        let weighted = curved * u32::from(finding.score_weight().millis()) / 1000;
        let room = 100u32.saturating_sub(penalty_so_far);
        let applied = weighted.min(room);
        penalties.add(finding.pillar(), applied);
        lines.push(DeductionLine {
            finding_id: finding.id().as_str().to_owned(),
            pillar: finding.pillar(),
            points: applied,
            veto_ceiling: None,
        });
    }

    let mut ceilings = PillarCeilings::default();
    let mut overall_ceiling: Option<u8> = None;
    for finding in input.findings {
        if finding.status() != Status::Fail {
            continue;
        }
        let Some(veto) = finding.veto() else {
            continue;
        };
        if let Some(ceiling) = veto.pillar_ceiling {
            ceilings.tighten(finding.pillar(), ceiling);
        }
        if let Some(ceiling) = veto.overall_ceiling {
            overall_ceiling = Some(overall_ceiling.map_or(ceiling, |current| current.min(ceiling)));
        }
        lines.push(DeductionLine {
            finding_id: finding.id().as_str().to_owned(),
            pillar: finding.pillar(),
            points: 0,
            veto_ceiling: veto.pillar_ceiling,
        });
    }

    let mut pillars = Vec::with_capacity(5);
    let mut weighted_sum = 0u32;
    let mut weight_sum = 0u32;
    let mut confidence_acc = 0u32;

    for pillar in Pillar::all() {
        let assessed = input.assessed.contains(&pillar)
            || input
                .findings
                .iter()
                .any(|finding| finding.pillar() == pillar);
        if !assessed {
            pillars.push(PillarScore {
                pillar,
                score: None,
                confidence_millis: 0,
                veto_ceiling: None,
            });
            continue;
        }
        let mut value = 100u32.saturating_sub(penalties.get(pillar));
        let ceiling = ceilings.get(pillar);
        if let Some(ceiling) = ceiling {
            value = value.min(u32::from(ceiling));
        }
        let score_u8 = u8::try_from(value).unwrap_or(0);
        let confidence = pillar_confidence(input.findings, pillar);
        let weight = pillar_weight(pillar);
        weighted_sum += u32::from(score_u8) * weight;
        weight_sum += weight;
        confidence_acc += u32::from(confidence) * weight;
        pillars.push(PillarScore {
            pillar,
            score: Some(score_u8),
            confidence_millis: confidence,
            veto_ceiling: ceiling,
        });
    }

    let mut keep_score = (weighted_sum + weight_sum / 2)
        .checked_div(weight_sum)
        .and_then(|value| u8::try_from(value).ok());
    if let (Some(score_value), Some(ceiling)) = (keep_score, overall_ceiling) {
        keep_score = Some(score_value.min(ceiling));
    }
    let confidence_millis = u16::try_from(confidence_acc / 1000).unwrap_or(0);

    ScoreReport {
        band: keep_score.map(Band::from_score),
        keep_score,
        confidence_millis,
        pillars,
        lines,
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct PillarPenalties {
    storage: u32,
    stability: u32,
    performance: u32,
    security: u32,
    network: u32,
}

impl PillarPenalties {
    fn get(self, pillar: Pillar) -> u32 {
        match pillar {
            Pillar::Storage => self.storage,
            Pillar::Stability => self.stability,
            Pillar::Performance => self.performance,
            Pillar::Security => self.security,
            Pillar::Network => self.network,
        }
    }

    fn add(&mut self, pillar: Pillar, points: u32) {
        let slot = match pillar {
            Pillar::Storage => &mut self.storage,
            Pillar::Stability => &mut self.stability,
            Pillar::Performance => &mut self.performance,
            Pillar::Security => &mut self.security,
            Pillar::Network => &mut self.network,
        };
        *slot += points;
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct PillarCeilings {
    storage: Option<u8>,
    stability: Option<u8>,
    performance: Option<u8>,
    security: Option<u8>,
    network: Option<u8>,
}

impl PillarCeilings {
    fn get(self, pillar: Pillar) -> Option<u8> {
        match pillar {
            Pillar::Storage => self.storage,
            Pillar::Stability => self.stability,
            Pillar::Performance => self.performance,
            Pillar::Security => self.security,
            Pillar::Network => self.network,
        }
    }

    fn tighten(&mut self, pillar: Pillar, ceiling: u8) {
        let slot = match pillar {
            Pillar::Storage => &mut self.storage,
            Pillar::Stability => &mut self.stability,
            Pillar::Performance => &mut self.performance,
            Pillar::Security => &mut self.security,
            Pillar::Network => &mut self.network,
        };
        *slot = Some(slot.map_or(ceiling, |current| current.min(ceiling)));
    }
}

fn pillar_weight(pillar: Pillar) -> u32 {
    match pillar {
        Pillar::Storage => STORAGE_WEIGHT,
        Pillar::Stability => STABILITY_WEIGHT,
        Pillar::Performance => PERFORMANCE_WEIGHT,
        Pillar::Security => SECURITY_WEIGHT,
        Pillar::Network => NETWORK_WEIGHT,
    }
}

fn pillar_confidence(findings: &[Finding], pillar: Pillar) -> u16 {
    let mut decisive = 0u32;
    let mut unknown = 0u32;
    for finding in findings {
        if finding.pillar() != pillar {
            continue;
        }
        match finding.status() {
            Status::Unknown => unknown += 1,
            Status::Pass | Status::Fail | Status::NotApplicable => decisive += 1,
        }
    }
    let total = decisive + unknown;
    if total == 0 {
        return 1000;
    }
    u16::try_from(decisive * 1000 / total).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{FindingId, LocalizedKey};
    use crate::model::{Finding, ScoreVeto, ScoreWeight};

    fn finding(
        id: &str,
        pillar: Pillar,
        severity: Severity,
        status: Status,
        veto: Option<ScoreVeto>,
    ) -> Finding {
        let mut builder = Finding::builder(FindingId::new(id).unwrap(), pillar)
            .severity(severity)
            .status(status)
            .title(LocalizedKey::new("finding.title").unwrap())
            .impact(LocalizedKey::new("finding.impact").unwrap())
            .weight(ScoreWeight::one());
        if let Some(veto) = veto {
            builder = builder.veto(veto);
        }
        builder.build().unwrap()
    }

    fn report(findings: &[Finding]) -> ScoreReport {
        score(ScoreInput {
            findings,
            assessed: &Pillar::all(),
        })
    }

    #[test]
    fn empty_assessed_machine_is_fortified() {
        let report = report(&[]);
        assert_eq!(report.keep_score, Some(100));
        assert_eq!(report.band, Some(Band::Fortified));
        assert_eq!(report.confidence_millis, 1000);
    }

    #[test]
    fn nothing_assessed_has_no_score() {
        let report = score(ScoreInput {
            findings: &[],
            assessed: &[],
        });
        assert_eq!(report.keep_score, None);
        assert_eq!(report.confidence_millis, 0);
    }

    #[test]
    fn one_critical_outweighs_many_notices() {
        let critical = finding(
            "STORAGE-001",
            Pillar::Storage,
            Severity::Critical,
            Status::Fail,
            None,
        );
        let critical_report = report(std::slice::from_ref(&critical));
        let storage = critical_report
            .pillars
            .iter()
            .find(|pillar| pillar.pillar == Pillar::Storage)
            .unwrap();
        assert_eq!(storage.score, Some(60));

        let notices: Vec<Finding> = (1..=15)
            .map(|index| {
                finding(
                    &format!("NET-{index:03}"),
                    Pillar::Network,
                    Severity::Notice,
                    Status::Fail,
                    None,
                )
            })
            .collect();
        let notice_report = report(&notices);
        let network = notice_report
            .pillars
            .iter()
            .find(|pillar| pillar.pillar == Pillar::Network)
            .unwrap();
        // 15 * 2 = 30, still under the kink, so the pillar stays healthier than one critical.
        assert_eq!(network.score, Some(70));
        assert!(network.score.unwrap() > storage.score.unwrap());
    }

    #[test]
    fn info_and_unknown_do_not_deduct() {
        let findings = [
            finding(
                "SEC-001",
                Pillar::Security,
                Severity::Info,
                Status::Fail,
                None,
            ),
            finding(
                "SEC-002",
                Pillar::Security,
                Severity::Critical,
                Status::Unknown,
                None,
            ),
        ];
        let report = report(&findings);
        let security = report
            .pillars
            .iter()
            .find(|pillar| pillar.pillar == Pillar::Security)
            .unwrap();
        assert_eq!(security.score, Some(100));
        assert_eq!(security.confidence_millis, 500);
    }

    #[test]
    fn smart_veto_caps_storage() {
        let veto = ScoreVeto::new(Some(30), None).unwrap();
        let findings = [finding(
            "STORAGE-010",
            Pillar::Storage,
            Severity::Critical,
            Status::Fail,
            Some(veto),
        )];
        let report = report(&findings);
        let storage = report
            .pillars
            .iter()
            .find(|pillar| pillar.pillar == Pillar::Storage)
            .unwrap();
        assert_eq!(storage.score, Some(30));
        assert_eq!(storage.veto_ceiling, Some(30));
    }

    #[test]
    fn curve_shrinks_notices_after_the_kink() {
        let notices: Vec<Finding> = (1..=20)
            .map(|index| {
                finding(
                    &format!("PERF-{index:03}"),
                    Pillar::Performance,
                    Severity::Notice,
                    Status::Fail,
                    None,
                )
            })
            .collect();
        let report = report(&notices);
        let line_sum: u32 = report
            .lines
            .iter()
            .filter(|line| line.pillar == Pillar::Performance)
            .map(|line| line.points)
            .sum();
        // 18 notices reach 36. The 19th and 20th deduct 1 each.
        assert_eq!(line_sum, 38);
    }

    #[test]
    fn golden_keep_score_matches_the_documented_example() {
        let findings = [
            finding(
                "STORAGE-003",
                Pillar::Storage,
                Severity::Warning,
                Status::Fail,
                None,
            ),
            finding(
                "SEC-004",
                Pillar::Security,
                Severity::Critical,
                Status::Fail,
                Some(ScoreVeto::new(Some(40), None).unwrap()),
            ),
        ];
        let report = report(&findings);
        // Storage 100-12=88, Security min(100-40, 40)=40, others 100.
        // (88*220 + 100*180 + 100*180 + 40*240 + 100*180 + 500) / 1000
        // = (19360 + 18000 + 18000 + 9600 + 18000 + 500) / 1000 = 83460/1000 = 83
        assert_eq!(report.keep_score, Some(83));
        assert_eq!(report.band, Some(Band::Sound));
    }
}
