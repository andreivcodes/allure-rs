//! # Allure
//!
//! A comprehensive Rust library for generating [Allure](https://allurereport.org/) test reports.
//!
//! This crate provides full feature parity with allure-js-commons, including:
//!
//! - Test metadata annotations (epic, feature, story, severity, owner, tags)
//! - Test steps with nesting support
//! - Attachments (text, JSON, binary files)
//! - BDD-style steps (given, when, then)
//! - Links to issue trackers and test management systems
//! - Flaky test support
//! - Environment and categories configuration
//!
//! ## Quick Start
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dev-dependencies]
//! allure = "0.1"
//! ```
//!
//! ## Basic Usage
//!
//! ```ignore
//! use allure::prelude::*;
//!
//! #[allure_test]
//! #[allure_epic("User Management")]
//! #[allure_feature("Authentication")]
//! #[allure_severity("critical")]
//! fn test_login() {
//!     step("Initialize user", || {
//!         // setup code
//!     });
//!
//!     step("Perform login", || {
//!         // test code
//!         assert!(true);
//!     });
//!
//!     attachment::text("Debug info", "Login successful");
//! }
//! ```
//!
//! ## BDD Style
//!
//! ```ignore
//! use allure::prelude::*;
//! use allure::bdd::*;
//!
//! #[allure_test]
//! fn test_user_registration() {
//!     given("a new user with valid email", || {
//!         User::new("test@example.com")
//!     });
//!
//!     when("the user submits registration form", || {
//!         // registration logic
//!     });
//!
//!     then("the user account should be created", || {
//!         assert!(true);
//!     });
//! }
//! ```
//!
//! ## Configuration
//!
//! ```ignore
//! use allure::configure;
//!
//! // Initialize before running tests (e.g., in a test setup or main)
//! configure()
//!     .results_dir("allure-results")
//!     .clean_results(true)
//!     .init()
//!     .unwrap();
//! ```
//!
//! ## Environment Info
//!
//! ```ignore
//! use allure::environment;
//!
//! environment()
//!     .set("rust_version", env!("CARGO_PKG_RUST_VERSION"))
//!     .set("os", std::env::consts::OS)
//!     .set_from_env("CI", "CI")
//!     .write()
//!     .unwrap();
//! ```
//!
//! ## Categories
//!
//! ```ignore
//! use allure::{categories, Category, Status};
//!
//! categories()
//!     .with_product_defects()
//!     .with_test_defects()
//!     .with_category(
//!         Category::new("Infrastructure Issues")
//!             .with_status(Status::Broken)
//!             .with_message_regex(".*timeout.*")
//!     )
//!     .write()
//!     .unwrap();
//! ```

// Re-export everything from allure-core
pub use allure_core::*;

// Re-export all proc macros
pub use allure_macros::{
    allure_description, allure_description_html, allure_epic, allure_epics, allure_feature,
    allure_features, allure_flaky, allure_id, allure_issue, allure_link, allure_owner,
    allure_parent_suite, allure_severity, allure_step, allure_step_fn, allure_stories,
    allure_story, allure_sub_suite, allure_suite, allure_suite_label, allure_tag, allure_tags,
    allure_test, allure_title, allure_tms,
};

/// Prelude module for convenient imports.
///
/// Use `use allure::prelude::*;` to import commonly used items.
pub mod prelude {
    // Proc macros
    pub use allure_macros::{
        allure_description, allure_description_html, allure_epic, allure_epics, allure_feature,
        allure_features, allure_flaky, allure_id, allure_issue, allure_link, allure_owner,
        allure_parent_suite, allure_severity, allure_step, allure_step_fn, allure_stories,
        allure_story, allure_sub_suite, allure_suite, allure_suite_label, allure_tag, allure_tags,
        allure_test, allure_title, allure_tms,
    };

    // Core types
    pub use allure_core::{
        Attachment, Category, Label, Link, Parameter, Severity, Status, StepResult, TestResult,
    };

    // Runtime functions
    pub use allure_core::{
        allure_id, attach_binary, attach_file, attach_json, attach_text, configure, description,
        description_html, display_name, epic, feature, flaky, issue, known_issue, label, link,
        log_step, owner, parameter, parent_suite, run_test, severity, step, story, sub_suite,
        suite, tag, tags, test_case_id, title, tms,
    };

    // Attachment module
    pub use allure_core::attachment;

    // BDD module
    pub use allure_core::bdd;
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_prelude_imports() {
        // Verify that prelude provides the expected items
        use crate::prelude::*;

        // This just verifies compilation
        let _ = Status::Passed;
        let _ = Severity::Critical;
    }
}
