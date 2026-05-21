//! Screen-routed shell bodies (Phase 2 + Phase 3 + Phase A).
//!
//! One module per screen. Each module exposes a `view()` function that
//! takes `&Cockpit + ThemeMode` and returns an `Element<Message>`. The
//! shell (`crate::shell::view`) dispatches on `Cockpit::current_screen`
//! to pick the right body. Phase 2 shipped `home / debug / charts`;
//! Phase 3 lands `strategies / risk / audit`; Phase A renames `charts`
//! → `lab` and adds placeholder routes.

pub mod audit;
/// Phase E — Compare matrix screen (ui-rethink-phase-e-compare R1.1-R1.4).
/// Toolbar + matrix body. Replaces the Phase A `placeholder::view` route.
pub mod compare;
pub mod control;
pub mod debug;
pub mod home;
/// Phase A — Lab screen (ex-`charts.rs`, T-D-2 rename). New default route
/// (R1.2). The legacy `Screen::Charts` variant auto-routes to `lab::view`
/// via the shell match arm (deprecated alias for backward compatibility).
pub mod lab;
/// Phase C — Live trading dashboard (ui-rethink-phase-c-sidebar-ia R2.1).
/// Replaces the legacy `home::view` 2×2 grid for the `Screen::Live` route.
/// `Screen::Home` (deprecated) also routes here via the compat shim (R5.2).
pub mod live;
/// Phase F — Memory screen (ui-rethink-phase-f-memory-models-assistant R1.1-R1.4).
/// Toolbar + cards list + optional side-drawer. Replaces the Phase A
/// `placeholder::view` route for `Screen::Memory`.
pub mod memory;
/// Phase F — Models screen (ui-rethink-phase-f-memory-models-assistant R2.1-R2.4).
/// Toolbar (family + status chips) + checkpoint list. Replaces the Phase A
/// `placeholder::view` route for `Screen::Models`.
pub mod models;
pub mod risk;
/// Phase C — Settings rollup (ui-rethink-phase-c-sidebar-ia R4.1).
/// Three-tab chrome wrapping `risk::view`, `control::view`, `debug::view`.
pub mod settings;
pub mod strategies;
/// Phase C — Strategy registry (ui-rethink-phase-c-sidebar-ia R3.1).
/// List-of-cards replacing the legacy `strategies::view` detail panel.
pub mod strategy_registry;
/// Phase D — Trail view screen (ui-rethink-phase-d-trail R2.1-R2.5).
/// List mode delegates verbatim to `screens::audit::view` (R10.1 byte-identity gate).
/// Trail mode renders the upstream node stack + side-drawer.
pub mod trail;
