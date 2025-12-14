//! Async test coverage for Allure.
//!
//! These tests verify that Allure macros and runtime work correctly
//! with async test frameworks like tokio.

use allure::prelude::*;
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
// Basic async test compatibility
// =============================================================================

#[tokio::test]
#[allure_epic("Async Support")]
#[allure_feature("Basic Async Tests")]
#[allure_story("Async test with allure_test")]
#[allure_test]
async fn test_basic_async() {
    let _dir = setup_results_dir();

    step("Async operation 1", || {
        // Synchronous step in async test
    });

    // Simulate async work
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

    step("Verify async completed", || {
        assert!(true);
    });
}

#[tokio::test]
#[allure_epic("Async Support")]
#[allure_feature("Basic Async Tests")]
#[allure_story("Multiple async operations")]
#[allure_test]
async fn test_multiple_async_operations() {
    let _dir = setup_results_dir();

    step("Start concurrent tasks", || {
        // Setup
    });

    let (result1, result2) = tokio::join!(
        async { 1 + 1 },
        async { 2 + 2 }
    );

    step("Verify results", || {
        assert_eq!(result1, 2);
        assert_eq!(result2, 4);
    });
}

// =============================================================================
// Async tests with metadata
// =============================================================================

#[tokio::test]
#[allure_epic("Async Support")]
#[allure_feature("Metadata in Async")]
#[allure_severity("critical")]
#[allure_owner("async-team")]
#[allure_tags("async", "smoke")]
#[allure_test]
async fn test_async_with_full_metadata() {
    let _dir = setup_results_dir();

    parameter("test_type", "async");
    parameter("runtime", "tokio");

    step("Perform async assertion", || {
        assert!(true);
    });

    attach_text("Async Log", "Test completed successfully in async context");
}

#[tokio::test]
#[allure_epic("Async Support")]
#[allure_feature("Metadata in Async")]
#[allure_title("Custom Async Test Title")]
#[allure_description("This test verifies metadata works in async context")]
#[allure_test]
async fn test_async_with_title_and_description() {
    let _dir = setup_results_dir();

    step("Verify title is set", || {
        // Title should be "Custom Async Test Title"
    });
}

// =============================================================================
// Async tests with steps and attachments
// =============================================================================

#[tokio::test]
#[allure_epic("Async Support")]
#[allure_feature("Steps in Async")]
#[allure_test]
async fn test_async_nested_steps() {
    let _dir = setup_results_dir();

    step("Outer step", || {
        step("Inner step 1", || {
            // Nested step
        });

        step("Inner step 2", || {
            // Another nested step
        });
    });

    // Async work between steps
    tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;

    step("Final step", || {
        assert!(true);
    });
}

#[tokio::test]
#[allure_epic("Async Support")]
#[allure_feature("Attachments in Async")]
#[allure_test]
async fn test_async_with_attachments() {
    let _dir = setup_results_dir();

    step("Create attachment", || {
        attach_text("Request", r#"{"method": "GET", "url": "/api/test"}"#);
    });

    // Simulate async API call
    let response = async {
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        r#"{"status": "ok"}"#
    }
    .await;

    step("Attach response", || {
        attach_text("Response", response);
    });
}

// =============================================================================
// Async tests with BDD steps
// =============================================================================

#[tokio::test]
#[allure_epic("Async Support")]
#[allure_feature("BDD in Async")]
#[allure_test]
async fn test_async_bdd_style() {
    let _dir = setup_results_dir();
    use allure::bdd::*;

    let user_id = given("a user exists", || {
        "user123".to_string()
    });

    // Async operation
    let fetch_result = async {
        tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        format!("User data for {}", user_id)
    }
    .await;

    when("the user data is fetched", || {
        parameter("result", &fetch_result);
    });

    then("the result should contain user ID", || {
        assert!(fetch_result.contains(&user_id));
    });
}

// =============================================================================
// Error handling in async tests
// =============================================================================

#[tokio::test]
#[allure_epic("Async Support")]
#[allure_feature("Error Handling")]
#[allure_test]
async fn test_async_result_ok() -> Result<(), Box<dyn std::error::Error>> {
    let _dir = setup_results_dir();

    step("Perform async operation", || {
        // Operation succeeds
    });

    let result: Result<i32, &str> = Ok(42);

    step("Verify result", || {
        assert!(result.is_ok());
    });

    Ok(())
}

#[tokio::test]
#[allure_epic("Async Support")]
#[allure_feature("Parallel Execution")]
#[allure_test]
async fn test_async_parallel_tasks() {
    let _dir = setup_results_dir();

    step("Launch parallel tasks", || {
        parameter("task_count", 3);
    });

    let tasks: Vec<_> = (0..3)
        .map(|i| {
            tokio::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
                i * 2
            })
        })
        .collect();

    let results: Vec<_> = futures::future::join_all(tasks)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    step("Verify all tasks completed", || {
        assert_eq!(results, vec![0, 2, 4]);
    });
}
