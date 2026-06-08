---
slug: cockpit-chart-cache
status: deprecated
owner: ui-designer
updated: 2026-06-08
---

<!-- status=deprecated == "measured Phase 1 → NO-GO, idea consciously
shelved." Not a shipped feature; not an open candidate. `deprecated` is
the allowed spec-lint status that means "decided against" and is exempt
from the tasks.md requirement (a NO-GO has no task list). -->


# Cockpit chart `canvas::Cache` hover-smoothness — Phase 1 MEASURE → NO-GO

> **Verdict: NO-GO. Measured 2026-06-08.** Wiring iced's
> `geometry::Cache` into the 6 chart `canvas::Program::draw` impls
> would save **≤ 0.1 % of a hover frame** (≈ 23 µs on a 17 ms frame).
> The geometry-build cost the cache eliminates is dwarfed by the
> tiny-skia rasterisation cost the cache does **not** touch. The
> stale-chart invalidation risk is not worth a sub-millisecond,
> imperceptible gain. **Phase 2 (implementation) intentionally not
> done.** The measurement harness is kept so the decision is
> reproducible if iced or chart data sizes change.

## Why (the flagged issue)

During the perf fix (commit `e7d6940`, "perf(cockpit): fix 1-3s
interaction lag") the ui-designer flagged that iced's `canvas::Cache` /
`geometry::Cache` is unused in all **6 chart `Canvas` `Program::draw`**
impls in `crates/ui/`:

| Widget | `Program` | Consumer |
| --- | --- | --- |
| `widgets/chart.rs` | `ChartProgram` | cockpit Lab/Charts (heavy) |
| `widgets/volume_histogram.rs` | `HistogramProgram` | cockpit Lab |
| `widgets/position_curve.rs` | `PositionCurveProgram` | cockpit Lab |
| `widgets/equity_curve.rs` | `EquityCurveProgram` | viewer / live / baseline |
| `widgets/drawdown_band.rs` | `DrawdownBandProgram` | viewer / baseline |
| `widgets/sparkline.rs` | `SparklineProgram` | cockpit Strategies |

Each calls `Frame::new()` and rebuilds every `Path` from scratch on
EVERY frame — including on every chart hover (iced fires
`shell.request_redraw()` on each `CursorMoved` that changes hover state
→ full geometry re-rasterisation). The hypothesis: cache the chart
geometry so it rebuilds only when the underlying DATA changes →
smoother cursor sweeps.

The perf-fix author flagged it as **optional future hover polish**, not
a confirmed win — hence Phase 1 measures before any implementation.

## Phase 1 — MEASURE (the go/no-go number)

`canvas::Cache` skips the **geometry-build** cost (the `Path::new` +
`Frame::stroke` / `fill` work inside `draw`) on a cache hit, but NOT
the **rasterisation** cost (tiny-skia still draws the cached geometry
every frame). So on a HOVER frame — a redraw with no data change, the
exact case the cache optimises — the cache's theoretical ceiling is:

```
speedup_ceiling = build_ns / frame_ns
```

### Method (honest, exact — no renderer reconstruction)

`widgets/chart_build_probe.rs` (feature `chart-build-probe`, zero
production cost — no-op ZST guard when off) brackets every chart `draw`
body with a thread-local nanosecond timer. The Phase-1 bench
`benches/chart_build_probe.rs` drives ONE `Emulator::screenshot` — the
FULL production hover frame (`view → update(RedrawRequested) → draw →
tiny-skia readback`, `iced_test::emulator::Emulator::screenshot`) — and
records both:

1. `chart_build_probe::accumulated()` — total time inside every chart
   `draw`'s build body == the work a cache hit would skip, and
2. the whole screenshot wall-time == the full hover frame.

This times the **exact** code the cache skips through the **exact**
production rasteriser. The Lab/Charts route at 1920×1080 instantiates
THREE chart canvases (price chart + 60 bars + 4 markers + axes,
volume histogram, position curve) — the most chart geometry of any
screen, i.e. the case the cache helps MOST.

### Result (release/bench profile, opt-level 3 — the canonical interactive profile)

| Route | Frame median | Geometry-build median | **Build fraction (cache ceiling)** |
| --- | ---: | ---: | ---: |
| Lab 1920×1080 | 17.351 ms | 0.023 ms | **0.1 %** |
| Lab 1280×720 (floor) | 12.038 ms | 0.005 ms | **0.0 %** |

The build is non-zero (the probe genuinely captures the chart draws),
but it is **≤ 0.1 %** of a hover frame. Absolute ceiling: ~23 µs saved
on a 17 ms frame.

### Why the build is so cheap

`Path::new(|b| …)` just pushes line segments into a lyon path buffer;
a 60-point polyline builds in microseconds even at opt-3. The expensive
part is tessellation + tiny-skia rasterisation to RGBA at 1920×1080 =
~2.1 M pixels, which happens in `renderer.screenshot()` — the part the
cache does **not** eliminate. The 17 ms frame is ~99.9 % raster + view
+ readback, ~0.1 % geometry build.

## Decision — NO-GO (Phase 2 not done)

The brief's own gate: *"If the geometry-build is a small fraction
(cache saves < ~20 % of the hover frame), STOP and report that
honestly."* Measured 0.1 % ≪ 20 %.

Implementing Phase 2 would mean:

- Threading a persistent `geometry::Cache` from `Cockpit` / viewer
  `Model` state down through ~10 `screens/*.rs` `view()` call sites into
  6 widget `view()` signatures and 6 `Program` structs (the programs are
  currently rebuilt fresh per `view()` with `series.clone()`, so the
  `Cache` cannot live in the `Program` — it must live in persistent app
  state and be borrowed in).
- Owning **cache invalidation** as a load-bearing correctness risk: a
  stale chart (cache not cleared on a data / theme / bounds change) is a
  bug worse than slowness. `geometry::Cache::draw` only auto-invalidates
  on a SIZE change; data and theme changes must be explicitly
  `cache.clear()`-ed.

That is real, invasive, risk-bearing work for an **imperceptible**
(sub-millisecond, < 0.1 %) gain. "Now it's fast" (16-31 ms → already
under the 100 ms instant bar per `e7d6940`) is already good enough. The
durable choice is to NOT ship it.

If a future hover-smoothness complaint arises, the lever is the
**raster** cost (e.g. wgpu GPU rasterisation), NOT the geometry cache —
the perf-fix commit already noted wgpu escalation was not needed for
the 24-40× dev→release win, and this measurement confirms the geometry
cache is not the lever either.

## What landed (files)

This is a measurement, not a feature ship. The kept artefacts:

- **`crates/ui/src/widgets/chart_build_probe.rs`** (NEW) — thread-local
  geometry-build timer + RAII guard. Gated behind the `chart-build-probe`
  cargo feature; compiles to a zero-sized no-op guard (empty `Drop`)
  when off → **zero production cost** (verified: feature-off lib build
  clean, `cargo test -p ui --lib` 428 green).
- **`crates/ui/benches/chart_build_probe.rs`** (NEW) — Phase-1
  build-vs-raster split bench. `required-features = ["chart-build-probe"]`
  so a plain `cargo bench -p ui` does not select it.
- **6 chart `draw` impls** — one unconditional line each
  (`let _build_timer = super::chart_build_probe::BuildTimer::start();`)
  at the top of `draw`. No-op when the feature is off; no `#[cfg]`
  clutter at the call sites.
- **`crates/ui/Cargo.toml`** — `chart-build-probe` feature + the
  `[[bench]]` stanza.
- **`crates/ui/src/widgets/mod.rs`** — module registration (`pub` only
  under the feature so the out-of-crate bench can read the accumulator).
- **`crates/ui/src/gallery/mod.rs`** — `chart_build_probe` added to the
  gallery's `EXCLUDED_FROM_GALLERY` list (it is a measurement helper,
  not a renderable widget — same bucket as `debug_renderer` /
  `throttled_spinner`).

### Reproduce the measurement

```bash
cargo bench -p ui --bench chart_build_probe --features chart-build-probe -- --profile-time 5
# Reads "CHART-BUILD-SPLIT […] build_fraction=0.1% …" to stderr.
```

## Verification

- `cargo test -p ui --lib` — **428 passed, 0 failed** (feature off).
- `tests/panel_snapshots.rs` — **97 passed, 0 failed**, snapshots
  **unchanged in content** (a no-op ZST guard cannot alter geometry).
- `tests/consistency.rs` (2), `tests/contrast.rs` (7),
  `tests/layout_invariants.rs` (11, the sidebar/layout invariant),
  `tests/gallery_snapshots.rs` (2) — all GREEN.
- `scripts/verify_anchors.sh` — **119 / 119** (no anchored file
  touched).
- Feature-off + feature-on lib builds clippy-clean for the new file.

### Flagged — pre-existing, NOT caused by this change

`tests/render_snapshots.rs` and `tests/visual_snapshots.rs` (full-frame
PNG pixel-diff baselines) are **broadly red on this machine** (48 / 51
visual + 8 render failures, across charts AND chart-free screens like
`memory__cold_boot_empty`, `models__steady_state`). Proven pre-existing:
stashing all of this change's edits (probe module not even compiled)
reproduces the **identical** failure set. This is environmental
visual-baseline drift (tiny-skia / font-renderer / platform delta from
where the baselines were committed), independent of the chart cache
work. Surfaced to the orchestrator as a separate spec-auditor item —
the committed visual baselines need re-grenerating on the current
toolchain or the suite needs a tolerance/quarantine policy.

## Changelog

- 2026-06-08 (ui-designer): Phase 1 MEASURE → **NO-GO**. Build/raster
  split on the heavy Lab hover frame is 0.1 % / 99.9 %; cache ceiling
  ≤ 23 µs on a 17 ms frame. Phase 2 (Cache plumbing + invalidation) not
  done — invasive, risk-bearing work for an imperceptible gain.
  Measurement harness (`chart_build_probe` module + bench, feature-gated,
  zero production cost) kept for reproducibility. Flagged pre-existing
  visual-baseline drift on the current machine (independent of this
  change).
