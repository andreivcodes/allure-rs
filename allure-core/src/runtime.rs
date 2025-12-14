//! Runtime context management for tracking test execution state.
//!
//! This module provides thread-local storage for synchronous tests and
//! optional tokio task-local storage for async tests.

use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::OnceLock;

use crate::enums::{ContentType, LabelName, LinkType, Severity, Status};
use crate::model::{Attachment, Label, StepResult, TestResult};
use crate::writer::{compute_history_id, generate_uuid, AllureWriter};

/// Global configuration for the Allure runtime.
static CONFIG: OnceLock<AllureConfig> = OnceLock::new();

/// Configuration for the Allure runtime.
#[derive(Debug, Clone)]
pub struct AllureConfig {
    /// Directory where results are written.
    pub results_dir: String,
    /// Whether to clean the results directory on init.
    pub clean_results: bool,
}

impl Default for AllureConfig {
    fn default() -> Self {
        Self {
            results_dir: crate::writer::DEFAULT_RESULTS_DIR.to_string(),
            clean_results: true,
        }
    }
}

/// Builder for configuring the Allure runtime.
#[derive(Debug, Default)]
pub struct AllureConfigBuilder {
    config: AllureConfig,
}

impl AllureConfigBuilder {
    /// Creates a new configuration builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the results directory.
    pub fn results_dir(mut self, path: impl Into<String>) -> Self {
        self.config.results_dir = path.into();
        self
    }

    /// Sets whether to clean the results directory.
    pub fn clean_results(mut self, clean: bool) -> Self {
        self.config.clean_results = clean;
        self
    }

    /// Initializes the Allure runtime with this configuration.
    pub fn init(self) -> std::io::Result<()> {
        let writer = AllureWriter::with_results_dir(&self.config.results_dir);
        writer.init(self.config.clean_results)?;
        CONFIG.set(self.config).ok();
        Ok(())
    }
}

/// Configures the Allure runtime.
pub fn configure() -> AllureConfigBuilder {
    AllureConfigBuilder::new()
}

/// Gets the current configuration or the default.
pub fn get_config() -> AllureConfig {
    CONFIG.get().cloned().unwrap_or_default()
}

/// Test context holding the current test result and step stack.
#[derive(Debug)]
pub struct TestContext {
    /// The current test result being built.
    pub result: TestResult,
    /// Stack of active steps (for nested steps).
    pub step_stack: Vec<StepResult>,
    /// The writer for this context.
    pub writer: AllureWriter,
}

impl TestContext {
    /// Creates a new test context.
    pub fn new(name: impl Into<String>, full_name: impl Into<String>) -> Self {
        let config = get_config();
        let uuid = generate_uuid();
        let mut result = TestResult::new(uuid, name.into());
        result.full_name = Some(full_name.into());

        // Add default labels
        result.labels.push(Label::language("rust"));
        result.labels.push(Label::framework("allure-rs"));

        // Add host and thread labels
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            result.labels.push(Label::host(hostname));
        } else if let Ok(hostname) = hostname::get() {
            if let Some(name) = hostname.to_str() {
                result.labels.push(Label::host(name));
            }
        }

        let thread_id = format!("{:?}", std::thread::current().id());
        result.labels.push(Label::thread(thread_id));

        Self {
            result,
            step_stack: Vec::new(),
            writer: AllureWriter::with_results_dir(config.results_dir),
        }
    }

    /// Adds a label to the current test.
    pub fn add_label(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.result.add_label(name, value);
    }

    /// Adds a label using a reserved name.
    pub fn add_label_name(&mut self, name: LabelName, value: impl Into<String>) {
        self.result.add_label_name(name, value);
    }

    /// Adds a link to the current test.
    pub fn add_link(&mut self, url: impl Into<String>, name: Option<String>, link_type: LinkType) {
        self.result.add_link(url, name, link_type);
    }

    /// Adds a parameter to the current test or step.
    pub fn add_parameter(&mut self, name: impl Into<String>, value: impl Into<String>) {
        if let Some(step) = self.step_stack.last_mut() {
            step.add_parameter(name, value);
        } else {
            self.result.add_parameter(name, value);
        }
    }

    /// Adds an attachment to the current test or step.
    pub fn add_attachment(&mut self, attachment: Attachment) {
        if let Some(step) = self.step_stack.last_mut() {
            step.add_attachment(attachment);
        } else {
            self.result.add_attachment(attachment);
        }
    }

    /// Starts a new step.
    pub fn start_step(&mut self, name: impl Into<String>) {
        let step = StepResult::new(name);
        self.step_stack.push(step);
    }

    /// Finishes the current step with the given status.
    pub fn finish_step(&mut self, status: Status, message: Option<String>, trace: Option<String>) {
        if let Some(mut step) = self.step_stack.pop() {
            match status {
                Status::Passed => step.pass(),
                Status::Failed => step.fail(message, trace),
                Status::Broken => step.broken(message, trace),
                _ => {
                    step.status = status;
                    step.stage = crate::enums::Stage::Finished;
                    step.stop = crate::model::current_time_ms();
                }
            }

            // Add the finished step to the parent (either another step or the test result)
            if let Some(parent_step) = self.step_stack.last_mut() {
                parent_step.add_step(step);
            } else {
                self.result.add_step(step);
            }
        }
    }

    /// Computes and sets the history ID based on the full name and parameters.
    pub fn compute_history_id(&mut self) {
        if let Some(ref full_name) = self.result.full_name {
            let history_id = compute_history_id(full_name, &self.result.parameters);
            self.result.history_id = Some(history_id);
        }
    }

    /// Finishes the test with the given status and writes the result.
    pub fn finish(&mut self, status: Status, message: Option<String>, trace: Option<String>) {
        // Finish any remaining open steps
        while !self.step_stack.is_empty() {
            self.finish_step(Status::Broken, Some("Step not completed".to_string()), None);
        }

        // Compute history ID before finishing
        self.compute_history_id();

        match status {
            Status::Passed => self.result.pass(),
            Status::Failed => self.result.fail(message, trace),
            Status::Broken => self.result.broken(message, trace),
            _ => {
                self.result.status = status;
                self.result.finish();
            }
        }

        // Write the result
        if let Err(e) = self.writer.write_test_result(&self.result) {
            eprintln!("Failed to write Allure test result: {}", e);
        }
    }

    /// Creates a text attachment.
    pub fn attach_text(&mut self, name: impl Into<String>, content: impl AsRef<str>) {
        match self.writer.write_text_attachment(name, content) {
            Ok(attachment) => self.add_attachment(attachment),
            Err(e) => eprintln!("Failed to write text attachment: {}", e),
        }
    }

    /// Creates a JSON attachment.
    pub fn attach_json<T: serde::Serialize>(&mut self, name: impl Into<String>, value: &T) {
        match self.writer.write_json_attachment(name, value) {
            Ok(attachment) => self.add_attachment(attachment),
            Err(e) => eprintln!("Failed to write JSON attachment: {}", e),
        }
    }

    /// Creates a binary attachment.
    pub fn attach_binary(
        &mut self,
        name: impl Into<String>,
        content: &[u8],
        content_type: ContentType,
    ) {
        match self
            .writer
            .write_binary_attachment(name, content, content_type)
        {
            Ok(attachment) => self.add_attachment(attachment),
            Err(e) => eprintln!("Failed to write binary attachment: {}", e),
        }
    }

    /// Attaches a file from the filesystem.
    pub fn attach_file(
        &mut self,
        name: impl Into<String>,
        path: impl AsRef<std::path::Path>,
        content_type: Option<ContentType>,
    ) {
        match self.writer.copy_file_attachment(name, path, content_type) {
            Ok(attachment) => self.add_attachment(attachment),
            Err(e) => eprintln!("Failed to copy file attachment: {}", e),
        }
    }
}

// Thread-local storage for synchronous tests
thread_local! {
    static CURRENT_CONTEXT: RefCell<Option<TestContext>> = const { RefCell::new(None) };
}

/// Sets the current test context for the thread.
pub fn set_context(ctx: TestContext) {
    CURRENT_CONTEXT.with(|c| {
        *c.borrow_mut() = Some(ctx);
    });
}

/// Takes the current test context, leaving None in its place.
pub fn take_context() -> Option<TestContext> {
    CURRENT_CONTEXT.with(|c| c.borrow_mut().take())
}

/// Executes a function with the current test context.
pub fn with_context<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut TestContext) -> R,
{
    CURRENT_CONTEXT.with(|c| {
        let mut ctx = c.borrow_mut();
        ctx.as_mut().map(f)
    })
}

/// Runs a test function with Allure tracking.
pub fn run_test<F>(name: &str, full_name: &str, f: F)
where
    F: FnOnce() + std::panic::UnwindSafe,
{
    let ctx = TestContext::new(name, full_name);
    set_context(ctx);

    let result = catch_unwind(AssertUnwindSafe(f));

    // Extract panic message if there was an error
    let (is_err, panic_payload) = match &result {
        Ok(()) => (false, None),
        Err(panic_info) => {
            let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                Some(s.to_string())
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                Some(s.clone())
            } else {
                Some("Test panicked".to_string())
            };
            (true, msg)
        }
    };

    // Finish the test context
    if let Some(mut ctx) = take_context() {
        if is_err {
            ctx.finish(Status::Failed, panic_payload, None);
        } else {
            ctx.finish(Status::Passed, None, None);
        }
    }

    // Re-panic if the test failed
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

// === Public API functions ===

/// Adds a label to the current test.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::label;
///
/// label("environment", "staging");
/// label("browser", "chrome");
/// ```
pub fn label(name: impl Into<String>, value: impl Into<String>) {
    with_context(|ctx| ctx.add_label(name, value));
}

/// Adds an epic label to the current test.
///
/// Epics represent high-level business capabilities in the BDD hierarchy.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::epic;
///
/// epic("User Management");
/// ```
pub fn epic(name: impl Into<String>) {
    with_context(|ctx| ctx.add_label_name(LabelName::Epic, name));
}

/// Adds a feature label to the current test.
///
/// Features represent specific functionality under an epic.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::feature;
///
/// feature("User Registration");
/// ```
pub fn feature(name: impl Into<String>) {
    with_context(|ctx| ctx.add_label_name(LabelName::Feature, name));
}

/// Adds a story label to the current test.
///
/// Stories represent user stories under a feature.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::story;
///
/// story("User can register with email");
/// ```
pub fn story(name: impl Into<String>) {
    with_context(|ctx| ctx.add_label_name(LabelName::Story, name));
}

/// Adds a suite label to the current test.
pub fn suite(name: impl Into<String>) {
    with_context(|ctx| ctx.add_label_name(LabelName::Suite, name));
}

/// Adds a parent suite label to the current test.
pub fn parent_suite(name: impl Into<String>) {
    with_context(|ctx| ctx.add_label_name(LabelName::ParentSuite, name));
}

/// Adds a sub-suite label to the current test.
pub fn sub_suite(name: impl Into<String>) {
    with_context(|ctx| ctx.add_label_name(LabelName::SubSuite, name));
}

/// Adds a severity label to the current test.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::severity;
/// use allure_core::Severity;
///
/// severity(Severity::Critical);
/// ```
pub fn severity(severity: Severity) {
    with_context(|ctx| ctx.add_label_name(LabelName::Severity, severity.as_str()));
}

/// Adds an owner label to the current test.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::owner;
///
/// owner("platform-team");
/// ```
pub fn owner(name: impl Into<String>) {
    with_context(|ctx| ctx.add_label_name(LabelName::Owner, name));
}

/// Adds a tag label to the current test.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::tag;
///
/// tag("smoke");
/// tag("regression");
/// ```
pub fn tag(name: impl Into<String>) {
    with_context(|ctx| ctx.add_label_name(LabelName::Tag, name));
}

/// Adds multiple tag labels to the current test.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::tags;
///
/// tags(&["smoke", "regression", "api"]);
/// ```
pub fn tags(names: &[&str]) {
    with_context(|ctx| {
        for name in names {
            ctx.add_label_name(LabelName::Tag, *name);
        }
    });
}

/// Adds an Allure ID label to the current test.
pub fn allure_id(id: impl Into<String>) {
    with_context(|ctx| ctx.add_label_name(LabelName::AllureId, id));
}

/// Sets a custom title for the current test.
///
/// This overrides the test name displayed in the Allure report.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::title;
///
/// title("User can login with valid credentials");
/// ```
pub fn title(name: impl Into<String>) {
    with_context(|ctx| ctx.result.name = name.into());
}

/// Sets the test description (markdown).
pub fn description(text: impl Into<String>) {
    with_context(|ctx| ctx.result.description = Some(text.into()));
}

/// Sets the test description (HTML).
pub fn description_html(html: impl Into<String>) {
    with_context(|ctx| ctx.result.description_html = Some(html.into()));
}

/// Adds an issue link to the current test.
pub fn issue(url: impl Into<String>, name: Option<String>) {
    with_context(|ctx| ctx.add_link(url, name, LinkType::Issue));
}

/// Adds a TMS link to the current test.
pub fn tms(url: impl Into<String>, name: Option<String>) {
    with_context(|ctx| ctx.add_link(url, name, LinkType::Tms));
}

/// Adds a generic link to the current test.
pub fn link(url: impl Into<String>, name: Option<String>) {
    with_context(|ctx| ctx.add_link(url, name, LinkType::Default));
}

/// Adds a parameter to the current test or step.
///
/// Parameters are displayed in the Allure report and can be used
/// to understand what inputs were used for a test run.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::parameter;
///
/// parameter("username", "john_doe");
/// parameter("count", 42);
/// ```
pub fn parameter(name: impl Into<String>, value: impl ToString) {
    with_context(|ctx| ctx.add_parameter(name, value.to_string()));
}

/// Executes a step with the given name and body.
///
/// Steps are the building blocks of test reports. They provide
/// a hierarchical view of what the test is doing and can be nested.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::step;
///
/// step("Login to application", || {
///     step("Enter credentials", || {
///         // Enter username and password
///     });
///     step("Click submit", || {
///         // Click the submit button
///     });
/// });
/// ```
///
/// Steps can also return values:
///
/// ```ignore
/// use allure_core::runtime::step;
///
/// let result = step("Calculate result", || {
///     2 + 2
/// });
/// assert_eq!(result, 4);
/// ```
pub fn step<F, R>(name: impl Into<String>, body: F) -> R
where
    F: FnOnce() -> R,
{
    let step_name = name.into();

    with_context(|ctx| ctx.start_step(&step_name));

    let result = catch_unwind(AssertUnwindSafe(body));

    match &result {
        Ok(_) => {
            with_context(|ctx| ctx.finish_step(Status::Passed, None, None));
        }
        Err(panic_info) => {
            let message = if let Some(s) = panic_info.downcast_ref::<&str>() {
                Some(s.to_string())
            } else if let Some(s) = panic_info.downcast_ref::<String>() {
                Some(s.clone())
            } else {
                Some("Step panicked".to_string())
            };
            with_context(|ctx| ctx.finish_step(Status::Failed, message, None));
        }
    }

    match result {
        Ok(value) => value,
        Err(e) => std::panic::resume_unwind(e),
    }
}

/// Logs a step without a body (for simple logging).
///
/// This is useful for logging actions that don't have a body,
/// such as noting an event or state.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::log_step;
/// use allure_core::Status;
///
/// log_step("Database connection established", Status::Passed);
/// log_step("Cache was cleared", Status::Passed);
/// ```
pub fn log_step(name: impl Into<String>, status: Status) {
    with_context(|ctx| {
        ctx.start_step(name);
        ctx.finish_step(status, None, None);
    });
}

/// Attaches text content to the current test or step.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::attach_text;
///
/// attach_text("API Response", r#"{"status": "ok"}"#);
/// attach_text("Log Output", "Test completed successfully");
/// ```
pub fn attach_text(name: impl Into<String>, content: impl AsRef<str>) {
    with_context(|ctx| ctx.attach_text(name, content));
}

/// Attaches JSON content to the current test or step.
///
/// The value is serialized to JSON using serde.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::attach_json;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct User {
///     name: String,
///     email: String,
/// }
///
/// let user = User {
///     name: "John".to_string(),
///     email: "john@example.com".to_string(),
/// };
///
/// attach_json("User Data", &user);
/// ```
pub fn attach_json<T: serde::Serialize>(name: impl Into<String>, value: &T) {
    with_context(|ctx| ctx.attach_json(name, value));
}

/// Attaches binary content to the current test or step.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::attach_binary;
/// use allure_core::ContentType;
///
/// let png_data: &[u8] = &[0x89, 0x50, 0x4E, 0x47]; // PNG header
/// attach_binary("Screenshot", png_data, ContentType::Png);
/// ```
pub fn attach_binary(name: impl Into<String>, content: &[u8], content_type: ContentType) {
    with_context(|ctx| ctx.attach_binary(name, content, content_type));
}

/// Marks the current test as flaky.
///
/// Flaky tests are tests that can fail intermittently due to
/// external factors like network issues or timing problems.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::flaky;
///
/// flaky();
/// // Test code that sometimes fails due to network issues
/// ```
pub fn flaky() {
    with_context(|ctx| {
        let details = ctx
            .result
            .status_details
            .get_or_insert_with(Default::default);
        details.flaky = Some(true);
    });
}

/// Marks the current test as muted.
///
/// Muted tests are tests whose results will not affect the statistics
/// in the Allure report. The test is still executed and documented,
/// but won't impact pass/fail metrics.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::muted;
///
/// muted();
/// // Test code that shouldn't affect statistics
/// ```
pub fn muted() {
    with_context(|ctx| {
        let details = ctx
            .result
            .status_details
            .get_or_insert_with(Default::default);
        details.muted = Some(true);
    });
}

/// Marks the current test as having a known issue.
///
/// This adds an issue link and marks the test status as having a known issue.
///
/// # Example
///
/// ```ignore
/// use allure_core::runtime::known_issue;
///
/// known_issue("https://github.com/example/project/issues/123");
/// ```
pub fn known_issue(issue_id: impl Into<String>) {
    let id = issue_id.into();
    with_context(|ctx| {
        let details = ctx
            .result
            .status_details
            .get_or_insert_with(Default::default);
        details.known = Some(true);
        // Also add as an issue link
        ctx.add_link(&id, Some(id.clone()), LinkType::Issue);
    });
}

/// Sets the display name for the current test.
///
/// This overrides the test name that was set when the test context was created.
pub fn display_name(name: impl Into<String>) {
    with_context(|ctx| ctx.result.name = name.into());
}

/// Sets the test case ID for the current test.
///
/// This is used to link the test to a test case in a test management system.
pub fn test_case_id(id: impl Into<String>) {
    with_context(|ctx| ctx.result.test_case_id = Some(id.into()));
}

/// Attaches a file from the filesystem to the current test or step.
///
/// The file is copied to the Allure results directory.
pub fn attach_file(
    name: impl Into<String>,
    path: impl AsRef<std::path::Path>,
    content_type: Option<ContentType>,
) {
    with_context(|ctx| ctx.attach_file(name, path, content_type));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder() {
        let config = AllureConfigBuilder::new()
            .results_dir("custom-results")
            .clean_results(false)
            .config;

        assert_eq!(config.results_dir, "custom-results");
        assert!(!config.clean_results);
    }

    #[test]
    fn test_context_creation() {
        let ctx = TestContext::new("My Test", "tests::my_test");
        assert_eq!(ctx.result.name, "My Test");
        assert_eq!(ctx.result.full_name, Some("tests::my_test".to_string()));
        assert!(ctx
            .result
            .labels
            .iter()
            .any(|l| l.name == "language" && l.value == "rust"));
    }

    #[test]
    fn test_step_nesting() {
        let mut ctx = TestContext::new("Test", "test::test");

        ctx.start_step("Step 1");
        ctx.start_step("Step 1.1");
        ctx.finish_step(Status::Passed, None, None);
        ctx.finish_step(Status::Passed, None, None);

        assert_eq!(ctx.result.steps.len(), 1);
        assert_eq!(ctx.result.steps[0].name, "Step 1");
        assert_eq!(ctx.result.steps[0].steps.len(), 1);
        assert_eq!(ctx.result.steps[0].steps[0].name, "Step 1.1");
    }

    #[test]
    fn test_thread_local_context() {
        let ctx = TestContext::new("Test", "test::test");
        set_context(ctx);

        with_context(|ctx| {
            ctx.add_label("custom", "value");
        });

        let ctx = take_context().unwrap();
        assert!(ctx
            .result
            .labels
            .iter()
            .any(|l| l.name == "custom" && l.value == "value"));
    }
}
