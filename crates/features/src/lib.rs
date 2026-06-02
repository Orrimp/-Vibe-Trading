//! Feature engineering and indicator library.
//!
//! Thin adapters over `kand` (batch) and `quantedge-ta` (streaming).
//! v0 ships SMA only (T21). v0.5 adds EMA, MACD, RSI, Bollinger Bands (T502).
//! v1 adds `math` (decimal_ln/sqrt), `ring_buffer`, and `cross_sectional` (T602–T603).

pub mod bbands;
pub mod cross_sectional;
pub mod ema;
pub mod macd;
pub mod math;
pub mod pairs;
pub mod ring_buffer;
pub mod rsi;
pub mod sma;

pub use bbands::{Bbands, BbandsBatch, BbandsStream, BbandsValue};
pub use cross_sectional::{
    ScoreError, decimal_std, score_trailing_log_return, score_vol_adjusted_return,
};
pub use ema::{Ema, EmaBatch, EmaStream};
pub use macd::{Macd, MacdBatch, MacdStream, MacdValue};
pub use math::{MathError, decimal_ln, decimal_sqrt};
pub use pairs::{PairScoreError, rolling_zscore, spread};
pub use ring_buffer::RingBuffer;
pub use rsi::{Rsi, RsiBatch, RsiStream};
pub use sma::{Sma, SmaBatch, SmaStream};
