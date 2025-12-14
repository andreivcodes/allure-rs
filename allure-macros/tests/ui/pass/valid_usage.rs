// Import from allure_rs instead of allure_macros directly.
// The macros generate code that references allure_rs::__private.
use allure_rs::{
    allure_test, allure_epic, allure_story, allure_severity, allure_owner, allure_tag, allure_id,
    allure_description, allure_flaky, allure_issue, allure_tms, allure_link, allure_step_fn,
};

#[allure_epic("User Management")]
#[allure_story("User can login")]
#[allure_severity("critical")]
#[allure_owner("test-team")]
#[allure_tag("smoke")]
#[allure_id("TC-001")]
#[allure_description("This test verifies user login functionality")]
#[allure_test]
fn test_valid_metadata() {
    assert!(true);
}

#[allure_flaky]
#[allure_test]
fn test_flaky_marker() {
    assert!(true);
}

#[allure_issue("https://github.com/example/issue/123")]
#[allure_tms("https://testops.example.com/case/456")]
#[allure_link("https://docs.example.com")]
#[allure_test]
fn test_links() {
    assert!(true);
}

#[allure_issue("https://github.com/example/issue/123", "Bug #123")]
#[allure_test]
fn test_issue_with_name() {
    assert!(true);
}

#[allure_step_fn("My custom step")]
fn helper_step() -> i32 {
    42
}

#[allure_test]
fn test_with_step_helper() {
    let result = helper_step();
    assert_eq!(result, 42);
}

#[allure_test("Custom Test Name")]
fn test_custom_name() {
    assert!(true);
}

fn main() {}
