//! Panel widgets.
//!
//! One module per cockpit panel. Each module exposes a single `view`
//! function that takes a `&Cockpit` (and sometimes the panel's own sub-state)
//! and returns an `iced::Element<Message>`. Business logic never lives here.
//!
//! Consistency contract (enforced in tests `tests/no_inline_strings.rs` and
//! `tests/no_inline_hex.rs`):
//! - No string literals — use `crate::strings::*`.
//! - No hex colors or magic-number `Length::Units(N)` — use `crate::theme::*`.

pub mod frame;
pub mod kill;
pub mod latency;
pub mod num;
pub mod pnl;
pub mod positions;
pub mod strategies;
pub mod tape;
