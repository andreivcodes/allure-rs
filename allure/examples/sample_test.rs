//! Sample test to generate Allure results for validation.

use allure::prelude::*;
use allure_core::runtime::{set_context, take_context, TestContext};
use allure_core::writer::AllureWriter;

fn main() {
    // Set up the results directory
    let results_dir = "allure-results";
    let writer = AllureWriter::with_results_dir(results_dir);
    writer.init(true).unwrap();

    // Run several test scenarios to generate output

    // 1. Simple passing test
    run_sample_test(&writer, "test_login", "auth::test_login", |_| {
        epic("User Management");
        feature("Authentication");
        severity(Severity::Critical);

        step("Initialize user session", || {
            // setup code
        });

        step("Perform login", || {
            // login code
        });
    });

    // 2. Test with BDD steps
    run_sample_test(
        &writer,
        "test_user_registration",
        "auth::test_user_registration",
        |_| {
            epic("User Management");
            feature("Registration");
            story("User can register with email");

            allure::bdd::given("a new user with valid email", || {});
            allure::bdd::when("the user submits registration form", || {});
            allure::bdd::then("the user account should be created", || {});
        },
    );

    // 3. Test with attachments
    run_sample_test(
        &writer,
        "test_with_attachments",
        "api::test_with_attachments",
        |writer| {
            suite("API Tests");

            step("Make API request", || {});

            // Create text attachment
            let attachment = writer
                .write_text_attachment("Response Body", r#"{"status": "ok"}"#)
                .unwrap();

            // We need to add this to the current context
            allure_core::runtime::with_context(|ctx| {
                ctx.result.attachments.push(attachment);
            });
        },
    );

    // 4. Test with parameters
    run_sample_test(
        &writer,
        "test_parameterized",
        "data::test_parameterized",
        |_| {
            parameter("username", "john_doe");
            parameter("role", "admin");
            test_case_id("TC-1234");

            step("Process user", || {});
        },
    );

    // 5. Flaky test
    run_sample_test(&writer, "test_flaky", "unstable::test_flaky", |_| {
        flaky();
        tag("flaky");
        tag("needs-fix");

        step("Sometimes fails", || {});
    });

    // 6. Test with nested steps
    run_sample_test(
        &writer,
        "test_nested_steps",
        "complex::test_nested_steps",
        |_| {
            step("Outer step 1", || {
                step("Inner step 1.1", || {});
                step("Inner step 1.2", || {});
            });

            step("Outer step 2", || {
                step("Inner step 2.1", || {
                    step("Deep step 2.1.1", || {});
                });
            });
        },
    );

    // 7. Test with links
    run_sample_test(
        &writer,
        "test_with_links",
        "issues::test_with_links",
        |_| {
            issue("JIRA-123", Some("Login bug".to_string()));
            tms("TMS-456", Some("Login test case".to_string()));
            link(
                "https://example.com/docs",
                Some("Documentation".to_string()),
            );

            step("Test with links", || {});
        },
    );

    // Write environment info
    allure::environment()
        .results_dir(results_dir)
        .set("rust_version", "1.75.0")
        .set("os", std::env::consts::OS)
        .set("arch", std::env::consts::ARCH)
        .write()
        .unwrap();

    // Write categories
    allure::categories()
        .results_dir(results_dir)
        .with_product_defects()
        .with_test_defects()
        .write()
        .unwrap();

    println!("Generated Allure results in {}", results_dir);
    println!(
        "Run: allure generate {} -o allure-report && allure open allure-report",
        results_dir
    );
}

fn run_sample_test<F>(writer: &AllureWriter, name: &str, full_name: &str, f: F)
where
    F: FnOnce(&AllureWriter),
{
    let ctx = TestContext::new(name.to_string(), full_name.to_string());
    set_context(ctx);

    f(writer);

    let mut ctx = take_context().expect("Context should exist");
    ctx.finish(Status::Passed, None, None);
    writer.write_test_result(&ctx.result).unwrap();
}
