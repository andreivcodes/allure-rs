//! UI tests for compile-time error messages.
//!
//! These tests verify that the macros produce helpful error messages
//! when used incorrectly.

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/fail/*.rs");
    t.pass("tests/ui/pass/*.rs");
}
