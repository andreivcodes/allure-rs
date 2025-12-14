use allure_macros::allure_test;

// allure_test can only be applied to functions, not structs
#[allure_test]
struct MyTest {
    value: i32,
}

fn main() {}
