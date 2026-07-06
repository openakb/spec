//! Integration layer: filesystem-backed tests (fixtures, examples, resolver).
#![allow(clippy::unwrap_used, clippy::expect_used)] // tests may panic on failure
#![allow(clippy::print_stderr)] // gating skips report via eprintln

mod conformance_tests;
mod content;
mod examples_tests;
