use allure_macros::{allure_test, allure_epic};

// Epic requires a string, not a number
#[allure_epic(123)]
#[allure_test]
fn test_epic_wrong_type() {
    assert!(true);
}

fn main() {}
