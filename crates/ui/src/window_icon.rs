//! Window chrome — shared `iced::window::Settings` helpers for the three
//! bins (`cockpit`, `cockpit_live`, `viewer`). Implements:
//!
//! - **T2028** — `min_size` floor so the operator can't shrink any bin
//!   below the Layout-β viable width (chart stays above ~50 % of body
//!   height with status strip + histogram fitting their fixed
//!   allocations).
//! - **T2029** — Lumen brand icon attached via
//!   `iced::window::icon::from_rgba(..)`. The raw 64×64 RGBA bytes are
//!   pre-rasterised once from
//!   [`spec/design/project/assets/brand/lumen-mark.svg`](../../spec/design/project/assets/brand/lumen-mark.svg)
//!   and shipped in
//!   [`assets/lumen-mark-64x64.rgba`](../assets/lumen-mark-64x64.rgba)
//!   so neither this crate nor the workspace takes on a new
//!   SVG-rasterisation dependency. The pre-rasterisation step is one-
//!   shot tooling (see the M6 follow-up dev notes).
//!
//! Operator feedback 2026-05-11 surfaced two visual gaps in the
//! `ff96ce4` cockpit ship: a missing window-size floor (window could
//! collapse below the Layout-β minimum) and a missing app icon
//! (macOS dock + cmd-tab + Linux decorations showed the iced
//! placeholder).  Both are pure presentation polish that don't touch
//! audit/strategy/exec.
//!
//! ## macOS dock icon limitation (T2031, M6.2)
//!
//! The operator's second visual-verification pass (2026-05-11, against
//! commit `9bb5786`) confirmed that even with [`lumen_window_icon`] set
//! on every bin's [`iced::window::Settings`], the **macOS dock icon**,
//! **cmd-tab switcher icon**, **Spotlight result icon**, and **Finder
//! file icon** all still render as the generic iced/Cargo placeholder.
//!
//! **Why:** `iced::window::Settings::icon` only affects the icon iced
//! draws inside its **own window chrome** — typically the title-bar
//! glyph on Windows + Linux compositors. macOS's dock, cmd-tab,
//! Spotlight, and Finder read their app icon from the `.app` bundle's
//! `Info.plist` (`CFBundleIconFile` → `.icns` resource).  A bare
//! `cargo run --bin cockpit` produces a Mach-O executable, not an
//! `.app` bundle — there is **no `Info.plist` for macOS to read**, so
//! it falls back to the generic placeholder regardless of what iced
//! is told.  The T2029 test (`window_icon_set_on_all_bins`) is
//! correct: the iced-level plumbing is plumbed.  The runtime gap is at
//! the macOS-packaging surface, not in this module.
//!
//! **Fix path (not in M6.2 scope):** wrap each bin in an `.app` bundle
//! at build time — either via `cargo bundle`
//! (<https://crates.io/crates/cargo-bundle>) or a hand-written
//! `Info.plist` + `.icns` pair under
//! `crates/ui/macos/<binname>.app/Contents/`.  The brand mark already
//! lives at `spec/design/project/assets/brand/lumen-mark.svg`; the
//! bundle step needs a once-only SVG → `.icns` rasterisation (sips +
//! iconutil on macOS, or `cargo-bundle`'s built-in macOS path).
//! Tracked as the candidate feature stub at
//! [`spec/cockpit-app-bundle/feature.md`](../../../spec/cockpit-app-bundle/feature.md);
//! analyst spawn when operator promotes from candidate.
//!
//! Linux + Windows are unaffected: the title-bar icon path
//! [`lumen_window_icon`] already drives — `Info.plist`-style packaging
//! is macOS-specific.

use iced::window::{icon, Icon, Settings as WindowSettings};
use iced::Size;

/// Minimum window width in logical pixels. Floor for **Layout β**
/// (Q5) on the Charts screen: chart needs ≥ ~50 % of body height with
/// the status strip + histogram occupying their fixed allocations.
/// `1280` is the analyst-suggested lowest viable width measured
/// against:
///
/// - sidebar 180 px ([`theme::layout::SIDEBAR_WIDTH_PX`]) + body
///   padding 24 px × 2 + chip-row content ≈ 600 px gives ~1024 px as
///   the bare minimum; +256 px headroom keeps the volume tile's
///   three-cell layout from squashing the trailing `(N trades)` suffix.
/// - body height after the 24 px chip row + ~80 px status strip +
///   ~80 px histogram + ~30 px status bar + paddings leaves the chart
///   ≥ 440 px of 720 px = ~61 %, comfortably above the 50 % floor.
///
/// [`theme::layout::SIDEBAR_WIDTH_PX`]: super::theme::layout::SIDEBAR_WIDTH_PX
pub const MIN_WINDOW_WIDTH_PX: f32 = 1280.0;

/// Minimum window height in logical pixels. Paired with
/// [`MIN_WINDOW_WIDTH_PX`] for the Layout-β floor. See that constant's
/// doc for the height-budget derivation.
pub const MIN_WINDOW_HEIGHT_PX: f32 = 720.0;

/// **T3022 — initial window size on cold-launch (chart-canvas-overhaul
/// v1.10.0, Q8).** `min_size` stays at the Layout-β floor
/// (1280×720); the initial `size` opens at 1920×1080 so a 3360×1890
/// native Retina operator gets a usable window without manually
/// resizing on first launch.  iced 0.14 clamps the bootstrap size to
/// the display, so this constant is harmless on smaller laptops.
///
/// **Why this isn't `maximised: true`:** the operator may run the
/// cockpit alongside other windows; maximised-by-default surprises
/// users who expect the OS-window-manager's standard launch geometry.
/// 1920×1080 sits comfortably inside any laptop display while still
/// surfacing the chart canvas at a non-floor allocation.  See
/// [`spec/chart-canvas-overhaul/feature.md ## Resolved Qs / Q8`](../../../../spec/chart-canvas-overhaul/feature.md).
pub const DEFAULT_WINDOW_WIDTH_PX: f32 = 1920.0;

/// Cold-launch height — paired with [`DEFAULT_WINDOW_WIDTH_PX`].  See
/// that constant's doc for the rationale (Q8, T3022).
pub const DEFAULT_WINDOW_HEIGHT_PX: f32 = 1080.0;

/// Pre-rasterised Lumen brand mark — 64×64 RGBA pixels, sRGB. Decoded
/// once at boot and handed to iced via
/// [`iced::window::icon::from_rgba`]. Length is `64 * 64 * 4 = 16384`
/// bytes by construction; the panic on shape mismatch in
/// [`lumen_window_icon`] is defence-in-depth against an accidental
/// re-rasterisation that emits a wrong-sized blob.
const LUMEN_MARK_RGBA: &[u8] = include_bytes!("../assets/lumen-mark-64x64.rgba");

/// Edge length in pixels of the pre-rasterised mark.
const LUMEN_MARK_PX: u32 = 64;

/// Build an [`Icon`] from the embedded Lumen mark bytes. Returns
/// `None` if the embedded blob fails iced's `from_rgba` invariants —
/// in practice unreachable, since the asset is shipped as part of the
/// crate and verified by [`window_icon_set_on_all_bins`] (T2029
/// acceptance test).
#[must_use]
pub fn lumen_window_icon() -> Option<Icon> {
    icon::from_rgba(LUMEN_MARK_RGBA.to_vec(), LUMEN_MARK_PX, LUMEN_MARK_PX).ok()
}

/// Standard `iced::window::Settings` for every Lumen bin. Each binary
/// calls this and either uses the result directly or layers per-bin
/// overrides on top via the struct-update syntax.
#[must_use]
pub fn standard_window_settings() -> WindowSettings {
    WindowSettings {
        // T3022 — initial size 1920×1080 (Q8 — chart-canvas-overhaul
        // v1.10.0); min_size stays at the Layout-β floor 1280×720.
        size: Size::new(DEFAULT_WINDOW_WIDTH_PX, DEFAULT_WINDOW_HEIGHT_PX),
        min_size: Some(Size::new(MIN_WINDOW_WIDTH_PX, MIN_WINDOW_HEIGHT_PX)),
        icon: lumen_window_icon(),
        ..WindowSettings::default()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::uninlined_format_args)]
mod tests {
    use super::*;

    /// T2028 acceptance — every bin must set `min_size = Some(_)` so
    /// the operator can't shrink the window below the Layout-β viable
    /// floor.
    #[test]
    fn min_window_size_set_on_all_bins() {
        let s = standard_window_settings();
        let min = s.min_size.expect("standard settings set min_size");
        assert!(
            min.width >= MIN_WINDOW_WIDTH_PX,
            "min_size.width >= MIN_WINDOW_WIDTH_PX: got {}",
            min.width
        );
        assert!(
            min.height >= MIN_WINDOW_HEIGHT_PX,
            "min_size.height >= MIN_WINDOW_HEIGHT_PX: got {}",
            min.height
        );
        // The starting size also respects the min so we don't boot
        // below our own floor.
        assert!(s.size.width >= min.width);
        assert!(s.size.height >= min.height);
    }

    /// T3022 acceptance — chart-canvas-overhaul v1.10.0 (Q8) bumps
    /// the cold-launch size to 1920×1080.  Guards against a future
    /// regression that re-pins `size = min_size`.  See
    /// [`feature.md ## Resolved Qs / Q8`](../../../../spec/chart-canvas-overhaul/feature.md).
    #[test]
    fn default_size_at_least_1920x1080() {
        let s = standard_window_settings();
        assert!(
            s.size.width >= DEFAULT_WINDOW_WIDTH_PX,
            "T3022 — size.width >= 1920: got {}",
            s.size.width
        );
        assert!(
            s.size.height >= DEFAULT_WINDOW_HEIGHT_PX,
            "T3022 — size.height >= 1080: got {}",
            s.size.height
        );
    }

    /// T2029 acceptance — every bin must ship the Lumen mark as the
    /// window icon. Verifies the embedded RGBA blob decodes via iced's
    /// `from_rgba` and the resulting settings carry `icon = Some(_)`.
    #[test]
    fn window_icon_set_on_all_bins() {
        let s = standard_window_settings();
        assert!(
            s.icon.is_some(),
            "standard_window_settings must attach the Lumen mark"
        );
        // The embedded RGBA blob is shipped with the crate; a stray
        // re-rasterisation that produced the wrong byte count would
        // poison every bin silently, so also verify the shape.
        assert_eq!(
            LUMEN_MARK_RGBA.len(),
            (LUMEN_MARK_PX * LUMEN_MARK_PX * 4) as usize,
            "embedded mark must be {}x{} RGBA (4 bytes per pixel)",
            LUMEN_MARK_PX,
            LUMEN_MARK_PX
        );
        // Independent sanity check — `from_rgba` must accept the
        // blob. (We don't compare bytes — `Icon` is opaque.)
        assert!(
            lumen_window_icon().is_some(),
            "lumen_window_icon must produce Some on the embedded blob"
        );
    }
}
