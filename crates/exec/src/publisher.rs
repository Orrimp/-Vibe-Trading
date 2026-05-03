//! `FillPublisher` — fanout trait for live-mode paper engine fills.
//!
//! ## Why a trait, not a direct `Arc<EventBus>` field?
//!
//! `crates/agent` already depends on `crates/exec` (via the
//! `PaperExecRouter` re-export). If `exec` were to depend on `agent`
//! to grab `EventBus`, the dep graph would cycle:
//! `exec → agent → exec`.  The trait inverts the dependency: `exec`
//! defines a tiny pub-trait local to itself; `agent::EventBus`
//! provides the `impl FillPublisher for EventBus` block; the live
//! paper engine accepts `Arc<dyn FillPublisher>` and never needs to
//! know about `agent` at all.
//!
//! ## Backtests
//!
//! Backtests construct the paper engine with [`NullPublisher`], which
//! is a no-op zero-sized type.  Backtest output bytes are unchanged
//! by this wiring (R15 / verify-anchors invariant) — the publisher
//! is only consulted on the live-mode path.
//!
//! ## Backpressure
//!
//! Both methods are non-blocking and infallible from the caller's
//! perspective.  The `EventBus` impl uses `broadcast::Sender::send`
//! which returns `SendError::Closed` when no subscribers are
//! attached — the impl swallows that error (matches the existing
//! `EventBus::publish_fill` policy).  A slow consumer gets
//! `RecvError::Lagged(n)` on the receive side; the publisher never
//! blocks.
use trading_core::{Fill, Position};

/// Side-effect channel for fills + positions emerging from the
/// live-mode paper engine.
///
/// See module docs for the dep-graph rationale.
pub trait FillPublisher: Send + Sync {
    /// Publish a fill event.  Implementations must not block.
    fn publish_fill(&self, fill: &Fill);

    /// Publish a position update.  Implementations must not block.
    fn publish_position(&self, pos: &Position);
}

/// No-op publisher used by backtests.  Zero-sized; allocates nothing.
///
/// The deterministic backtest path constructs the paper engine with
/// `NullPublisher::new()` so that no live-mode side effect can leak
/// into report bytes (see `scripts/verify_anchors.sh`).
#[derive(Debug, Default, Clone, Copy)]
pub struct NullPublisher;

impl NullPublisher {
    /// Construct a no-op publisher.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl FillPublisher for NullPublisher {
    fn publish_fill(&self, _fill: &Fill) {}
    fn publish_position(&self, _pos: &Position) {}
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use trading_core::{
        FeeTier, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol, Timestamp,
    };

    fn sample_fill() -> Fill {
        Fill {
            id: FillId::new(),
            order_id: OrderId::new(),
            symbol: Symbol::new("BTCUSDT"),
            side: Side::Buy,
            qty: Quantity::new(dec!(0.1)).unwrap(),
            price: Price::new(dec!(40_000)).unwrap(),
            fee: Money::from_decimal(dec!(1.6)),
            fee_tier: FeeTier::Taker,
            venue_ts: Timestamp::new(time::OffsetDateTime::UNIX_EPOCH),
            local_ts: Timestamp::new(time::OffsetDateTime::UNIX_EPOCH),
            liquidity: Liquidity::Taker,
        }
    }

    #[test]
    fn null_publisher_swallows_both_calls() {
        let pub_ = NullPublisher::new();
        let fill = sample_fill();
        let pos = Position::empty(Symbol::new("BTCUSDT"));
        pub_.publish_fill(&fill);
        pub_.publish_position(&pos);
        // Reaching here without panic is the assertion.
    }

    /// Sanity: trait is dyn-safe (object-safe).  If this fails to
    /// compile, the trait shape regressed and the live-mode wiring
    /// (which holds an `Arc<dyn FillPublisher + Send + Sync>`) is
    /// broken at the type level.
    #[test]
    fn fill_publisher_is_object_safe() {
        let _: std::sync::Arc<dyn FillPublisher> = std::sync::Arc::new(NullPublisher::new());
    }
}
