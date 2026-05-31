//! Cross-sectional momentum strategy modules (v1 T604–T606).
//!
//! - `config` — TOML deserializer + validator (`CrossSectionalMomentumConfig`)
//! - `selector` — top-K symbol selection with alphabetical tie-break
//! - `momentum` — `MomentumStrategy` implementing the `Strategy` trait

pub mod config;
pub mod momentum;
pub mod selector;

pub use config::{CrossSectionalLoadError, CrossSectionalMomentumConfig, Direction};
pub use momentum::MomentumStrategy;
pub use selector::top_k_long;
