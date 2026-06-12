//! Asset and currency types.
//!
//! `Money<C: Currency>` enforces currency separation at the type level —
//! you cannot add `Money<Usdt>` to `Money<Btc>` (compile-time error).
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

/// A tradeable asset.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Asset {
    Btc,
    Usdt,
    Eth,
    #[serde(untagged)]
    Other(SmolStr),
}

impl std::fmt::Display for Asset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Asset::Btc => write!(f, "BTC"),
            Asset::Usdt => write!(f, "USDT"),
            Asset::Eth => write!(f, "ETH"),
            Asset::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Marker trait for currencies. Implemented by unit structs.
/// Enforces `Money<A> + Money<B>` only compiles when `A == B`.
pub trait Currency: Copy + Eq + Send + Sync + 'static {
    const CODE: &'static str;
}

/// US Dollar stablecoin (Tether).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Usdt;
impl Currency for Usdt {
    const CODE: &'static str = "USDT";
}

/// Bitcoin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Btc;
impl Currency for Btc {
    const CODE: &'static str = "BTC";
}

/// Ether.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Eth;
impl Currency for Eth {
    const CODE: &'static str = "ETH";
}
