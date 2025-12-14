//! UI tests for compile-time error messages.
//!
//! These tests verify that the macros produce helpful error messages
//! when used incorrectly.
//!
//! Note: Pass tests that require allure-rs are located in the allure crate
//! to avoid circular dependencies during publishing.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
}
