//! Runtime context management for tracking test execution state.
//!
//! This module provides thread-local storage for synchronous tests and
//! optional tokio task-local storage for async tests.

use std::backtrace::Backtrace;
use std::cell::RefCell;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::OnceLock;
#[cfg(feature = "tokio")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "tokio")]
type SharedAsyncContext = Arc<Mutex<Option<TestContext>>>;
#[cfg(feature = "tokio")]
type GlobalAsyncContext = Mutex<Option<SharedAsyncContext>>;

use crate::enums::{ContentType, LabelName, LinkType, Severity, Status};
use crate::model::{Attachment, Label, Parameter, StepResult, TestResult, TestResultContainer};
use crate::writer::{compute_history_id, generate_uuid, AllureWriter};

/// Global configuration for the Allure runtime.
static CONFIG: OnceLock<AllureConfig> = OnceLock::new();

#[cfg(feature = "tokio")]
tokio::task_local! {
    static TOKIO_CONTEXT: RefCell<Option<SharedAsyncContext>>;
}

#[cfg(feature = "tokio")]
fn global_async_context() -> &'static GlobalAsyncContext {
    static GLOBAL: OnceLock<GlobalAsyncContext> = OnceLock::new();
    GLOBAL.get_or_init(|| Mutex::new(None))
}

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

    /// Adds a parameter with custom options (hidden/masked/excluded).
    pub fn add_parameter_struct(&mut self, parameter: Parameter) {
        if let Some(step) = self.step_stack.last_mut() {
            step.parameters.push(parameter);
        } else {
            self.result.parameters.push(parameter);
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
            Status::Skipped => {
                if message.is_some() || trace.is_some() {
                    self.result.status_details = Some(crate::model::StatusDetails {
                        message,
                        trace,
                        ..Default::default()
                    });
                }
                self.result.status = status;
                self.result.finish();
            }
            _ => {
                self.result.status = status;
                self.result.finish();
            }
        }

        // Write the result
        if let Err(e) = self.writer.write_test_result(&self.result) {
            eprintln!("Failed to write Allure test result: {}", e);
        }

        // Emit a container linking this test (even if no fixtures are present yet)
        let mut container = TestResultContainer::new(generate_uuid());
        container.children.push(self.result.uuid.clone());
        container.start = Some(self.result.start);
        container.stop = Some(self.result.stop);
        if let Err(e) = self.writer.write_container(&container) {
            eprintln!("Failed to write Allure container: {}", e);
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
    #[cfg(feature = "tokio")]
    {
        if let Ok(context) = TOKIO_CONTEXT.try_with(|c| {
            let handle_opt = c.borrow().clone();
            handle_opt.and_then(|handle| {
                let mut guard = handle.lock().unwrap();
                guard.take()
            })
        }) {
            if context.is_some() {
                return context;
            }
        }
    }

    let thread_local = CURRENT_CONTEXT.with(|c| c.borrow_mut().take());
    if thread_local.is_some() {
        return thread_local;
    }

    #[cfg(feature = "tokio")]
    {
        let global = global_async_context().lock().unwrap().clone();
        if let Some(handle) = global {
            let mut guard = handle.lock().unwrap();
            if let Some(ctx) = guard.take() {
                return Some(ctx);
            }
        }
    }

    None
}

/// Executes a function with the current test context.
pub fn with_context<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut TestContext) -> R,
{
    let mut f_opt = Some(f);

    #[cfg(feature = "tokio")]
    {
        if let Ok(result) = TOKIO_CONTEXT.try_with(|c| {
            let handle_opt = c.borrow().clone();
            if let Some(handle) = handle_opt {
                let mut guard = handle.lock().unwrap();
                if let Some(ctx) = guard.as_mut() {
                    if let Some(func) = f_opt.take() {
                        return Some(func(ctx));
                    }
                }
            }
            None
        }) {
            if result.is_some() {
                return result;
            }
        }
    }

    let thread_local = CURRENT_CONTEXT
        .with(|c| {
            let mut ctx = c.borrow_mut();
            if let Some(ctx) = ctx.as_mut() {
                if let Some(func) = f_opt.take() {
                    return Some(func(ctx));
                }
            }
            None
        })
        .or_else(|| {
            #[cfg(feature = "tokio")]
            {
                let handle_opt = global_async_context().lock().unwrap().clone();
                if let Some(handle) = handle_opt {
                    let mut guard = handle.lock().unwrap();
                    if let Some(ctx) = guard.as_mut() {
                        if let Some(func) = f_opt.take() {
                            return Some(func(ctx));
                        }
                    }
                }
            }
            None
        });

    thread_local
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
            let trace = capture_trace();
            ctx.finish(Status::Failed, panic_payload, trace);
        } else {
            ctx.finish(Status::Passed, None, None);
        }
    }

    // Re-panic if the test failed
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

/// Executes a closure with a temporary test context for documentation examples.
///
/// This function is useful for running doc tests that use runtime functions
/// like `step()`, `label()`, etc. without needing the full test infrastructure.
/// No test results are written to disk.
///
/// # Example
///
/// ```
/// use allure_core::runtime::{with_test_context, step, epic};
///
/// with_test_context(|| {
///     epic("My Epic");
///     step("Do something", || {
///         // test code
///     });
/// });
/// ```
#[doc(hidden)]
pub fn with_test_context<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let ctx = TestContext::new("doctest", "doctest::example");
    set_context(ctx);
    let result = f();
    let _ = take_context(); // cleanup without writing
    result
}

// === Public API functions ===

/// Adds a label to the current test.
///
/// # Example
///
/// ```
/// use allure_core::runtime::{with_test_context, label};
///
/// with_test_context(|| {
///     label("environment", "staging");
///     label("browser", "chrome");
/// });
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
/// ```
/// use allure_core::runtime::{with_test_context, epic};
///
/// with_test_context(|| {
///     epic("User Management");
/// });
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
/// ```
/// use allure_core::runtime::{with_test_context, feature};
///
/// with_test_context(|| {
///     feature("User Registration");
/// });
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
/// ```
/// use allure_core::runtime::{with_test_context, story};
///
/// with_test_context(|| {
///     story("User can register with email");
/// });
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
/// ```
/// use allure_core::runtime::{with_test_context, severity};
/// use allure_core::Severity;
///
/// with_test_context(|| {
///     severity(Severity::Critical);
/// });
/// ```
pub fn severity(severity: Severity) {
    with_context(|ctx| ctx.add_label_name(LabelName::Severity, severity.as_str()));
}

/// Adds an owner label to the current test.
///
/// # Example
///
/// ```
/// use allure_core::runtime::{with_test_context, owner};
///
/// with_test_context(|| {
///     owner("platform-team");
/// });
/// ```
pub fn owner(name: impl Into<String>) {
    with_context(|ctx| ctx.add_label_name(LabelName::Owner, name));
}

/// Adds a tag label to the current test.
///
/// # Example
///
/// ```
/// use allure_core::runtime::{with_test_context, tag};
///
/// with_test_context(|| {
///     tag("smoke");
///     tag("regression");
/// });
/// ```
pub fn tag(name: impl Into<String>) {
    with_context(|ctx| ctx.add_label_name(LabelName::Tag, name));
}

/// Adds multiple tag labels to the current test.
///
/// # Example
///
/// ```
/// use allure_core::runtime::{with_test_context, tags};
///
/// with_test_context(|| {
///     tags(&["smoke", "regression", "api"]);
/// });
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
/// ```
/// use allure_core::runtime::{with_test_context, title};
///
/// with_test_context(|| {
///     title("User can login with valid credentials");
/// });
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
/// ```
/// use allure_core::runtime::{with_test_context, parameter};
///
/// with_test_context(|| {
///     parameter("username", "john_doe");
///     parameter("count", 42);
/// });
/// ```
pub fn parameter(name: impl Into<String>, value: impl ToString) {
    with_context(|ctx| ctx.add_parameter(name, value.to_string()));
}

/// Adds a parameter hidden from display (value not shown in the report).
pub fn parameter_hidden(name: impl Into<String>, value: impl ToString) {
    with_context(|ctx| ctx.add_parameter_struct(Parameter::hidden(name, value.to_string())));
}

/// Adds a parameter with a masked value (e.g., passwords).
pub fn parameter_masked(name: impl Into<String>, value: impl ToString) {
    with_context(|ctx| ctx.add_parameter_struct(Parameter::masked(name, value.to_string())));
}

/// Adds a parameter excluded from history ID calculation.
pub fn parameter_excluded(name: impl Into<String>, value: impl ToString) {
    with_context(|ctx| ctx.add_parameter_struct(Parameter::excluded(name, value.to_string())));
}

/// Executes a step with the given name and body.
///
/// Steps are the building blocks of test reports. They provide
/// a hierarchical view of what the test is doing and can be nested.
///
/// # Example
///
/// ```
/// use allure_core::runtime::{with_test_context, step};
///
/// with_test_context(|| {
///     step("Login to application", || {
///         step("Enter credentials", || {
///             // Enter username and password
///         });
///         step("Click submit", || {
///             // Click the submit button
///         });
///     });
/// });
/// ```
///
/// Steps can also return values:
///
/// ```
/// use allure_core::runtime::{with_test_context, step};
///
/// with_test_context(|| {
///     let result = step("Calculate result", || {
///         2 + 2
///     });
///     assert_eq!(result, 4);
/// });
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
            let trace = capture_trace();
            with_context(|ctx| ctx.finish_step(Status::Failed, message, trace));
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
/// ```
/// use allure_core::runtime::{with_test_context, log_step};
/// use allure_core::Status;
///
/// with_test_context(|| {
///     log_step("Database connection established", Status::Passed);
///     log_step("Cache was cleared", Status::Passed);
/// });
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
/// ```
/// use allure_core::runtime::{with_test_context, attach_text};
///
/// with_test_context(|| {
///     attach_text("API Response", r#"{"status": "ok"}"#);
///     attach_text("Log Output", "Test completed successfully");
/// });
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
/// ```
/// use allure_core::runtime::{with_test_context, attach_json};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct User {
///     name: String,
///     email: String,
/// }
///
/// with_test_context(|| {
///     let user = User {
///         name: "John".to_string(),
///         email: "john@example.com".to_string(),
///     };
///     attach_json("User Data", &user);
/// });
/// ```
pub fn attach_json<T: serde::Serialize>(name: impl Into<String>, value: &T) {
    with_context(|ctx| ctx.attach_json(name, value));
}

/// Attaches binary content to the current test or step.
///
/// # Example
///
/// ```
/// use allure_core::runtime::{with_test_context, attach_binary};
/// use allure_core::ContentType;
///
/// with_test_context(|| {
///     let png_data: &[u8] = &[0x89, 0x50, 0x4E, 0x47]; // PNG header
///     attach_binary("Screenshot", png_data, ContentType::Png);
/// });
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
/// ```
/// use allure_core::runtime::{with_test_context, flaky};
///
/// with_test_context(|| {
///     flaky();
///     // Test code that sometimes fails due to network issues
/// });
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
/// ```
/// use allure_core::runtime::{with_test_context, muted};
///
/// with_test_context(|| {
///     muted();
///     // Test code that shouldn't affect statistics
/// });
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
/// ```
/// use allure_core::runtime::{with_test_context, known_issue};
///
/// with_test_context(|| {
///     known_issue("https://github.com/example/project/issues/123");
/// });
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

/// Marks the current test as skipped and finalizes the result.
pub fn skip(reason: impl Into<String>) {
    let reason = reason.into();
    if let Some(mut ctx) = take_context() {
        ctx.finish(Status::Skipped, Some(reason), None);
    }
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

/// Captures a backtrace as a string when available.
fn capture_trace() -> Option<String> {
    let bt = Backtrace::force_capture();
    let snapshot = format!("{bt:?}");
    if snapshot.contains("disabled") {
        return None;
    }
    Some(snapshot)
}

/// Executes an async block with a task-local test context (tokio only).
#[cfg(feature = "tokio")]
pub async fn with_async_context<F, R>(ctx: TestContext, fut: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let handle = Arc::new(Mutex::new(Some(ctx)));
    {
        let mut slot = global_async_context().lock().unwrap();
        *slot = Some(handle.clone());
    }

    let cell = RefCell::new(Some(handle));
    let result = TOKIO_CONTEXT.scope(cell, fut).await;

    let mut slot = global_async_context().lock().unwrap();
    slot.take();

    result
}

/// Executes an async block with a thread-local test context (non-tokio fallback).
#[cfg(not(feature = "tokio"))]
pub async fn with_async_context<F, R>(ctx: TestContext, fut: F) -> R
where
    F: std::future::Future<Output = R>,
{
    set_context(ctx);
    let result = fut.await;
    let _ = take_context();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::path::PathBuf;

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

    #[test]
    fn test_capture_trace_runs() {
        // We only assert that it does not panic and returns an Option.
        let _maybe_trace = capture_trace();
    }

    #[test]
    fn test_run_test_writes_results_and_container_on_panic() {
        let desired_dir = PathBuf::from("target/allure-runtime-tests");
        let _ = std::fs::remove_dir_all(&desired_dir);

        let config_ref = CONFIG.get_or_init(|| AllureConfig {
            results_dir: desired_dir.to_string_lossy().to_string(),
            clean_results: true,
        });
        let dir = PathBuf::from(&config_ref.results_dir);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let outcome = std::panic::catch_unwind(|| {
            run_test("panic_test", "runtime::panic_test", || {
                panic!("runtime boom");
            });
        });
        assert!(outcome.is_err());

        let mut result_files = Vec::new();
        let mut container_files = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                if name.contains("-result.json") {
                    result_files.push(path.clone());
                } else if name.contains("-container.json") {
                    container_files.push(path.clone());
                }
            }
        }

        assert!(!result_files.is_empty());
        assert!(!container_files.is_empty());

        let result_json: Value =
            serde_json::from_str(&std::fs::read_to_string(&result_files[0]).unwrap()).unwrap();
        assert_eq!(result_json["status"], "failed");
        assert!(result_json["statusDetails"]["message"]
            .as_str()
            .unwrap()
            .contains("runtime boom"));
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(flavor = "current_thread")]
    async fn test_take_context_reads_tokio_task_local() {
        let ctx = TestContext::new("tokio_ctx", "module::tokio_ctx");
        let taken = with_async_context(ctx, async {
            let inner = take_context();
            assert!(inner.is_some());
            inner.unwrap().result.name
        })
        .await;
        assert_eq!(taken, "tokio_ctx");
    }

    #[cfg(feature = "tokio")]
    #[tokio::test(flavor = "current_thread")]
    async fn test_with_context_uses_tokio_task_local() {
        let ctx = TestContext::new("tokio_ctx", "module::tokio_ctx");
        with_async_context(ctx, async {
            let mut seen = None;
            with_context(|c| {
                seen = Some(c.result.name.clone());
            });
            assert_eq!(seen.as_deref(), Some("tokio_ctx"));
        })
        .await;
    }

    #[test]
    fn test_with_test_context_clears_after_use() {
        with_test_context(|| {
            label("temp", "value");
        });
        assert!(take_context().is_none());
    }

    #[test]
    fn test_tags_and_metadata_helpers() {
        let ctx = TestContext::new("meta", "module::meta");
        set_context(ctx);

        label("env", "staging");
        tags(&["smoke", "api"]);
        title("Custom Title");
        description("Markdown");
        description_html("<p>HTML</p>");
        test_case_id("TC-1");

        let ctx = take_context().unwrap();
        assert_eq!(ctx.result.name, "Custom Title");
        assert_eq!(ctx.result.description.as_deref(), Some("Markdown"));
        assert_eq!(ctx.result.description_html.as_deref(), Some("<p>HTML</p>"));
        assert_eq!(ctx.result.test_case_id.as_deref(), Some("TC-1"));
        assert!(ctx.result.labels.iter().any(|l| l.value == "staging"));
        assert!(ctx.result.labels.iter().any(|l| l.value == "smoke"));
        assert!(ctx.result.labels.iter().any(|l| l.value == "api"));
    }

    #[test]
    fn test_step_failure_records_message_and_rethrows() {
        let ctx = TestContext::new("step_fail", "module::step_fail");
        set_context(ctx);

        let result = std::panic::catch_unwind(|| {
            step("will panic", || panic!("boom step"));
        });
        assert!(result.is_err());

        let ctx = take_context().unwrap();
        assert_eq!(ctx.result.steps.len(), 1);
        let step = &ctx.result.steps[0];
        assert_eq!(step.status, Status::Failed);
        assert!(step
            .status_details
            .as_ref()
            .unwrap()
            .message
            .as_ref()
            .unwrap()
            .contains("boom step"));
    }

    #[test]
    fn test_finish_step_skipped_branch() {
        let mut ctx = TestContext::new("skip_step", "module::skip_step");
        ctx.start_step("inner");
        ctx.finish_step(
            Status::Skipped,
            Some("not run".into()),
            Some("trace".into()),
        );
        assert_eq!(ctx.result.steps[0].status, Status::Skipped);
        assert_eq!(ctx.result.steps[0].stage, crate::enums::Stage::Finished);
    }

    #[test]
    fn test_finish_step_broken_and_unknown_branches() {
        let mut ctx = TestContext::new("broken_step", "module::broken_step");
        ctx.start_step("broken");
        ctx.finish_step(Status::Broken, Some("oops".into()), None);
        assert_eq!(ctx.result.steps[0].status, Status::Broken);
        assert!(ctx.result.steps[0]
            .status_details
            .as_ref()
            .unwrap()
            .message
            .as_ref()
            .unwrap()
            .contains("oops"));

        ctx.start_step("unknown");
        ctx.finish_step(Status::Unknown, None, None);
        assert_eq!(ctx.result.steps[1].status, Status::Unknown);
        assert_eq!(ctx.result.steps[1].stage, crate::enums::Stage::Finished);
    }

    #[test]
    fn test_muted_sets_flag() {
        let ctx = TestContext::new("muted_test", "module::muted_test");
        set_context(ctx);
        muted();
        let ctx = take_context().unwrap();
        let details = ctx.result.status_details.unwrap();
        assert_eq!(details.muted, Some(true));
    }

    #[test]
    fn test_host_env_override_used_in_context_creation() {
        std::env::set_var("HOSTNAME", "test-host");
        let ctx = TestContext::new("hosted", "module::hosted");
        assert!(ctx
            .result
            .labels
            .iter()
            .any(|l| l.name == "host" && l.value == "test-host"));
    }

    #[test]
    fn test_add_parameter_struct_applies_to_steps() {
        let mut ctx = TestContext::new("params", "module::params");
        ctx.start_step("outer");
        ctx.add_parameter_struct(crate::model::Parameter::excluded("k", "v"));
        assert_eq!(ctx.step_stack[0].parameters.len(), 1);
        assert_eq!(ctx.step_stack[0].parameters[0].excluded, Some(true));
    }

    #[test]
    fn test_finish_writes_and_breaks_unfinished_steps() {
        let temp = tempfile::tempdir().unwrap();
        CONFIG.get_or_init(|| AllureConfig {
            results_dir: temp.path().to_string_lossy().to_string(),
            clean_results: true,
        });
        let mut ctx = TestContext::new("unclosed", "module::unclosed");
        ctx.start_step("still running");
        ctx.finish(Status::Passed, None, None);
        assert_eq!(ctx.result.steps[0].status, Status::Broken);
        assert!(ctx.result.steps[0]
            .status_details
            .as_ref()
            .unwrap()
            .message
            .as_ref()
            .unwrap()
            .contains("Step not completed"));
    }

    #[test]
    fn test_finish_handles_broken_status_with_details() {
        let temp = tempfile::tempdir().unwrap();
        CONFIG.get_or_init(|| AllureConfig {
            results_dir: temp.path().to_string_lossy().to_string(),
            clean_results: true,
        });
        let mut ctx = TestContext::new("broken_test", "module::broken_test");
        ctx.finish(Status::Broken, Some("fail".into()), Some("trace".into()));
        assert_eq!(ctx.result.status, Status::Broken);
        let details = ctx.result.status_details.as_ref().unwrap();
        assert_eq!(details.message.as_deref(), Some("fail"));
        assert_eq!(details.trace.as_deref(), Some("trace"));
    }

    #[test]
    fn test_context_creation_uses_hostname_when_env_missing() {
        // Temporarily remove HOSTNAME to exercise hostname crate path
        let original = std::env::var("HOSTNAME").ok();
        std::env::remove_var("HOSTNAME");
        let ctx = TestContext::new("host", "module::host");
        if let Some(orig) = original {
            std::env::set_var("HOSTNAME", orig);
        }
        assert!(ctx.result.labels.iter().any(|l| l.name == "host"));
    }
}
