//! Individual trade tick.
use serde::{Deserialize, Serialize};

use crate::money::{Price, Quantity};
use crate::symbol::{Side, Symbol};
use crate::time::Timestamp;

/// A single executed trade from the venue feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tick {
    pub symbol: Symbol,
    pub venue_ts: Timestamp,
    pub local_recv_ts: Timestamp,
    pub price: Price,
    pub qty: Quantity,
    /// Aggressor side.
    pub side: Side,
    pub trade_id: u64,
}
