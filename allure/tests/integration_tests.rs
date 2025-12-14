//! Integration tests for Allure-RS
//!
//! These tests validate that the library produces correct Allure output files.

use allure::prelude::*;
use allure::{bdd, Category};
use allure_core::enums::ContentType;
use allure_core::model::{FixtureResult, TestResultContainer};
use allure_core::runtime::{self, set_context, take_context, TestContext};
use allure_core::writer::AllureWriter;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test helper that provides a temporary directory and writer.
struct TestHelper {
    temp_dir: TempDir,
    writer: AllureWriter,
}

impl TestHelper {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let writer = AllureWriter::with_results_dir(temp_dir.path());
        writer.init(true).expect("Failed to init writer");
        Self { temp_dir, writer }
    }

    fn results_dir(&self) -> PathBuf {
        self.temp_dir.path().to_path_buf()
    }

    /// Reads all result JSON files from the results directory.
    fn read_result_files(&self) -> Vec<Value> {
        let mut results = Vec::new();
        for entry in fs::read_dir(self.results_dir()).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false)
                && path.to_string_lossy().contains("-result.json")
            {
                let content = fs::read_to_string(&path).unwrap();
                let json: Value = serde_json::from_str(&content).unwrap();
                results.push(json);
            }
        }
        results
    }

    /// Reads all container JSON files from the results directory.
    fn read_container_files(&self) -> Vec<Value> {
        let mut containers = Vec::new();
        for entry in fs::read_dir(self.results_dir()).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false)
                && path.to_string_lossy().contains("-container.json")
            {
                let content = fs::read_to_string(&path).unwrap();
                let json: Value = serde_json::from_str(&content).unwrap();
                containers.push(json);
            }
        }
        containers
    }

    /// Reads all attachment files from the results directory.
    fn read_attachment_files(&self) -> Vec<(String, Vec<u8>)> {
        let mut attachments = Vec::new();
        for entry in fs::read_dir(self.results_dir()).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.to_string_lossy().contains("-attachment.") {
                let filename = path.file_name().unwrap().to_string_lossy().to_string();
                let content = fs::read(&path).unwrap();
                attachments.push((filename, content));
            }
        }
        attachments
    }

    /// Creates a test context, runs a function with it, and writes the result.
    fn run_test<F>(&self, name: &str, full_name: &str, f: F) -> PathBuf
    where
        F: FnOnce(),
    {
        let ctx = TestContext::new(name.to_string(), full_name.to_string());
        set_context(ctx);

        f();

        let mut ctx = take_context().expect("Context should exist");
        ctx.finish(Status::Passed, None, None);
        self.writer.write_test_result(&ctx.result).unwrap()
    }

    /// Creates a test that panics and captures the failure.
    fn run_failing_test<F>(&self, name: &str, full_name: &str, f: F) -> PathBuf
    where
        F: FnOnce(),
    {
        let ctx = TestContext::new(name.to_string(), full_name.to_string());
        set_context(ctx);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        let mut ctx = take_context().expect("Context should exist");
        match result {
            Ok(_) => ctx.finish(Status::Passed, None, None),
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    Some(s.to_string())
                } else if let Some(s) = e.downcast_ref::<String>() {
                    Some(s.clone())
                } else {
                    Some("Test panicked".to_string())
                };
                ctx.finish(Status::Failed, msg, None);
            }
        }
        self.writer.write_test_result(&ctx.result).unwrap()
    }
}

// =============================================================================
// 1. Basic Test Result Generation
// =============================================================================

#[test]
fn test_generates_result_json_file() {
    let helper = TestHelper::new();
    let path = helper.run_test("my_test", "module::my_test", || {});

    assert!(path.exists());
    assert!(path.to_string_lossy().contains("-result.json"));
}

#[test]
fn test_result_has_correct_uuid_format() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {});

    let results = helper.read_result_files();
    assert_eq!(results.len(), 1);

    let uuid = results[0]["uuid"].as_str().unwrap();
    // UUID v4 format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    assert_eq!(uuid.len(), 36);
    assert!(uuid.chars().filter(|c| *c == '-').count() == 4);
}

#[test]
fn test_result_has_correct_timestamps() {
    let helper = TestHelper::new();
    let start = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    helper.run_test("my_test", "module::my_test", || {
        std::thread::sleep(std::time::Duration::from_millis(10));
    });

    let end = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let results = helper.read_result_files();
    let result = &results[0];

    let start_time = result["start"].as_i64().unwrap();
    let stop_time = result["stop"].as_i64().unwrap();

    assert!(start_time >= start);
    assert!(stop_time <= end);
    assert!(stop_time >= start_time);
}

#[test]
fn test_passed_test_has_passed_status() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {});

    let results = helper.read_result_files();
    assert_eq!(results[0]["status"], "passed");
}

#[test]
fn test_failed_test_has_failed_status() {
    let helper = TestHelper::new();
    helper.run_failing_test("my_test", "module::my_test", || {
        panic!("Test failure message");
    });

    let results = helper.read_result_files();
    assert_eq!(results[0]["status"], "failed");
}

#[test]
fn test_panicked_test_captures_message() {
    let helper = TestHelper::new();
    helper.run_failing_test("my_test", "module::my_test", || {
        panic!("Expected panic message");
    });

    let results = helper.read_result_files();
    let status_details = &results[0]["statusDetails"];
    assert!(status_details["message"]
        .as_str()
        .unwrap()
        .contains("Expected panic message"));
}

#[test]
fn test_result_has_correct_name_and_full_name() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "my_module::my_test", || {});

    let results = helper.read_result_files();
    assert_eq!(results[0]["name"], "my_test");
    assert_eq!(results[0]["fullName"], "my_module::my_test");
}

#[test]
fn test_result_has_finished_stage() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {});

    let results = helper.read_result_files();
    assert_eq!(results[0]["stage"], "finished");
}

// =============================================================================
// 2. Test Metadata Labels
// =============================================================================

#[test]
fn test_epic_label_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::epic("User Management");
    });

    let results = helper.read_result_files();
    let labels = results[0]["labels"].as_array().unwrap();
    let epic_label = labels.iter().find(|l| l["name"] == "epic").unwrap();
    assert_eq!(epic_label["value"], "User Management");
}

#[test]
fn test_feature_label_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::feature("Authentication");
    });

    let results = helper.read_result_files();
    let labels = results[0]["labels"].as_array().unwrap();
    let feature_label = labels.iter().find(|l| l["name"] == "feature").unwrap();
    assert_eq!(feature_label["value"], "Authentication");
}

#[test]
fn test_story_label_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::story("User can login");
    });

    let results = helper.read_result_files();
    let labels = results[0]["labels"].as_array().unwrap();
    let story_label = labels.iter().find(|l| l["name"] == "story").unwrap();
    assert_eq!(story_label["value"], "User can login");
}

#[test]
fn test_suite_labels_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::parent_suite("Parent Suite");
        runtime::suite("Main Suite");
        runtime::sub_suite("Sub Suite");
    });

    let results = helper.read_result_files();
    let labels = results[0]["labels"].as_array().unwrap();

    let parent = labels.iter().find(|l| l["name"] == "parentSuite").unwrap();
    let suite = labels.iter().find(|l| l["name"] == "suite").unwrap();
    let sub = labels.iter().find(|l| l["name"] == "subSuite").unwrap();

    assert_eq!(parent["value"], "Parent Suite");
    assert_eq!(suite["value"], "Main Suite");
    assert_eq!(sub["value"], "Sub Suite");
}

#[test]
fn test_severity_label_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::severity(Severity::Critical);
    });

    let results = helper.read_result_files();
    let labels = results[0]["labels"].as_array().unwrap();
    let severity_label = labels.iter().find(|l| l["name"] == "severity").unwrap();
    assert_eq!(severity_label["value"], "critical");
}

#[test]
fn test_owner_label_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::owner("John Doe");
    });

    let results = helper.read_result_files();
    let labels = results[0]["labels"].as_array().unwrap();
    let owner_label = labels.iter().find(|l| l["name"] == "owner").unwrap();
    assert_eq!(owner_label["value"], "John Doe");
}

#[test]
fn test_tag_labels_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::tag("smoke");
        runtime::tag("regression");
    });

    let results = helper.read_result_files();
    let labels = results[0]["labels"].as_array().unwrap();
    let tag_labels: Vec<_> = labels.iter().filter(|l| l["name"] == "tag").collect();

    assert_eq!(tag_labels.len(), 2);
    let values: Vec<_> = tag_labels
        .iter()
        .map(|l| l["value"].as_str().unwrap())
        .collect();
    assert!(values.contains(&"smoke"));
    assert!(values.contains(&"regression"));
}

#[test]
fn test_allure_id_label_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::allure_id("TC-123");
    });

    let results = helper.read_result_files();
    let labels = results[0]["labels"].as_array().unwrap();
    let id_label = labels.iter().find(|l| l["name"] == "AS_ID").unwrap();
    assert_eq!(id_label["value"], "TC-123");
}

#[test]
fn test_description_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::description("This test validates user login");
    });

    let results = helper.read_result_files();
    assert_eq!(results[0]["description"], "This test validates user login");
}

#[test]
fn test_description_html_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::description_html("<p>This test validates <b>user login</b></p>");
    });

    let results = helper.read_result_files();
    assert_eq!(
        results[0]["descriptionHtml"],
        "<p>This test validates <b>user login</b></p>"
    );
}

// =============================================================================
// 3. Links
// =============================================================================

#[test]
fn test_issue_link_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::issue("ISSUE-123", None);
    });

    let results = helper.read_result_files();
    let links = results[0]["links"].as_array().unwrap();
    let issue_link = links.iter().find(|l| l["type"] == "issue").unwrap();
    assert_eq!(issue_link["url"], "ISSUE-123");
}

#[test]
fn test_tms_link_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::tms("TMS-456", None);
    });

    let results = helper.read_result_files();
    let links = results[0]["links"].as_array().unwrap();
    let tms_link = links.iter().find(|l| l["type"] == "tms").unwrap();
    assert_eq!(tms_link["url"], "TMS-456");
}

#[test]
fn test_generic_link_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::link("https://example.com/docs", None);
    });

    let results = helper.read_result_files();
    let links = results[0]["links"].as_array().unwrap();
    assert!(links.iter().any(|l| l["url"] == "https://example.com/docs"));
}

#[test]
fn test_link_with_name() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::link("https://example.com", Some("Example Site".to_string()));
    });

    let results = helper.read_result_files();
    let links = results[0]["links"].as_array().unwrap();
    let link = links
        .iter()
        .find(|l| l["url"] == "https://example.com")
        .unwrap();
    assert_eq!(link["name"], "Example Site");
    // Default link type
    assert_eq!(link["type"], "link");
}

// =============================================================================
// 4. Parameters
// =============================================================================

#[test]
fn test_parameter_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::parameter("username", "john_doe");
    });

    let results = helper.read_result_files();
    let params = results[0]["parameters"].as_array().unwrap();
    let param = params.iter().find(|p| p["name"] == "username").unwrap();
    assert_eq!(param["value"], "john_doe");
}

#[test]
fn test_multiple_parameters() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::parameter("username", "john");
        runtime::parameter("role", "admin");
        runtime::parameter("count", "42");
    });

    let results = helper.read_result_files();
    let params = results[0]["parameters"].as_array().unwrap();
    assert_eq!(params.len(), 3);
}

// =============================================================================
// 5. Steps
// =============================================================================

#[test]
fn test_step_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        step("Step 1", || {
            // Step body
        });
    });

    let results = helper.read_result_files();
    let steps = results[0]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0]["name"], "Step 1");
    assert_eq!(steps[0]["status"], "passed");
}

#[test]
fn test_nested_steps() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        step("Parent Step", || {
            step("Child Step", || {
                // Nested step body
            });
        });
    });

    let results = helper.read_result_files();
    let steps = results[0]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0]["name"], "Parent Step");

    let nested_steps = steps[0]["steps"].as_array().unwrap();
    assert_eq!(nested_steps.len(), 1);
    assert_eq!(nested_steps[0]["name"], "Child Step");
}

#[test]
fn test_step_with_return_value() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        let result = step("Calculate", || 42);
        assert_eq!(result, 42);
    });

    let results = helper.read_result_files();
    let steps = results[0]["steps"].as_array().unwrap();
    assert_eq!(steps[0]["name"], "Calculate");
    assert_eq!(steps[0]["status"], "passed");
}

#[test]
fn test_step_timing() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        step("Slow Step", || {
            std::thread::sleep(std::time::Duration::from_millis(10));
        });
    });

    let results = helper.read_result_files();
    let step = &results[0]["steps"].as_array().unwrap()[0];
    let start = step["start"].as_i64().unwrap();
    let stop = step["stop"].as_i64().unwrap();
    assert!(stop >= start);
    // At least 10ms should have passed
    assert!(stop - start >= 10);
}

#[test]
fn test_log_step() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::log_step("Log message", Status::Passed);
    });

    let results = helper.read_result_files();
    let steps = results[0]["steps"].as_array().unwrap();
    assert_eq!(steps[0]["name"], "Log message");
    assert_eq!(steps[0]["status"], "passed");
}

// =============================================================================
// 6. Attachments
// =============================================================================

// Attachment tests use the writer directly since TestContext uses global config
// which doesn't point to our temp directory.

#[test]
fn test_text_attachment_created() {
    let helper = TestHelper::new();

    // Create attachment using the writer directly
    let attachment = helper
        .writer
        .write_text_attachment("Log Output", "Some log content here")
        .unwrap();

    let attachments = helper.read_attachment_files();
    assert_eq!(attachments.len(), 1);
    assert!(attachments[0].0.ends_with(".txt"));
    assert_eq!(
        String::from_utf8_lossy(&attachments[0].1),
        "Some log content here"
    );
    assert_eq!(attachment.name, "Log Output");
}

#[test]
fn test_json_attachment_created() {
    let helper = TestHelper::new();

    #[derive(serde::Serialize)]
    struct Data {
        key: String,
        value: i32,
    }

    let attachment = helper
        .writer
        .write_json_attachment(
            "Response",
            &Data {
                key: "test".to_string(),
                value: 42,
            },
        )
        .unwrap();

    let attachments = helper.read_attachment_files();
    assert_eq!(attachments.len(), 1);
    assert!(attachments[0].0.ends_with(".json"));

    let content = String::from_utf8_lossy(&attachments[0].1);
    assert!(content.contains("\"key\": \"test\""));
    assert!(content.contains("\"value\": 42"));
    assert_eq!(attachment.name, "Response");
}

#[test]
fn test_binary_attachment_created() {
    let helper = TestHelper::new();

    let png_data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic bytes
    let attachment = helper
        .writer
        .write_binary_attachment("Screenshot", &png_data, ContentType::Png)
        .unwrap();

    let attachments = helper.read_attachment_files();
    assert_eq!(attachments.len(), 1);
    assert!(attachments[0].0.ends_with(".png"));
    assert_eq!(attachments[0].1, png_data);
    assert_eq!(attachment.name, "Screenshot");
}

#[test]
fn test_attachment_reference_in_result() {
    let helper = TestHelper::new();

    // Create a test result with an attachment reference
    let mut result =
        allure_core::model::TestResult::new("test-uuid".to_string(), "my_test".to_string());

    // Write attachment and add reference to result
    let attachment = helper
        .writer
        .write_text_attachment("Debug Info", "debug content")
        .unwrap();
    result.attachments.push(attachment);
    result.pass();

    helper.writer.write_test_result(&result).unwrap();

    let results = helper.read_result_files();
    let attachments = results[0]["attachments"].as_array().unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0]["name"], "Debug Info");
    assert!(attachments[0]["source"].as_str().unwrap().ends_with(".txt"));
    assert_eq!(attachments[0]["type"], "text/plain");
}

// =============================================================================
// 7. BDD Steps
// =============================================================================

#[test]
fn test_given_step_format() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        bdd::given("a user exists", || {});
    });

    let results = helper.read_result_files();
    let steps = results[0]["steps"].as_array().unwrap();
    assert_eq!(steps[0]["name"], "Given a user exists");
}

#[test]
fn test_when_step_format() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        bdd::when("the user logs in", || {});
    });

    let results = helper.read_result_files();
    let steps = results[0]["steps"].as_array().unwrap();
    assert_eq!(steps[0]["name"], "When the user logs in");
}

#[test]
fn test_then_step_format() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        bdd::then("the user is authenticated", || {});
    });

    let results = helper.read_result_files();
    let steps = results[0]["steps"].as_array().unwrap();
    assert_eq!(steps[0]["name"], "Then the user is authenticated");
}

#[test]
fn test_and_step_format() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        bdd::and("the session is active", || {});
    });

    let results = helper.read_result_files();
    let steps = results[0]["steps"].as_array().unwrap();
    assert_eq!(steps[0]["name"], "And the session is active");
}

#[test]
fn test_but_step_format() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        bdd::but("the admin panel is hidden", || {});
    });

    let results = helper.read_result_files();
    let steps = results[0]["steps"].as_array().unwrap();
    assert_eq!(steps[0]["name"], "But the admin panel is hidden");
}

#[test]
fn test_bdd_full_scenario() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        bdd::given("a registered user", || {});
        bdd::when("the user enters credentials", || {});
        bdd::and("clicks the login button", || {});
        bdd::then("the user sees the dashboard", || {});
    });

    let results = helper.read_result_files();
    let steps = results[0]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 4);
    assert_eq!(steps[0]["name"], "Given a registered user");
    assert_eq!(steps[1]["name"], "When the user enters credentials");
    assert_eq!(steps[2]["name"], "And clicks the login button");
    assert_eq!(steps[3]["name"], "Then the user sees the dashboard");
}

// =============================================================================
// 8. Flaky/Muted/Known Issues
// =============================================================================

#[test]
fn test_flaky_flag_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::flaky();
    });

    let results = helper.read_result_files();
    let status_details = &results[0]["statusDetails"];
    assert_eq!(status_details["flaky"], true);
}

#[test]
fn test_known_issue_flag_and_link() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::known_issue("BUG-123");
    });

    let results = helper.read_result_files();

    // Check the known flag in statusDetails
    let status_details = &results[0]["statusDetails"];
    assert_eq!(status_details["known"], true);

    // Check the link
    let links = results[0]["links"].as_array().unwrap();
    let issue_link = links.iter().find(|l| l["type"] == "issue");
    assert!(issue_link.is_some());
    assert_eq!(issue_link.unwrap()["url"], "BUG-123");
}

// =============================================================================
// 9. History ID
// =============================================================================

#[test]
fn test_history_id_generated() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {});

    let results = helper.read_result_files();
    let history_id = results[0]["historyId"].as_str().unwrap();
    assert!(!history_id.is_empty());
    // MD5 hash is 32 hex characters
    assert_eq!(history_id.len(), 32);
}

#[test]
fn test_history_id_stable_for_same_test() {
    let helper1 = TestHelper::new();
    helper1.run_test("my_test", "module::my_test", || {});
    let results1 = helper1.read_result_files();
    let history_id1 = results1[0]["historyId"].as_str().unwrap().to_string();

    let helper2 = TestHelper::new();
    helper2.run_test("my_test", "module::my_test", || {});
    let results2 = helper2.read_result_files();
    let history_id2 = results2[0]["historyId"].as_str().unwrap();

    assert_eq!(history_id1, history_id2);
}

#[test]
fn test_history_id_changes_with_fullname() {
    let helper1 = TestHelper::new();
    helper1.run_test("my_test", "module_a::my_test", || {});
    let results1 = helper1.read_result_files();
    let history_id1 = results1[0]["historyId"].as_str().unwrap().to_string();

    let helper2 = TestHelper::new();
    helper2.run_test("my_test", "module_b::my_test", || {});
    let results2 = helper2.read_result_files();
    let history_id2 = results2[0]["historyId"].as_str().unwrap();

    assert_ne!(history_id1, history_id2);
}

// =============================================================================
// 10. Environment & Categories
// =============================================================================

#[test]
fn test_environment_properties_file_created() {
    let helper = TestHelper::new();

    let env = vec![
        ("os".to_string(), "linux".to_string()),
        ("rust_version".to_string(), "1.75.0".to_string()),
    ];

    let path = helper.writer.write_environment(&env).unwrap();
    assert!(path.exists());

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("os=linux"));
    assert!(content.contains("rust_version=1.75.0"));
}

#[test]
fn test_categories_json_file_created() {
    let helper = TestHelper::new();

    let categories = vec![
        Category::new("Infrastructure Issues").with_status(Status::Broken),
        Category::new("Product Defects").with_status(Status::Failed),
    ];

    let path = helper.writer.write_categories(&categories).unwrap();
    assert!(path.exists());

    let content = fs::read_to_string(&path).unwrap();
    assert!(content.contains("Infrastructure Issues"));
    assert!(content.contains("Product Defects"));
}

// =============================================================================
// 11. Container/Fixtures
// =============================================================================

#[test]
fn test_container_json_file_created() {
    let helper = TestHelper::new();

    let container = TestResultContainer::new("container-uuid".to_string());
    let path = helper.writer.write_container(&container).unwrap();

    assert!(path.exists());
    assert!(path.to_string_lossy().contains("-container.json"));
}

#[test]
fn test_container_links_to_tests() {
    let helper = TestHelper::new();

    let mut container = TestResultContainer::new("container-uuid".to_string());
    container.children.push("test-uuid-1".to_string());
    container.children.push("test-uuid-2".to_string());

    helper.writer.write_container(&container).unwrap();

    let containers = helper.read_container_files();
    let children = containers[0]["children"].as_array().unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0], "test-uuid-1");
    assert_eq!(children[1], "test-uuid-2");
}

#[test]
fn test_before_fixture_in_container() {
    let helper = TestHelper::new();

    let mut container = TestResultContainer::new("container-uuid".to_string());

    let mut before = FixtureResult::new("Setup".to_string());
    before.pass();
    container.befores.push(before);

    helper.writer.write_container(&container).unwrap();

    let containers = helper.read_container_files();
    let befores = containers[0]["befores"].as_array().unwrap();
    assert_eq!(befores.len(), 1);
    assert_eq!(befores[0]["name"], "Setup");
    assert_eq!(befores[0]["status"], "passed");
}

#[test]
fn test_after_fixture_in_container() {
    let helper = TestHelper::new();

    let mut container = TestResultContainer::new("container-uuid".to_string());

    let mut after = FixtureResult::new("Teardown".to_string());
    after.pass();
    container.afters.push(after);

    helper.writer.write_container(&container).unwrap();

    let containers = helper.read_container_files();
    let afters = containers[0]["afters"].as_array().unwrap();
    assert_eq!(afters.len(), 1);
    assert_eq!(afters[0]["name"], "Teardown");
    assert_eq!(afters[0]["status"], "passed");
}

// =============================================================================
// 12. Proc Macro Validation Tests (compile-time)
// =============================================================================
// These tests verify that the macros compile correctly.
// The actual runtime behavior is tested through the helper functions above.

#[cfg(test)]
mod macro_compile_tests {
    use allure::prelude::*;

    // This module exists primarily to verify that macros compile correctly.
    // The #[ignore] tests are meant to be run manually or to validate compilation.

    #[test]
    #[ignore = "requires manual verification of output"]
    fn verify_allure_test_macro_compiles() {
        // Just verify that this compiles
        fn sample_test_fn() {
            step("sample step", || {});
        }
        sample_test_fn();
    }

    #[test]
    fn test_severity_enum_all_variants() {
        // Verify all severity variants work
        let _blocker = Severity::Blocker;
        let _critical = Severity::Critical;
        let _normal = Severity::Normal;
        let _minor = Severity::Minor;
        let _trivial = Severity::Trivial;
    }

    #[test]
    fn test_status_enum_all_variants() {
        let _passed = Status::Passed;
        let _failed = Status::Failed;
        let _broken = Status::Broken;
        let _skipped = Status::Skipped;
    }
}

// =============================================================================
// 13. Display Name and Test Case ID
// =============================================================================

#[test]
fn test_display_name_changes_test_name() {
    let helper = TestHelper::new();
    helper.run_test("original_name", "module::original_name", || {
        runtime::display_name("Custom Display Name");
    });

    let results = helper.read_result_files();
    assert_eq!(results[0]["name"], "Custom Display Name");
}

#[test]
fn test_test_case_id_in_output() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        runtime::test_case_id("TC-12345");
    });

    let results = helper.read_result_files();
    assert_eq!(results[0]["testCaseId"], "TC-12345");
}

#[test]
fn test_file_attachment_created() {
    let helper = TestHelper::new();

    // Create a temp file to attach
    let file_path = helper.results_dir().join("source_file.txt");
    fs::write(&file_path, "File content to attach").unwrap();

    // Copy file as attachment
    let attachment = helper
        .writer
        .copy_file_attachment("Source File", &file_path, None)
        .unwrap();

    let attachments = helper.read_attachment_files();
    assert_eq!(attachments.len(), 1);
    assert!(attachments[0].0.ends_with(".txt"));
    assert_eq!(
        String::from_utf8_lossy(&attachments[0].1),
        "File content to attach"
    );
    assert_eq!(attachment.name, "Source File");
}

// =============================================================================
// 14. Edge Cases and Error Handling
// =============================================================================

#[test]
fn test_empty_step_name() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        step("", || {});
    });

    let results = helper.read_result_files();
    let steps = results[0]["steps"].as_array().unwrap();
    assert_eq!(steps[0]["name"], "");
}

#[test]
fn test_unicode_in_names() {
    let helper = TestHelper::new();
    helper.run_test(
        "tест_с_юникодом",
        "модуль::тест",
        || {
            step("Шаг с юникодом 日本語", || {});
            runtime::epic("Эпик");
        },
    );

    let results = helper.read_result_files();
    assert_eq!(results[0]["name"], "tест_с_юникодом");

    let steps = results[0]["steps"].as_array().unwrap();
    assert_eq!(steps[0]["name"], "Шаг с юникодом 日本語");
}

#[test]
fn test_special_characters_in_attachment_content() {
    let helper = TestHelper::new();

    helper
        .writer
        .write_text_attachment("Special Chars", "Line1\nLine2\t\"quoted\"\r\n")
        .unwrap();

    let attachments = helper.read_attachment_files();
    assert_eq!(attachments.len(), 1);
    assert_eq!(
        String::from_utf8_lossy(&attachments[0].1),
        "Line1\nLine2\t\"quoted\"\r\n"
    );
}

#[test]
fn test_many_steps() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        for i in 0..100 {
            step(&format!("Step {}", i), || {});
        }
    });

    let results = helper.read_result_files();
    let steps = results[0]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 100);
}

#[test]
fn test_deeply_nested_steps() {
    let helper = TestHelper::new();
    helper.run_test("my_test", "module::my_test", || {
        step("Level 1", || {
            step("Level 2", || {
                step("Level 3", || {
                    step("Level 4", || {
                        step("Level 5", || {});
                    });
                });
            });
        });
    });

    let results = helper.read_result_files();
    let l1 = &results[0]["steps"].as_array().unwrap()[0];
    let l2 = &l1["steps"].as_array().unwrap()[0];
    let l3 = &l2["steps"].as_array().unwrap()[0];
    let l4 = &l3["steps"].as_array().unwrap()[0];
    let l5 = &l4["steps"].as_array().unwrap()[0];

    assert_eq!(l1["name"], "Level 1");
    assert_eq!(l2["name"], "Level 2");
    assert_eq!(l3["name"], "Level 3");
    assert_eq!(l4["name"], "Level 4");
    assert_eq!(l5["name"], "Level 5");
}
