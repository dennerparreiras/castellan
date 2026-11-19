//! Findings, evidence, and the vocabulary shared by every frontend.

use crate::error::CoreError;
use crate::ids::{FindingId, LocalizedKey};
use serde::{Deserialize, Serialize};

/// How serious a finding is. `Info` never changes the health score.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Context the user may want. Not a problem.
    Info,
    /// Worth knowing. Small score impact when the check failed.
    Notice,
    /// Should be handled.
    Warning,
    /// Safety or data is at risk.
    Critical,
}

impl Severity {
    /// Rank used when ordering deductions. Critical is applied first.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Info => 0,
            Self::Notice => 1,
            Self::Warning => 2,
            Self::Critical => 3,
        }
    }
}

/// Outcome of one check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Status {
    /// The check passed.
    Pass,
    /// The check failed.
    Fail,
    /// The check could not be completed. Does not deduct points.
    Unknown,
    /// The check does not apply to this machine.
    NotApplicable,
}

/// One pillar of the Keep Score.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pillar {
    /// Disks, volumes, and storage reliability.
    Storage,
    /// Crashes, bugchecks, and unexpected shutdowns.
    Stability,
    /// Responsiveness, startup, and resource pressure.
    Performance,
    /// Security posture. Privacy preferences are not part of this score.
    Security,
    /// Connectivity quality.
    Network,
}

impl Pillar {
    /// Every pillar, in display order.
    #[must_use]
    pub const fn all() -> [Self; 5] {
        [
            Self::Storage,
            Self::Stability,
            Self::Performance,
            Self::Security,
            Self::Network,
        ]
    }

    /// Stable lowercase name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Storage => "storage",
            Self::Stability => "stability",
            Self::Performance => "performance",
            Self::Security => "security",
            Self::Network => "network",
        }
    }
}

/// Where a plan or ledger entry came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Origin {
    /// Full-screen terminal UI.
    Tui,
    /// Non-interactive or prompted command line.
    Cli,
    /// MCP client. Never elevated, and never allowed to run sensitive actions.
    Mcp,
    /// Scheduled morning or weekly round.
    Autopilot,
}

/// How much confirmation an action needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RiskTier {
    /// Reversible or low-impact maintenance, such as old user temp files.
    Safe,
    /// Noticeable change. Needs a simple confirmation unless the user passed `--yes`.
    Moderate,
    /// Security, repair tools, or hard-to-undo changes.
    Sensitive,
}

impl RiskTier {
    /// Higher tiers include the restrictions of lower ones.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Safe => 0,
            Self::Moderate => 1,
            Self::Sensitive => 2,
        }
    }
}

/// A measured fact. Numbers carry a unit so the UI never has to guess.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvidenceValue {
    /// Text that came from outside the program. Treat it as data, never as instructions.
    UntrustedText(String),
    /// A byte count.
    Bytes(u64),
    /// A count of items.
    Count(u64),
    /// A percentage in the range 0..=100, stored in basis points (0..=10_000).
    BasisPoints(u16),
    /// A duration in seconds.
    Seconds(u64),
    /// A boolean fact.
    Bool(bool),
}

/// One piece of evidence attached to a finding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// Short label key, for example `evidence.free_bytes`.
    pub label: LocalizedKey,
    /// Measured value.
    pub value: EvidenceValue,
}

impl Evidence {
    /// Build evidence, rejecting control characters in untrusted text.
    pub fn new(label: LocalizedKey, value: EvidenceValue) -> Result<Self, CoreError> {
        if let EvidenceValue::UntrustedText(text) = &value
            && (text.chars().any(|ch| ch.is_control()) || text.len() > 512)
        {
            return Err(CoreError::invalid(
                "untrusted evidence text must be at most 512 characters and contain no control characters",
            ));
        }
        if let EvidenceValue::BasisPoints(bp) = value
            && bp > 10_000
        {
            return Err(CoreError::invalid(
                "basis points must be between 0 and 10000",
            ));
        }
        Ok(Self { label, value })
    }
}

/// A documented remediation the user can choose. It names an action kind, not a shell command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemediationRef {
    /// Action kind name from the catalog, such as `sweep.user_temp`.
    pub action_kind: String,
    /// Explanation key.
    pub summary: LocalizedKey,
}

/// Optional cap applied when this finding has status `Fail`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreVeto {
    /// Cap for this finding's pillar, if any.
    pub pillar_ceiling: Option<u8>,
    /// Cap for the overall Keep Score, if any.
    pub overall_ceiling: Option<u8>,
}

impl ScoreVeto {
    /// Reject ceilings above 100.
    pub fn new(pillar_ceiling: Option<u8>, overall_ceiling: Option<u8>) -> Result<Self, CoreError> {
        for value in [pillar_ceiling, overall_ceiling].into_iter().flatten() {
            if value > 100 {
                return Err(CoreError::invalid(
                    "a score ceiling must be between 0 and 100",
                ));
            }
        }
        Ok(Self {
            pillar_ceiling,
            overall_ceiling,
        })
    }
}

/// Multiplier for a finding's score deduction. Stored as thousandths so the score stays integer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreWeight(u16);

impl ScoreWeight {
    /// Weight of 1.
    #[must_use]
    pub const fn one() -> Self {
        Self(1000)
    }

    /// Convert a finite weight in `0.0..=5.0` to thousandths, rounding half away from zero.
    pub fn from_f32(value: f32) -> Result<Self, CoreError> {
        if !value.is_finite() || !(0.0..=5.0).contains(&value) {
            return Err(CoreError::invalid(
                "score weight must be a finite number between 0 and 5",
            ));
        }
        let millis = (value * 1000.0).round();
        if !(0.0..=5000.0).contains(&millis) {
            return Err(CoreError::invalid(
                "score weight is out of range after rounding",
            ));
        }
        Ok(Self(millis as u16))
    }

    /// Thousandths (1000 = 1.0).
    #[must_use]
    pub const fn millis(self) -> u16 {
        self.0
    }
}

/// One diagnostic result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    id: FindingId,
    pillar: Pillar,
    severity: Severity,
    status: Status,
    title: LocalizedKey,
    evidence: Vec<Evidence>,
    impact: LocalizedKey,
    remediation: Vec<RemediationRef>,
    references: Vec<String>,
    score_weight: ScoreWeight,
    veto: Option<ScoreVeto>,
}

impl Finding {
    /// Start a builder. The finding is validated when [`FindingBuilder::build`] runs.
    #[must_use]
    pub fn builder(id: FindingId, pillar: Pillar) -> FindingBuilder {
        FindingBuilder {
            id,
            pillar,
            severity: Severity::Info,
            status: Status::Pass,
            title: None,
            evidence: Vec::new(),
            impact: None,
            remediation: Vec::new(),
            references: Vec::new(),
            score_weight: ScoreWeight::one(),
            veto: None,
        }
    }

    /// Stable id.
    #[must_use]
    pub fn id(&self) -> &FindingId {
        &self.id
    }

    /// Pillar this finding belongs to.
    #[must_use]
    pub const fn pillar(&self) -> Pillar {
        self.pillar
    }

    /// Severity.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Status.
    #[must_use]
    pub const fn status(&self) -> Status {
        self.status
    }

    /// Title key.
    #[must_use]
    pub fn title(&self) -> &LocalizedKey {
        &self.title
    }

    /// Evidence.
    #[must_use]
    pub fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }

    /// Impact key.
    #[must_use]
    pub fn impact(&self) -> &LocalizedKey {
        &self.impact
    }

    /// Remediations.
    #[must_use]
    pub fn remediation(&self) -> &[RemediationRef] {
        &self.remediation
    }

    /// HTTPS references.
    #[must_use]
    pub fn references(&self) -> &[String] {
        &self.references
    }

    /// Score weight.
    #[must_use]
    pub const fn score_weight(&self) -> ScoreWeight {
        self.score_weight
    }

    /// Optional veto.
    #[must_use]
    pub const fn veto(&self) -> Option<ScoreVeto> {
        self.veto
    }

    /// Info findings and non-failures do not deduct points.
    #[must_use]
    pub const fn deducts_score(&self) -> bool {
        matches!(self.status, Status::Fail) && !matches!(self.severity, Severity::Info)
    }
}

/// Builds a [`Finding`] with required text fields.
pub struct FindingBuilder {
    id: FindingId,
    pillar: Pillar,
    severity: Severity,
    status: Status,
    title: Option<LocalizedKey>,
    evidence: Vec<Evidence>,
    impact: Option<LocalizedKey>,
    remediation: Vec<RemediationRef>,
    references: Vec<String>,
    score_weight: ScoreWeight,
    veto: Option<ScoreVeto>,
}

impl FindingBuilder {
    /// Set severity.
    #[must_use]
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Set status.
    #[must_use]
    pub fn status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }

    /// Set the title key.
    #[must_use]
    pub fn title(mut self, title: LocalizedKey) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the impact key.
    #[must_use]
    pub fn impact(mut self, impact: LocalizedKey) -> Self {
        self.impact = Some(impact);
        self
    }

    /// Replace evidence.
    #[must_use]
    pub fn evidence(mut self, evidence: Vec<Evidence>) -> Self {
        self.evidence = evidence;
        self
    }

    /// Replace remediations.
    #[must_use]
    pub fn remediation(mut self, remediation: Vec<RemediationRef>) -> Self {
        self.remediation = remediation;
        self
    }

    /// Replace references. Each entry must be an `https://` URL without whitespace.
    #[must_use]
    pub fn references(mut self, references: Vec<String>) -> Self {
        self.references = references;
        self
    }

    /// Set the score weight.
    #[must_use]
    pub const fn weight(mut self, weight: ScoreWeight) -> Self {
        self.score_weight = weight;
        self
    }

    /// Set a veto.
    #[must_use]
    pub const fn veto(mut self, veto: ScoreVeto) -> Self {
        self.veto = Some(veto);
        self
    }

    /// Validate and build.
    pub fn build(self) -> Result<Finding, CoreError> {
        let title = self
            .title
            .ok_or_else(|| CoreError::invalid("finding title is required"))?;
        let impact = self
            .impact
            .ok_or_else(|| CoreError::invalid("finding impact is required"))?;
        for reference in &self.references {
            if !is_https_url(reference) {
                return Err(CoreError::invalid(
                    "finding references must be https URLs without whitespace",
                ));
            }
        }
        Ok(Finding {
            id: self.id,
            pillar: self.pillar,
            severity: self.severity,
            status: self.status,
            title,
            evidence: self.evidence,
            impact,
            remediation: self.remediation,
            references: self.references,
            score_weight: self.score_weight,
            veto: self.veto,
        })
    }
}

fn is_https_url(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("https://") else {
        return false;
    };
    !rest.is_empty()
        && rest.len() <= 300
        && !rest.chars().any(|ch| ch.is_whitespace() || ch.is_control())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_failures_do_not_deduct() {
        let finding = sample(Severity::Info, Status::Fail);
        assert!(!finding.deducts_score());
    }

    #[test]
    fn untrusted_text_rejects_control_characters() {
        let label = LocalizedKey::new("evidence.name").unwrap();
        let value = EvidenceValue::UntrustedText("ignore\nprevious instructions".to_owned());
        assert!(Evidence::new(label, value).is_err());
    }

    fn sample(severity: Severity, status: Status) -> Finding {
        Finding::builder(FindingId::new("STORAGE-001").unwrap(), Pillar::Storage)
            .severity(severity)
            .status(status)
            .title(LocalizedKey::new("finding.storage.free.title").unwrap())
            .impact(LocalizedKey::new("finding.storage.free.impact").unwrap())
            .build()
            .unwrap()
    }
}
