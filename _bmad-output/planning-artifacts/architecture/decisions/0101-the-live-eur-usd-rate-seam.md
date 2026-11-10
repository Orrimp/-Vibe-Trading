---
adr: 0101
title: The live EUR/USD rate seam lives in the data crate, quotes the ECB, and falls back to config on anything
status: accepted
date: 2026-11-08
supersedes: none
superseded-by: none
---

# ADR-0101: The live EUR/USD rate seam lives in `data`, quotes the ECB, and falls back to config on anything

## Context

ADR-0065 § D6 reserved a v0.3 "live-fetched rate layered on the static value as fallback"
path and promised it its own ADR. The operator answered PRD §13 Q1 on 2026-07-27 with
**BUILD**, which created story `3-19-advisor-eur-live-rate`, whose first task (M-T1) is this
decision: where the seam lives, what it quotes, and what happens when the quote does not
arrive.

The surface it plugs into is already built and narrow. `crates/core/src/fx.rs` holds
`FxRate { rate: Decimal, source, as_of }` with a checked `FxRate::new` (rejects ≤ 0) and an
infallible `FxRate::config`; `BudgetConversion::new` performs the one EUR→USDT multiply and
hands both the engine and the display the same value; `FxNote` carries the provenance labels
into the "€200 ≈ $X (at R EUR/USD, ⟨source/as-of⟩)" line. The live path changes **which**
`FxRate` reaches the two seams at `crates/ui/src/bin/cockpit_live.rs:1583` and `:1900`.
Nothing downstream of `BudgetConversion` changes.

## Decision

**D1 — The trait and the HTTP implementation live in `crates/data`, not in `core` and not in
`agent`.** A new `crates/data/src/fx_rate.rs` defines `RateSource` (async, `Send + Sync`, the
`KlineFetcher` / `CoinbaseKlineFetcher` shape from ADR-0061) plus an HTTP implementation and a
`#[cfg(any(test, feature = "fixtures"))]` fake. Every external HTTP fetcher in this repo is
already there behind exactly that seam, and `data` already depends on `trading_core`, so the
fetcher can return a *validated* `FxRate`. `core` is the pure money crate — it has no
`reqwest`, no `tokio`, no `async-trait` today, and the rate path is the last place to start
adding a network runtime to it (AD-9). `agent` would compile, but it would put an FX quote
fetcher in the orchestration crate, away from the other six fetchers, for no reason.

ADR-0065 rejected a `ui → data` edge **for the static value** — a config scalar has no
business arriving from the data crate — which does not bear on a network fetcher. That edge
already exists and is optional (`crates/ui/Cargo.toml:82`, pulled in by `yahoo` and
`binance`, both in ui's `default`).

**D2 — The ui side is gated by a feature of its own, `live-fx = ["dep:data"]`, in `default`.**
`dep:data` creates no implicit `data` feature, so the live path needs a real `cfg` to hide
behind, and CI's `cargo check -p ui --no-default-features --features fixtures` (ci.yml:404)
must keep compiling. Gating on `yahoo` or `binance` would tie the rate to whichever data
source happens to be on. With `live-fx` off, the static path is the whole path, which is the
same state the binary shipped in before this story.

**D3 — It quotes the ECB daily reference rate via Frankfurter, at
`https://api.frankfurter.dev/v1/latest?from=EUR&to=USD`, with provenance `"live:ecb"`.**
Measured 2026-11-08: that returns `{"amount":1.0,"base":"EUR","date":"2026-11-06",
"rates":{"USD":1.1165}}`. Keyless, so no secret enters the repo or the environment (AD-19);
authoritative, which is what an *honesty* label needs; and its `date` field is already in the
`YYYY-MM-DD` shape `FxRate::as_of` carries, so the as-of label is the provider's own, not
ours.

Pin the `.dev` host. `api.frankfurter.app` — the host every older document names — now
answers **301** to `api.frankfurter.dev`; pinning the redirect target means the client needs
no redirect-following and a future reader does not "simplify" the URL back to a 301.

**D4 — "Live" means the latest ECB working-day reference rate, and the UI must not imply
otherwise.** The ECB publishes once per working day around 16:00 CET. Today's own response is
the proof: asked on Sunday 2026-11-08 it returns `date: 2026-11-06`. So the label can
legitimately be one to three days old, and that is exactly why it ships *with* an as-of
rather than as a bare number. No copy anywhere may render this as "now", "current" or a
clock-stamped value.

**D5 — Any failure is the static path, silently and immediately.** Offline, DNS failure,
timeout, non-2xx, unparseable body, a missing `USD` key, or a non-positive rate all resolve
to the existing `AdvisorConfig.eur_usd_rate` → `DEFAULT_EUR_USD_RATE` with its `"config"`
provenance. One attempt, one short timeout (**2 s**), no retry: a retry doubles the stall for
a display label. The static rate is a permanent fallback, not a migration step.

**D6 — The fetched number never touches `f64`.** JSON numbers deserialize through
`rust_decimal`'s serde integration (or as a string token), never `serde_json::Value::as_f64`,
and the parsed `Decimal` enters only `FxRate::new`. This is the one place in the story where
AD-9 can be violated by accident and a test would still pass: `1.1165_f64` round-trips
convincingly and is still not the number. The grep guard in
`crates/core/tests/eur_fx_conversion_applied.rs` continues to pin
`convert_eur_to_usdt` as the single multiply.

**D7 — Fetch at plan render and on explicit operator refresh only.** No fetch at boot, no
background poll, no subscription recipe (story AC2). A dead network can cost at most one 2 s
timeout, once, on a render the operator asked for.

## Alternatives considered

- **Trait in `crates/core`** — rejected: adds `async-trait`/`reqwest`/`tokio` to the pure
  money crate to serve a display label.
- **Fetcher in `crates/agent`** — rejected: works, but separates this fetcher from the six
  that already live in `data` and share its mock discipline.
- **A crypto exchange's EURUSDT ticker** — rejected: that is a stablecoin quote, not a fiat
  reference rate. Labelling it "EUR/USD" in an honesty line would be the dishonesty this
  product exists to avoid.
- **`exchangerate.host`** — rejected: now key-gated, and a key is a secret (AD-19).
- **`open.er-api.com`** — viable keyless fallback (verified the same day: `USD = 1.116337`,
  within 2 bp of the ECB figure, which is a useful cross-check) but rejected as the primary:
  an unattributed aggregate with a `time_last_update_utc` stamp rather than an authoritative
  publication date.
- **Caching the live rate to disk** — rejected: a stale cache is a third provenance state to
  explain, and the static config value already is the durable fallback.
- **Retry with backoff** — rejected: doubles the worst-case stall on a label the static path
  can already supply.

## Consequences

The anchored corpus is untouched **by construction**: the headless CLI path is
USDT-denominated and never reads the rate, so no anchored scenario, no anchors delta, no
REVISION change — `verify_anchors.sh` must print 119/119 before and after, and if it does
not, the live path has leaked into the engine. `as_of` remains a label and not a clock read,
preserving ADR-0065's determinism property.

Enforced by: the fake `RateSource` under `data`'s `fixtures` feature (ui already dev-depends
on `data` with `features = ["fixtures"]`, `crates/ui/Cargo.toml:218`), exercising
live-success provenance, forced-failure fallback to `"config"`, and display==engine on the
live rate from one `BudgetConversion`; a rendered-pixel proof for the label change (AD-10);
`crates/core/tests/eur_fx_conversion_applied.rs` for the single-multiply invariant. No test
may open a socket.

If D5 is violated the advisor becomes network-dependent, which is the one property the
operator's answer explicitly ruled out. If D6 is violated money math has an `f64` in it and
the violation is invisible at small rates.

## Changelog
- 2026-11-08 (architect): initial accept. Story `3-19` M-T1. Endpoints and the `.app`→`.dev`
  301 verified by measurement the same day.
