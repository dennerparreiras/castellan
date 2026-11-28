//! Terminal UI. `update` is pure. `view` only draws the model it is given.
//!
//! The keep is a night fortress. Unmeasured pillars stay dark, and no gauge
//! invents a number the engine did not return.

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

mod art;
mod model;
mod rooms;
mod theme;
mod ui;
mod view;

pub use model::{
    Attention, Drive, Facts, KeepWitness, Key, Memory, Model, Msg, Pulse, SWEEP_ROWS, Session,
    SweepReading, SweepSize, Tab, update,
};
pub use theme::Theme;
pub use view::{pointer, view};

#[cfg(test)]
mod tests;
