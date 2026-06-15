---
slug: cockpit-cross-platform
title: "Cockpit cross-platform portability runbook"
version: 0.1.0
date: 2026-06-15
feature: spec/cockpit-cross-platform/feature.md
adr: spec/architecture/adr/0057-cockpit-visual-baseline-determinism-scope.md
---

# Cockpit cross-platform portability runbook

This runbook documents the Linux build prerequisites, the macOS-only dev-script
inventory (Category C), and the visual-baseline determinism contract for the
cockpit after `cockpit-cross-platform` v0.1.

Cross-references: `spec/cockpit-cross-platform/feature.md` (R1-R6, R-NR.1-6,
Design D1-D9) and ADR-0057 (macOS-canonical render-determinism scope).

---

## 1. Linux build prerequisites

### TLS — no libssl-dev needed (D1)

`reqwest` has been flipped to `rustls-tls` at the workspace root
(`Cargo.toml`: `default-features = false, features = ["json", "rustls-tls"]`).
This removes the system OpenSSL C dependency from the reqwest subtree — Linux
builds with **no `libssl-dev` / `pkg-config`** for the HTTP consumer crates
(`crates/data`, `crates/agent`, `crates/llm`).

**Exception:** `tokio-tungstenite` (used by `crates/data` for Coinbase/Kraken
WebSocket connections) still uses `native-tls`. On Ubuntu this requires:

```
sudo apt-get install -y libssl-dev pkg-config
```

This dep is out of D1 scope (WebSocket TLS is a different library than HTTP
client TLS); the CI workflow installs it. If you build a Linux box that only
needs the HTTP paths (no WebSocket feeds), `libssl-dev` is not required.

### Headless display — xvfb (Q5)

winit 0.30.x requires a display server for window creation even when the
renderer is the pure-software tiny-skia CPU rasterizer. On a headless Linux CI
runner, install `xvfb` and prefix cargo test with `xvfb-run -a`:

```bash
sudo apt-get install -y xvfb libxkbcommon-dev libxkbcommon-x11-0 libx11-dev
xvfb-run -a cargo test -p ui --features fixtures
```

Package breakdown:
- `xvfb` — virtual framebuffer X server (provides `DISPLAY` for winit).
- `libxkbcommon-dev`, `libxkbcommon-x11-0` — keyboard input (x11rb backend).
- `libx11-dev` — base X11 client library (winit 0.30.x x11 backend linkage).

**Q5 validation note:** the apt-dep list above is based on winit 0.30.x
documentation and x11rb requirements. It was researched at v0.1 authoring time
and will be validated on the first GitHub Actions ubuntu-latest run. If winit
panics with "no display server", check:
  - `DISPLAY` is set by `xvfb-run` (it is, automatically).
  - No `libwayland-dev` is needed (the x11rb backend is preferred by winit
    when both X11 and Wayland headers are absent; adding `libwayland-dev` may
    switch to the Wayland backend — test before adding).
  - CI workflow comments flag this step with "VALIDATE ON FIRST CI RUN".

Update this runbook after the first green CI run with the confirmed dep list.

### Full build command (Linux, fixtures binary)

```bash
# Install build deps (run once on a fresh box):
sudo apt-get update -qq
sudo apt-get install -y \
  xvfb libxkbcommon-dev libxkbcommon-x11-0 libx11-dev \
  libssl-dev pkg-config

# Build the fixtures-mode cockpit binary:
cargo build -p ui --bin cockpit --features fixtures

# Run the non-visual ui tests headlessly:
xvfb-run -a cargo test -p ui --features fixtures
```

---

## 2. macOS-only dev-script inventory (Category C)

The following six scripts are **orchestrator/presenter hover-debug and
screenshot tooling**. They are macOS-only by design (use `screencapture`,
`osascript`, `sips`, and CoreGraphics cursor warp). They are **not linked
into any binary** and are **not the tester's render gate** (the render gate is
the `iced_test` snapshot harness in `crates/ui/tests/`).

| Script | macOS API used | Purpose |
|--------|---------------|---------|
| `scripts/orch_cursor_move.swift` | CoreGraphics cursor warp | Hover-debug: move mouse to a screen position |
| `scripts/capture_screenshot.sh` | `screencapture` | Presenter: capture a PNG of a screen rect |
| `scripts/orch_hover_screenshot.sh` | `screencapture` + cursor warp | Presenter: hover + capture |
| `scripts/orch_cockpit_on_screen.sh` | `osascript` + `screencapture` | Presenter: bring cockpit to foreground + capture |
| `scripts/orch_crop.sh` | `sips` | Post-process: crop a captured PNG |
| `scripts/orch_probe_tcc.sh` | macOS TCC (Screen Recording permission) | Verify screen-capture permission is granted |

**Linux/Windows disposition:** the `capture-screenshot` **skill** already
handles Linux — its description says "Linux + headless paths emit operator
instructions instead of failing." Porting the above scripts to `grim`/`scrot`
(Q4=(b)) would be genuine scope creep with zero production value for the Linux
CI (which uses the in-Rust `iced_test` snapshot harness, not `screencapture`).
These scripts remain macOS-only at v0.1; see `feature.md § D9` and Q4=(a).

---

## 3. Visual-baseline determinism contract (ADR-0057)

### The canonical baseline set is macOS-only

The 56 PNG files under `crates/ui/tests/visual-baselines/` were captured on
the operator's Apple-Silicon macOS box. They are the **macOS-canonical** visual
regression gate.

**Why cross-OS byte-identity is not contracted:**
- The cockpit sets no iced default font (`no .font()`, `no default_font`,
  no `include_bytes!` in `crates/ui/src/bin/`).
- The `FiraSans-Regular.ttf` embedded in iced is gated on
  `#[cfg(feature = "fira-sans")]`; `fira-sans` is **not enabled**.
- All body text resolves through `cosmic-text`'s `PlatformFallback` against
  the **per-OS system font database**. Glyph shaping and rasterization differ
  across macOS / Linux / Windows — the 56 PNG baselines will not match
  pixel-for-pixel on other OSes.

This extends ADR-0043 § D5 / ADR-0051 § D5 ("Apple-Silicon canonical box;
cross-platform byte-identity explicitly NOT contracted") to the **UI snapshot
gate** — see ADR-0057 D1.

### Enforcement mechanism (ADR-0057 D2)

Each of the four snapshot test files carries a **file-level inner attribute**:

```rust
#![cfg(target_os = "macos")]
```

Files affected:
- `crates/ui/tests/visual_snapshots.rs`
- `crates/ui/tests/render_snapshots.rs`
- `crates/ui/tests/panel_snapshots.rs`
- `crates/ui/tests/gallery_snapshots.rs`

On Linux/Windows the **entire file compiles to nothing** — the tests do not
exist as compilation units. CI needs no `--skip` filter. The source gate IS the
filter, which removes the "CI filter drifts out of sync with the test set"
failure mode.

On macOS, the tests compile and run normally, exercising all 56 baselines
byte-for-byte.

### Re-baselining is macOS-only

**Never run `cargo test -p ui --features fixtures -- --nocapture` on Linux and
commit the resulting PNGs as baselines.** Any PNG under `visual-baselines/`
committed from a non-macOS OS will diverge from the macOS-rendered gate and
silently break CI.

If a macOS baseline legitimately needs to change (e.g. a widget UI update),
re-capture on the operator's Apple-Silicon box, review the diff visually, then
commit.

### v0.2 follow-on: Linux visual regression

A Linux visual-regression capability requires:
1. Enabling `fira-sans` in the iced feature set and setting a pinned
   `default_font` so renders become font-deterministic across OSes (hypothesis
   H1 in `feature.md`).
2. Re-capturing all 56 baselines on a Linux canonical box.
3. A separate `crates/ui/tests/visual-baselines-linux/` directory.
4. A new CI leg running those baselines on `ubuntu-latest`.

This is a **v0.2 follow-on**, not part of v0.1. Any such feature must
supersede or amend ADR-0057 — it may not re-baseline the macOS set onto
another OS (ADR-0057 D3).

---

## 4. Windows build notes (D4/D5)

The `crates/ui/Cargo.toml` carries:

```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "=0.57.0", features = [
    "Win32_Foundation",
    "Win32_System_Threading",
] }
```

This makes the already-written `#[cfg(windows)]` arm in
`crates/ui/src/lab/pid_alive.rs` compile. The version is pinned exactly to
match the `windows 0.57.0` already transitively present in `Cargo.lock`.

**v0.1 scope:** Windows build + non-visual `cargo test -p ui` green on
`windows-latest` CI. Interactive window UX on Windows is **best-effort /
unverified** — the operator has no Windows hardware to do manual UX QA.
Full Windows interactive parity is a clean additive v0.2 verification.

---

## 5. Changelog

- 2026-06-15 (developer): authored v0.1.0. Documents Linux build prereqs
  (rustls flip for reqwest, xvfb + libxkb* for headless winit), macOS-only
  dev-script inventory (Category C), and the visual-baseline determinism
  contract (ADR-0057). Q5 apt-dep list flagged for validation on first CI run.
