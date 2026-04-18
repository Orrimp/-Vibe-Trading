//! Feature engineering and indicator library.
//!
//! Thin adapters over `kand` (batch) and `quantedge-ta` (streaming).
//! v0 ships SMA only (T21).

pub mod sma;

pub use sma::{Sma, SmaBatch, SmaStream};
