#![deny(clippy::float_arithmetic)]
//! Reflection memory — lesson cards, regime / outcome classifiers,
//! deterministic 32-dim embedding, top-K retrieval, and the
//! `ReflectionStore` trait + sqlite default impl.
//!
//! v1 ships **deterministic** card generation (Q1 = Option A, no LLM
//! provider, no `expense:llm:*` ledger impact).  An LLM-enrichment
//! follow-up brief replaces `post_mortem_analyst::generate_card`'s
//! body behind the same name.
//!
//! ## Q4 = report-only
//!
//! The trader's `Strategy` trait does not consume retrieval; the only
//! caller of [`retrieve_top_k`] is the operator success report's
//! memory-highlights renderer.  Trader-side wiring is a follow-up
//! brief named `reflection-memory-trader-wiring`; the negative-confirm
//! test at `crates/reflection/tests/no_strategy_caller.rs` guards the
//! invariant.
//!
//! ## Q5 — periodic distillation deferred
//!
//! Distillation (product.md layer 4) lands in a follow-up brief
//! `reflection-memory-distillation` once cards are on disk and the v2
//! LLM consumer exists.  Nothing in this crate addresses it.
//!
//! ## No `f64` rule
//!
//! `#![deny(clippy::float_arithmetic)]` is enabled at the crate root
//! so any `f64` arithmetic in money / score / embedding compute is a
//! compile error.  The cosine helper at `embedding::cosine` and every
//! `Decimal` helper used here is `Decimal`-only.

pub mod embedding;
pub mod outcome;
pub mod post_mortem_analyst;
pub mod regime;
pub mod retrieval;
pub mod store;
pub mod types;
pub mod writer;

pub use embedding::{EMBEDDING_DIM, STRATEGY_SLOTS, cosine, embed};
pub use outcome::{OUTCOME_THRESHOLD_PCT, OutcomeClass, classify_outcome};
pub use regime::{RegimeError, RegimeTag, classify_regime};
pub use retrieval::{RetrievalError, retrieve_top_k};
pub use store::{ReflectionStore, ReflectionStoreError};
pub use types::{
    ClosedTrade, LessonCard, LessonCardWriteRequest, RetrievalQuery, SymbolOrPair, card_id,
};
pub use writer::{ReflectionWriter, TryEnqueueError};

/// Default top-K at report time (Q3e — analyst strawman pinned).
///
/// Pinned in one place so a future architect grep-changes here and
/// re-locks the two `report-sample-*` body-SHA-256 anchors at
/// `spec/anchors.toml:67-75`.
pub const REPORT_TIME_TOP_K: usize = 5;
