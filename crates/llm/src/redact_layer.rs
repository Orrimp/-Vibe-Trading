//! Tracing-subscriber Layer that redacts secrets from event fields
//! BEFORE they hit downstream sinks (audit ledger, stdout, file).
//!
//! Wraps the pure-fn [`crate::redact::redact`] (T1915) — no separate
//! sanitisation logic (R-NR.1).
//!
//! # Design
//!
//! The `RedactLayer` uses a **thread-local field-override map** pattern:
//!
//! 1. `on_event` intercepts every event (recursion-guarded via a thread-local
//!    `REDACTING` flag per D-RED-5).
//! 2. A [`CollectingVisitor`] collects every string-shaped field value and
//!    runs it through the closed regex rule set (D-RED-1).
//! 3. If any field is redacted, the Layer stores the `(field_name, redacted_value)`
//!    pairs in a thread-local `REDACTED_FIELDS` map.
//! 4. A [`RedactingFormatFields`] (used by [`crate::tracing_init`] inside
//!    `fmt::Layer`) reads from `REDACTED_FIELDS` during field formatting and
//!    substitutes redacted values. This is the **correct approach** for
//!    tracing-subscriber 0.3.x: event field rewriting via a custom
//!    `FormatFields` impl that reads the override map.
//! 5. In WARN mode (`REDACT_LAYER_MODE=warn`), the Layer emits a meta-event
//!    via a SEPARATE `tracing::warn!` call AFTER the thread-local is cleared
//!    (outside the reentrancy guard) — using a `PENDING_META_EVENTS` thread-
//!    local queue that is drained after `on_event` returns.
//!
//! # Why not "emit-redacted + filter-original"?
//!
//! The D-RED-3 spec suggested "emit-redacted + filter-original" as the v0.1.0
//! shape. This was invalidated by tracing's reentrancy guard: calling
//! `tracing::info!(...)` from inside `on_event` is silently dropped
//! (tracing detects reentrancy and no-ops the inner dispatch). The thread-
//! local override map is the correct replacement — it decouples the redaction
//! from the dispatch cycle.
//!
//! # Usage
//!
//! Every binary calls [`crate::tracing_init::install_global`] instead of
//! `tracing_subscriber::fmt().init()`. The helper composes the `RedactLayer`
//! BEFORE `fmt::Layer` with `RedactingFormatFields` in the registry chain
//! (R1.4 ordering).
//!
//! ## Per-site opt-out (D-RED-4)
//!
//! Mark a field exempt from redaction by adding both `__redact_skip` and
//! `__redact_reason` fields to the same tracing event:
//!
//! ```no_compile
//! tracing::info!(
//!     api_key_doc = "sk-ant-EXAMPLE",
//!     __redact_skip = "api_key_doc",
//!     __redact_reason = "documentation example; not a real key",
//!     "API key field name doc",
//! );
//! ```
//!
//! Missing `__redact_reason` → skip is NOT applied; a meta-event records the
//! missing-reason (fail-safe-closed per D-RED-4).

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

use regex::{Regex, RegexSet};
use tracing::{Event, Subscriber};
use tracing_subscriber::Layer;
use tracing_subscriber::layer::Context;

use crate::redact::redact;

// ── Rule names (D-RED-1 ratification table order) ─────────────────────────────

const RULE_ANTHROPIC_KEY: &str = "anthropic_key";
const RULE_OPENAI_PROJ_KEY: &str = "openai_proj_key";
const RULE_OPENAI_KEY: &str = "openai_key";
const RULE_BEARER_TOKEN: &str = "bearer_token";
const RULE_JWT: &str = "jwt";
const RULE_AWS_ACCESS: &str = "aws_access";
const RULE_AWS_SECRET_CONTEXT: &str = "aws_secret_context";
const RULE_PASSWORD_FIELD_NAME: &str = "password_field_name";
const RULE_ENTROPY_FALLBACK: &str = "entropy_fallback";

/// Rule names parallel to `VALUE_PATTERNS` (index matches, 0..6).
const VALUE_RULE_NAMES: &[&str] = &[
    RULE_ANTHROPIC_KEY,      // 0
    RULE_OPENAI_PROJ_KEY,    // 1
    RULE_OPENAI_KEY,         // 2
    RULE_BEARER_TOKEN,       // 3
    RULE_JWT,                // 4
    RULE_AWS_ACCESS,         // 5
    RULE_AWS_SECRET_CONTEXT, // 6
];

// ── Value-match patterns (order must match VALUE_RULE_NAMES; first-match wins)

const VALUE_PATTERNS: &[&str] = &[
    r"(?i)sk-ant-[A-Za-z0-9_\-]{16,}",  // 0 anthropic_key
    r"(?i)sk-proj-[A-Za-z0-9_\-]{16,}", // 1 openai_proj_key
    r"(?i)sk-[A-Za-z0-9_\-]{16,}",      // 2 openai_key
    r"Bearer\s+[A-Za-z0-9._\-=]{20,}",  // 3 bearer_token
    r"eyJ[A-Za-z0-9._\-]+\.eyJ[A-Za-z0-9._\-]+\.[A-Za-z0-9._\-]+", // 4 jwt
    r"AKIA[0-9A-Z]{16}",                // 5 aws_access
    r"[A-Za-z0-9/+=]{40}",              // 6 aws_secret_context (contextual)
];

const PASSWORD_FIELD_NAMES: &[&str] = &[
    "password",
    "pwd",
    "passwd",
    "secret",
    "api_key",
    "apikey",
    "auth_token",
    "bearer",
];

const AWS_CONTEXT_FRAGMENTS: &[&str] = &["secret", "access", "token"];

const ENTROPY_THRESHOLD: f64 = 4.5;
const ENTROPY_MIN_LEN: usize = 32;
const ENTROPY_FIELD_FRAGMENTS: &[&str] = &["key", "token", "secret"];

// ── Compiled regex ────────────────────────────────────────────────────────────

fn value_regex_set() -> &'static RegexSet {
    static SET: OnceLock<RegexSet> = OnceLock::new();
    SET.get_or_init(|| RegexSet::new(VALUE_PATTERNS).expect("static patterns compile"))
}

fn bearer_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"Bearer\s+([A-Za-z0-9._\-=]{20,})").expect("static bearer regex compiles")
    })
}

// ── Per-process redaction counter ─────────────────────────────────────────────

static REDACTION_COUNTER: AtomicU32 = AtomicU32::new(0);

// ── Thread-local state ────────────────────────────────────────────────────────

thread_local! {
    /// Map from field name → redacted value, populated by `RedactLayer::on_event`
    /// and read by `RedactingFormatFields` during fmt field formatting.
    static REDACTED_FIELDS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());

    /// Reentrancy guard: set to `true` while `on_event` is dispatching.
    /// Prevents recursive tracing calls inside `on_event` from re-entering
    /// the Layer's processing logic (D-RED-5).
    static REDACTING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };

    /// Test-only meta-event capture: (field_name, rule_key) pairs recorded
    /// during WARN-mode redaction. Tests call [`take_meta_events`] to drain.
    /// In production, `emit_meta_to_stderr` is the observable output.
    static META_EVENTS: RefCell<Vec<(String, String)>> = const { RefCell::new(Vec::new()) };
}

/// Read the current redacted-fields map (for test seams + custom formatters).
///
/// Returns a snapshot of the current thread's `REDACTED_FIELDS`. Consumed
/// by [`RedactingFormatFields`] during field rendering.
pub fn take_redacted_fields() -> HashMap<String, String> {
    REDACTED_FIELDS.with(|m| {
        let mut guard = m.borrow_mut();
        let out = guard.clone();
        guard.clear();
        out
    })
}

/// Peek at the current redacted-fields map WITHOUT clearing it.
/// Used by [`RedactingFormatFields`] during field rendering, and by tests
/// to inspect what the `RedactLayer` stored for the last event.
pub fn peek_redacted_fields() -> HashMap<String, String> {
    REDACTED_FIELDS.with(|m| m.borrow().clone())
}

/// Drain the test-only meta-event capture queue.
///
/// Returns `(field_name, rule_key)` pairs accumulated since the last call.
/// In WARN mode (or gate+verbose), `RedactLayer` also writes to stderr via
/// [`emit_meta_to_stderr`] for production observability. This function
/// provides a programmatic seam for integration tests.
pub fn take_meta_events() -> Vec<(String, String)> {
    META_EVENTS.with(|m| m.borrow_mut().drain(..).collect())
}

// ── RedactMode ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RedactMode {
    Warn,
    Gate,
}

// ── RedactLayer ───────────────────────────────────────────────────────────────

/// Tracing-subscriber `Layer` that redacts secret-shaped field values.
///
/// Stores redacted field overrides in a thread-local that the companion
/// [`RedactingFormatFields`] reads during event formatting.
pub struct RedactLayer {
    mode: RedactMode,
    verbose: bool,
}

impl RedactLayer {
    /// Construct from environment: `REDACT_LAYER_MODE=warn|gate` (default `warn`).
    #[must_use]
    pub fn from_env() -> Self {
        let mode_str = std::env::var("REDACT_LAYER_MODE").unwrap_or_else(|_| "warn".to_string());
        let mode = match mode_str.to_lowercase().as_str() {
            "warn" => RedactMode::Warn,
            "gate" => RedactMode::Gate,
            other => {
                eprintln!(
                    "redact_layer: invalid REDACT_LAYER_MODE={other:?}; defaulting to 'warn'"
                );
                RedactMode::Warn
            }
        };
        let verbose = matches!(std::env::var("REDACT_LAYER_VERBOSE").as_deref(), Ok("1"));
        Self { mode, verbose }
    }

    /// Construct in WARN mode. For tests and direct construction.
    #[must_use]
    pub fn warn_mode() -> Self {
        Self {
            mode: RedactMode::Warn,
            verbose: false,
        }
    }

    /// Construct in gate mode (no meta-events unless verbose).
    #[must_use]
    pub fn gate_mode() -> Self {
        Self {
            mode: RedactMode::Gate,
            verbose: false,
        }
    }

    /// Construct in gate mode with verbose meta-events.
    #[must_use]
    pub fn gate_verbose_mode() -> Self {
        Self {
            mode: RedactMode::Gate,
            verbose: true,
        }
    }
}

/// Construct a `RedactLayer` from environment (equivalent to `RedactLayer::from_env`).
pub fn redact_tracing_layer<S>() -> RedactLayer
where
    S: Subscriber,
{
    RedactLayer::from_env()
}

// ── redact_str — public test seam (D-RED-6) ───────────────────────────────────

/// Apply the closed regex rule set to a single string value.
///
/// Returns `Borrowed(s)` when no rule matches; `Owned(redacted)` on match.
/// This is the **unit-test surface** for the rule set.
///
/// Field-name-context rules (`password_field_name`, `aws_secret_context`,
/// `entropy_fallback`) require a field name. This function passes `""` as
/// the field name, so contextual rules won't fire for unknown-name calls.
/// Use [`apply_redaction_for_field`] to supply a field name.
#[must_use]
pub fn redact_str(s: &str) -> Cow<'_, str> {
    let (cow, _rule) = apply_redaction_for_field(s, "");
    cow
}

/// Apply the full 9-rule set with field-name context.
///
/// Returns `(Cow, rule_name)` where `rule_name` identifies which rule fired
/// (`""` if no match).
#[must_use]
pub(crate) fn apply_redaction_for_field<'s>(
    value: &'s str,
    field_name: &str,
) -> (Cow<'s, str>, &'static str) {
    let field_lower = field_name.to_lowercase();

    // Rule: password_field_name — exact match on field NAME.
    if PASSWORD_FIELD_NAMES.iter().any(|&n| field_lower == n) {
        return (Cow::Owned(redact(value)), RULE_PASSWORD_FIELD_NAME);
    }

    // Rules 0-6: value regex set. Iterate in order (first match wins).
    let matches = value_regex_set().matches(value);
    for idx in matches.into_iter() {
        if idx == 6 {
            // aws_secret_context: field name must contain "secret", "access", or "token".
            if AWS_CONTEXT_FRAGMENTS
                .iter()
                .any(|&f| field_lower.contains(f))
            {
                return (Cow::Owned(redact(value)), RULE_AWS_SECRET_CONTEXT);
            }
            continue;
        }
        if idx == 3 {
            // bearer_token: extract the token part for cleaner redact() output.
            let redacted_val = if let Some(cap) = bearer_regex().captures(value) {
                let token = &cap[1];
                format!("Bearer {}", redact(token))
            } else {
                redact(value)
            };
            return (Cow::Owned(redacted_val), RULE_BEARER_TOKEN);
        }
        return (
            Cow::Owned(redact(value)),
            VALUE_RULE_NAMES
                .get(idx)
                .copied()
                .unwrap_or("value_pattern"),
        );
    }

    // entropy_fallback: >= 32 chars + high Shannon entropy + key/token/secret field name.
    if value.chars().count() >= ENTROPY_MIN_LEN {
        let name_matches = ENTROPY_FIELD_FRAGMENTS
            .iter()
            .any(|&f| field_lower.contains(f));
        if name_matches && shannon_entropy(value) >= ENTROPY_THRESHOLD {
            return (Cow::Owned(redact(value)), RULE_ENTROPY_FALLBACK);
        }
    }

    (Cow::Borrowed(value), "")
}

/// Shannon entropy (bits per char) of a UTF-8 string.
///
/// Uses floating-point arithmetic — allowed here because this is a pattern-
/// matching heuristic (entropy_fallback rule), not a financial calculation.
/// `clippy::float_arithmetic` is suppressed locally per CLAUDE.md: "no `f64`
/// in money/price/qty calculation" — entropy is not a monetary value.
#[allow(clippy::float_arithmetic)]
fn shannon_entropy(s: &str) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let len = s.len() as f64;
    let mut counts = [0u32; 256];
    for &b in s.as_bytes() {
        counts[b as usize] += 1;
    }
    counts.iter().filter(|&&c| c > 0).fold(0.0f64, |acc, &c| {
        let p = c as f64 / len;
        acc - p * p.log2()
    })
}

// ── Field collector ───────────────────────────────────────────────────────────

#[derive(Default)]
struct CollectedFields {
    fields: Vec<(String, String)>,
    skip_field: Option<String>,
    skip_reason: Option<String>,
}

struct CollectingVisitor<'f> {
    state: &'f mut CollectedFields,
}

impl tracing::field::Visit for CollectingVisitor<'_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "__redact_skip" => {
                self.state.skip_field = Some(value.to_string());
            }
            "__redact_reason" => {
                self.state.skip_reason = Some(value.to_string());
            }
            _ => {
                self.state
                    .fields
                    .push((field.name().to_string(), value.to_string()));
            }
        }
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        match field.name() {
            "__redact_skip" => {
                self.state.skip_field = Some(format!("{value:?}").trim_matches('"').to_string());
            }
            "__redact_reason" => {
                self.state.skip_reason = Some(format!("{value:?}").trim_matches('"').to_string());
            }
            _ => {
                self.state
                    .fields
                    .push((field.name().to_string(), format!("{value:?}")));
            }
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.state
            .fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.state
            .fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.state
            .fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.state
            .fields
            .push((field.name().to_string(), value.to_string()));
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        self.state
            .fields
            .push((field.name().to_string(), value.to_string()));
    }
}

// ── Layer impl ────────────────────────────────────────────────────────────────

impl<S> Layer<S> for RedactLayer
where
    S: Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        // Reentrancy guard: prevent recursive on_event processing.
        if REDACTING.with(|r| r.get()) {
            return;
        }
        REDACTING.with(|r| r.set(true));

        // Collect all fields.
        let mut collected = CollectedFields::default();
        event.record(&mut CollectingVisitor {
            state: &mut collected,
        });

        // Resolve skip set (D-RED-4).
        let skip_field: Option<&str> = match (&collected.skip_field, &collected.skip_reason) {
            (Some(f), Some(r)) if !r.is_empty() => Some(f.as_str()),
            (Some(f), _) => {
                // Missing reason → no skip; write WARN to stderr.
                // (tracing::warn! inside on_event is dropped by tracing's reentrancy guard.)
                emit_meta_to_stderr("missing_reason", f, "", 0, self.mode, self.verbose);
                None
            }
            _ => None,
        };

        // Apply rule set to each field.
        let mut redact_map: HashMap<String, String> = HashMap::new();
        let mut redaction_list: Vec<(String, String)> = Vec::new();

        for (name, value) in &collected.fields {
            if skip_field == Some(name.as_str()) {
                continue;
            }
            let (redacted, rule) = apply_redaction_for_field(value, name);
            if let Cow::Owned(rv) = redacted {
                redact_map.insert(name.clone(), rv);
                redaction_list.push((name.clone(), rule.to_string()));
            }
        }

        // Emit WARN meta-events (D-RED-5) — write directly to stderr because
        // tracing::warn!() from inside on_event is dropped by tracing-core's
        // reentrancy guard (confirmed by debug testing). eprintln! to stderr is
        // always visible to the operator and safe from reentrancy.
        if self.mode == RedactMode::Warn || self.verbose {
            for (field_name, rule_key) in &redaction_list {
                let count = REDACTION_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
                emit_meta_to_stderr(
                    "redacted",
                    field_name,
                    rule_key,
                    count,
                    self.mode,
                    self.verbose,
                );
            }
        } else {
            REDACTION_COUNTER.fetch_add(redaction_list.len() as u32, Ordering::Relaxed);
        }

        // Store redacted fields in thread-local for the downstream formatter.
        if !redact_map.is_empty() {
            REDACTED_FIELDS.with(|m| *m.borrow_mut() = redact_map);
        }

        REDACTING.with(|r| r.set(false));
    }
}

/// Write a meta-event directly to stderr AND record it in the test-only
/// `META_EVENTS` thread-local (D-RED-5 WARN-mode side channel).
///
/// `tracing::warn!` cannot be called from inside `on_event` — tracing-core
/// drops recursive dispatches. `eprintln!` to stderr bypasses the reentrancy
/// guard and is greppable by the operator for `llm::redact_layer::meta`.
///
/// The `META_EVENTS` thread-local records (field_name, rule_key) pairs so
/// integration tests can drain them via [`take_meta_events`] without
/// capturing stderr.
fn emit_meta_to_stderr(
    kind: &str,
    field_name: &str,
    rule_key: &str,
    count: u32,
    mode: RedactMode,
    verbose: bool,
) {
    if mode != RedactMode::Warn && !verbose {
        return;
    }
    // Record in test-only thread-local (always, regardless of mode gate).
    META_EVENTS.with(|m| {
        m.borrow_mut()
            .push((field_name.to_string(), kind.to_string()));
    });
    if kind == "missing_reason" {
        eprintln!(
            "WARN llm::redact_layer::meta: __redact_skip present but __redact_reason \
             missing or empty for field={field_name}; skip NOT applied (fail-safe-closed)"
        );
    } else {
        eprintln!(
            "WARN llm::redact_layer::meta: redacted field matched rule \
             field_name={field_name} rule={rule_key} count_so_far={count}"
        );
    }
}

// ── RedactingFormatFields ─────────────────────────────────────────────────────

/// A `FormatFields` implementation that substitutes redacted values from the
/// thread-local `REDACTED_FIELDS` map during event field formatting.
///
/// Used by [`crate::tracing_init::install_global`] as the `N` type parameter
/// for `fmt::Layer`. When a field has a redacted override in the thread-local
/// map, the override is written instead of the original value.
pub struct RedactingFormatFields;

impl<'writer> tracing_subscriber::fmt::FormatFields<'writer> for RedactingFormatFields {
    fn format_fields<R: tracing_subscriber::field::RecordFields>(
        &self,
        writer: tracing_subscriber::fmt::format::Writer<'writer>,
        fields: R,
    ) -> std::fmt::Result {
        // Peek at the thread-local redaction map (don't consume it — multiple
        // calls may happen per event for different format layers).
        let overrides = peek_redacted_fields();
        if overrides.is_empty() {
            // Fast path: no redactions; use the default Pretty/Compact formatter.
            tracing_subscriber::fmt::format::DefaultFields::new().format_fields(writer, fields)
        } else {
            // Slow path: write fields with overrides applied.
            let mut visitor = RedactingFieldWriter {
                writer,
                overrides: &overrides,
                first: true,
            };
            fields.record(&mut visitor);
            Ok(())
        }
    }
}

struct RedactingFieldWriter<'w, 'o> {
    writer: tracing_subscriber::fmt::format::Writer<'w>,
    overrides: &'o HashMap<String, String>,
    first: bool,
}

impl tracing::field::Visit for RedactingFieldWriter<'_, '_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        let name = field.name();
        // Skip internal marker fields.
        if name.starts_with("__redact_") {
            return;
        }
        let effective = self
            .overrides
            .get(name)
            .map(|s| s.as_str())
            .unwrap_or(value);
        self.write_field(name, effective);
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let name = field.name();
        if name.starts_with("__redact_") {
            return;
        }
        if let Some(override_val) = self.overrides.get(name) {
            self.write_field(name, override_val.as_str());
        } else {
            let rendered = format!("{value:?}");
            self.write_field(name, &rendered);
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        if field.name().starts_with("__redact_") {
            return;
        }
        self.write_field(field.name(), &value.to_string());
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        if field.name().starts_with("__redact_") {
            return;
        }
        self.write_field(field.name(), &value.to_string());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        if field.name().starts_with("__redact_") {
            return;
        }
        self.write_field(field.name(), &value.to_string());
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        if field.name().starts_with("__redact_") {
            return;
        }
        self.write_field(field.name(), &value.to_string());
    }

    fn record_error(
        &mut self,
        field: &tracing::field::Field,
        value: &(dyn std::error::Error + 'static),
    ) {
        if field.name().starts_with("__redact_") {
            return;
        }
        self.write_field(field.name(), &value.to_string());
    }
}

impl RedactingFieldWriter<'_, '_> {
    fn write_field(&mut self, name: &str, value: &str) {
        if !self.first {
            let _ = std::fmt::Write::write_str(&mut self.writer, " ");
        }
        self.first = false;
        let _ = std::fmt::Write::write_fmt(&mut self.writer, format_args!("{name}={value}"));
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Rule unit tests per D-RED-1 / R4.1 ───────────────────────────────────

    #[test]
    fn anthropic_key_redacted() {
        let key = "sk-ant-api03-ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abc";
        let out = redact_str(key);
        assert!(
            !out.contains("ABCDEFGHIJKLMNOP"),
            "anthropic key not redacted: {out}"
        );
        assert!(
            out.contains("***"),
            "missing *** in anthropic key redaction: {out}"
        );
    }

    #[test]
    fn openai_proj_key_redacted() {
        let key = "sk-proj-AbCdEfGhIjKlMnOpQrStUvWxYz0123456789AB";
        let out = redact_str(key);
        assert!(
            !out.contains("AbCdEfGhIjKlMnOpQrS"),
            "openai proj key not redacted: {out}"
        );
        assert!(out.contains("***"), "missing *** in openai proj key: {out}");
    }

    #[test]
    fn openai_key_redacted() {
        let key = "sk-somegenerickey0123456789abcdef0123456789";
        let out = redact_str(key);
        assert!(
            !out.contains("somegenerickey012"),
            "openai key not redacted: {out}"
        );
        assert!(out.contains("***"), "missing *** in openai key: {out}");
    }

    #[test]
    fn bearer_token_redacted() {
        let s = "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9abc123456789";
        let out = redact_str(s);
        assert!(out.starts_with("Bearer "), "bearer prefix lost: {out}");
        assert!(
            !out.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9abc123"),
            "bearer not redacted: {out}"
        );
        assert!(out.contains("***"), "missing *** in bearer: {out}");
    }

    #[test]
    fn jwt_redacted() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9\
                   .eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ\
                   .SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let out = redact_str(jwt);
        assert!(out.contains("***"), "JWT not redacted: {out}");
        assert!(
            !out.contains("eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI"),
            "JWT payload not redacted: {out}"
        );
    }

    #[test]
    fn aws_access_key_redacted() {
        let key = "AKIAIOSFODNN7EXAMPLE";
        let out = redact_str(key);
        assert!(out.contains("***"), "AWS access key not redacted: {out}");
        assert!(
            !out.contains("IOSFODNN7EXAMPL"),
            "AWS access key leaks: {out}"
        );
    }

    #[test]
    fn aws_secret_context_redacted_with_matching_field_name() {
        let value = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let (out, rule) = apply_redaction_for_field(value, "secret_key");
        assert!(
            out.contains("***"),
            "aws_secret_context not redacted: {out}"
        );
        assert_eq!(rule, RULE_AWS_SECRET_CONTEXT);
    }

    #[test]
    fn aws_secret_context_not_redacted_without_matching_field_name() {
        let value = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
        let (out, _rule) = apply_redaction_for_field(value, "plain_value");
        assert!(
            !out.contains("***"),
            "aws_secret_context fired on non-context field: {out}"
        );
    }

    #[test]
    fn password_field_name_redacted() {
        let (out, rule) = apply_redaction_for_field("hunter2", "password");
        assert!(out.contains("***"), "password field not redacted: {out}");
        assert_eq!(rule, RULE_PASSWORD_FIELD_NAME);
    }

    #[test]
    fn api_key_field_name_redacted() {
        let (out, rule) = apply_redaction_for_field("somevalue12345678", "api_key");
        assert!(out.contains("***"), "api_key field not redacted: {out}");
        assert_eq!(rule, RULE_PASSWORD_FIELD_NAME);
    }

    #[test]
    fn entropy_fallback_redacted_for_key_field() {
        // "session_token_value" contains "token" → entropy_fallback context.
        // NOT in PASSWORD_FIELD_NAMES (not an exact match).
        let high_entropy = "aB3#kP9@mZ2$nQ7&rX4!sY1*tW6^vU0_wV5"; // 36 chars
        let (out, rule) = apply_redaction_for_field(high_entropy, "session_token_value");
        assert!(
            out.contains("***"),
            "entropy_fallback did not fire for session_token_value: {out}"
        );
        assert_eq!(rule, RULE_ENTROPY_FALLBACK);
    }

    #[test]
    fn entropy_fallback_not_redacted_for_non_key_field() {
        let high_entropy = "aB3#kP9@mZ2$nQ7&rX4!sY1*tW6^vU0_wV5";
        let (out, _rule) = apply_redaction_for_field(high_entropy, "description");
        assert!(
            !out.contains("***"),
            "entropy_fallback incorrectly fired on non-key field: {out}"
        );
    }

    // ── Negative tests ────────────────────────────────────────────────────────

    #[test]
    fn plain_prose_not_redacted() {
        let s = "The quick brown fox jumps over the lazy dog";
        assert_eq!(redact_str(s), Cow::Borrowed(s));
    }

    #[test]
    fn short_string_not_redacted() {
        assert_eq!(redact_str("hello"), Cow::Borrowed("hello"));
    }

    #[test]
    fn numeric_field_not_redacted() {
        assert_eq!(redact_str("123456789"), Cow::Borrowed("123456789"));
    }

    // ── Pure-fn parity self-test per R4.4 ────────────────────────────────────

    #[test]
    fn t1915_parity_anthropic_key() {
        let key = "sk-ant-secret-12345";
        let expected = redact(key);
        let actual = redact_str(key);
        assert_eq!(
            actual.as_ref(),
            expected.as_str(),
            "parity failure for anthropic key"
        );
    }

    #[test]
    fn t1915_parity_below_threshold() {
        let key = "short";
        let actual = redact_str(key);
        assert_eq!(
            actual.as_ref(),
            key,
            "below-threshold should not be redacted: {actual}"
        );
    }

    #[test]
    fn t1915_parity_openai_proj() {
        let key = "sk-proj-AbCdEf1234567890XXXXXXXXXXXX";
        let expected = redact(key);
        let actual = redact_str(key);
        assert_eq!(
            actual.as_ref(),
            expected.as_str(),
            "parity failure for openai proj key"
        );
    }

    #[test]
    fn t1915_parity_bare_key_no_field_name() {
        // 32-char hex with no field name → no pattern match, no field-name-context.
        let key = "abcdef0123456789abcdef0123456789";
        let actual = redact_str(key);
        assert_eq!(
            actual.as_ref(),
            key,
            "bare key with no field name should not be redacted: {actual}"
        );
    }

    // ── Shannon entropy ───────────────────────────────────────────────────────

    #[test]
    fn entropy_low_for_uniform() {
        let e = shannon_entropy("aaaaaaaaaaaaaaaa");
        assert!(e < 0.001, "uniform string has non-zero entropy: {e}");
    }

    #[test]
    fn entropy_high_for_mixed() {
        let e = shannon_entropy("aB3#kP9@mZ2$nQ7&rX4!sY1*tW6^vU0_wV5");
        assert!(e >= ENTROPY_THRESHOLD, "mixed string entropy too low: {e}");
    }

    // ── env-var mode parse ────────────────────────────────────────────────────

    #[test]
    fn from_env_warn_mode_is_default() {
        let layer = RedactLayer::warn_mode();
        assert_eq!(layer.mode, RedactMode::Warn);
    }

    #[test]
    fn from_env_gate_mode_constructor() {
        let layer = RedactLayer::gate_mode();
        assert_eq!(layer.mode, RedactMode::Gate);
    }

    // ── Thread-local state tests ──────────────────────────────────────────────

    #[test]
    fn redacted_fields_thread_local_stores_and_peeks() {
        // Manually populate the thread-local.
        REDACTED_FIELDS.with(|m| {
            let mut g = m.borrow_mut();
            g.insert("password".to_string(), "***".to_string());
        });
        let peek = peek_redacted_fields();
        assert!(
            peek.contains_key("password"),
            "thread-local not populated: {peek:?}"
        );
        // take_redacted_fields clears it.
        let taken = take_redacted_fields();
        assert!(
            taken.contains_key("password"),
            "take_redacted_fields returned wrong: {taken:?}"
        );
        let after_take = peek_redacted_fields();
        assert!(
            after_take.is_empty(),
            "thread-local not cleared after take: {after_take:?}"
        );
    }

    // ── Falsification probe P-RED-3 (D-RED-9): #[ignore] ────────────────────
    //
    // Run via: `cargo test -p llm -- --ignored p_red_3`
    //
    // To execute the probe:
    // 1. Modify `apply_redaction_for_field` to always return `Cow::Borrowed`.
    // 2. `cargo test -p llm p_red_3_rule_set_load_bearing -- --ignored`.
    // 3. Observe: the assertion changes from "*** present" to "no *** = rules empty".
    // 4. Revert.
    #[ignore]
    #[test]
    fn p_red_3_rule_set_load_bearing() {
        let key = "sk-ant-api03-ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abc";
        let out = redact_str(key);
        assert!(
            out.contains("***"),
            "PROBE P-RED-3: rule set is load-bearing (*** present = rules active). \
             Empty the rule set and re-run: this assertion will fail (no ***), \
             proving rules are required."
        );
    }
}
