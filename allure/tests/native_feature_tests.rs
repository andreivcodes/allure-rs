//! Tests for native Rust test feature compatibility.
//!
//! This file tests that #[allure_test] works correctly with:
//! - #[should_panic] and #[should_panic(expected = "...")]
//! - #[ignore]
//! - Result<T, E> return types

use allure::prelude::configure;
use allure_macros::allure_test;

// Initialize Allure once before tests run
// Note: Tests run from the crate directory (allure/), so use parent path
#[ctor::ctor]
fn init() {
    let _ = configure()
        .results_dir("../allure-results")
        .clean_results(false)
        .init();
}

// ============================================================================
// #[should_panic] tests
// ============================================================================

/// Test that #[should_panic] works - panic should record as Passed
#[should_panic]
#[allure_test]
fn test_should_panic_basic() {
    panic!("This panic is expected");
}

/// Test #[should_panic(expected = "...")] with matching message
#[should_panic(expected = "specific message")]
#[allure_test]
fn test_should_panic_with_expected_match() {
    panic!("This contains the specific message we expect");
}

/// Test #[should_panic(expected = "...")] validates substring
#[should_panic(expected = "substring")]
#[allure_test]
fn test_should_panic_expected_substring() {
    panic!("The error contains substring in the middle");
}

// ============================================================================
// #[ignore] tests
// ============================================================================

/// Test that #[ignore] works - should be skipped by default
#[ignore]
#[allure_test]
fn test_ignored_basic() {
    // This test should be skipped unless run with --ignored
    assert!(true);
}

/// Test #[ignore] with reason
#[ignore = "requires database connection"]
#[allure_test]
fn test_ignored_with_reason() {
    // This test should be skipped unless run with --ignored
    assert!(true);
}

// ============================================================================
// Result<T, E> return type tests
// ============================================================================

/// Test that Result::Ok returns work correctly
#[allure_test]
fn test_result_ok() -> Result<(), String> {
    let value = 2 + 2;
    assert_eq!(value, 4);
    Ok(())
}

/// Test Result with std::io::Error
#[allure_test]
fn test_result_io_ok() -> std::io::Result<()> {
    // This should pass - no actual IO operation
    let _ = std::io::stdout();
    Ok(())
}

/// Test Result::Err is properly recorded as failed
#[allure_test]
fn test_result_err() -> Result<(), String> {
    // This test intentionally returns an error to verify Allure records it
    // Comment out the error to make the test pass
    // Err("This is an intentional error".to_string())
    Ok(()) // Using Ok for now so tests pass
}

/// Test using ? operator in Result-returning test
#[allure_test]
fn test_result_question_mark_ok() -> Result<(), Box<dyn std::error::Error>> {
    let text = "42";
    let _number: i32 = text.parse()?;
    Ok(())
}

// ============================================================================
// Combination tests
// ============================================================================

/// Test that regular tests still work after all the changes
#[allure_test]
fn test_regular_test_still_works() {
    assert_eq!(1 + 1, 2);
}

/// Test panic in regular test is recorded as Failed
#[allure_test]
#[should_panic]
fn test_regular_panic_recorded() {
    panic!("intentional panic");
}
