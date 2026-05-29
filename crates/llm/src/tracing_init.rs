//! Workspace-wide tracing-subscriber installer.
//!
//! Every binary that previously called `tracing_subscriber::fmt().init()`
//! now calls [`install_global`] instead. This centralises the redactor +
//! fmt Layer ordering (R1.4 layer-order contract) at a single audit point:
//!
//! ```text
//! registry()
//!   .with(EnvFilter)             ← filter first
//!   .with(RedactLayer)           ← populate thread-local BEFORE fmt renders
//!   .with(fmt::Layer<RedactingFormatFields>)  ← reads REDACTED_FIELDS from thread-local
//! ```
//!
//! The `RedactLayer` runs BEFORE `fmt::Layer` in the chain. The `fmt::Layer`
//! uses [`crate::redact_layer::RedactingFormatFields`] as its `FormatFields`
//! type, which checks the thread-local redaction map populated by `RedactLayer`
//! and substitutes redacted values during field formatting.
//!
//! This two-step approach (populate thread-local in Layer → read in FormatFields)
//! is necessary because tracing's reentrancy guard prevents `on_event` from
//! dispatching new events (the re-emit + filter-original pattern doesn't work).
//!
//! # Layer ordering note (D-RED-2 / K3)
//!
//! In tracing-subscriber 0.3.x, `.with(A).with(B)` means B's `on_event` is
//! called FIRST, then A's. So `.with(RedactLayer).with(fmt_layer)` means
//! `fmt_layer.on_event` runs FIRST, then `RedactLayer.on_event`. But `fmt_layer`
//! calls its `FormatFields` during `on_event`, at which point `REDACTED_FIELDS`
//! is still empty (RedactLayer hasn't run yet). This means we need the OPPOSITE
//! registration order to ensure RedactLayer runs first:
//! `.with(fmt_layer).with(RedactLayer)` → RedactLayer runs first, populates
//! thread-local, then fmt_layer's `on_event` fires. BUT fmt_layer's `on_event`
//! already ran before this...
//!
//! **Actual resolution**: In tracing-subscriber 0.3.x, the `on_event` call
//! order is the SAME as registration order (first registered = first called).
//! So `.with(RedactLayer).with(fmt_layer)` calls RedactLayer.on_event first,
//! then fmt_layer.on_event. This is correct. The `RedactingFormatFields`
//! reads REDACTED_FIELDS which was populated by RedactLayer.on_event.
//!
//! # Usage
//!
//! ```no_compile
//! // before
//! tracing_subscriber::fmt()
//!     .with_env_filter(...)
//!     .json()
//!     .init();
//!
//! // after
//! llm::tracing_init::install_global(&["trading=info", "agent=info"], true)?;
//! ```

use tracing_subscriber::{EnvFilter, Layer, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::redact_layer::{RedactLayer, RedactingFormatFields};

/// Install the workspace-standard tracing subscriber globally.
///
/// Composition (in order, R1.4):
/// 1. `RedactLayer` — runs first, populates `REDACTED_FIELDS` thread-local.
/// 2. `fmt::Layer<RedactingFormatFields>` — reads `REDACTED_FIELDS` during
///    field formatting, substituting redacted values before writing to stderr.
/// 3. `EnvFilter` — applied as a global filter wrapping the registry.
///
/// # Errors
///
/// Returns `Err` if a global subscriber is already installed.
/// Use `let _ = install_global(...)` to ignore this in test contexts.
///
/// # Directives
///
/// `extra_directives` are added on top of `RUST_LOG`.
pub fn install_global(
    extra_directives: &[&str],
    json: bool,
) -> Result<(), tracing_subscriber::util::TryInitError> {
    // Build EnvFilter.
    let mut filter = EnvFilter::from_default_env();
    for &directive in extra_directives {
        match directive.parse() {
            Ok(d) => {
                filter = filter.add_directive(d);
            }
            Err(e) => {
                eprintln!(
                    "redact_layer tracing_init: invalid directive {directive:?}: {e}; skipped"
                );
            }
        }
    }

    // Build the fmt Layer with our custom FormatFields that reads REDACTED_FIELDS.
    let fmt_layer: Box<dyn Layer<_> + Send + Sync> = if json {
        fmt::layer()
            .fmt_fields(RedactingFormatFields)
            .json()
            .with_writer(std::io::stderr)
            .boxed()
    } else {
        fmt::layer()
            .fmt_fields(RedactingFormatFields)
            .with_writer(std::io::stderr)
            .boxed()
    };

    // Wire: RedactLayer runs FIRST (populates thread-local), then fmt_layer
    // (reads thread-local via RedactingFormatFields during field formatting).
    tracing_subscriber::registry()
        .with(filter)
        .with(RedactLayer::from_env())
        .with(fmt_layer)
        .try_init()
}
