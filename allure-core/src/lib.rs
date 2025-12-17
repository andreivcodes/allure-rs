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
    log_step, muted, owner, parameter, parameter_excluded, parameter_hidden, parameter_masked,
    parent_suite, run_test, severity, skip, step, story, sub_suite, suite, tag, tags, test_case_id,
    title, tms, with_async_context, with_context, with_test_context, AllureConfig,
    AllureConfigBuilder, TestContext,
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

    /// Attaches an Allure image diff payload.
    pub fn image_diff(name: impl Into<String>, content: &[u8]) {
        attach_binary(name, content, ContentType::ImageDiff);
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
    use crate::runtime::{set_context, take_context, TestContext};
    use crate::writer::AllureWriter;
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

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

    #[test]
    fn test_builder_defaults_use_default_dir() {
        let env_builder = EnvironmentBuilder::default();
        assert_eq!(env_builder.results_dir, DEFAULT_RESULTS_DIR);

        let cat_builder = CategoriesBuilder::default();
        assert_eq!(cat_builder.results_dir, DEFAULT_RESULTS_DIR);
    }

    #[test]
    fn test_attachment_helpers_write_all_types() {
        let temp = tempdir().unwrap();
        let mut ctx = TestContext::new("attach", "module::attach");
        ctx.writer = AllureWriter::with_results_dir(temp.path());
        ctx.writer.init(true).unwrap();

        let file_path = temp.path().join("sample.txt");
        fs::write(&file_path, "sample file").unwrap();

        set_context(ctx);

        attachment::text("Text", "hello");
        attachment::json("Json", &json!({"field": "value"}));
        attachment::binary("Bin", b"\x00\x01", ContentType::Zip);
        attachment::png("Png", b"png-bytes");
        attachment::jpeg("Jpeg", b"jpeg-bytes");
        attachment::html("Html", "<p>hi</p>");
        attachment::xml("Xml", "<xml/>");
        attachment::csv("Csv", "a,b,c");
        attachment::image_diff("Diff", b"{}");
        attachment::file("File", &file_path, None);

        let ctx = take_context().unwrap();
        assert_eq!(ctx.result.attachments.len(), 10);

        let mut kinds = Vec::new();
        for att in &ctx.result.attachments {
            let path = temp.path().join(&att.source);
            assert!(path.exists(), "attachment file missing: {}", att.source);
            kinds.push(att.r#type.clone());
        }

        let has = |mime: &str| kinds.iter().any(|t| t.as_deref() == Some(mime));
        assert!(has("text/plain"));
        assert!(has("application/json"));
        assert!(has("application/zip"));
        assert!(has("image/png"));
        assert!(has("image/jpeg"));
        assert!(has("text/html"));
        assert!(has("application/xml"));
        assert!(has("text/csv"));
        assert!(has("application/vnd.allure.image.diff"));
    }

    #[test]
    fn test_environment_builder_set_from_env_and_write() {
        let temp = tempdir().unwrap();
        std::env::set_var("ALLURE_ENV_TEST_KEY", "from_env");

        let path = environment()
            .results_dir(temp.path().to_string_lossy().to_string())
            .set("key", "value")
            .set_from_env("env_key", "ALLURE_ENV_TEST_KEY")
            .write()
            .unwrap();

        let contents = fs::read_to_string(path).unwrap();
        assert!(contents.contains("key=value"));
        assert!(contents.contains("env_key=from_env"));
    }

    #[test]
    fn test_categories_builder_write_includes_defaults() {
        let temp = tempdir().unwrap();
        let path = categories()
            .results_dir(temp.path().to_string_lossy().to_string())
            .with_product_defects()
            .with_test_defects()
            .with_category(
                Category::new("Custom")
                    .with_status(Status::Skipped)
                    .with_message_regex("oops")
                    .with_trace_regex("trace")
                    .as_flaky(),
            )
            .write()
            .unwrap();

        let contents = fs::read_to_string(path).unwrap();
        let cats: Vec<Category> = serde_json::from_str(&contents).unwrap();
        assert_eq!(cats.len(), 3);
        assert!(cats.iter().any(|c| c.name == "Product defects"));
        assert!(cats.iter().any(|c| c.name == "Test defects"));
        let custom = cats.iter().find(|c| c.name == "Custom").unwrap();
        assert_eq!(custom.matched_statuses, vec![Status::Skipped]);
        assert_eq!(custom.message_regex.as_deref(), Some("oops"));
        assert_eq!(custom.trace_regex.as_deref(), Some("trace"));
        assert_eq!(custom.flaky, Some(true));
    }
}
