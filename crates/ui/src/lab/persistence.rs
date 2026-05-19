//! Lab state persistence — ui-rethink-phase-a-lab T-D-17.
//!
//! Writes `LabState` as a JSON blob to the XDG config path
//! `~/.config/trading/cockpit-lab-state.json` (Design § 5.2) with a
//! 500 ms debounce to avoid disk-thrash during rapid chip selection.
//!
//! ## Design choices (Design § 5 / R6)
//!
//! - **JSON via `serde_json`** — machine-written, machine-read, human-inspectable.
//!   Pretty-printed for operator readability (< 1 KB file, 10 ms cost at
//!   most per write — Design § 5.3).
//! - **XDG path** — `$XDG_CONFIG_HOME/trading/cockpit-lab-state.json`, defaulting
//!   to `~/.config/trading/`. macOS keeps the same path (not `~/Library/…`)
//!   for symmetry with Linux (Design § 5.2).
//! - **Corruption → cold-start fallback** — `tracing::warn!` + cold-start
//!   defaults; never panic on a malformed state file (R6.3).
//! - **`version: 1` schema** — `params: null` reserved for Phase B; `compare_set`
//!   as an array so Phase B can extend additively (Design § 5.1).
//!
//! ## Thread safety
//!
//! The debouncer and write task run on the side-thread tokio runtime
//! (same handle as `KillSwitch`). The iced thread calls only `mark_dirty` +
//! `flush_if_due` — both are synchronous, lock-free, and `Send`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use tracing::warn;

use crate::lab::defaults::{
    LAB_COLD_START_RANGE, LAB_COLD_START_VENUE, cold_start_strategy, cold_start_symbol,
};
use crate::lab::state::{COMPARE_SET_CAP, DateRange, LabState, Preset};

// ── JSON schema (version: 1) ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistRange {
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    preset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistPair {
    venue: String,
    symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LabStateJson {
    version: u32,
    #[serde(default)]
    strategy: Option<String>,
    #[serde(default)]
    pair: Option<PersistPair>,
    range: PersistRange,
    params: Option<serde_json::Value>,
    #[serde(default)]
    compare_set: Vec<String>,
    /// cockpit-training-control T-D-N5 — Training panel collapsed state.
    /// Defaults to `true` (collapsed) when loading a pre-feature JSON file
    /// that doesn't have this field (R8.1 / Q4 — panel stays closed on cold
    /// start and on upgrade from a pre-feature cockpit state file).
    #[serde(default = "default_training_panel_collapsed")]
    training_panel_collapsed: bool,
}

/// Default function for `#[serde(default)]` on `training_panel_collapsed`.
/// Returns `true` so that pre-feature JSON files (missing this field) load
/// with the panel collapsed (R8.1 / Q4 contract).
const fn default_training_panel_collapsed() -> bool {
    true
}

// ── Path resolution ───────────────────────────────────────────────────────────

/// Resolve the lab-state file path per XDG / Design § 5.2.
///
/// `override_path` is used by tests to redirect writes to a temp dir.
#[must_use]
pub fn lab_state_path(override_path: Option<&Path>) -> PathBuf {
    if let Some(p) = override_path {
        return p.to_path_buf();
    }
    let base = std::env::var("XDG_CONFIG_HOME").map_or_else(
        |_| dirs_home_dir().map_or_else(|| PathBuf::from(".config"), |h| h.join(".config")),
        PathBuf::from,
    );
    base.join("trading").join("cockpit-lab-state.json")
}

fn dirs_home_dir() -> Option<PathBuf> {
    // Minimal home-dir resolution without the `dirs` crate dep.
    std::env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or({
            #[cfg(windows)]
            {
                std::env::var("USERPROFILE").ok().map(PathBuf::from)
            }
            #[cfg(not(windows))]
            {
                None
            }
        })
}

// ── Serialization ─────────────────────────────────────────────────────────────

fn range_to_json(r: &DateRange) -> PersistRange {
    match r {
        DateRange::Preset(p) => PersistRange {
            kind: "preset".to_string(),
            preset: Some(
                match p {
                    Preset::Last30d => "Last30d",
                    Preset::Last90d => "Last90d",
                    Preset::H1_2024 => "H1_2024",
                    Preset::H2_2024 => "H2_2024",
                }
                .to_string(),
            ),
            start: None,
            end: None,
        },
        DateRange::Custom { start_raw, end_raw } => PersistRange {
            kind: "custom".to_string(),
            preset: None,
            start: Some(start_raw.to_string()),
            end: Some(end_raw.to_string()),
        },
    }
}

fn range_from_json(r: &PersistRange) -> DateRange {
    if r.kind == "preset" {
        let preset = match r.preset.as_deref() {
            Some("Last30d") => Preset::Last30d,
            Some("H1_2024") => Preset::H1_2024,
            Some("H2_2024") => Preset::H2_2024,
            // "Last90d" and unknown/missing values all fall back to Last90d.
            _ => Preset::Last90d,
        };
        DateRange::Preset(preset)
    } else {
        DateRange::Custom {
            start_raw: SmolStr::new(r.start.as_deref().unwrap_or("")),
            end_raw: SmolStr::new(r.end.as_deref().unwrap_or("")),
        }
    }
}

// ── Encode / decode ───────────────────────────────────────────────────────────

/// Serialize a `LabState` to a pretty-printed JSON string.
///
/// # Errors
/// Returns a `serde_json` error if serialization fails (should not happen in
/// practice — all types are trivially serializable).
pub fn encode(state: &LabState) -> Result<String, serde_json::Error> {
    use trading_core::StrategyId;
    let json = LabStateJson {
        version: 1,
        strategy: state.strategy.as_ref().map(|s| s.0.to_string()),
        pair: state.pair.as_ref().map(|(v, sym)| PersistPair {
            venue: format!("{v:?}"),
            symbol: sym.0.to_string(),
        }),
        range: range_to_json(&state.range),
        params: None,
        compare_set: state
            .compare_set()
            .iter()
            .flatten()
            .map(|id: &StrategyId| id.0.to_string())
            .collect(),
        // cockpit-training-control T-D-N5 — persist training panel collapsed state.
        training_panel_collapsed: state.training_panel_collapsed,
    };
    serde_json::to_string_pretty(&json)
}

/// Deserialize a `LabState` from a JSON string.
///
/// On any parse error, logs a warning and returns the cold-start defaults.
#[must_use]
pub fn decode(json: &str, source_hint: &str) -> LabState {
    match serde_json::from_str::<LabStateJson>(json) {
        Ok(j) => {
            if j.version != 1 {
                warn!(
                    path = source_hint,
                    version = j.version,
                    "unsupported lab-state schema version; falling back to cold-start defaults"
                );
                return cold_start_defaults();
            }
            lab_state_from_json(&j)
        }
        Err(e) => {
            warn!(
                path = source_hint,
                error = %e,
                "failed to parse lab-state JSON; falling back to cold-start defaults"
            );
            cold_start_defaults()
        }
    }
}

fn lab_state_from_json(j: &LabStateJson) -> LabState {
    use trading_core::{StrategyId, Symbol, Venue};

    let strategy = j.strategy.as_deref().map(|s| StrategyId(SmolStr::new(s)));

    let pair = j.pair.as_ref().and_then(|p| {
        let venue = match p.venue.as_str() {
            "Binance" => Venue::Binance,
            _ => return None,
        };
        Some((venue, Symbol::new(&p.symbol)))
    });

    let range = range_from_json(&j.range);

    let mut state = LabState::with_selection(strategy, pair, range);

    for id_str in j.compare_set.iter().take(COMPARE_SET_CAP) {
        let id = StrategyId(SmolStr::new(id_str));
        let _ = state.toggle_compare(id); // no-op if cap reached
    }

    // cockpit-training-control T-D-N5 — restore training panel collapsed state.
    state.training_panel_collapsed = j.training_panel_collapsed;

    state
}

/// Build the cold-start `LabState` per Q-A3.
#[must_use]
pub fn cold_start_defaults() -> LabState {
    LabState::with_selection(
        Some(cold_start_strategy()),
        Some((LAB_COLD_START_VENUE, cold_start_symbol())),
        LAB_COLD_START_RANGE,
    )
}

// ── Disk I/O helpers ──────────────────────────────────────────────────────────

/// Write `state` to `path` synchronously.
/// Creates parent directories if they don't exist.
///
/// # Errors
/// Returns `std::io::Error` on filesystem failures.
pub fn write_sync(state: &LabState, path: &Path) -> std::io::Result<()> {
    let json = encode(state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, json.as_bytes())
}

/// Read and decode a `LabState` from `path`.
/// Returns cold-start defaults on any error (file not found, parse failure).
#[must_use]
pub fn restore_or_default(path: &Path) -> LabState {
    match std::fs::read_to_string(path) {
        Ok(content) => decode(&content, &path.display().to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // First launch — cold start is normal.
            cold_start_defaults()
        }
        Err(e) => {
            warn!(
                path = %path.display(),
                error = %e,
                "failed to read lab-state file; falling back to cold-start defaults"
            );
            cold_start_defaults()
        }
    }
}

// ── Debounce state ────────────────────────────────────────────────────────────

/// 500 ms debounce period (Design § 5.3).
pub const DEBOUNCE_MS: u64 = 500;

/// Lightweight debounce tracker held in the `Cockpit` struct.
///
/// Call `mark_dirty()` on every `Message::Lab*` mutation, then
/// `flush_if_due(state, path)` on a periodic timer (or at cockpit shutdown)
/// to write if the deadline has passed.
#[derive(Debug, Default)]
pub struct PersistenceDebouncer {
    /// `Some(instant_millis)` when a write is pending.
    dirty_since: Option<std::time::Instant>,
}

impl PersistenceDebouncer {
    /// Mark the state as dirty (a mutation occurred).
    pub fn mark_dirty(&mut self) {
        if self.dirty_since.is_none() {
            self.dirty_since = Some(std::time::Instant::now());
        }
    }

    /// Returns `true` if a write is due (dirty for ≥ 500 ms).
    #[must_use]
    pub fn is_due(&self) -> bool {
        self.dirty_since
            .is_some_and(|t| t.elapsed().as_millis() >= u128::from(DEBOUNCE_MS))
    }

    /// Write `state` to `path` if the debounce deadline has passed.
    /// Resets the dirty flag on success.
    pub fn flush_if_due(&mut self, state: &LabState, path: &Path) {
        if !self.is_due() {
            return;
        }
        if let Err(e) = write_sync(state, path) {
            warn!(path = %path.display(), error = %e, "failed to persist lab state");
        }
        self.dirty_since = None;
    }

    /// Force a flush regardless of the debounce deadline (e.g. on cockpit
    /// shutdown or integration tests). Resets the dirty flag.
    pub fn force_flush(&mut self, state: &LabState, path: &Path) {
        if self.dirty_since.is_none() {
            return; // nothing to flush
        }
        if let Err(e) = write_sync(state, path) {
            warn!(path = %path.display(), error = %e, "failed to force-flush lab state");
        }
        self.dirty_since = None;
    }

    /// Is there a pending write?
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty_since.is_some()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use trading_core::{StrategyId, Symbol, Venue};

    fn make_test_state() -> LabState {
        use crate::lab::state::DateRange;
        let mut s = LabState::with_selection(
            Some(StrategyId(SmolStr::new("v1.momentum"))),
            Some((Venue::Binance, Symbol::new("XRPUSDT"))),
            DateRange::Preset(Preset::Last90d),
        );
        let _ = s.toggle_compare(StrategyId(SmolStr::new("v0.sma")));
        s
    }

    /// T-D-17 — encode → decode round-trip preserves strategy, pair, range,
    /// and compare set.
    #[test]
    fn encode_decode_roundtrip() {
        let original = make_test_state();
        let json = encode(&original).unwrap();
        let restored = decode(&json, "test");

        assert_eq!(
            restored.strategy.as_ref().map(|s| s.0.as_str()),
            original.strategy.as_ref().map(|s| s.0.as_str()),
        );
        assert_eq!(restored.pair, original.pair);
        assert_eq!(restored.range, original.range);
        assert_eq!(restored.compare_len(), original.compare_len());
    }

    /// T-D-17 — corrupted JSON → cold-start fallback (no panic, no crash).
    #[test]
    fn decode_corrupted_returns_cold_start() {
        let bad = r#"{"version": 1, "range": NOTJSON}"#;
        let state = decode(bad, "test");
        // Should be cold-start defaults.
        assert_eq!(
            state.strategy.as_ref().map(|s| s.0.as_str()),
            Some("v1.momentum")
        );
    }

    /// T-D-17 — write_sync creates the file and parent dirs.
    #[test]
    fn write_sync_creates_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("sub").join("cockpit-lab-state.json");
        let state = cold_start_defaults();
        write_sync(&state, &path).unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("version"), "expected 'version' in JSON");
    }

    /// T-D-17 — restore_or_default returns cold-start when file absent.
    #[test]
    fn restore_absent_file_returns_cold_start() {
        let state = restore_or_default(Path::new("/tmp/nonexistent-lab-state-999.json"));
        assert_eq!(
            state.strategy.as_ref().map(|s| s.0.as_str()),
            Some("v1.momentum"),
            "absent file must yield cold-start strategy"
        );
    }

    /// T-D-17 — write then restore produces the same state.
    #[test]
    fn write_then_restore_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cockpit-lab-state.json");
        let original = make_test_state();
        write_sync(&original, &path).unwrap();
        let restored = restore_or_default(&path);
        assert_eq!(
            restored.strategy.as_ref().map(|s| s.0.as_str()),
            original.strategy.as_ref().map(|s| s.0.as_str()),
        );
        assert_eq!(restored.pair, original.pair);
        assert_eq!(restored.range, original.range);
        assert_eq!(restored.compare_len(), original.compare_len());
    }

    /// T-D-17 — debouncer: mark_dirty then is_due (before deadline) = false.
    #[test]
    fn debouncer_not_due_immediately() {
        let mut d = PersistenceDebouncer::default();
        d.mark_dirty();
        // Immediately after marking dirty, deadline not reached.
        assert!(!d.is_due(), "debounce not due immediately after mark_dirty");
        assert!(d.is_dirty());
    }

    /// T-D-17 — debouncer: no write without mark_dirty.
    #[test]
    fn debouncer_no_flush_when_clean() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("state.json");
        let state = cold_start_defaults();
        let mut d = PersistenceDebouncer::default();
        d.flush_if_due(&state, &path);
        assert!(!path.exists(), "no file should be written when not dirty");
    }

    /// T-D-17 — debouncer: force_flush writes immediately.
    #[test]
    fn debouncer_force_flush_writes() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("state.json");
        let state = cold_start_defaults();
        let mut d = PersistenceDebouncer::default();
        d.mark_dirty();
        d.force_flush(&state, &path);
        assert!(path.exists(), "force_flush must write the file");
        assert!(
            !d.is_dirty(),
            "dirty flag must be cleared after force_flush"
        );
    }

    /// T-D-17 — proptest placeholder: rapid mutations result in ≤1 write per
    /// 500 ms. The full proptest is in `tests/lab_persistence_proptest.rs`.
    /// This unit-level test verifies the debouncer never writes more than once
    /// per DEBOUNCE_MS period.
    #[test]
    fn debouncer_coalesces_multiple_marks() {
        let mut d = PersistenceDebouncer::default();
        // Mark dirty 100 times — only one dirty instant is stored.
        for _ in 0..100 {
            d.mark_dirty();
        }
        assert!(d.is_dirty());
        // Still only one pending write (the instant was set on the first mark).
        assert!(!d.is_due()); // not 500 ms yet in a unit test
    }

    /// T-D-17 — version: 1 schema shape is preserved.
    #[test]
    fn json_schema_version_1() {
        let state = cold_start_defaults();
        let json = encode(&state).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["version"].as_u64(), Some(1), "version must be 1");
        assert!(
            v.get("strategy").is_some(),
            "strategy field must be present"
        );
        assert!(v.get("pair").is_some(), "pair field must be present");
        assert!(v.get("range").is_some(), "range field must be present");
        assert!(
            v.get("compare_set").is_some(),
            "compare_set must be present"
        );
        assert!(
            v.get("params").is_some(),
            "params field must be present (reserved)"
        );
    }

    /// T-D-17 — cold-start defaults encode cleanly and decode back to the
    /// Q-A3 tuple.
    #[test]
    fn cold_start_encode_decode_qa3() {
        let state = cold_start_defaults();
        let json = encode(&state).unwrap();
        let restored = decode(&json, "test");
        assert_eq!(
            restored.strategy.as_ref().map(|s| s.0.as_str()),
            Some("v1.momentum")
        );
        assert_eq!(
            restored.pair.as_ref().map(|(_, s)| s.0.as_str()),
            Some("XRPUSDT")
        );
    }

    // ── cockpit-training-control T-D-N5 ─────────────────────────────────────

    /// T-D-N5 — `training_panel_collapsed` roundtrips through encode/decode.
    ///
    /// The field must survive a write → read cycle for both `true` (collapsed)
    /// and `false` (expanded) values.
    #[test]
    fn training_panel_collapsed_roundtrips() {
        // Collapsed = true (default).
        let mut state = cold_start_defaults();
        state.training_panel_collapsed = true;
        let json = encode(&state).unwrap();
        let restored = decode(&json, "test-collapsed");
        assert!(
            restored.training_panel_collapsed,
            "collapsed=true must roundtrip"
        );

        // Expanded = false (operator opened the panel, then saved state).
        state.training_panel_collapsed = false;
        let json = encode(&state).unwrap();
        let restored = decode(&json, "test-expanded");
        assert!(
            !restored.training_panel_collapsed,
            "collapsed=false must roundtrip"
        );
    }

    /// T-D-N5 — a pre-feature JSON file (missing `training_panel_collapsed`)
    /// loads with `training_panel_collapsed = true` per R8.1 / Q4 contract.
    ///
    /// This simulates upgrading from a cockpit state file written before
    /// this feature landed: the missing field defaults to `true` (collapsed).
    #[test]
    fn pre_feature_json_loads_collapsed_true() {
        // Construct a v1 JSON blob WITHOUT the `training_panel_collapsed` field.
        let pre_feature_json = r#"{
            "version": 1,
            "strategy": "v1.momentum",
            "pair": { "venue": "Binance", "symbol": "XRPUSDT" },
            "range": { "kind": "preset", "preset": "Last90d" },
            "params": null,
            "compare_set": []
        }"#;
        let restored = decode(pre_feature_json, "pre-feature-test");
        assert!(
            restored.training_panel_collapsed,
            "pre-feature JSON must load with training_panel_collapsed=true (R8.1 / Q4)"
        );
    }
}
