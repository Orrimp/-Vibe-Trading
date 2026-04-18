//! Test modules that live inside the crate to avoid the stdlib `core`
//! shadowing issue in external integration tests.
#[cfg(test)]
mod order_tests;
