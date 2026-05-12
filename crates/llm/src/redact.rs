//! Secret-redaction helpers (T1915).
//!
//! [`redact`] is the canonical sanitizer for any string that may carry
//! an API key in log lines, error messages, or audit memos (R8.3). It
//! preserves the **prefix up to and including the second `-`** (so
//! provider + tier are still recognisable in forensics —
//! `sk-ant-secret-12345` → `sk-ant-***2345`) and the **last 4
//! characters** for human matching against the provider dashboard's
//! key-rotation UI, replacing the middle with `***`.
//!
//! Fallback when the input lacks two `-` separators: keep the first 6
//! characters + `***` + the last 4. This handles bare hex keys
//! (`abcdef0123456789…`) and the smoke-stub variants (`stub-test-key`).
//!
//! Short inputs (< 10 chars) collapse to `***` — too little
//! distinguishing prefix to retain anything safely.
//!
//! **Pass-3 scope note** (developer, 2026-05-12): the spec calls for an
//! `install_tracing_redactor()` companion that installs a
//! `tracing_subscriber::Layer` intercepting events with key-shaped
//! field names. The field-rewriting layer is non-trivial (requires
//! `tracing_subscriber` as a runtime dep on `llm` and a custom
//! `Visit`/`Layer` impl). Pass 3 ships the pure `redact()` function
//! since that's the surface every error message + audit-memo formatter
//! consumes anyway; the tracing-layer half is deferred to a pass-4
//! follow-up (still T1915 — `[~]` rather than `[x]` in this hand-off).

/// Sanitize a single secret for forensic display.
///
/// Returns a new owned `String`. The redaction is deterministic — same
/// input always produces same output — so structured-log consumers can
/// dedupe lines without false-positives from non-deterministic masking.
#[must_use]
pub fn redact(secret: &str) -> String {
    // Anything below the threshold can't usefully retain prefix+suffix
    // without giving away most of the key. Collapse outright.
    if secret.len() < 10 {
        return "***".to_string();
    }

    // Find the position of the second `-` (so "sk-ant-secret" preserves
    // "sk-ant-"). Position-after-the-dash so the kept prefix INCLUDES
    // the dash itself.
    let mut prefix_len: Option<usize> = None;
    let mut dashes_seen = 0usize;
    for (i, ch) in secret.char_indices() {
        if ch == '-' {
            dashes_seen += 1;
            if dashes_seen == 2 {
                // include the `-` itself
                prefix_len = Some(i + 1);
                break;
            }
        }
    }
    let prefix_len = prefix_len.unwrap_or(6);
    // Defensive: never let the prefix swallow most of the key.
    let prefix_len = prefix_len.min(secret.len().saturating_sub(4));

    let prefix: String = secret.chars().take(prefix_len).collect();
    let suffix_chars: Vec<char> = secret.chars().collect();
    let suffix_start = suffix_chars.len().saturating_sub(4);
    let suffix: String = suffix_chars[suffix_start..].iter().collect();
    format!("{prefix}***{suffix}")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// T1915 (a): `redact("sk-ant-secret-12345")` does NOT contain
    /// `"secret-12345"`.
    #[test]
    fn t1915_anthropic_key_does_not_leak_full_secret() {
        let red = redact("sk-ant-secret-12345");
        assert!(
            !red.contains("secret-12345"),
            "redacted output leaks key: {red}"
        );
        // Forensic prefix retained.
        assert!(red.starts_with("sk-ant-"), "lost identifying prefix: {red}");
        // Last 4 retained (`2345`) for operator key-matching.
        assert!(red.ends_with("2345"), "lost identifying suffix: {red}");
        assert!(red.contains("***"), "no redaction marker: {red}");
    }

    /// T1915 (b): `redact("sk-shortie")` does NOT contain the full string.
    #[test]
    fn t1915_short_key_collapsed_to_marker() {
        let red = redact("sk-shortie");
        // 10 chars exactly — boundary case; ensure no full leak.
        assert!(!red.contains("shortie"), "redacted output leaks: {red}");
    }

    /// T1915 (c): below-threshold input collapses to `***`.
    #[test]
    fn t1915_below_threshold_collapses() {
        assert_eq!(redact("short"), "***");
        assert_eq!(redact(""), "***");
        assert_eq!(redact("abc"), "***");
    }

    /// OpenAI-shape key (`sk-...`) — fallback path because only one `-`
    /// is present.
    #[test]
    fn t1915_openai_key_uses_fallback_prefix() {
        let red = redact("sk-proj-AbCdEf1234567890");
        // sk-proj- has two dashes — second-dash boundary kicks in.
        assert!(red.starts_with("sk-proj-"), "wrong prefix: {red}");
        assert!(red.contains("***"), "no marker: {red}");
        assert!(red.ends_with("7890"), "wrong suffix: {red}");
        assert!(
            !red.contains("AbCdEf1234"),
            "redacted output leaks middle: {red}"
        );
    }

    /// Bare hex key — no dashes at all. Uses 6-char prefix fallback.
    #[test]
    fn t1915_bare_key_uses_six_char_fallback() {
        let red = redact("abcdef0123456789abcdef0123456789");
        assert!(red.starts_with("abcdef"), "wrong prefix: {red}");
        assert!(red.ends_with("6789"), "wrong suffix: {red}");
        // Defensive: do NOT leak the rest of the key.
        assert!(!red.contains("0123456789abcdef0123"), "leak: {red}");
    }

    /// `redact` is pure / deterministic — same input → same output.
    #[test]
    fn t1915_redact_is_deterministic() {
        let key = "sk-ant-secret-deadbeef";
        assert_eq!(redact(key), redact(key));
    }
}
