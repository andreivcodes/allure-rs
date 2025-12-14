//! Allure Core - Core types and runtime for Allure test reporting.
//!
//! This crate provides the foundational types and runtime infrastructure for
//! generating Allure test reports in Rust. It includes:
//!
//! - The complete Allure data model (test results, steps, attachments, etc.)
//! - Enum types for status, stage, severity, and other classifications
//! - A file writer for outputting results to the `allure-results` directory
//! - Runtime context management for tracking test execution state
//!
//! # Example
//!
//! ```no_run
//! use allure_core::{configure, runtime, enums::Severity};
//!
//! // Initialize the Allure runtime
//! configure()
//!     .results_dir("allure-results")
//!     .clean_results(true)
//!     .init()
//!     .unwrap();
//!
//! // In a test, you can use the runtime API
//! runtime::epic("My Epic");
//! runtime::feature("My Feature");
//! runtime::severity(Severity::Critical);
//!
//! runtime::step("Do something", || {
//!     // test code here
//! });
//! ```

pub mod enums;
pub mod error;
pub mod model;
pub mod runtime;
pub mod writer;

// Re-exports for convenience
pub use enums::{ContentType, LabelName, LinkType, ParameterMode, Severity, Stage, Status};
pub use error::{AllureError, AllureResult};
pub use model::{
    Attachment, Category, FixtureResult, Label, Link, Parameter, StatusDetails, StepResult,
    TestResult, TestResultContainer,
};
pub use runtime::{
    allure_id, attach_binary, attach_file, attach_json, attach_text, configure, description,
    description_html, display_name, epic, feature, flaky, issue, known_issue, label, link,
    log_step, muted, owner, parameter, parent_suite, run_test, severity, step, story, sub_suite,
    suite, tag, tags, test_case_id, title, tms, with_context, AllureConfig, AllureConfigBuilder,
    TestContext,
};
pub use writer::{compute_history_id, generate_uuid, AllureWriter, DEFAULT_RESULTS_DIR};

// Re-export futures for async panic handling in macros
#[cfg(feature = "async")]
pub use futures;

/// BDD-style step functions for behavior-driven testing.
pub mod bdd {
    use crate::runtime::step;

    /// Executes a "Given" step (precondition).
    pub fn given<F, R>(description: impl Into<String>, body: F) -> R
    where
        F: FnOnce() -> R,
    {
        step(format!("Given {}", description.into()), body)
    }

    /// Executes a "When" step (action).
    pub fn when<F, R>(description: impl Into<String>, body: F) -> R
    where
        F: FnOnce() -> R,
    {
        step(format!("When {}", description.into()), body)
    }

    /// Executes a "Then" step (assertion).
    pub fn then<F, R>(description: impl Into<String>, body: F) -> R
    where
        F: FnOnce() -> R,
    {
        step(format!("Then {}", description.into()), body)
    }

    /// Executes an "And" step (continuation).
    pub fn and<F, R>(description: impl Into<String>, body: F) -> R
    where
        F: FnOnce() -> R,
    {
        step(format!("And {}", description.into()), body)
    }

    /// Executes a "But" step (negative continuation).
    pub fn but<F, R>(description: impl Into<String>, body: F) -> R
    where
        F: FnOnce() -> R,
    {
        step(format!("But {}", description.into()), body)
    }
}

/// Attachment helper module with convenience functions.
pub mod attachment {
    use crate::enums::ContentType;
    use crate::runtime::{attach_binary, attach_file as attach_file_fn, attach_json, attach_text};

    /// Attaches text content.
    pub fn text(name: impl Into<String>, content: impl AsRef<str>) {
        attach_text(name, content);
    }

    /// Attaches JSON content.
    pub fn json<T: serde::Serialize>(name: impl Into<String>, value: &T) {
        attach_json(name, value);
    }

    /// Attaches binary content.
    pub fn binary(name: impl Into<String>, content: &[u8], content_type: ContentType) {
        attach_binary(name, content, content_type);
    }

    /// Attaches a file from the filesystem.
    pub fn file(
        name: impl Into<String>,
        path: impl AsRef<std::path::Path>,
        content_type: Option<ContentType>,
    ) {
        attach_file_fn(name, path, content_type);
    }

    /// Attaches a PNG image.
    pub fn png(name: impl Into<String>, content: &[u8]) {
        attach_binary(name, content, ContentType::Png);
    }

    /// Attaches a JPEG image.
    pub fn jpeg(name: impl Into<String>, content: &[u8]) {
        attach_binary(name, content, ContentType::Jpeg);
    }

    /// Attaches HTML content.
    pub fn html(name: impl Into<String>, content: impl AsRef<str>) {
        attach_binary(name, content.as_ref().as_bytes(), ContentType::Html);
    }

    /// Attaches XML content.
    pub fn xml(name: impl Into<String>, content: impl AsRef<str>) {
        attach_binary(name, content.as_ref().as_bytes(), ContentType::Xml);
    }

    /// Attaches CSV content.
    pub fn csv(name: impl Into<String>, content: impl AsRef<str>) {
        attach_binary(name, content.as_ref().as_bytes(), ContentType::Csv);
    }
}

/// Environment info builder for generating `environment.properties`.
pub struct EnvironmentBuilder {
    properties: Vec<(String, String)>,
    results_dir: String,
}

impl EnvironmentBuilder {
    /// Creates a new environment builder.
    pub fn new() -> Self {
        Self {
            properties: Vec::new(),
            results_dir: DEFAULT_RESULTS_DIR.to_string(),
        }
    }

    /// Sets the results directory.
    pub fn results_dir(mut self, path: impl Into<String>) -> Self {
        self.results_dir = path.into();
        self
    }

    /// Adds a key-value pair.
    pub fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.push((key.into(), value.into()));
        self
    }

    /// Adds a key-value pair from an environment variable.
    pub fn set_from_env(mut self, key: impl Into<String>, env_var: &str) -> Self {
        if let Ok(value) = std::env::var(env_var) {
            self.properties.push((key.into(), value));
        }
        self
    }

    /// Writes the environment.properties file.
    pub fn write(self) -> std::io::Result<std::path::PathBuf> {
        let writer = AllureWriter::with_results_dir(&self.results_dir);
        writer.write_environment(&self.properties)
    }
}

impl Default for EnvironmentBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a new environment builder.
pub fn environment() -> EnvironmentBuilder {
    EnvironmentBuilder::new()
}

/// Categories configuration builder.
pub struct CategoriesBuilder {
    categories: Vec<Category>,
    results_dir: String,
}

impl CategoriesBuilder {
    /// Creates a new categories builder.
    pub fn new() -> Self {
        Self {
            categories: Vec::new(),
            results_dir: DEFAULT_RESULTS_DIR.to_string(),
        }
    }

    /// Sets the results directory.
    pub fn results_dir(mut self, path: impl Into<String>) -> Self {
        self.results_dir = path.into();
        self
    }

    /// Adds a category.
    pub fn with_category(mut self, category: Category) -> Self {
        self.categories.push(category);
        self
    }

    /// Adds the default product defects category.
    pub fn with_product_defects(mut self) -> Self {
        self.categories
            .push(Category::new("Product defects").with_status(Status::Failed));
        self
    }

    /// Adds the default test defects category.
    pub fn with_test_defects(mut self) -> Self {
        self.categories
            .push(Category::new("Test defects").with_status(Status::Broken));
        self
    }

    /// Writes the categories.json file.
    pub fn write(self) -> std::io::Result<std::path::PathBuf> {
        let writer = AllureWriter::with_results_dir(&self.results_dir);
        writer.write_categories(&self.categories)
    }
}

impl Default for CategoriesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Creates a new categories builder.
pub fn categories() -> CategoriesBuilder {
    CategoriesBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bdd_step_names() {
        // Just verify the API compiles and returns values correctly
        let result = bdd::given("a value", || 42);
        assert_eq!(result, 42);

        let result = bdd::when("something happens", || "ok");
        assert_eq!(result, "ok");

        let result = bdd::then("we check", || true);
        assert!(result);
    }

    #[test]
    fn test_environment_builder() {
        let builder = environment().set("key1", "value1").set("key2", "value2");

        assert_eq!(builder.properties.len(), 2);
    }

    #[test]
    fn test_categories_builder() {
        let builder = categories()
            .with_product_defects()
            .with_test_defects()
            .with_category(Category::new("Custom").with_status(Status::Skipped));

        assert_eq!(builder.categories.len(), 3);
    }
}
