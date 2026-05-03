//! R8 — What changed (strategy lifecycle events).
//!
//! Filters `strategy_events_since(period_start)` to the four lifecycle
//! kinds (`Load`, `Swap`, `Unload`, `Reject`) and renders a
//! chronological bullet list per R8.2.  Empty period renders the R8.3
//! sentinel literal.

use std::fmt::Write;

use time::format_description::well_known::Rfc3339;
use trading_core::{StrategyEventKind, StrategyEventView};

/// R8 inputs: the unfiltered `strategy_events_since` slice over the
/// period.  The renderer filters to lifecycle kinds.
#[derive(Debug, Clone)]
pub struct WhatChangedInputs {
    /// All strategy events in the report period (oldest first).
    pub events: Vec<StrategyEventView>,
}

/// Render the R8 "what changed" section.
///
/// One bullet per `Load` / `Swap` / `Unload` / `Reject` event.  When
/// no lifecycle events fired, renders the literal R8.3 string.
#[must_use]
pub fn render(inputs: &WhatChangedInputs) -> String {
    let mut out = String::with_capacity(384);
    out.push_str("## What changed\n\n");

    let mut any_lifecycle = false;
    for ev in &inputs.events {
        if !is_lifecycle(&ev.kind) {
            continue;
        }
        any_lifecycle = true;
        let _ = writeln!(out, "{}", format_event(ev));
    }

    if !any_lifecycle {
        out.push_str("_no strategy lifecycle events in this period._\n");
    }
    out
}

fn is_lifecycle(k: &StrategyEventKind) -> bool {
    matches!(
        k,
        StrategyEventKind::Load
            | StrategyEventKind::Swap
            | StrategyEventKind::Unload
            | StrategyEventKind::Reject,
    )
}

fn format_event(ev: &StrategyEventView) -> String {
    let ts = ev
        .ts
        .inner()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "invalid-ts".to_string());
    let sid = ev
        .strategy_id
        .as_ref()
        .map_or("(none)".to_string(), |s| s.0.to_string());
    let mut extras = String::new();
    match ev.kind {
        StrategyEventKind::Load => {
            if let Some(sp) = &ev.source_path {
                let _ = write!(extras, " source={sp}");
            }
            if let Some(h) = &ev.new_hash {
                let _ = write!(extras, " new_hash={}", short_hash(h));
            }
        }
        StrategyEventKind::Swap => {
            if let Some(h) = &ev.old_hash {
                let _ = write!(extras, " old_hash={}", short_hash(h));
            }
            if let Some(h) = &ev.new_hash {
                let _ = write!(extras, " new_hash={}", short_hash(h));
            }
        }
        StrategyEventKind::Unload => {
            if let Some(h) = &ev.old_hash {
                let _ = write!(extras, " old_hash={}", short_hash(h));
            }
        }
        StrategyEventKind::Reject => {
            if let Some(c) = &ev.error_code {
                let _ = write!(extras, " error_code={c}");
            }
        }
        _ => {}
    }
    format!("- {ts} [{kind}] strategy_id={sid}{extras}", kind = ev.kind)
}

fn short_hash(h: &smol_str::SmolStr) -> String {
    let s = h.as_str();
    if s.len() >= 8 {
        format!("{}..", &s[..6])
    } else {
        s.to_string()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use smol_str::SmolStr;
    use time::OffsetDateTime;
    use trading_core::{StrategyId, Timestamp};

    fn ev(kind: StrategyEventKind, sid: &str, ts: &str) -> StrategyEventView {
        StrategyEventView {
            id: SmolStr::new("550e8400-0000-0000-0000-000000000000"),
            ts: Timestamp::new(OffsetDateTime::parse(ts, &Rfc3339).unwrap()),
            kind,
            strategy_id: Some(StrategyId::new(sid)),
            old_hash: None,
            new_hash: None,
            source_path: None,
            operator: SmolStr::new("system"),
            error_code: None,
            error_summary: None,
        }
    }

    #[test]
    fn t813_what_changed_empty_renders_sentinel() {
        let inp = WhatChangedInputs { events: vec![] };
        let body = render(&inp);
        assert!(body.contains("## What changed"));
        assert!(body.contains("_no strategy lifecycle events in this period._"));
    }

    #[test]
    fn t813_what_changed_load_swap_unload_reject_chronological() {
        let mut load = ev(
            StrategyEventKind::Load,
            "alpha",
            "2026-04-29T00:00:00.000000Z",
        );
        load.source_path = Some(SmolStr::new("config/strategies/alpha.toml"));
        let swap = ev(
            StrategyEventKind::Swap,
            "alpha",
            "2026-04-30T00:00:00.000000Z",
        );

        let inp = WhatChangedInputs {
            events: vec![load, swap],
        };
        let body = render(&inp);
        assert!(body.contains("[Load] strategy_id=alpha"));
        assert!(body.contains("source=config/strategies/alpha.toml"));
        assert!(body.contains("[Swap] strategy_id=alpha"));
        // Chronological: Load before Swap.
        let load_pos = body.find("[Load]").unwrap();
        let swap_pos = body.find("[Swap]").unwrap();
        assert!(load_pos < swap_pos);
    }

    #[test]
    fn t813_what_changed_filters_non_lifecycle_events() {
        // RebalanceRejected and FeedReconnect are NOT lifecycle events.
        let rebal = ev(
            StrategyEventKind::RebalanceRejected,
            "beta",
            "2026-04-29T00:00:00.000000Z",
        );
        let feed = ev(
            StrategyEventKind::FeedReconnect,
            "gamma",
            "2026-04-30T00:00:00.000000Z",
        );
        let inp = WhatChangedInputs {
            events: vec![rebal, feed],
        };
        let body = render(&inp);
        // No lifecycle events → sentinel.
        assert!(body.contains("_no strategy lifecycle events in this period._"));
    }

    #[test]
    fn t813_what_changed_byte_stable_across_runs() {
        let load = ev(
            StrategyEventKind::Load,
            "alpha",
            "2026-04-29T00:00:00.000000Z",
        );
        let inp = WhatChangedInputs { events: vec![load] };
        let a = render(&inp);
        let b = render(&inp);
        assert_eq!(a, b);
    }
}
