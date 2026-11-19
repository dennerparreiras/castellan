//! Policy precedence. A lower layer can only tighten a higher layer.
//!
//! Order, from strongest to weakest: compiled rules, machine policy, user config, CLI flags.

use crate::action::{Action, ActionKind, SweepTargetId};
use crate::error::CoreError;
use crate::model::{Origin, RiskTier};
use serde::{Deserialize, Serialize};

/// Default minimum age for temporary files: 24 hours.
pub const DEFAULT_MIN_AGE_SECS: u64 = 24 * 60 * 60;

/// Absolute floor. Nothing in config can go below this.
pub const ABSOLUTE_MIN_AGE_SECS: u64 = 60 * 60;

/// Default autopilot sanity cap: 50 decimal gigabytes.
pub const DEFAULT_SANITY_MAX_BYTES: u64 = 50_000_000_000;

/// Default autopilot sanity cap: 500 thousand files.
pub const DEFAULT_SANITY_MAX_FILES: u64 = 500_000;

/// Machine policy. Only an administrator is supposed to write this file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachinePolicy {
    /// Schema version. Only `1` is accepted.
    pub schema_version: u32,
    /// When true, every mutation is refused.
    pub lockdown: bool,
    /// Action kinds the elevated autopilot may run. Anything absent is refused.
    pub autopilot_actions: Vec<String>,
    /// Minimum age the machine requires for sweeps.
    pub min_age_secs: u64,
    /// Autopilot sweeps larger than this need a person.
    pub sanity_max_bytes: u64,
    /// Autopilot sweeps with more files than this need a person.
    pub sanity_max_files: u64,
    /// MCP may execute safe actions when true. Default is false.
    pub mcp_allow_safe_actions: bool,
    /// Reserved. Moderate actions still need a human outside MCP.
    pub mcp_allow_moderate_with_approval: bool,
}

impl MachinePolicy {
    /// Conservative default used when no policy file has been installed yet.
    #[must_use]
    pub fn compiled_default() -> Self {
        Self {
            schema_version: 1,
            lockdown: false,
            autopilot_actions: vec![
                ActionKind::SweepUserTemp.as_str().to_owned(),
                ActionKind::SweepWindowsTemp.as_str().to_owned(),
                ActionKind::FlushDns.as_str().to_owned(),
            ],
            min_age_secs: DEFAULT_MIN_AGE_SECS,
            sanity_max_bytes: DEFAULT_SANITY_MAX_BYTES,
            sanity_max_files: DEFAULT_SANITY_MAX_FILES,
            mcp_allow_safe_actions: false,
            mcp_allow_moderate_with_approval: false,
        }
    }

    /// Parse and validate TOML.
    pub fn from_toml(text: &str) -> Result<Self, CoreError> {
        let policy: Self = toml::from_str(text).map_err(|error| {
            CoreError::config(format!(
                "machine policy could not be parsed ({error}). Unknown fields are rejected"
            ))
        })?;
        policy.validate()?;
        Ok(policy)
    }

    /// Check ranges and the autopilot allow-list.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema_version != 1 {
            return Err(CoreError::config("machine policy schema_version must be 1"));
        }
        if self.min_age_secs < ABSOLUTE_MIN_AGE_SECS {
            return Err(CoreError::config(
                "machine policy min_age_secs cannot be below 3600",
            ));
        }
        for name in &self.autopilot_actions {
            let kind = ActionKind::parse(name)?;
            if !kind.autopilot_listable() || kind.risk() != RiskTier::Safe {
                return Err(CoreError::config(format!(
                    "autopilot cannot list `{name}`. Only safe, explicitly listable actions are allowed"
                )));
            }
        }
        Ok(())
    }
}

/// User preferences. These can turn features off. They cannot grant new powers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UserPreferences {
    /// Schema version. Only `1` is accepted.
    pub schema_version: u32,
    /// Targets the user turned off.
    pub disabled_targets: Vec<String>,
    /// User's minimum sweep age. The effective age is at least this large.
    pub min_age_secs: u64,
    /// User asked for read-only mode.
    pub lockdown: bool,
}

impl UserPreferences {
    /// Defaults that do not disable the compiled morning round.
    #[must_use]
    pub fn compiled_default() -> Self {
        Self {
            schema_version: 1,
            disabled_targets: Vec::new(),
            min_age_secs: DEFAULT_MIN_AGE_SECS,
            lockdown: false,
        }
    }

    /// Parse and validate TOML.
    pub fn from_toml(text: &str) -> Result<Self, CoreError> {
        let prefs: Self = toml::from_str(text).map_err(|error| {
            CoreError::config(format!(
                "user config could not be parsed ({error}). Unknown fields are rejected"
            ))
        })?;
        prefs.validate()?;
        Ok(prefs)
    }

    /// Check ranges and target names.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema_version != 1 {
            return Err(CoreError::config("user config schema_version must be 1"));
        }
        if self.min_age_secs < ABSOLUTE_MIN_AGE_SECS {
            return Err(CoreError::config("user min_age_secs cannot be below 3600"));
        }
        for name in &self.disabled_targets {
            parse_target(name)?;
        }
        Ok(())
    }
}

/// Why a mutation was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DenyReason {
    /// `lockdown = true` on the machine or in the user config.
    Lockdown,
    /// The user disabled this sweep target.
    TargetDisabled,
    /// Autopilot may only run kinds listed in the machine policy.
    NotInAutopilotPolicy,
    /// This origin cannot execute this risk tier.
    OriginForbidden,
    /// The action failed its own validation.
    InvalidAction,
}

/// A permission decision. Allow carries the age the sweep must honor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// The action may proceed, subject to confirmation.
    Allow {
        /// Effective minimum age. Always at least as strict as every layer.
        min_age_secs: u64,
    },
    /// The action must not run.
    Deny {
        /// Why.
        reason: DenyReason,
    },
}

/// Resolve permission for one action.
#[must_use]
pub fn decide(
    action: &Action,
    origin: Origin,
    policy: &MachinePolicy,
    user: &UserPreferences,
    cli_min_age_secs: Option<u64>,
) -> Decision {
    if action.validate().is_err() {
        return Decision::Deny {
            reason: DenyReason::InvalidAction,
        };
    }
    if policy.lockdown || user.lockdown {
        return Decision::Deny {
            reason: DenyReason::Lockdown,
        };
    }
    if !origin_allows(origin, action.risk(), policy) {
        return Decision::Deny {
            reason: DenyReason::OriginForbidden,
        };
    }
    if origin == Origin::Autopilot
        && !policy
            .autopilot_actions
            .iter()
            .any(|name| name == action.kind().as_str())
    {
        return Decision::Deny {
            reason: DenyReason::NotInAutopilotPolicy,
        };
    }
    if let Action::Sweep { target, .. } = action
        && user
            .disabled_targets
            .iter()
            .any(|name| name == target.as_str())
    {
        return Decision::Deny {
            reason: DenyReason::TargetDisabled,
        };
    }
    let requested = action_age(action).unwrap_or(ABSOLUTE_MIN_AGE_SECS);
    let cli_age = cli_min_age_secs.unwrap_or(ABSOLUTE_MIN_AGE_SECS);
    let min_age_secs = requested
        .max(policy.min_age_secs)
        .max(user.min_age_secs)
        .max(cli_age)
        .max(ABSOLUTE_MIN_AGE_SECS);
    Decision::Allow { min_age_secs }
}

/// Whether an origin may execute a tier under this machine policy.
#[must_use]
pub const fn origin_allows(origin: Origin, tier: RiskTier, policy: &MachinePolicy) -> bool {
    match (origin, tier) {
        (_, _) if policy.lockdown => false,
        (Origin::Autopilot, RiskTier::Sensitive | RiskTier::Moderate) => false,
        (Origin::Mcp, RiskTier::Sensitive | RiskTier::Moderate) => false,
        (Origin::Mcp, RiskTier::Safe) => policy.mcp_allow_safe_actions,
        (Origin::Autopilot, RiskTier::Safe)
        | (Origin::Tui | Origin::Cli, RiskTier::Safe | RiskTier::Moderate | RiskTier::Sensitive) => {
            true
        }
    }
}

fn action_age(action: &Action) -> Option<u64> {
    match action {
        Action::Sweep { min_age_secs, .. } => Some(*min_age_secs),
        Action::FlushDns => None,
    }
}

fn parse_target(name: &str) -> Result<SweepTargetId, CoreError> {
    match name {
        "user_temp" => Ok(SweepTargetId::UserTemp),
        "windows_temp" => Ok(SweepTargetId::WindowsTemp),
        _ => Err(CoreError::config(format!("unknown sweep target `{name}`"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;

    fn sweep(age: u64) -> Action {
        Action::Sweep {
            target: SweepTargetId::UserTemp,
            min_age_secs: age,
        }
    }

    #[test]
    fn lockdown_beats_every_lower_layer() {
        let mut policy = MachinePolicy::compiled_default();
        policy.lockdown = true;
        let user = UserPreferences::compiled_default();
        let decision = decide(&sweep(86_400), Origin::Cli, &policy, &user, None);
        assert_eq!(
            decision,
            Decision::Deny {
                reason: DenyReason::Lockdown
            }
        );
    }

    #[test]
    fn user_cannot_enable_a_target_the_policy_withheld_from_autopilot() {
        let mut policy = MachinePolicy::compiled_default();
        policy.autopilot_actions.clear();
        let user = UserPreferences::compiled_default();
        let decision = decide(&sweep(86_400), Origin::Autopilot, &policy, &user, None);
        assert_eq!(
            decision,
            Decision::Deny {
                reason: DenyReason::NotInAutopilotPolicy
            }
        );
    }

    #[test]
    fn user_can_disable_a_target_the_policy_allows() {
        let policy = MachinePolicy::compiled_default();
        let mut user = UserPreferences::compiled_default();
        user.disabled_targets.push("user_temp".to_owned());
        let decision = decide(&sweep(86_400), Origin::Cli, &policy, &user, None);
        assert_eq!(
            decision,
            Decision::Deny {
                reason: DenyReason::TargetDisabled
            }
        );
    }

    #[test]
    fn effective_age_is_the_strictest_layer() {
        let mut policy = MachinePolicy::compiled_default();
        policy.min_age_secs = 48 * 3600;
        let mut user = UserPreferences::compiled_default();
        user.min_age_secs = 72 * 3600;
        let decision = decide(&sweep(3600), Origin::Cli, &policy, &user, Some(24 * 3600));
        match decision {
            Decision::Allow { min_age_secs } => assert_eq!(min_age_secs, 72 * 3600),
            Decision::Deny { reason } => panic!("expected allow, got {reason:?}"),
        }
    }

    #[test]
    fn mcp_cannot_execute_even_when_the_user_asks() {
        let mut policy = MachinePolicy::compiled_default();
        policy.mcp_allow_safe_actions = false;
        let user = UserPreferences::compiled_default();
        let decision = decide(&Action::FlushDns, Origin::Mcp, &policy, &user, None);
        assert_eq!(
            decision,
            Decision::Deny {
                reason: DenyReason::OriginForbidden
            }
        );
    }

    #[test]
    fn unknown_policy_fields_are_rejected() {
        let text = r#"
schema_version = 1
lockdown = false
autopilot_actions = []
min_age_secs = 86400
sanity_max_bytes = 1
sanity_max_files = 1
mcp_allow_safe_actions = true
mcp_allow_moderate_with_approval = true
run_command = "not-a-catalog-entry"
"#;
        assert!(MachinePolicy::from_toml(text).is_err());
    }

    #[test]
    fn sensitive_tier_is_forbidden_for_autopilot_and_mcp() {
        let policy = MachinePolicy::compiled_default();
        assert!(!origin_allows(
            Origin::Autopilot,
            RiskTier::Sensitive,
            &policy
        ));
        assert!(!origin_allows(Origin::Mcp, RiskTier::Sensitive, &policy));
        assert!(!origin_allows(Origin::Mcp, RiskTier::Moderate, &policy));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn lower_layers_never_shorten_the_age(
            policy_age in ABSOLUTE_MIN_AGE_SECS..200_000u64,
            user_age in ABSOLUTE_MIN_AGE_SECS..200_000u64,
            cli_age in ABSOLUTE_MIN_AGE_SECS..200_000u64,
            action_age in ABSOLUTE_MIN_AGE_SECS..200_000u64,
        ) {
            let mut policy = MachinePolicy::compiled_default();
            policy.min_age_secs = policy_age;
            let mut user = UserPreferences::compiled_default();
            user.min_age_secs = user_age;
            let decision = decide(
                &Action::Sweep { target: SweepTargetId::UserTemp, min_age_secs: action_age },
                Origin::Cli,
                &policy,
                &user,
                Some(cli_age),
            );
            match decision {
                Decision::Allow { min_age_secs } => {
                    prop_assert!(min_age_secs >= policy_age);
                    prop_assert!(min_age_secs >= user_age);
                    prop_assert!(min_age_secs >= cli_age);
                    prop_assert!(min_age_secs >= action_age);
                }
                Decision::Deny { .. } => prop_assert!(false, "sweep should be allowed"),
            }
        }

        #[test]
        fn lockdown_anywhere_denies(policy_lock in proptest::bool::ANY, user_lock in proptest::bool::ANY) {
            let mut policy = MachinePolicy::compiled_default();
            policy.lockdown = policy_lock;
            let mut user = UserPreferences::compiled_default();
            user.lockdown = user_lock;
            let decision = decide(
                &Action::FlushDns,
                Origin::Cli,
                &policy,
                &user,
                None,
            );
            if policy_lock || user_lock {
                let denied = matches!(decision, Decision::Deny { reason: DenyReason::Lockdown });
                prop_assert!(denied);
            } else {
                let allowed = matches!(decision, Decision::Allow { .. });
                prop_assert!(allowed);
            }
        }
    }
}
