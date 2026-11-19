//! Pure domain for Castellan.
//!
//! This crate does not touch the disk, the registry, the network, or `unsafe`.
//! Every mutation the product can perform is an [`Action`]. Every score is a pure
//! function of findings. Frontends (TUI, CLI, MCP) render these types. They do not
//! invent their own.

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

pub mod action;
pub mod error;
pub mod ids;
pub mod model;
pub mod policy;
pub mod score;
pub mod text;

pub use action::{
    Action, ActionKind, ActionPlan, ConfirmFlags, Confirmation, PlannedStep, SweepTargetId,
    confirmation_for,
};
pub use error::{CoreError, ErrorCode};
pub use ids::{FindingId, LocalizedKey, PlanId, ProbeId};
pub use model::{
    Evidence, EvidenceValue, Finding, Origin, Pillar, RemediationRef, RiskTier, ScoreVeto,
    ScoreWeight, Severity, Status,
};
pub use policy::{Decision, DenyReason, MachinePolicy, UserPreferences, decide, origin_allows};
pub use score::{Band, DeductionLine, PillarScore, ScoreInput, ScoreReport, display_band, score};
pub use text::{Language, PersonaMoment, format_bytes, persona};
