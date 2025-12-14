//! UI tests for macro compilation.
//!
//! These tests verify that macros compile correctly when used properly.
//! Compile-fail tests are in the allure-macros crate.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/pass/*.rs");
}
