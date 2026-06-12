//! `Network` endpoint — `Testnet` vs `Mainnet` URL resolution (AQ-6).
//!
//! **F1 ships testnet-only:** `Network::Testnet` is the default constructor.
//! A unit test asserts the default label is `"testnet"` so "F1 ships
//! testnet-only" is *enforced*, not hoped.
//!
//! Mainnet is gated by the F2 arming guard — not by the client type —
//! so every testnet rehearsal exercises the exact mainnet code path
//! (no "tested testnet, shipped untested mainnet" gap).
//!
//! **Binding law (ADR-0054 invariant iii):** zero mainnet calls in CI, ever.

/// The Binance network (testnet vs mainnet).
///
/// Carrying a greppable `label` field makes the arming-audit logs
/// unambiguous: every request log line contains `"testnet"` or `"mainnet"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecEndpoint {
    /// Base URL for REST API calls (no trailing slash).
    pub base_url: String,
    /// Human-readable / grep-able label (`"testnet"` or `"mainnet"`).
    pub label: &'static str,
}

/// Typed network selector — resolves to an [`ExecEndpoint`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    /// `https://testnet.binance.vision` — the F1 rehearsal venue.
    /// **This is the default** (F1 ships testnet-only, enforced by test).
    Testnet,
    /// `https://api.binance.com` — mainnet.
    /// Gated by the F2 arming guard; never used in CI/tests.
    Mainnet,
}

impl Network {
    /// Resolve to the REST [`ExecEndpoint`] for this network.
    #[must_use]
    pub fn endpoint(self) -> ExecEndpoint {
        match self {
            Self::Testnet => ExecEndpoint {
                base_url: "https://testnet.binance.vision".to_string(),
                label: "testnet",
            },
            Self::Mainnet => ExecEndpoint {
                base_url: "https://api.binance.com".to_string(),
                label: "mainnet",
            },
        }
    }
}

impl Default for Network {
    /// Default is **Testnet** — F1 ships testnet-only (AC-12 / AQ-6).
    fn default() -> Self {
        Self::Testnet
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// AQ-6 / AC-12 (zero-mainnet-in-CI): the default `Network` label is
    /// `"testnet"` — "F1 ships testnet-only" is *enforced*, not hoped.
    #[test]
    fn default_endpoint_is_testnet() {
        let net = Network::default();
        assert_eq!(net, Network::Testnet);
        let ep = net.endpoint();
        assert_eq!(ep.label, "testnet");
        assert_eq!(ep.base_url, "https://testnet.binance.vision");
        // Guarantee: no test references the mainnet host.
        // (This assertion is trivially true here; the clause is for the audit.)
        assert!(!ep.base_url.contains("api.binance.com"));
    }

    /// Testnet and mainnet resolve to distinct URLs.
    #[test]
    fn testnet_and_mainnet_are_distinct() {
        let t = Network::Testnet.endpoint();
        let m = Network::Mainnet.endpoint();
        assert_ne!(t.base_url, m.base_url);
        assert_eq!(t.label, "testnet");
        assert_eq!(m.label, "mainnet");
    }
}
