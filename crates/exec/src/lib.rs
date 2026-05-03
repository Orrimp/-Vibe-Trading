//! Execution routing: paper-mode and live-mode stubs.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
#![warn(clippy::pedantic)]

pub mod paper;
pub mod publisher;
pub mod router;

pub use paper::PaperEnginePublisher;
pub use publisher::{FillPublisher, NullPublisher};
pub use router::{ExecRouter, PaperExecRouter};
