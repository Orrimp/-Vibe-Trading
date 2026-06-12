//! Domain types for the live execution client.
//!
//! `OrderAck`, `OrderRef`, `OrderStatus`, `AccountSnapshot`.
//! All money fields are `Decimal` — no `f64` (AC-9).

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use trading_core::asset::Asset;

// ── OrderRef ──────────────────────────────────────────────────────────────────

/// A reference to a placed order, used for status queries and cancels.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderRef {
    /// The `newClientOrderId` used when the order was placed.
    pub client_order_id: String,
    /// The exchange-assigned order ID (from the `orderId` field in the ack).
    pub exchange_order_id: u64,
    /// Symbol, e.g. `"BTCUSDT"`.
    pub symbol: String,
}

// ── OrderAck ──────────────────────────────────────────────────────────────────

/// Acknowledgement returned by `POST /api/v3/order`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderAck {
    /// Echo of the `newClientOrderId` we sent.
    pub client_order_id: String,
    /// Exchange-assigned order ID.
    pub exchange_order_id: u64,
    /// Symbol.
    pub symbol: String,
    /// Raw status string from the exchange (e.g. `"NEW"`, `"FILLED"`).
    pub status: String,
    /// Executed quantity so far (Decimal, from `executedQty`).
    pub executed_qty: Decimal,
    /// Original requested quantity.
    pub orig_qty: Decimal,
}

impl OrderAck {
    /// Derive an [`OrderRef`] from this ack (for follow-up status / cancel).
    #[must_use]
    pub fn as_ref(&self) -> OrderRef {
        OrderRef {
            client_order_id: self.client_order_id.clone(),
            exchange_order_id: self.exchange_order_id,
            symbol: self.symbol.clone(),
        }
    }
}

// ── OrderStatus ───────────────────────────────────────────────────────────────

/// Status of an order as returned by `GET /api/v3/order`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatusKind {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
    #[serde(other)]
    Unknown,
}

/// Full order status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderStatus {
    pub client_order_id: String,
    pub exchange_order_id: u64,
    pub symbol: String,
    pub status: OrderStatusKind,
    pub orig_qty: Decimal,
    pub executed_qty: Decimal,
    /// Whether this order is known to the exchange.
    pub exists: bool,
}

// ── Balance / AccountSnapshot ──────────────────────────────────────────────────

/// Free + locked balance for a single asset.
///
/// All fields are `Decimal` (AC-9 / ADR-0003).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub free: Decimal,
    pub locked: Decimal,
}

impl Balance {
    /// Total balance (`free + locked`).
    #[must_use]
    pub fn total(&self) -> Decimal {
        self.free + self.locked
    }
}

/// Account snapshot from `GET /api/v3/account`.
///
/// `balances` is a sorted map so reconciliation set-membership comparisons
/// are deterministic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSnapshot {
    /// Per-asset balances (free + locked), sorted by asset for determinism.
    pub balances: BTreeMap<Asset, Balance>,
}

// ── Serde helpers for Binance REST responses ──────────────────────────────────

/// Binance `POST /api/v3/order` response shape.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BinanceOrderResponse {
    pub order_id: u64,
    pub client_order_id: String,
    pub symbol: String,
    pub status: String,
    pub executed_qty: String,
    pub orig_qty: String,
}

/// Binance `GET /api/v3/order` response shape.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BinanceOrderStatusResponse {
    pub order_id: u64,
    pub client_order_id: String,
    pub symbol: String,
    pub status: String,
    pub orig_qty: String,
    pub executed_qty: String,
}

/// Binance `GET /api/v3/account` balance entry.
#[derive(Debug, Deserialize)]
pub struct BinanceAccountBalance {
    pub asset: String,
    pub free: String,
    pub locked: String,
}

/// Binance `GET /api/v3/account` response shape.
#[derive(Debug, Deserialize)]
pub struct BinanceAccountResponse {
    pub balances: Vec<BinanceAccountBalance>,
}

/// Binance error envelope (returned for non-2xx responses).
#[derive(Debug, Deserialize)]
pub(crate) struct BinanceErrorResponse {
    pub code: i32,
    pub msg: String,
}

/// Binance `GET /api/v3/time` response.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BinanceServerTime {
    pub server_time: u64,
}

/// Binance `GET /api/v3/order` when order is not found returns a specific error.
/// We model "not found" as a flag on [`OrderStatus`] (`exists = false`).
pub(crate) const BINANCE_CODE_ORDER_NOT_FOUND: i32 = -2013;

/// Parse a Decimal from a Binance string field (Binance returns numbers as
/// strings in many responses).
pub(crate) fn parse_decimal(s: &str) -> Decimal {
    s.parse().unwrap_or(Decimal::ZERO)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_total_is_decimal() {
        let b = Balance {
            free: Decimal::new(15, 1),  // 1.5
            locked: Decimal::new(5, 1), // 0.5
        };
        assert_eq!(b.total(), Decimal::new(2, 0));
    }

    #[test]
    fn order_ack_as_ref() {
        let ack = OrderAck {
            client_order_id: "abc-123".to_string(),
            exchange_order_id: 999,
            symbol: "BTCUSDT".to_string(),
            status: "NEW".to_string(),
            executed_qty: Decimal::ZERO,
            orig_qty: Decimal::new(1, 3), // 0.001
        };
        let r = ack.as_ref();
        assert_eq!(r.client_order_id, "abc-123");
        assert_eq!(r.exchange_order_id, 999);
    }
}
