//! Actual Allure tests that generate results to allure-results directory.
//!
//! These tests use the #[allure_test] macro and write real Allure results
//! that can be viewed with `cargo allure`.
//!
//! IMPORTANT: Metadata attributes (#[allure_epic], #[allure_feature], etc.) must come
//! BEFORE #[allure_test] due to Rust's proc macro processing order.

// Use allure_macros directly for attribute macros
use allure_macros::{
    allure_description, allure_epic, allure_feature, allure_flaky, allure_owner, allure_severity,
    allure_suite_label, allure_tag, allure_test,
};

// Runtime functions from prelude
use allure::prelude::{bdd, configure, description, display_name, flaky, parameter, step, test_case_id};

// Initialize Allure once before tests run
// Note: Tests run from the crate directory (allure/), so use parent path
#[ctor::ctor]
fn init() {
    let _ = configure()
        .results_dir("../allure-results")
        .clean_results(false) // Don't clean between test runs
        .init();
}

#[allure_epic("User Management")]
#[allure_feature("Authentication")]
#[allure_severity("critical")]
#[allure_test]
fn test_user_login() {
    step("Open login page", || {
        // Simulate opening page
    });

    step("Enter credentials", || {
        parameter("username", "test_user");
        parameter("password", "***");
    });

    step("Click login button", || {
        // Simulate click
    });

    step("Verify login success", || {
        assert!(true, "Login should succeed");
    });
}

#[allure_epic("User Management")]
#[allure_feature("Registration")]
#[allure_severity("normal")]
#[allure_test]
fn test_user_registration() {
    bdd::given("a new user", || {
        parameter("email", "new@example.com");
    });

    bdd::when("the user submits registration", || {
        // Registration logic
    });

    bdd::then("the account is created", || {
        assert!(true);
    });

    bdd::and("a welcome email is sent", || {
        // Email verification
    });
}

#[allure_epic("Shopping")]
#[allure_feature("Cart")]
#[allure_tag("smoke")]
#[allure_tag("cart")]
#[allure_test]
fn test_add_to_cart() {
    step("Browse products", || {
        step("Open category", || {});
        step("Select product", || {
            parameter("product_id", "SKU-12345");
        });
    });

    step("Add to cart", || {
        parameter("quantity", "2");
    });

    step("Verify cart", || {
        assert!(true, "Product should be in cart");
    });
}

#[allure_suite_label("API Tests")]
#[allure_owner("Backend Team")]
#[allure_severity("blocker")]
#[allure_test]
fn test_api_health_check() {
    step("Send health check request", || {
        parameter("endpoint", "/api/health");
        parameter("method", "GET");
    });

    step("Verify response", || {
        let status = 200;
        assert_eq!(status, 200, "Health check should return 200");
    });

    description("This test verifies the API health endpoint is responding correctly.");
}

#[allure_flaky]
#[allure_tag("flaky")]
#[allure_test]
fn test_flaky_network_operation() {
    flaky();

    step("Attempt network operation", || {
        // This might fail sometimes due to network issues
        assert!(true);
    });
}

#[allure_epic("Data Processing")]
#[allure_feature("Calculations")]
#[allure_test]
fn test_nested_steps_demo() {
    step("Level 1 - Prepare data", || {
        step("Level 2 - Load from source", || {
            step("Level 3 - Parse format", || {
                // Deep nesting
            });
        });

        step("Level 2 - Validate data", || {
            parameter("records", "100");
        });
    });

    step("Level 1 - Process data", || {
        step("Level 2 - Transform", || {});
        step("Level 2 - Aggregate", || {});
    });

    step("Level 1 - Output results", || {});
}

#[allure_description("Test with custom display name")]
#[allure_test]
fn test_display_name_example() {
    display_name("Custom Test Display Name");
    test_case_id("TC-001");

    step("Execute test", || {
        assert!(true);
    });
}
