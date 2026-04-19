//! Feature engineering and indicator library.
//!
//! Thin adapters over `kand` (batch) and `quantedge-ta` (streaming).
//! v0 ships SMA only (T21). v0.5 adds EMA, MACD, RSI, Bollinger Bands (T502).

pub mod bbands;
pub mod ema;
pub mod macd;
pub mod rsi;
pub mod sma;

pub use bbands::{Bbands, BbandsBatch, BbandsStream, BbandsValue};
pub use ema::{Ema, EmaBatch, EmaStream};
pub use macd::{Macd, MacdBatch, MacdStream, MacdValue};
pub use rsi::{Rsi, RsiBatch, RsiStream};
pub use sma::{Sma, SmaBatch, SmaStream};
