//! Safety kernel.
//!
//! Modules propose [`castellan_core::Action`] values. This crate is the only one
//! that may turn an approved action into a deletion or a DNS flush, and only after
//! policy, the deny-list, and the per-node checks have all passed.

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

pub mod decide;
pub mod live;
pub mod path;
pub mod sweep;

/// Permission check used before any sweep starts. Lower layers cannot widen policy.
pub fn authorize(
    action: &castellan_core::Action,
    origin: castellan_core::Origin,
    policy: &castellan_core::MachinePolicy,
    user: &castellan_core::UserPreferences,
    cli_min_age_secs: Option<u64>,
) -> castellan_core::Decision {
    castellan_core::decide(action, origin, policy, user, cli_min_age_secs)
}

pub use decide::{NodeDecision, NodeFacts, SkipReason, decide_node, hostile_component};
pub use live::{LiveScan, scan_no_follow};
pub use path::{DenyHit, MachineRoots, NormalizedPath, deny_list};
pub use sweep::{
    GuardError, SanityLimits, Skipped, SweepReport, SweepView, is_temp_sandbox, mutation_allowed,
    run_sweep,
};
