//! YAML front-matter writer (R10.1 / Q7).
//!
//! Renders the 12-field front-matter block in the locked order
//! documented in `spec/features/operator-success-reports.md` →
//! "Front-matter schema".  All run-varying fields (timestamps, host,
//! pid, git commit, binary version, run id, ledger sha, data source,
//! reconciliation status) live here — never in the body.  See
//! `crates/reports/src/render/mod.rs` for the body-vs-front-matter
//! discipline.
//!
//! The writer enumerates fields in a fixed `Vec → write` loop so the
//! key order is byte-stable across runs (R-9 in the risk register).

use std::fmt::Write;

/// All 12 front-matter fields, in the locked render order.
#[derive(Debug, Clone)]
pub struct FrontMatter {
    /// Period slug — e.g. `7d`, `weekly`, `since:2026-01-01T00:00:00Z`.
    pub period: String,
    /// `period_start` as RFC3339 (microsecond precision).
    pub period_start: String,
    /// `period_end` as RFC3339 (microsecond precision).
    pub period_end: String,
    /// Wall-clock of the render itself (RFC3339).
    pub generated: String,
    /// Hex prefix returned by [`crate::run_id::compute`].
    pub run_id: String,
    /// `sha256` of the `SQLite` ledger file at render time (64 hex
    /// chars).
    pub ledger_snapshot_sha: String,
    /// Optional seed for fixture / test runs (`0x<hex>`); empty for
    /// production renders.
    pub seed: Option<String>,
    /// Either `live-ledger` or `fixture:<path>`.
    pub data_source: String,
    /// Wall-clock seconds spent in the render path (float).
    pub wall_clock_s: String,
    /// Cargo `pkg-version` of the reports crate.
    pub binary_version: String,
    /// 40-char git commit; `n/a` if absent at build time.
    pub git_commit: String,
    /// `std::process::id()` of the rendering process.
    pub agent_pid: u32,
    /// Hostname; `unknown` on failure.
    pub host: String,
    /// Mirror of the R11 reconciliation outcome — `PASS` or `FAIL`.
    pub reconciliation: String,
}

impl FrontMatter {
    /// Render the front-matter block including the leading and trailing
    /// `---` fences plus the trailing `\n\n` body separator that the
    /// hashing convention slices on.
    ///
    /// Field order is fixed at v1+:
    /// `period, period_start, period_end, generated, run_id,
    /// ledger_snapshot_sha, seed, data_source, wall_clock_s,
    /// binary_version, git_commit, agent_pid, host, reconciliation`.
    ///
    /// Note that `seed` is emitted as an explicit empty value when
    /// `None` so the line ordering is byte-stable across runs.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(640);
        out.push_str("---\n");
        let _ = writeln!(out, "period: {}", self.period);
        let _ = writeln!(out, "period_start: {}", self.period_start);
        let _ = writeln!(out, "period_end: {}", self.period_end);
        let _ = writeln!(out, "generated: {}", self.generated);
        let _ = writeln!(out, "run_id: {}", self.run_id);
        let _ = writeln!(out, "ledger_snapshot_sha: {}", self.ledger_snapshot_sha);
        match &self.seed {
            Some(s) => {
                let _ = writeln!(out, "seed: {s}");
            }
            None => {
                // Emit explicit empty so the line ordering is byte-stable.
                let _ = writeln!(out, "seed:");
            }
        }
        let _ = writeln!(out, "data_source: {}", self.data_source);
        let _ = writeln!(out, "wall_clock_s: {}", self.wall_clock_s);
        let _ = writeln!(out, "binary_version: {}", self.binary_version);
        let _ = writeln!(out, "git_commit: {}", self.git_commit);
        let _ = writeln!(out, "agent_pid: {}", self.agent_pid);
        let _ = writeln!(out, "host: {}", self.host);
        let _ = writeln!(out, "reconciliation: {}", self.reconciliation);
        out.push_str("---\n\n");
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn fixture() -> FrontMatter {
        FrontMatter {
            period: "7d".into(),
            period_start: "2026-04-24T00:00:00.000000Z".into(),
            period_end: "2026-05-01T00:00:00.000000Z".into(),
            generated: "2026-05-01T12:00:00.000000Z".into(),
            run_id: "deadbeefcafebabe".into(),
            ledger_snapshot_sha: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .into(),
            seed: Some("0xC0FFEE".into()),
            data_source: "fixture:/tmp/sample.db".into(),
            wall_clock_s: "0.123".into(),
            binary_version: "0.1.0".into(),
            git_commit: "n/a".into(),
            agent_pid: 12345,
            host: "darwin-test".into(),
            reconciliation: "PASS".into(),
        }
    }

    #[test]
    fn t807_front_matter_renders_all_12_fields_in_order() {
        let s = fixture().render();
        // Locked order: assert that each field appears once and that
        // earlier fields come before later fields.
        let order = [
            "period:",
            "period_start:",
            "period_end:",
            "generated:",
            "run_id:",
            "ledger_snapshot_sha:",
            "seed:",
            "data_source:",
            "wall_clock_s:",
            "binary_version:",
            "git_commit:",
            "agent_pid:",
            "host:",
            "reconciliation:",
        ];
        // Track positions as Option<usize> — `None` represents "haven't
        // seen any key yet" so the first key always fits.
        let mut last_pos: Option<usize> = None;
        for key in order {
            let pos = s.find(key).unwrap_or_else(|| panic!("missing key {key}"));
            if let Some(prev) = last_pos {
                assert!(pos > prev, "key {key} out of order in {s}");
            }
            last_pos = Some(pos);
        }
    }

    #[test]
    fn t807_front_matter_starts_and_ends_with_fence() {
        let s = fixture().render();
        assert!(s.starts_with("---\n"));
        assert!(s.ends_with("---\n\n"));
    }

    #[test]
    fn t807_front_matter_seed_none_renders_empty_value() {
        let mut fm = fixture();
        fm.seed = None;
        let s = fm.render();
        // Bare key with empty value (parsable YAML, treated as null).
        assert!(s.contains("\nseed:\n"));
    }

    #[test]
    fn t807_front_matter_two_renders_byte_identical() {
        let fm = fixture();
        let a = fm.render();
        let b = fm.render();
        assert_eq!(a, b);
    }

    #[test]
    fn t807_front_matter_yaml_minimally_parseable() {
        // Sanity: each non-fence line is `key: value` shape, lowercase
        // snake_case key, scalar value.  Operators grep / awk this
        // without a YAML library so we keep the surface simple.
        let s = fixture().render();
        for line in s.lines() {
            if line == "---" || line.is_empty() {
                continue;
            }
            assert!(line.contains(':'), "no colon: {line}");
            let key = line.split_once(':').map_or("", |(k, _)| k);
            assert!(
                key.bytes()
                    .all(|b| b.is_ascii_lowercase() || b == b'_' || b.is_ascii_digit()),
                "key not snake_case: {key}"
            );
        }
    }
}
