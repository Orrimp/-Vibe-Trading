//! Screen-routed shell bodies (Phase 2 + Phase 3).
//!
//! One module per screen. Each module exposes a `view()` function that
//! takes `&Cockpit + ThemeMode` and returns an `Element<Message>`. The
//! shell (`crate::shell::view`) dispatches on `Cockpit::current_screen`
//! to pick the right body. Phase 2 shipped `home / debug / charts`;
//! Phase 3 lands `strategies / risk / audit`.

pub mod audit;
pub mod charts;
pub mod control;
pub mod debug;
pub mod home;
pub mod risk;
pub mod strategies;
