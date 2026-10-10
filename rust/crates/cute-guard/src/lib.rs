//! cute-guard — CuteCity guard-tour mother module (organelle `gd`).
//!
//! Domain model for security-guard patrol operations: sites, checkpoints,
//! tours, check-ins, incidents, and machine-emitted exception alerts.
//! Every proof event seals into a `lucidota-receipt/1` envelope
//! (see [`receipt`]); tour compliance is evaluated by [`engine`].
//!
//! Design notes live in SPEC.md next to this file.

pub mod engine;
pub mod model;
pub mod organelle;
pub mod receipt;

pub use model::*;
pub use receipt::{ReceiptStatus, seal_envelope, verify_chain, verify_envelope};
