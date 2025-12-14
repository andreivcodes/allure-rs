//! Allure data model types for test results, steps, attachments, and containers.

use serde::{Deserialize, Serialize};

use crate::enums::{LabelName, LinkType, ParameterMode, Severity, Stage, Status};

/// Main test result structure written to `{uuid}-result.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    /// Unique identifier for this test result
    pub uuid: String,

    /// History ID for tracking test across runs (MD5 of fullName + parameters)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_id: Option<String>,

    /// Test case ID for Allure TestOps integration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_case_id: Option<String>,

    /// Test name (display title)
    pub name: String,

    /// Fully qualified test name (module::function)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,

    /// Markdown description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// HTML description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description_html: Option<String>,

    /// Test result status
    pub status: Status,

    /// Additional status details (message, trace, flaky, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_details: Option<StatusDetails>,

    /// Test execution stage
    pub stage: Stage,

    /// Test steps
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<StepResult>,

    /// Test attachments
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,

    /// Test parameters
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,

    /// Test labels (tags, severity, owner, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<Label>,

    /// External links (issues, TMS, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,

    /// Test start time (Unix timestamp in milliseconds)
    pub start: i64,

    /// Test stop time (Unix timestamp in milliseconds)
    pub stop: i64,
}

impl TestResult {
    /// Creates a new test result with the given name and UUID.
    pub fn new(uuid: String, name: String) -> Self {
        let now = current_time_ms();
        Self {
            uuid,
            history_id: None,
            test_case_id: None,
            name,
            full_name: None,
            description: None,
            description_html: None,
            status: Status::Unknown,
            status_details: None,
            stage: Stage::Running,
            steps: Vec::new(),
            attachments: Vec::new(),
            parameters: Vec::new(),
            labels: Vec::new(),
            links: Vec::new(),
            start: now,
            stop: now,
        }
    }

    /// Adds a label to the test result.
    pub fn add_label(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.labels.push(Label {
            name: name.into(),
            value: value.into(),
        });
    }

    /// Adds a label using a reserved label name.
    pub fn add_label_name(&mut self, name: LabelName, value: impl Into<String>) {
        self.add_label(name.as_str(), value);
    }

    /// Adds a link to the test result.
    pub fn add_link(&mut self, url: impl Into<String>, name: Option<String>, link_type: LinkType) {
        self.links.push(Link {
            name,
            url: url.into(),
            r#type: Some(link_type),
        });
    }

    /// Adds a parameter to the test result.
    pub fn add_parameter(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.parameters.push(Parameter {
            name: name.into(),
            value: value.into(),
            excluded: None,
            mode: None,
        });
    }

    /// Adds an attachment to the test result.
    pub fn add_attachment(&mut self, attachment: Attachment) {
        self.attachments.push(attachment);
    }

    /// Adds a step to the test result.
    pub fn add_step(&mut self, step: StepResult) {
        self.steps.push(step);
    }

    /// Sets the test status.
    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    /// Marks the test as finished with the current time.
    pub fn finish(&mut self) {
        self.stop = current_time_ms();
        self.stage = Stage::Finished;
    }

    /// Marks the test as passed.
    pub fn pass(&mut self) {
        self.status = Status::Passed;
        self.finish();
    }

    /// Marks the test as failed with an optional message.
    pub fn fail(&mut self, message: Option<String>, trace: Option<String>) {
        self.status = Status::Failed;
        if message.is_some() || trace.is_some() {
            self.status_details = Some(StatusDetails {
                message,
                trace,
                ..Default::default()
            });
        }
        self.finish();
    }

    /// Marks the test as broken with an optional message.
    pub fn broken(&mut self, message: Option<String>, trace: Option<String>) {
        self.status = Status::Broken;
        if message.is_some() || trace.is_some() {
            self.status_details = Some(StatusDetails {
                message,
                trace,
                ..Default::default()
            });
        }
        self.finish();
    }
}

/// Step result within a test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepResult {
    /// Optional UUID for the step
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,

    /// Step name (display title)
    pub name: String,

    /// Step status
    pub status: Status,

    /// Additional status details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_details: Option<StatusDetails>,

    /// Step execution stage
    pub stage: Stage,

    /// Nested steps
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<StepResult>,

    /// Step attachments
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,

    /// Step parameters
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,

    /// Step start time (Unix timestamp in milliseconds)
    pub start: i64,

    /// Step stop time (Unix timestamp in milliseconds)
    pub stop: i64,
}

impl StepResult {
    /// Creates a new step with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        let now = current_time_ms();
        Self {
            uuid: None,
            name: name.into(),
            status: Status::Unknown,
            status_details: None,
            stage: Stage::Running,
            steps: Vec::new(),
            attachments: Vec::new(),
            parameters: Vec::new(),
            start: now,
            stop: now,
        }
    }

    /// Adds a nested step.
    pub fn add_step(&mut self, step: StepResult) {
        self.steps.push(step);
    }

    /// Adds an attachment to the step.
    pub fn add_attachment(&mut self, attachment: Attachment) {
        self.attachments.push(attachment);
    }

    /// Adds a parameter to the step.
    pub fn add_parameter(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.parameters.push(Parameter {
            name: name.into(),
            value: value.into(),
            excluded: None,
            mode: None,
        });
    }

    /// Marks the step as passed.
    pub fn pass(&mut self) {
        self.status = Status::Passed;
        self.stage = Stage::Finished;
        self.stop = current_time_ms();
    }

    /// Marks the step as failed.
    pub fn fail(&mut self, message: Option<String>, trace: Option<String>) {
        self.status = Status::Failed;
        self.stage = Stage::Finished;
        self.stop = current_time_ms();
        if message.is_some() || trace.is_some() {
            self.status_details = Some(StatusDetails {
                message,
                trace,
                ..Default::default()
            });
        }
    }

    /// Marks the step as broken.
    pub fn broken(&mut self, message: Option<String>, trace: Option<String>) {
        self.status = Status::Broken;
        self.stage = Stage::Finished;
        self.stop = current_time_ms();
        if message.is_some() || trace.is_some() {
            self.status_details = Some(StatusDetails {
                message,
                trace,
                ..Default::default()
            });
        }
    }
}

/// Additional status details for test results and steps.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusDetails {
    /// Whether this is a known issue
    #[serde(skip_serializing_if = "Option::is_none")]
    pub known: Option<bool>,

    /// Whether the test is muted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub muted: Option<bool>,

    /// Whether the test is flaky
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flaky: Option<bool>,

    /// Error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Stack trace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,
}

/// Label for categorizing and filtering tests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    /// Label name (can be a reserved name or custom)
    pub name: String,

    /// Label value
    pub value: String,
}

impl Label {
    /// Creates a new label.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Creates a label from a reserved label name.
    pub fn from_name(name: LabelName, value: impl Into<String>) -> Self {
        Self::new(name.as_str(), value)
    }

    /// Creates an epic label.
    pub fn epic(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::Epic, value)
    }

    /// Creates a feature label.
    pub fn feature(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::Feature, value)
    }

    /// Creates a story label.
    pub fn story(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::Story, value)
    }

    /// Creates a suite label.
    pub fn suite(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::Suite, value)
    }

    /// Creates a parent suite label.
    pub fn parent_suite(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::ParentSuite, value)
    }

    /// Creates a sub-suite label.
    pub fn sub_suite(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::SubSuite, value)
    }

    /// Creates a severity label.
    pub fn severity(severity: Severity) -> Self {
        Self::from_name(LabelName::Severity, severity.as_str())
    }

    /// Creates an owner label.
    pub fn owner(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::Owner, value)
    }

    /// Creates a tag label.
    pub fn tag(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::Tag, value)
    }

    /// Creates an Allure ID label.
    pub fn allure_id(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::AllureId, value)
    }

    /// Creates a host label.
    pub fn host(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::Host, value)
    }

    /// Creates a thread label.
    pub fn thread(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::Thread, value)
    }

    /// Creates a framework label.
    pub fn framework(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::Framework, value)
    }

    /// Creates a language label.
    pub fn language(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::Language, value)
    }

    /// Creates a package label.
    pub fn package(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::Package, value)
    }

    /// Creates a test class label.
    pub fn test_class(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::TestClass, value)
    }

    /// Creates a test method label.
    pub fn test_method(value: impl Into<String>) -> Self {
        Self::from_name(LabelName::TestMethod, value)
    }
}

/// External link associated with a test.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// Link display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Link URL
    pub url: String,

    /// Link type (issue, tms, or custom)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<LinkType>,
}

impl Link {
    /// Creates a new link.
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            name: None,
            url: url.into(),
            r#type: None,
        }
    }

    /// Creates a link with a name.
    pub fn with_name(url: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            url: url.into(),
            r#type: None,
        }
    }

    /// Creates an issue link.
    pub fn issue(url: impl Into<String>, name: Option<String>) -> Self {
        Self {
            name,
            url: url.into(),
            r#type: Some(LinkType::Issue),
        }
    }

    /// Creates a TMS (Test Management System) link.
    pub fn tms(url: impl Into<String>, name: Option<String>) -> Self {
        Self {
            name,
            url: url.into(),
            r#type: Some(LinkType::Tms),
        }
    }
}

/// Test parameter with optional display options.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name
    pub name: String,

    /// Parameter value
    pub value: String,

    /// Whether to exclude from history ID calculation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded: Option<bool>,

    /// Display mode (default, hidden, masked)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ParameterMode>,
}

impl Parameter {
    /// Creates a new parameter.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            excluded: None,
            mode: None,
        }
    }

    /// Creates a parameter that is excluded from history ID calculation.
    pub fn excluded(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            excluded: Some(true),
            mode: None,
        }
    }

    /// Creates a hidden parameter.
    pub fn hidden(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            excluded: None,
            mode: Some(ParameterMode::Hidden),
        }
    }

    /// Creates a masked parameter (for sensitive values).
    pub fn masked(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            excluded: None,
            mode: Some(ParameterMode::Masked),
        }
    }
}

/// Attachment file reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    /// Attachment display name
    pub name: String,

    /// Source file name (UUID-based)
    pub source: String,

    /// MIME type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

impl Attachment {
    /// Creates a new attachment.
    pub fn new(
        name: impl Into<String>,
        source: impl Into<String>,
        mime_type: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            source: source.into(),
            r#type: mime_type,
        }
    }
}

/// Container for test fixtures (setup/teardown).
/// Written to `{uuid}-container.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResultContainer {
    /// Unique identifier for this container
    pub uuid: String,

    /// Container name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// UUIDs of test results that use this fixture
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<String>,

    /// Setup/before fixtures
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub befores: Vec<FixtureResult>,

    /// Teardown/after fixtures
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub afters: Vec<FixtureResult>,

    /// Container start time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,

    /// Container stop time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<i64>,
}

impl TestResultContainer {
    /// Creates a new container with the given UUID.
    pub fn new(uuid: String) -> Self {
        Self {
            uuid,
            name: None,
            children: Vec::new(),
            befores: Vec::new(),
            afters: Vec::new(),
            start: None,
            stop: None,
        }
    }

    /// Adds a test result UUID as a child of this container.
    pub fn add_child(&mut self, test_uuid: String) {
        self.children.push(test_uuid);
    }

    /// Adds a before fixture.
    pub fn add_before(&mut self, fixture: FixtureResult) {
        self.befores.push(fixture);
    }

    /// Adds an after fixture.
    pub fn add_after(&mut self, fixture: FixtureResult) {
        self.afters.push(fixture);
    }
}

/// Fixture result (setup or teardown).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FixtureResult {
    /// Fixture name
    pub name: String,

    /// Fixture status
    pub status: Status,

    /// Additional status details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_details: Option<StatusDetails>,

    /// Fixture execution stage
    pub stage: Stage,

    /// Nested steps
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<StepResult>,

    /// Fixture attachments
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachments: Vec<Attachment>,

    /// Fixture parameters
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,

    /// Fixture start time
    pub start: i64,

    /// Fixture stop time
    pub stop: i64,
}

impl FixtureResult {
    /// Creates a new fixture with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        let now = current_time_ms();
        Self {
            name: name.into(),
            status: Status::Unknown,
            status_details: None,
            stage: Stage::Running,
            steps: Vec::new(),
            attachments: Vec::new(),
            parameters: Vec::new(),
            start: now,
            stop: now,
        }
    }

    /// Marks the fixture as passed.
    pub fn pass(&mut self) {
        self.status = Status::Passed;
        self.stage = Stage::Finished;
        self.stop = current_time_ms();
    }

    /// Marks the fixture as failed.
    pub fn fail(&mut self, message: Option<String>, trace: Option<String>) {
        self.status = Status::Failed;
        self.stage = Stage::Finished;
        self.stop = current_time_ms();
        if message.is_some() || trace.is_some() {
            self.status_details = Some(StatusDetails {
                message,
                trace,
                ..Default::default()
            });
        }
    }
}

/// Category definition for defect classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Category {
    /// Category name
    pub name: String,

    /// Statuses that match this category
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub matched_statuses: Vec<Status>,

    /// Regex pattern to match against error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_regex: Option<String>,

    /// Regex pattern to match against stack trace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_regex: Option<String>,

    /// Whether matching tests should be marked as flaky
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flaky: Option<bool>,
}

impl Category {
    /// Creates a new category with the given name.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            matched_statuses: Vec::new(),
            message_regex: None,
            trace_regex: None,
            flaky: None,
        }
    }

    /// Adds a matched status.
    pub fn with_status(mut self, status: Status) -> Self {
        self.matched_statuses.push(status);
        self
    }

    /// Sets the message regex pattern.
    pub fn with_message_regex(mut self, regex: impl Into<String>) -> Self {
        self.message_regex = Some(regex.into());
        self
    }

    /// Sets the trace regex pattern.
    pub fn with_trace_regex(mut self, regex: impl Into<String>) -> Self {
        self.trace_regex = Some(regex.into());
        self
    }

    /// Marks matching tests as flaky.
    pub fn as_flaky(mut self) -> Self {
        self.flaky = Some(true);
        self
    }
}

/// Returns the current time in milliseconds since Unix epoch.
pub fn current_time_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_result_new() {
        let result = TestResult::new("test-uuid".to_string(), "Test Name".to_string());
        assert_eq!(result.uuid, "test-uuid");
        assert_eq!(result.name, "Test Name");
        assert_eq!(result.status, Status::Unknown);
        assert_eq!(result.stage, Stage::Running);
    }

    #[test]
    fn test_test_result_serialization() {
        let mut result = TestResult::new("uuid-123".to_string(), "My Test".to_string());
        result.add_label_name(LabelName::Epic, "Identity");
        result.add_label_name(LabelName::Severity, "critical");
        result.pass();

        let json = serde_json::to_string_pretty(&result).unwrap();
        assert!(json.contains("\"uuid\": \"uuid-123\""));
        assert!(json.contains("\"name\": \"My Test\""));
        assert!(json.contains("\"status\": \"passed\""));
        assert!(json.contains("\"epic\""));
    }

    #[test]
    fn test_step_result() {
        let mut step = StepResult::new("Step 1");
        step.add_parameter("input", "value");
        step.pass();

        assert_eq!(step.status, Status::Passed);
        assert_eq!(step.stage, Stage::Finished);
        assert_eq!(step.parameters.len(), 1);
    }

    #[test]
    fn test_label_constructors() {
        let epic = Label::epic("My Epic");
        assert_eq!(epic.name, "epic");
        assert_eq!(epic.value, "My Epic");

        let severity = Label::severity(Severity::Critical);
        assert_eq!(severity.name, "severity");
        assert_eq!(severity.value, "critical");
    }

    #[test]
    fn test_link_constructors() {
        let issue = Link::issue("https://jira.com/PROJ-123", Some("PROJ-123".to_string()));
        assert_eq!(issue.r#type, Some(LinkType::Issue));
        assert_eq!(issue.url, "https://jira.com/PROJ-123");
    }

    #[test]
    fn test_parameter_modes() {
        let masked = Parameter::masked("password", "secret123");
        assert_eq!(masked.mode, Some(ParameterMode::Masked));

        let excluded = Parameter::excluded("timestamp", "123456");
        assert_eq!(excluded.excluded, Some(true));
    }

    #[test]
    fn test_container() {
        let mut container = TestResultContainer::new("container-uuid".to_string());
        container.add_child("test-1".to_string());
        container.add_child("test-2".to_string());

        let mut before = FixtureResult::new("setup");
        before.pass();
        container.add_before(before);

        assert_eq!(container.children.len(), 2);
        assert_eq!(container.befores.len(), 1);
    }

    #[test]
    fn test_category() {
        let category = Category::new("Infrastructure Issues")
            .with_status(Status::Broken)
            .with_message_regex(".*timeout.*")
            .as_flaky();

        assert_eq!(category.name, "Infrastructure Issues");
        assert_eq!(category.matched_statuses, vec![Status::Broken]);
        assert_eq!(category.message_regex, Some(".*timeout.*".to_string()));
        assert_eq!(category.flaky, Some(true));
    }
}
