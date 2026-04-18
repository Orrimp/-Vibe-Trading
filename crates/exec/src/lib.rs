//! Execution routing: paper-mode and live-mode stubs.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
#![warn(clippy::pedantic)]

pub mod router;

pub use router::{ExecRouter, PaperExecRouter};
