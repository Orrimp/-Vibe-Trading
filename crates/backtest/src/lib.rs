//! Backtest engine: `MatchingEngine` trait, `PaperEngine`, backtest loop.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
#![warn(clippy::pedantic)]

pub mod engine;
pub mod paper;

pub use engine::MatchingEngine;
pub use paper::PaperEngine;

/// Compute the deterministic-content SHA-256 of a backtest report.
///
/// # Determinism convention
///
/// The YAML front matter of every backtest report contains a `generated:` field
/// with a wall-clock timestamp.  That field is intentionally excluded from the
/// hash so that two runs of the same scenario at the same seed produce
/// byte-identical hashes even though they were run at different wall-clock times.
///
/// Everything from the first line **after** the closing `---` of the front
/// matter is included in the hash.  The front matter spans from the first `---`
/// line (inclusive) to the second `---` line (inclusive); only the body that
/// follows is hashed.
///
/// This function is the single source of truth for the hashing convention; both
/// the report writer and the T33 determinism test call it so that the comparison
/// is always apples-to-apples.
#[must_use]
pub fn report_body_hash(report: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};

    // Skip the YAML front matter (everything up to and including the closing
    // `---` delimiter).  The front matter starts at the first `---` line and
    // ends at the next `---` line.
    let body = extract_report_body(report);

    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    hasher.finalize().to_vec()
}

/// Extract the report body (everything after the YAML front-matter block).
///
/// The front matter is the region from the first `---` line up to and including
/// the second `---` line.  Everything that follows is the "body".  If no valid
/// front matter is found the entire string is returned as-is so the hash still
/// works on hand-crafted strings.
#[must_use]
pub fn extract_report_body(report: &str) -> &str {
    let mut dash_count = 0usize;
    let mut body_start = 0usize;

    for line in report.split_inclusive('\n') {
        body_start += line.len();
        if line.trim_end() == "---" {
            dash_count += 1;
            if dash_count == 2 {
                // body_start now points just past the closing `---` line
                break;
            }
        }
    }

    if dash_count < 2 {
        // No valid front-matter delimiter found — hash the whole thing
        return report;
    }

    &report[body_start..]
}
