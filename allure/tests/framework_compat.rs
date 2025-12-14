//! Tests demonstrating compatibility with popular Rust test frameworks.
//!
//! This module verifies that allure macros work correctly with:
//! - rstest for parameterized tests and fixtures
//! - test-case for parameterized tests

use allure::prelude::*;
use rstest::{fixture, rstest};
use tempfile::TempDir;

/// Helper to set up test results directory
fn setup_results_dir() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let _ = allure::configure()
        .results_dir(dir.path().to_str().unwrap())
        .clean_results(true)
        .init();
    dir
}

// =============================================================================
// rstest compatibility tests
// =============================================================================

/// Fixture providing a test user name
#[fixture]
fn user_name() -> String {
    "test_user".to_string()
}

/// Fixture providing a test email
#[fixture]
fn user_email() -> String {
    "test@example.com".to_string()
}

#[rstest]
#[allure_epic("Framework Compatibility")]
#[allure_feature("rstest Integration")]
#[allure_story("Basic rstest fixture")]
#[allure_test]
fn test_rstest_with_fixture(user_name: String) {
    let _dir = setup_results_dir();

    step("Verify fixture value", || {
        assert_eq!(user_name, "test_user");
    });
}

#[rstest]
#[allure_epic("Framework Compatibility")]
#[allure_feature("rstest Integration")]
#[allure_story("Multiple fixtures")]
#[allure_test]
fn test_rstest_with_multiple_fixtures(user_name: String, user_email: String) {
    let _dir = setup_results_dir();

    step("Verify username fixture", || {
        assert_eq!(user_name, "test_user");
    });

    step("Verify email fixture", || {
        assert_eq!(user_email, "test@example.com");
    });
}

#[rstest]
#[case(1, 2, 3)]
#[case(2, 3, 5)]
#[case(10, 20, 30)]
#[allure_epic("Framework Compatibility")]
#[allure_feature("rstest Integration")]
#[allure_story("Parameterized test cases")]
#[allure_test]
fn test_rstest_parameterized(#[case] a: i32, #[case] b: i32, #[case] expected: i32) {
    let _dir = setup_results_dir();

    parameter("a", a);
    parameter("b", b);
    parameter("expected", expected);

    step("Calculate sum", || {
        let result = a + b;
        assert_eq!(result, expected);
    });
}

#[rstest]
#[case("hello", 5)]
#[case("world", 5)]
#[case("rust", 4)]
#[allure_epic("Framework Compatibility")]
#[allure_feature("rstest Integration")]
#[allure_story("String parameter tests")]
#[allure_test]
fn test_rstest_string_params(#[case] input: &str, #[case] expected_len: usize) {
    let _dir = setup_results_dir();

    parameter("input", input);
    parameter("expected_len", expected_len);

    step("Verify string length", || {
        assert_eq!(input.len(), expected_len);
    });
}

// =============================================================================
// test-case compatibility tests
// =============================================================================

#[test_case::test_case(1, 1, 2 ; "one plus one")]
#[test_case::test_case(2, 2, 4 ; "two plus two")]
#[test_case::test_case(0, 5, 5 ; "zero plus five")]
#[allure_epic("Framework Compatibility")]
#[allure_feature("test-case Integration")]
#[allure_story("Basic parameterized tests")]
#[allure_test]
fn test_testcase_basic(a: i32, b: i32, expected: i32) {
    let _dir = setup_results_dir();

    parameter("a", a);
    parameter("b", b);
    parameter("expected", expected);

    step("Calculate sum", || {
        assert_eq!(a + b, expected);
    });
}

#[test_case::test_case("hello" => 5 ; "hello has 5 chars")]
#[test_case::test_case("rust" => 4 ; "rust has 4 chars")]
#[test_case::test_case("" => 0 ; "empty string has 0 chars")]
#[allure_epic("Framework Compatibility")]
#[allure_feature("test-case Integration")]
#[allure_story("Tests with return values")]
#[allure_test]
fn test_testcase_with_return(input: &str) -> usize {
    let _dir = setup_results_dir();

    parameter("input", input);

    step("Calculate length", || input.len())
}

#[test_case::test_case(vec![1, 2, 3] => 6 ; "sum of 1,2,3")]
#[test_case::test_case(vec![10, 20] => 30 ; "sum of 10,20")]
#[test_case::test_case(vec![] => 0 ; "sum of empty")]
#[allure_epic("Framework Compatibility")]
#[allure_feature("test-case Integration")]
#[allure_story("Complex data types")]
#[allure_test]
fn test_testcase_complex_types(numbers: Vec<i32>) -> i32 {
    let _dir = setup_results_dir();

    parameter("numbers", format!("{:?}", numbers));

    step("Sum all numbers", || numbers.iter().sum())
}

// =============================================================================
// Combined framework tests
// =============================================================================

/// Test demonstrating that Allure labels work with rstest
#[rstest]
#[case("admin")]
#[case("user")]
#[case("guest")]
#[allure_epic("Framework Compatibility")]
#[allure_feature("Combined Usage")]
#[allure_severity("critical")]
#[allure_owner("test-team")]
#[allure_tags("smoke", "regression")]
#[allure_test]
fn test_full_metadata_with_rstest(#[case] role: &str) {
    let _dir = setup_results_dir();

    parameter("role", role);

    step("Verify role access", || {
        assert!(!role.is_empty());
    });

    attach_text("Test Data", format!("Testing role: {}", role));
}

/// Test demonstrating nested steps with parameterized tests
#[test_case::test_case("create", "user" ; "create user")]
#[test_case::test_case("update", "profile" ; "update profile")]
#[test_case::test_case("delete", "session" ; "delete session")]
#[allure_epic("Framework Compatibility")]
#[allure_feature("Combined Usage")]
#[allure_test]
fn test_nested_steps_with_testcase(action: &str, resource: &str) {
    let _dir = setup_results_dir();

    parameter("action", action);
    parameter("resource", resource);

    step("Prepare request", || {
        step("Set headers", || {
            // Simulate setting headers
        });

        step("Set body", || {
            // Simulate setting body
        });
    });

    step("Execute action", || {
        step(&format!("Perform {} on {}", action, resource), || {
            // Simulate action
        });
    });

    step("Verify result", || {
        assert!(!action.is_empty());
        assert!(!resource.is_empty());
    });
}
