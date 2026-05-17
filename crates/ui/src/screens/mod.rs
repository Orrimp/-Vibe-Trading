//! Screen-routed shell bodies (Phase 2 + Phase 3 + Phase A).
//!
//! One module per screen. Each module exposes a `view()` function that
//! takes `&Cockpit + ThemeMode` and returns an `Element<Message>`. The
//! shell (`crate::shell::view`) dispatches on `Cockpit::current_screen`
//! to pick the right body. Phase 2 shipped `home / debug / charts`;
//! Phase 3 lands `strategies / risk / audit`; Phase A renames `charts`
//! → `lab` and adds placeholder routes.

pub mod audit;
pub mod control;
pub mod debug;
pub mod home;
/// Phase A — Lab screen (ex-`charts.rs`, T-D-2 rename). New default route
/// (R1.2). The legacy `Screen::Charts` variant auto-routes to `lab::view`
/// via the shell match arm (deprecated alias for backward compatibility).
pub mod lab;
pub mod risk;
pub mod strategies;
