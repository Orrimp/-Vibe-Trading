//! Composed strategy engine — T503–T507.
//!
//! Public surface for v0.5 config-driven strategies.

pub mod ast;
pub mod config;
pub mod error;
pub mod hash;
pub mod node;
pub mod parser;
pub mod typecheck;

pub use config::{ComposedStrategyConfig, Sizing, Stage};
pub use error::StrategyLoadError;
pub use node::ComposedStrategy;
