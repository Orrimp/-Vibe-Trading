//! Self-test integration harness for `fixtures::visual_fail_html`.
//!
//! Exposes the `#[cfg(test)]` module inside `visual_fail_html.rs` as a
//! named cargo integration-test target (`--test visual_fail_html_self_test`).
//! Each test is a thin re-export of the inner `#[test]` functions, exercised
//! under `tempfile::TempDir` so no state escapes the workspace.
//!
//! Run with:
//!   cargo test -p ui --test visual_fail_html_self_test \
//!     --no-default-features --features live

#[path = "fixtures/mod.rs"]
mod fixtures;

// The self-tests live in the `#[cfg(test)] mod tests` block inside
// `visual_fail_html.rs`. Cargo's test compilation picks them up
// automatically when the integration target imports the module.
//
// We do not need additional wrapper functions here — the `#[test]` fns
// inside `fixtures::visual_fail_html::tests` are compiled in because
// Cargo runs integration test targets with `cfg(test)` enabled.
