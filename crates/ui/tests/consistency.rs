//! Consistency self-audit enforced as tests.
//!
//! The ui-designer contract says:
//! - All copy lives in `ui::strings` — no inline `"..."` user-visible strings
//!   inside widget files.
//! - All colors / spacing / font sizes flow from `ui::theme` — no inline hex,
//!   no magic-number `Length::Units(N)`.
//!
//! "Drift starts with just one exception." These tests are the gate.
//!
//! What counts as inline:
//! - string literals in widget files (any `"..."` that isn't a ref to
//!   `strings::SOMETHING` or a format-string template that contains zero
//!   user-visible characters — most `format!("{x}")` templates count as
//!   user-visible and must be routed through strings).
//! - hex codes matching `#[0-9a-fA-F]{6}` anywhere in widget files.
//!
//! These tests read the widget sources from disk. That's fragile if the
//! file layout changes, but cheap and independent of iced internals.

use std::fs;
use std::path::{Path, PathBuf};

fn widget_sources() -> Vec<PathBuf> {
    let widgets_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/widgets");
    fs::read_dir(&widgets_dir)
        .expect("widgets dir exists")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("rs"))
        // `num.rs`, `frame.rs` are allowed infrastructure; still scanned.
        // ui-quality-gate-overhaul M2-B (2026-05-15): `debug_renderer.rs`
        // is gated at file-floor by `#![cfg(feature = "render-debug")]`
        // — it is diagnostic-only, compiled away on default builds, and
        // its string literals are operator-facing panic messages /
        // tracing event names, not user-facing UI copy. Skip it from
        // the consistency audit.
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_none_or(|name| name != "debug_renderer.rs")
        })
        .collect()
}

/// Skip string literals that are clearly structural (formatting / padding)
/// rather than user-visible prose. The rule of thumb:
/// - Inside a `format!()` template, anything that is only punctuation,
///   digits, and format placeholders is fine.
/// - Any literal that routes a value that itself came from `ui::strings`
///   is fine (tested by "no alphabetic character is outside of a
///   placeholder").
fn is_structural_literal(lit: &str) -> bool {
    let core = lit.trim_matches('"');
    if core.is_empty() {
        return true;
    }

    // Strip `{...}` placeholders so only the literal prose survives. If the
    // remaining text has zero alphabetic characters, there is no operator-
    // visible prose — the literal is a format template or a separator.
    let mut stripped = String::with_capacity(core.len());
    let mut depth = 0u32;
    for c in core.chars() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            _ if depth > 0 => {}
            _ => stripped.push(c),
        }
    }

    if !stripped.chars().any(|c| c.is_alphabetic()) {
        return true;
    }

    // Single-letter sign prefixes like `"-"`, `"+"` survive with a non-empty
    // `stripped` but are obviously not prose.
    if stripped.len() == 1 && !stripped.chars().next().unwrap_or(' ').is_alphabetic() {
        return true;
    }

    false
}

/// Extract string literals, ignoring:
/// - doc-comments (`//`, `//!`, `///`),
/// - attribute-line strings (`#[cfg(feature = "…")]`),
/// - anything inside a `#[cfg(test)]` module (those are test-only literals).
/// - lines that contain a `tracing::` macro call (trace_span!, error!,
///   warn!, info!, debug!, trace!) — the first arg there is a span /
///   event name, not user-visible UI copy. Added 2026-05-15 for the
///   ui-quality-gate-overhaul M2-A instrumentation; see
///   `spec/ui-quality-gate-overhaul/feature.md ## Q2`.
///
/// This is not a full Rust parser, just good enough for the cockpit files
/// which never use raw strings.
fn collect_string_literals(src: &str) -> Vec<(usize, String)> {
    let mut lits = Vec::new();
    let mut in_cfg_test = false;
    let mut cfg_test_depth = 0i32;
    // Track multi-line `tracing::*!(...)` macro invocations: once we see
    // `tracing::` open the macro, swallow string literals until the
    // matching close-paren at depth 0.
    let mut in_tracing_macro = false;
    let mut tracing_paren_depth = 0i32;
    for (lineno, line) in src.lines().enumerate() {
        let trimmed = line.trim_start();

        // Track `#[cfg(test)]` modules: once we see the attribute, count
        // braces to find the matching close.
        if trimmed.starts_with("#[cfg(test)]") {
            in_cfg_test = true;
            cfg_test_depth = 0;
            continue;
        }
        if in_cfg_test {
            for c in line.chars() {
                if c == '{' {
                    cfg_test_depth += 1;
                } else if c == '}' {
                    cfg_test_depth -= 1;
                    if cfg_test_depth <= 0 {
                        in_cfg_test = false;
                        cfg_test_depth = 0;
                    }
                }
            }
            continue;
        }

        if trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with("#![") || trimmed.starts_with("#[") {
            continue;
        }

        // Track entry into / exit from a `tracing::*!(...)` macro call.
        // The macro can span multiple lines; we count parens at depth 0
        // (ignoring inside string literals — line content is fine since
        // tracing fields don't carry close-paren inside strings).
        if !in_tracing_macro
            && (line.contains("tracing::trace_span!")
                || line.contains("tracing::error!")
                || line.contains("tracing::warn!")
                || line.contains("tracing::info!")
                || line.contains("tracing::debug!")
                || line.contains("tracing::trace!"))
        {
            in_tracing_macro = true;
            tracing_paren_depth = 0;
        }
        if in_tracing_macro {
            for c in line.chars() {
                match c {
                    '(' => tracing_paren_depth += 1,
                    ')' => {
                        tracing_paren_depth -= 1;
                        if tracing_paren_depth <= 0 {
                            in_tracing_macro = false;
                            tracing_paren_depth = 0;
                        }
                    }
                    _ => {}
                }
            }
            // Skip the line entirely — every literal inside a tracing
            // macro call is a span/event name or structured-field
            // value, not user-visible UI copy.
            continue;
        }
        // Simple state machine: scan for `"..."` not preceded by `'`.
        let bytes = line.as_bytes();
        let mut i = 0;
        let mut in_str = false;
        let mut start = 0;
        let mut escape = false;
        while i < bytes.len() {
            let c = bytes[i];
            if in_str {
                if escape {
                    escape = false;
                } else if c == b'\\' {
                    escape = true;
                } else if c == b'"' {
                    let lit = &line[start..=i];
                    lits.push((lineno + 1, lit.to_string()));
                    in_str = false;
                }
            } else if c == b'"'
                && (i == 0 || bytes[i - 1] != b'\'')
                && !line[..i].trim_end().ends_with("r#")
            {
                in_str = true;
                start = i;
            }
            i += 1;
        }
    }
    lits
}

#[test]
fn no_inline_user_visible_strings_in_widgets() {
    let mut violations = Vec::new();
    for path in widget_sources() {
        let src = fs::read_to_string(&path).expect("read widget source");
        for (lineno, lit) in collect_string_literals(&src) {
            if is_structural_literal(&lit) {
                continue;
            }
            violations.push(format!("{}:{} → {}", path.display(), lineno, lit));
        }
    }
    assert!(
        violations.is_empty(),
        "inline user-visible string literals inside widgets — route via \
         `ui::strings`:\n{}",
        violations.join("\n")
    );
}

#[test]
fn no_inline_hex_colors_in_widgets_or_state() {
    // Widgets and state must not carry hex colors — only `theme.rs` may.
    let mut violations = Vec::new();
    let mut sources = widget_sources();
    sources.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/state.rs"));
    sources.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/strings.rs"));

    let hex_re = regex_lite::compile(r"#[0-9a-fA-F]{6}").expect("regex compiles");

    for path in sources {
        let src = fs::read_to_string(&path).expect("read src");
        for (lineno, line) in src.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            if hex_re.is_match(line) {
                violations.push(format!(
                    "{}:{} → {}",
                    path.display(),
                    lineno + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "inline hex colors outside theme.rs:\n{}",
        violations.join("\n")
    );
}

// ── Minimal regex shim ──────────────────────────────────────────────────────
//
// We avoid an extra dependency (`regex`) just to match `#[0-9a-fA-F]{6}`.
// Hand-roll the exact pattern; yes this is ugly, yes it keeps the Cargo.toml
// clean.

mod regex_lite {
    pub fn compile(_pattern: &str) -> Result<Regex, ()> {
        Ok(Regex)
    }
    pub struct Regex;
    impl Regex {
        pub fn is_match(&self, haystack: &str) -> bool {
            let bytes = haystack.as_bytes();
            let mut i = 0;
            while i + 6 < bytes.len() {
                if bytes[i] == b'#' {
                    let chunk = &bytes[i + 1..i + 7];
                    if chunk.iter().all(|c| c.is_ascii_hexdigit()) {
                        // Make sure the 7th char isn't another hexdigit so we
                        // only catch 6-digit tokens.
                        let next = bytes.get(i + 7).copied().unwrap_or(b' ');
                        if !next.is_ascii_hexdigit() {
                            return true;
                        }
                    }
                }
                i += 1;
            }
            false
        }
    }
}
