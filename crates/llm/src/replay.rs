//! Replay-cache module — schema constant, canonical-JSON request hash,
//! and the strict-only `ReplayProvider` (T1919 / T1920 / T1922).
//!
//! ## What lives here
//!
//! - [`SUPPORTED_SCHEMA_VERSION`] — the `schema_version` ceiling
//!   [`ReplayProvider::open`] asserts against on the first row it reads.
//!   Forward-compat hook for the eventual v3 cache evolution
//!   (Design § Q8b).
//! - [`request_hash`] — SHA-256 hex over canonical JSON of the
//!   load-bearing `ChatRequest` fields. Pure function; deterministic by
//!   construction (HashMap iteration sorted, deep-key-sort on every JSON
//!   object).
//! - [`ReplayProvider`] — `LlmProvider` impl that resolves every
//!   `complete(...)` against the SQLite cache **and only the cache**.
//!   D2 operator lock at v2.0.0: a cache miss surfaces as
//!   [`crate::error::LlmError::ReplayMiss`] carrying `{ hash, provider,
//!   model }` — there is no best-effort fallthrough to a live provider.
//!
//! `RecordingProvider` (T1921 — the write half) lives in the sibling
//! [`crate::recording`] module so the read-only [`ReplayProvider`] type
//! has no compile dependency on the write path (and so a future
//! research-only crate-feature can strip the write half out cheanly).
//!
//! ## Canonical-JSON divergence flag (developer, pass 5, 2026-05-12)
//!
//! Design § Q8a's strawman names the `serde-canonical-json` crate. That
//! crate is **not in the offline lockfile** and v2.0.0 ships against a
//! sandboxed `cargo` (no `crates.io` network). The shipped implementation
//! at [`canonical_json_string`] reuses `serde_json::Value` + a manual
//! deep-sort over `Map<String, Value>` to produce a byte-stable
//! canonical form — same surface as the strawman crate would have
//! emitted for the request shape we hash. Determinism is enforced by
//! [`tests::t1920_canonical_json_is_deterministic_1000x`] — 1000
//! repeated hashes of the same request produce the same SHA-256, and a
//! sibling test confirms the `correlation_id` field is excluded (so two
//! requests differing only in correlation id share a hash). When the
//! offline-network situation changes, swap the body of
//! `canonical_json_string` for `serde_canonical_json::to_string` —
//! call sites and downstream `request_hash` are unaffected.
//!
//! ## Cache-miss D2 lock (developer, pass 5, 2026-05-12)
//!
//! The brief's "STRICT-ONLY per D2 operator lock" wording prohibits a
//! best-effort fallthrough to a live provider on a cache miss. This is
//! enforced **by absence**: [`ReplayProvider`] holds no inner provider
//! — the miss branch returns `LlmError::ReplayMiss { hash, provider,
//! model }` with no escape hatch.

use std::path::Path;

use async_trait::async_trait;
use serde::Serialize;
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::error::LlmError;
use crate::trait_def::{
    ChatMessage, ChatRequest, ChatResponse, LlmProvider, ModelId, SystemBlock,
};
use crate::tools::ToolSchema;
use crate::ProviderKind;

/// Supported `schema_version` ceiling for the replay cache (Design § Q8b).
///
/// [`ReplayProvider::open`] reads `schema_version` from the first
/// `llm_replay` row and refuses any value > this constant with an
/// `LlmError::Provider { provider: Other("replay"), message: "unknown
/// schema version" }`. A v3 evolution adds the new column via a sibling
/// migration file + bumps this constant to `2`. Migration files are
/// strictly additive — we never mutate `001_llm_replay.sql`.
pub const SUPPORTED_SCHEMA_VERSION: i32 = 1;

// ── Canonical JSON + request_hash (T1920) ───────────────────────────────────

/// The load-bearing slice of a [`ChatRequest`] we hash for replay-cache
/// lookup. **`correlation_id` is excluded** — a fresh per-call UUID would
/// otherwise make every replay miss (Design § Q8a).
///
/// `temperature` is included because `None` vs `Some(0.0)` is a meaningful
/// distinction on every provider.
#[derive(Debug, Serialize)]
struct CanonicalRequestView<'a> {
    model: &'a ModelId,
    system: &'a Vec<SystemBlock>,
    messages: &'a Vec<ChatMessage>,
    tools: &'a Vec<ToolSchema>,
    max_tokens: u32,
    temperature: Option<f32>,
}

impl<'a> From<&'a ChatRequest> for CanonicalRequestView<'a> {
    fn from(req: &'a ChatRequest) -> Self {
        Self {
            model: &req.model,
            system: &req.system,
            messages: &req.messages,
            tools: &req.tools,
            max_tokens: req.max_tokens,
            temperature: req.temperature,
        }
    }
}

/// Serialize `value` to a canonical-JSON `String` with deep-sorted object
/// keys. See module-level divergence note for why this isn't
/// `serde-canonical-json` at v2.0.0.
///
/// # Errors
///
/// Returns [`LlmError::InvalidResponse`] if the value cannot be encoded
/// (in practice this is impossible for `CanonicalRequestView` because
/// every field type round-trips via serde JSON — but we propagate the
/// error rather than panic to honour the no-`unwrap()` rule).
pub fn canonical_json_string<T: Serialize>(value: &T) -> Result<String, LlmError> {
    let raw: serde_json::Value = serde_json::to_value(value)
        .map_err(|e| LlmError::InvalidResponse(format!("canonical_json: encode: {e}")))?;
    let sorted = sort_value(raw);
    serde_json::to_string(&sorted)
        .map_err(|e| LlmError::InvalidResponse(format!("canonical_json: render: {e}")))
}

/// Recursively sort every `Map` in `value` by key. Arrays keep their
/// element order — array ordering is semantically load-bearing
/// (`messages: [user, assistant, user]` is not the same conversation as
/// `[user, user, assistant]`).
fn sort_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            // BTreeMap is sorted by key by construction; converting back
            // through `serde_json::Map` preserves insertion order, which
            // (combined with the BTreeMap pass) yields key-sorted output.
            let mut sorted: std::collections::BTreeMap<String, serde_json::Value> =
                std::collections::BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k, sort_value(v));
            }
            let mut out = serde_json::Map::with_capacity(sorted.len());
            for (k, v) in sorted {
                out.insert(k, v);
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(sort_value).collect())
        }
        leaf => leaf,
    }
}

/// SHA-256 hex of the canonical JSON of `req`'s load-bearing fields.
///
/// Deterministic by construction: deep-sort + serde JSON =
/// byte-stable canonical form on every call. **`correlation_id` is
/// excluded.** Two requests differing only in `correlation_id` produce
/// the same hash; two requests differing in `temperature None` vs
/// `Some(0.0)` produce different hashes.
///
/// # Errors
///
/// Returns [`LlmError::InvalidResponse`] if the canonical-JSON serialise
/// fails — impossible for `ChatRequest`'s field types in practice, but
/// surfaced rather than panicked.
pub fn request_hash(req: &ChatRequest) -> Result<String, LlmError> {
    let view = CanonicalRequestView::from(req);
    let canonical = canonical_json_string(&view)?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let digest = hasher.finalize();
    Ok(hex_lower(&digest))
}

/// Hex-encode (lowercase) — small enough to inline here rather than
/// pull in a `hex` crate dependency.
fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

// ── ReplayProvider (T1922) ───────────────────────────────────────────────────

/// Strict-only replay reader. Implements [`LlmProvider`] by resolving
/// every `complete(...)` against the SQLite replay cache; a cache miss
/// surfaces as [`LlmError::ReplayMiss`] (no best-effort fallthrough).
///
/// The cache is opened read-only (`PRAGMA query_only = 1`) so a research
/// run cannot accidentally mutate the fixture DB. The
/// `SUPPORTED_SCHEMA_VERSION` check fires once on first read.
pub struct ReplayProvider {
    pool: SqlitePool,
    /// `provider_kind` to return from [`LlmProvider::provider_kind`]. We
    /// don't have a leaf inner here — research mode's
    /// `BudgetedProvider::provider_kind()` reports `Other("replay")` so
    /// the cost sink labels research-mode events as the replay surface
    /// rather than masquerading as a real provider (Design § Q8 +
    /// product.md line 292 — research mode is "no LLM cost (cached
    /// responses replay)").
    advertised_kind: ProviderKind,
}

impl std::fmt::Debug for ReplayProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplayProvider")
            .field("advertised_kind", &self.advertised_kind)
            .finish()
    }
}

impl ReplayProvider {
    /// Open the replay cache at `path` read-only. Asserts
    /// `schema_version <= SUPPORTED_SCHEMA_VERSION` on the first row;
    /// empty fixtures are permitted (no schema row to read).
    ///
    /// # Errors
    ///
    /// - [`LlmError::Provider`] when the SQLite file cannot be opened
    ///   OR the cache holds a row with `schema_version >
    ///   SUPPORTED_SCHEMA_VERSION`.
    pub async fn open(path: &Path) -> Result<Self, LlmError> {
        let opts = SqliteConnectOptions::new()
            .filename(path)
            .read_only(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await
            .map_err(|e| LlmError::Provider {
                provider: ProviderKind::Other("replay".to_string()),
                message: format!("open replay cache at {}: {e}", path.display()),
            })?;

        // Schema-version gate. `MAX(schema_version)` returns NULL on an
        // empty table — we accept that as "no rows yet" rather than
        // raising. The gate kicks in once any row exists.
        let row: Option<(i64,)> =
            sqlx::query_as("SELECT MAX(schema_version) FROM llm_replay")
                .fetch_optional(&pool)
                .await
                .map_err(|e| LlmError::Provider {
                    provider: ProviderKind::Other("replay".to_string()),
                    message: format!("read schema_version: {e}"),
                })?;
        if let Some((max_v,)) = row {
            // max_v is signed i64 from SQLite; cast carefully.
            if max_v > i64::from(SUPPORTED_SCHEMA_VERSION) {
                return Err(LlmError::Provider {
                    provider: ProviderKind::Other("replay".to_string()),
                    message: format!(
                        "unknown schema version {max_v} > supported \
                         {SUPPORTED_SCHEMA_VERSION}; refresh the fixture \
                         or upgrade the llm crate"
                    ),
                });
            }
        }

        Ok(Self {
            pool,
            advertised_kind: ProviderKind::Other("replay".to_string()),
        })
    }
}

#[async_trait]
impl LlmProvider for ReplayProvider {
    fn name(&self) -> &str {
        "replay"
    }

    fn provider_kind(&self) -> ProviderKind {
        self.advertised_kind.clone()
    }

    async fn complete(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        let hash = request_hash(&request)?;
        let model = request.model.as_str().to_string();

        let row: Option<(String,)> =
            sqlx::query_as("SELECT response_json FROM llm_replay WHERE request_hash = ?")
                .bind(&hash)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| LlmError::Provider {
                    provider: ProviderKind::Other("replay".to_string()),
                    message: format!("lookup hash {hash}: {e}"),
                })?;

        let response_json = match row {
            Some((s,)) => s,
            None => {
                // D2 STRICT REPLAY — no fallthrough at v2.0.0.
                tracing::warn!(
                    target: "llm.replay",
                    hash = %hash,
                    model = %model,
                    "replay_miss"
                );
                return Err(LlmError::ReplayMiss {
                    hash,
                    // The advertised kind is the replay surface itself
                    // — the caller's `BudgetedProvider` already sees
                    // `provider_kind() = Other("replay")` so reporting
                    // the recorded provider here would mislead.
                    provider: self.advertised_kind.clone(),
                    model,
                });
            }
        };

        let response: ChatResponse = serde_json::from_str(&response_json)
            .map_err(|e| LlmError::InvalidResponse(format!("decode cached response: {e}")))?;
        Ok(response)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trait_def::{
        ChatMessage, ContentBlock, MessageRole, ModelId, StopReason, TokenUsage,
    };
    use cost::{AgentRole, LlmTier};
    use uuid::Uuid;

    fn sample_request(seed: u8) -> ChatRequest {
        let mut r = ChatRequest::new(
            ModelId::new("claude-opus-4-7"),
            LlmTier::DeepThink,
            AgentRole::Trader,
        );
        r.max_tokens = 256;
        r.temperature = Some(0.7);
        r.messages.push(ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text(format!("hello {seed}"))],
        });
        r
    }

    /// T1920 (a) determinism gate — 1000 iterations of `request_hash`
    /// against the same request produce the same hex digest.
    #[test]
    fn t1920_canonical_json_is_deterministic_1000x() {
        let req = sample_request(0);
        let baseline = request_hash(&req).expect("hash baseline");
        for _ in 0..1000 {
            let again = request_hash(&req).expect("hash iter");
            assert_eq!(again, baseline, "request_hash drifted across iterations");
        }
        assert_eq!(baseline.len(), 64, "SHA-256 hex must be 64 chars");
    }

    /// T1920 (b) — two requests differing only in `correlation_id`
    /// share a hash (D2 / Q8a: correlation_id is excluded).
    #[test]
    fn t1920_correlation_id_excluded_from_hash() {
        let mut a = sample_request(7);
        let mut b = sample_request(7);
        // Force divergent correlation ids.
        a.correlation_id = Uuid::from_u128(0x1111_1111_1111_1111_1111_1111_1111_1111);
        b.correlation_id = Uuid::from_u128(0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF);

        let ha = request_hash(&a).unwrap();
        let hb = request_hash(&b).unwrap();
        assert_eq!(ha, hb, "correlation_id must be excluded from hash");
    }

    /// T1920 (c) — `temperature: None` vs `Some(0.0)` produce DIFFERENT
    /// hashes. Both shapes change the sampled output distribution at the
    /// provider, so they're distinct cache keys.
    #[test]
    fn t1920_temperature_none_vs_some_zero_diverge() {
        let mut a = sample_request(0);
        a.temperature = None;
        let mut b = sample_request(0);
        b.temperature = Some(0.0);

        let ha = request_hash(&a).unwrap();
        let hb = request_hash(&b).unwrap();
        assert_ne!(ha, hb, "temperature None vs Some(0.0) must hash differently");
    }

    /// T1919 — `SUPPORTED_SCHEMA_VERSION` is `1` at v2.0.0.
    #[test]
    fn t1919_supported_schema_version_is_one() {
        assert_eq!(SUPPORTED_SCHEMA_VERSION, 1);
    }

    /// T1922 — strict-only: a cache miss surfaces `LlmError::ReplayMiss`
    /// carrying the lookup `hash` + `model`. Build an empty fixture (no
    /// rows) and assert any `complete(...)` against it misses.
    #[tokio::test]
    async fn t1922_strict_miss_returns_structured_replay_miss() {
        let td = tempfile::tempdir().unwrap();
        let db_path = td.path().join("replay.db");

        // Run the migration via RecordingProvider::open's side effect
        // (cheaper than re-importing sqlx::migrate! here). The test
        // crate uses RecordingProvider only to create the schema.
        use crate::recording::RecordingProvider;
        let inner = NoopInner;
        let _rec = RecordingProvider::open(inner, &db_path)
            .await
            .expect("migration");

        let replay = ReplayProvider::open(&db_path)
            .await
            .expect("open empty fixture");
        let req = sample_request(0);
        let err = replay.complete(req.clone()).await.expect_err("miss");

        match err {
            LlmError::ReplayMiss {
                hash,
                provider,
                model,
            } => {
                assert_eq!(hash, request_hash(&req).unwrap());
                assert!(
                    matches!(provider, ProviderKind::Other(ref s) if s == "replay"),
                    "advertised kind should be Other(\"replay\"), got {provider:?}"
                );
                assert_eq!(model, "claude-opus-4-7");
            }
            other => panic!("expected ReplayMiss, got {other:?}"),
        }
    }

    /// T1922 — round-trip: record one request via `RecordingProvider`,
    /// open `ReplayProvider` against the same DB, assert byte-identical
    /// `ChatResponse`.
    #[tokio::test]
    async fn t1922_round_trip_byte_identical() {
        let td = tempfile::tempdir().unwrap();
        let db_path = td.path().join("replay.db");

        use crate::recording::RecordingProvider;

        let canned = ChatResponse {
            content: vec![ContentBlock::Text("OK".into())],
            stop_reason: StopReason::EndTurn,
            usage: TokenUsage {
                tokens_in: 12,
                tokens_out: 1,
                tokens_cached_in: 0,
            },
            model: ModelId::new("claude-opus-4-7"),
            correlation_id: Uuid::nil(),
        };
        let inner = ProgrammedInner {
            response: canned.clone(),
            kind: ProviderKind::Anthropic,
        };
        let rec = RecordingProvider::open(inner, &db_path).await.unwrap();

        let req = sample_request(42);
        let _ = rec.complete(req.clone()).await.expect("record ok");

        let replay = ReplayProvider::open(&db_path).await.unwrap();
        let got = replay.complete(req).await.expect("replay hit");
        assert_eq!(got, canned, "replay must be byte-identical");
    }

    // ── Test-only inner providers ────────────────────────────────────

    /// Inner provider that always fails — used to prove the
    /// `ReplayProvider` doesn't reach an inner on a miss (it has none).
    struct NoopInner;
    #[async_trait]
    impl LlmProvider for NoopInner {
        fn name(&self) -> &str {
            "noop"
        }
        fn provider_kind(&self) -> ProviderKind {
            ProviderKind::Anthropic
        }
        async fn complete(&self, _req: ChatRequest) -> Result<ChatResponse, LlmError> {
            Err(LlmError::InvalidResponse(
                "NoopInner should not be called".into(),
            ))
        }
    }

    /// Inner provider that returns a programmed response — used by the
    /// round-trip test to seed the cache.
    struct ProgrammedInner {
        response: ChatResponse,
        kind: ProviderKind,
    }
    #[async_trait]
    impl LlmProvider for ProgrammedInner {
        fn name(&self) -> &str {
            "programmed"
        }
        fn provider_kind(&self) -> ProviderKind {
            self.kind.clone()
        }
        async fn complete(&self, _req: ChatRequest) -> Result<ChatResponse, LlmError> {
            Ok(self.response.clone())
        }
    }
}
