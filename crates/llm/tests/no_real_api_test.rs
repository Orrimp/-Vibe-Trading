//! T1940 / R14.4 / V4 — static-grep invariant: no test under
//! `crates/llm/tests/` may import a `reqwest::Client` against a URL
//! that is NOT a wiremock-spawned `MockServer.uri()` OR a localhost
//! pattern (`http://localhost:` / `http://127.0.0.1:`).
//!
//! This catches the regression mode where a developer hand-edits a
//! test to talk to `api.anthropic.com` directly during local
//! iteration and forgets to revert; the workspace test suite would
//! then leak outbound HTTPS (V4 failure) and surface real API costs.
//!
//! ## What the test does
//!
//! Walks every `*.rs` file under `crates/llm/tests/`, extracts every
//! URL string literal (matched by `https://` / `http://` prefix), and
//! for each one asserts that either:
//!
//! 1. The URL is in a **wiremock spawn pattern** — i.e. the same file
//!    references `mock_server.uri()` somewhere (the wiremock harness
//!    pattern reused across `smoke_harness.rs`, `recording_test.rs`,
//!    etc.); OR
//! 2. The URL is a **localhost pattern** (`http://localhost:` /
//!    `http://127.0.0.1:`) — Ollama's default + ad-hoc test servers.
//!
//! ## Scope
//!
//! - **Walked**: every `.rs` file directly under `crates/llm/tests/`
//!   plus files under `crates/llm/tests/fixtures/`.
//! - **Skipped**: this file itself (the test references real-API
//!   hostnames in this docstring to call them out).
//! - **Skipped**: any `// ALLOW-REAL-API: <reason>` comment line
//!   silences the gate for the immediately-following URL (operator
//!   override; should be rare).

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map_or_else(|| crate_dir.clone(), Path::to_path_buf)
}

fn tests_dir() -> PathBuf {
    workspace_root().join("crates").join("llm").join("tests")
}

fn this_file_name() -> &'static str {
    // The test file's own name — skip it during the walk so the
    // example URLs in this docstring don't trip the gate.
    "no_real_api_test.rs"
}

fn walk_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_rs_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            if path.file_name().is_some_and(|n| n == this_file_name()) {
                continue;
            }
            out.push(path);
        }
    }
}

/// Extract every URL substring starting with `http://` or `https://`
/// inside double-quoted string literals. Returns `(line_number,
/// url_text)` pairs.
fn extract_urls(content: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        // Treat any `https?://...` substring inside double quotes as
        // an HTTP-target literal. This is a coarse match — false
        // positives surface as informative failures rather than
        // missed violations.
        let mut search_from = 0;
        while let Some(pos) = line[search_from..].find("\"http") {
            let abs = search_from + pos;
            let after_quote = abs + 1;
            // Find the closing quote.
            let Some(close_rel) = line[after_quote..].find('"') else {
                break;
            };
            let url = &line[after_quote..after_quote + close_rel];
            // Skip URLs in comments — find `//` before this position
            // on the same line.
            let comment_pos = line[..abs].find("//");
            if comment_pos.is_some() {
                search_from = after_quote + close_rel + 1;
                continue;
            }
            if url.starts_with("http://") || url.starts_with("https://") {
                found.push((idx + 1, url.to_string()));
            }
            search_from = after_quote + close_rel + 1;
        }
    }
    found
}

/// A URL is OK if it's a localhost pattern OR the same file uses
/// `mock_server.uri()` / `MockServer::start()` (wiremock spawn).
fn url_is_wiremock_or_localhost(url: &str, file_content: &str) -> bool {
    if url.starts_with("http://localhost:") || url.starts_with("http://127.0.0.1:") {
        return true;
    }
    // Wiremock-spawn pattern: the same file uses MockServer.
    file_content.contains("mock_server.uri()")
        || file_content.contains("MockServer::start")
        || file_content.contains("MockServer::new")
}

/// T1940 — every URL literal in every test file is either
/// wiremock-bound or localhost.
#[test]
fn t1940_no_real_api_calls_in_tests() {
    let mut files = Vec::new();
    walk_rs_files(&tests_dir(), &mut files);

    let mut violations: Vec<String> = Vec::new();
    for file in &files {
        let Ok(content) = std::fs::read_to_string(file) else {
            continue;
        };
        for (line_no, url) in extract_urls(&content) {
            // Operator override: `// ALLOW-REAL-API: <reason>` on
            // the same or preceding line.
            let lines: Vec<&str> = content.lines().collect();
            let allow = line_no
                .checked_sub(1)
                .and_then(|i| lines.get(i.saturating_sub(1)))
                .is_some_and(|l| l.contains("ALLOW-REAL-API"))
                || lines
                    .get(line_no - 1)
                    .is_some_and(|l| l.contains("ALLOW-REAL-API"));
            if allow {
                continue;
            }
            if !url_is_wiremock_or_localhost(&url, &content) {
                violations.push(format!(
                    "{}:{}: URL `{url}` is not wiremock-bound or localhost",
                    file.display(),
                    line_no
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "T1940 V4 violation: tests imported real-API URLs without wiremock \
         or localhost backing:\n\n{}\n\nFix: spawn a `wiremock::MockServer`, \
         or comment the URL with `// ALLOW-REAL-API: <reason>` to silence.",
        violations.join("\n")
    );
}
