//! Closed set of mutations. There is no "run this command" variant.

use crate::error::CoreError;
use crate::model::{Origin, RiskTier};
use serde::{Deserialize, Serialize};

/// Cleanup targets the guard knows how to resolve. Paths are never stored here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SweepTargetId {
    /// The current user's temporary directory, from `GetTempPath2W`.
    UserTemp,
    /// `%SystemRoot%\Temp`, from `GetSystemWindowsDirectoryW`.
    WindowsTemp,
}

impl SweepTargetId {
    /// Stable catalog name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::UserTemp => "user_temp",
            Self::WindowsTemp => "windows_temp",
        }
    }

    /// Risk of deleting aged files in this target.
    #[must_use]
    pub const fn risk(self) -> RiskTier {
        match self {
            Self::UserTemp | Self::WindowsTemp => RiskTier::Safe,
        }
    }

    /// Windows Temp is shared, so the elevated broker is what can finish the job.
    #[must_use]
    pub const fn needs_elevation(self) -> bool {
        match self {
            Self::UserTemp => false,
            Self::WindowsTemp => true,
        }
    }
}

/// A catalogued mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionKind {
    /// Delete aged files in the user temp target.
    SweepUserTemp,
    /// Delete aged files in Windows Temp.
    SweepWindowsTemp,
    /// Flush the DNS resolver cache.
    FlushDns,
}

impl ActionKind {
    /// Parse a catalog name.
    pub fn parse(value: &str) -> Result<Self, CoreError> {
        match value {
            "sweep.user_temp" => Ok(Self::SweepUserTemp),
            "sweep.windows_temp" => Ok(Self::SweepWindowsTemp),
            "net.flush_dns" => Ok(Self::FlushDns),
            _ => Err(CoreError::invalid(format!(
                "unknown action kind `{value}`. Actions are a closed catalog, not free text"
            ))),
        }
    }

    /// Stable name used in policy files.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SweepUserTemp => "sweep.user_temp",
            Self::SweepWindowsTemp => "sweep.windows_temp",
            Self::FlushDns => "net.flush_dns",
        }
    }

    /// Whether the morning round is allowed to list this kind.
    /// Sensitive work and anything not yet justified stays off this list.
    #[must_use]
    pub const fn autopilot_listable(self) -> bool {
        match self {
            Self::SweepUserTemp | Self::SweepWindowsTemp | Self::FlushDns => true,
        }
    }

    /// Risk tier.
    #[must_use]
    pub const fn risk(self) -> RiskTier {
        match self {
            Self::SweepUserTemp | Self::SweepWindowsTemp | Self::FlushDns => RiskTier::Safe,
        }
    }
}

/// One typed mutation. Parameters are ids and numbers, never raw paths or shell strings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Delete aged temporary files for one catalogued target.
    Sweep {
        /// Which target.
        target: SweepTargetId,
        /// Minimum age in seconds. Both creation and modification must be older.
        min_age_secs: u64,
    },
    /// Flush the DNS resolver cache. This does not reset Winsock.
    FlushDns,
}

impl Action {
    /// Catalog kind.
    #[must_use]
    pub const fn kind(&self) -> ActionKind {
        match self {
            Self::Sweep {
                target: SweepTargetId::UserTemp,
                ..
            } => ActionKind::SweepUserTemp,
            Self::Sweep {
                target: SweepTargetId::WindowsTemp,
                ..
            } => ActionKind::SweepWindowsTemp,
            Self::FlushDns => ActionKind::FlushDns,
        }
    }

    /// Risk tier declared by the action, not by the caller.
    #[must_use]
    pub const fn risk(&self) -> RiskTier {
        self.kind().risk()
    }

    /// Whether the elevated broker is required.
    #[must_use]
    pub const fn needs_elevation(&self) -> bool {
        match self {
            Self::Sweep { target, .. } => target.needs_elevation(),
            Self::FlushDns => false,
        }
    }

    /// Deleted temporary files cannot be restored. DNS flush needs no undo.
    #[must_use]
    pub const fn reversible(&self) -> bool {
        match self {
            Self::Sweep { .. } => false,
            Self::FlushDns => false,
        }
    }

    /// Localization key for the human summary.
    #[must_use]
    pub const fn summary_key(&self) -> &'static str {
        match self {
            Self::Sweep {
                target: SweepTargetId::UserTemp,
                ..
            } => "action.sweep.user_temp",
            Self::Sweep {
                target: SweepTargetId::WindowsTemp,
                ..
            } => "action.sweep.windows_temp",
            Self::FlushDns => "action.net.flush_dns",
        }
    }

    /// Reject ages that would let a sweep delete files created moments ago.
    pub fn validate(&self) -> Result<(), CoreError> {
        if let Self::Sweep { min_age_secs, .. } = self {
            // One hour is the floor. The product default is 24 hours.
            if *min_age_secs < 3_600 {
                return Err(CoreError::invalid(
                    "sweep min age must be at least 3600 seconds",
                ));
            }
        }
        Ok(())
    }
}

/// One step inside a plan, with the facts the UI must show before anything runs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedStep {
    /// The mutation.
    pub action: Action,
    /// Copied from the action so a serialized plan is self-describing.
    pub risk: RiskTier,
    /// Whether the broker must be elevated.
    pub needs_elevation: bool,
    /// Whether an inverse was stored.
    pub reversible: bool,
    /// Summary key.
    pub summary_key: String,
}

impl PlannedStep {
    /// Build a step from an action.
    pub fn from_action(action: Action) -> Result<Self, CoreError> {
        action.validate()?;
        Ok(Self {
            risk: action.risk(),
            needs_elevation: action.needs_elevation(),
            reversible: action.reversible(),
            summary_key: action.summary_key().to_owned(),
            action,
        })
    }
}

/// A batch of mutations that share one confirmation and one ledger entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionPlan {
    /// Stable id.
    pub id: crate::ids::PlanId,
    /// Who asked.
    pub origin: Origin,
    /// Steps in order.
    pub steps: Vec<PlannedStep>,
    /// Estimated bytes the sweep would free, when known.
    pub estimated_bytes: Option<u64>,
}

impl ActionPlan {
    /// Build a plan. An empty plan is rejected.
    pub fn new(
        id: crate::ids::PlanId,
        origin: Origin,
        actions: Vec<Action>,
        estimated_bytes: Option<u64>,
    ) -> Result<Self, CoreError> {
        if actions.is_empty() {
            return Err(CoreError::invalid("an action plan needs at least one step"));
        }
        let mut steps = Vec::with_capacity(actions.len());
        for action in actions {
            steps.push(PlannedStep::from_action(action)?);
        }
        Ok(Self {
            id,
            origin,
            steps,
            estimated_bytes,
        })
    }
}

/// What the UI or CLI must do before execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confirmation {
    /// Show a summary and proceed.
    SummaryOnly,
    /// Ask for a yes/no confirmation.
    Simple,
    /// Require the word CONFIRM, or `--yes --i-understand` on the CLI.
    Phrase,
    /// A human must approve outside the MCP channel.
    HumanApproval,
    /// The origin is not allowed to run this tier.
    Forbidden,
}

/// Flags that affect confirmation, never permission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfirmFlags {
    /// The user passed `--yes`.
    pub yes: bool,
    /// The user passed `--i-understand` for a sensitive action.
    pub i_understand: bool,
}

/// Decide the confirmation a tier requires for an origin.
#[must_use]
pub const fn confirmation_for(tier: RiskTier, origin: Origin, flags: ConfirmFlags) -> Confirmation {
    match (tier, origin) {
        (RiskTier::Sensitive, Origin::Autopilot | Origin::Mcp) => Confirmation::Forbidden,
        (RiskTier::Moderate, Origin::Mcp) => Confirmation::HumanApproval,
        (RiskTier::Moderate, Origin::Autopilot) => Confirmation::SummaryOnly,
        (RiskTier::Safe, _) => Confirmation::SummaryOnly,
        (RiskTier::Moderate, Origin::Tui | Origin::Cli) => {
            if flags.yes {
                Confirmation::SummaryOnly
            } else {
                Confirmation::Simple
            }
        }
        (RiskTier::Sensitive, Origin::Tui | Origin::Cli) => {
            if flags.yes && flags.i_understand {
                Confirmation::SummaryOnly
            } else {
                Confirmation::Phrase
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_never_runs_from_mcp_or_autopilot() {
        let flags = ConfirmFlags {
            yes: true,
            i_understand: true,
        };
        assert_eq!(
            confirmation_for(RiskTier::Sensitive, Origin::Mcp, flags),
            Confirmation::Forbidden
        );
        assert_eq!(
            confirmation_for(RiskTier::Sensitive, Origin::Autopilot, flags),
            Confirmation::Forbidden
        );
    }

    #[test]
    fn sensitive_cli_needs_the_understand_flag() {
        let yes_only = ConfirmFlags {
            yes: true,
            i_understand: false,
        };
        assert_eq!(
            confirmation_for(RiskTier::Sensitive, Origin::Cli, yes_only),
            Confirmation::Phrase
        );
        let both = ConfirmFlags {
            yes: true,
            i_understand: true,
        };
        assert_eq!(
            confirmation_for(RiskTier::Sensitive, Origin::Cli, both),
            Confirmation::SummaryOnly
        );
    }

    #[test]
    fn sweep_rejects_a_short_age() {
        let action = Action::Sweep {
            target: SweepTargetId::UserTemp,
            min_age_secs: 60,
        };
        assert!(action.validate().is_err());
    }

    #[test]
    fn unknown_action_kind_is_rejected() {
        assert!(ActionKind::parse("run command").is_err());
        assert!(ActionKind::parse("not-a-catalog-entry").is_err());
    }
}
