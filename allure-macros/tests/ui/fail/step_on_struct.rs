use allure_macros::allure_step_fn;

// step can only be applied to functions, not structs
#[allure_step_fn("my step")]
struct MyStep {
    value: i32,
}

fn main() {}
