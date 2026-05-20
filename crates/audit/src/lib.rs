//! Audit ledger: double-entry journal, chart of accounts, reconciliation.
//!
//! Backing store: `SQLite` via `sqlx`. The `sqlx-ledger` crate (v0.11.14)
//! is Postgres-only and therefore incompatible with the single-binary deploy
//! goal — we implement the same double-entry semantics with raw `sqlx` +
//! `SQLite` migrations. See `spec/architecture.md#audit--ledger`.
//!
//! Public API is `SQLite`-type-free — only `Decimal` / `core` types escape.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
#![warn(clippy::pedantic)]

pub mod bootstrap;
pub mod journal;
pub mod ledger;
pub mod query;
pub mod tick;

pub use ledger::Ledger;
