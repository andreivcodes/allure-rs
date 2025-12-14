//! Allure enums for test result status, stage, severity, and other classifications.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Test result status indicating the outcome of a test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Status {
    /// Test passed successfully
    Passed,
    /// Test failed due to assertion failure (product defect)
    Failed,
    /// Test broken due to unexpected error (test defect)
    Broken,
    /// Test was skipped
    Skipped,
    /// Test status is unknown
    #[default]
    Unknown,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Passed => write!(f, "passed"),
            Status::Failed => write!(f, "failed"),
            Status::Broken => write!(f, "broken"),
            Status::Skipped => write!(f, "skipped"),
            Status::Unknown => write!(f, "unknown"),
        }
    }
}

/// Test execution stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Stage {
    /// Test is scheduled but not started
    Scheduled,
    /// Test is currently running
    Running,
    /// Test has finished execution
    #[default]
    Finished,
    /// Test is pending
    Pending,
    /// Test was interrupted
    Interrupted,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stage::Scheduled => write!(f, "scheduled"),
            Stage::Running => write!(f, "running"),
            Stage::Finished => write!(f, "finished"),
            Stage::Pending => write!(f, "pending"),
            Stage::Interrupted => write!(f, "interrupted"),
        }
    }
}

/// Test severity level for prioritization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Severity {
    /// System is unusable, blocker issue
    Blocker,
    /// Major functionality is broken
    Critical,
    /// Standard test importance
    #[default]
    Normal,
    /// Minor issues
    Minor,
    /// Cosmetic or trivial issues
    Trivial,
}

impl Severity {
    /// Returns the string representation used in Allure labels.
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Blocker => "blocker",
            Severity::Critical => "critical",
            Severity::Normal => "normal",
            Severity::Minor => "minor",
            Severity::Trivial => "trivial",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Link type for external references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum LinkType {
    /// Default link type
    #[default]
    #[serde(rename = "link")]
    Default,
    /// Link to issue tracker
    Issue,
    /// Link to test management system
    Tms,
}

impl fmt::Display for LinkType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LinkType::Default => write!(f, "link"),
            LinkType::Issue => write!(f, "issue"),
            LinkType::Tms => write!(f, "tms"),
        }
    }
}

/// Parameter display mode in reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ParameterMode {
    /// Show parameter value as-is
    #[default]
    Default,
    /// Hide parameter value completely
    Hidden,
    /// Mask parameter value (e.g., for passwords)
    Masked,
}

impl fmt::Display for ParameterMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterMode::Default => write!(f, "default"),
            ParameterMode::Hidden => write!(f, "hidden"),
            ParameterMode::Masked => write!(f, "masked"),
        }
    }
}

/// Content type for attachments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ContentType {
    /// Plain text content
    #[default]
    Text,
    /// JSON content
    Json,
    /// XML content
    Xml,
    /// HTML content
    Html,
    /// CSV (comma-separated values) content
    Csv,
    /// TSV (tab-separated values) content
    Tsv,
    /// CSS stylesheet content
    Css,
    /// URI list content
    Uri,
    /// SVG image content
    Svg,
    /// PNG image content
    Png,
    /// JPEG image content
    Jpeg,
    /// WebM video content
    Webm,
    /// MP4 video content
    Mp4,
    /// ZIP archive content
    Zip,
    /// Allure image diff content
    #[serde(rename = "imagediff")]
    ImageDiff,
}

impl ContentType {
    /// Returns the MIME type string.
    pub fn as_mime(&self) -> &'static str {
        match self {
            ContentType::Text => "text/plain",
            ContentType::Json => "application/json",
            ContentType::Xml => "application/xml",
            ContentType::Html => "text/html",
            ContentType::Csv => "text/csv",
            ContentType::Tsv => "text/tab-separated-values",
            ContentType::Css => "text/css",
            ContentType::Uri => "text/uri-list",
            ContentType::Svg => "image/svg+xml",
            ContentType::Png => "image/png",
            ContentType::Jpeg => "image/jpeg",
            ContentType::Webm => "video/webm",
            ContentType::Mp4 => "video/mp4",
            ContentType::Zip => "application/zip",
            ContentType::ImageDiff => "application/vnd.allure.image.diff",
        }
    }

    /// Returns the file extension for this content type.
    pub fn extension(&self) -> &'static str {
        match self {
            ContentType::Text => "txt",
            ContentType::Json => "json",
            ContentType::Xml => "xml",
            ContentType::Html => "html",
            ContentType::Csv => "csv",
            ContentType::Tsv => "tsv",
            ContentType::Css => "css",
            ContentType::Uri => "uri",
            ContentType::Svg => "svg",
            ContentType::Png => "png",
            ContentType::Jpeg => "jpg",
            ContentType::Webm => "webm",
            ContentType::Mp4 => "mp4",
            ContentType::Zip => "zip",
            ContentType::ImageDiff => "imagediff",
        }
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_mime())
    }
}

/// Reserved label names used by Allure for special purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum LabelName {
    /// Allure test case ID (AS_ID)
    #[serde(rename = "AS_ID")]
    AllureId,
    /// Test suite name
    #[serde(rename = "suite")]
    Suite,
    /// Parent suite name
    #[serde(rename = "parentSuite")]
    ParentSuite,
    /// Sub-suite name
    #[serde(rename = "subSuite")]
    SubSuite,
    /// Epic (top-level business capability)
    #[serde(rename = "epic")]
    Epic,
    /// Feature under epic
    #[serde(rename = "feature")]
    Feature,
    /// User story under feature
    #[serde(rename = "story")]
    Story,
    /// Test severity
    #[serde(rename = "severity")]
    Severity,
    /// Test tag
    #[default]
    #[serde(rename = "tag")]
    Tag,
    /// Test owner/maintainer
    #[serde(rename = "owner")]
    Owner,
    /// Execution host
    #[serde(rename = "host")]
    Host,
    /// Thread ID
    #[serde(rename = "thread")]
    Thread,
    /// Test method name
    #[serde(rename = "testMethod")]
    TestMethod,
    /// Test class name
    #[serde(rename = "testClass")]
    TestClass,
    /// Package/module name
    #[serde(rename = "package")]
    Package,
    /// Test framework name
    #[serde(rename = "framework")]
    Framework,
    /// Programming language
    #[serde(rename = "language")]
    Language,
}

impl LabelName {
    /// Returns the string name used in Allure JSON.
    pub fn as_str(&self) -> &'static str {
        match self {
            LabelName::AllureId => "AS_ID",
            LabelName::Suite => "suite",
            LabelName::ParentSuite => "parentSuite",
            LabelName::SubSuite => "subSuite",
            LabelName::Epic => "epic",
            LabelName::Feature => "feature",
            LabelName::Story => "story",
            LabelName::Severity => "severity",
            LabelName::Tag => "tag",
            LabelName::Owner => "owner",
            LabelName::Host => "host",
            LabelName::Thread => "thread",
            LabelName::TestMethod => "testMethod",
            LabelName::TestClass => "testClass",
            LabelName::Package => "package",
            LabelName::Framework => "framework",
            LabelName::Language => "language",
        }
    }
}

impl fmt::Display for LabelName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_serialization() {
        assert_eq!(
            serde_json::to_string(&Status::Passed).unwrap(),
            "\"passed\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Failed).unwrap(),
            "\"failed\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Broken).unwrap(),
            "\"broken\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Skipped).unwrap(),
            "\"skipped\""
        );
        assert_eq!(
            serde_json::to_string(&Status::Unknown).unwrap(),
            "\"unknown\""
        );
    }

    #[test]
    fn test_stage_serialization() {
        assert_eq!(
            serde_json::to_string(&Stage::Finished).unwrap(),
            "\"finished\""
        );
        assert_eq!(
            serde_json::to_string(&Stage::Running).unwrap(),
            "\"running\""
        );
    }

    #[test]
    fn test_severity_as_str() {
        assert_eq!(Severity::Blocker.as_str(), "blocker");
        assert_eq!(Severity::Critical.as_str(), "critical");
        assert_eq!(Severity::Normal.as_str(), "normal");
    }

    #[test]
    fn test_content_type_mime() {
        assert_eq!(ContentType::Json.as_mime(), "application/json");
        assert_eq!(ContentType::Png.as_mime(), "image/png");
    }

    #[test]
    fn test_label_name_as_str() {
        assert_eq!(LabelName::Epic.as_str(), "epic");
        assert_eq!(LabelName::AllureId.as_str(), "AS_ID");
        assert_eq!(LabelName::ParentSuite.as_str(), "parentSuite");
    }

    #[test]
    fn test_status_display() {
        assert_eq!(format!("{}", Status::Passed), "passed");
        assert_eq!(format!("{}", Status::Failed), "failed");
        assert_eq!(format!("{}", Status::Broken), "broken");
    }

    #[test]
    fn test_stage_display() {
        assert_eq!(format!("{}", Stage::Running), "running");
        assert_eq!(format!("{}", Stage::Finished), "finished");
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", Severity::Critical), "critical");
        assert_eq!(format!("{}", Severity::Blocker), "blocker");
    }

    #[test]
    fn test_link_type_display() {
        assert_eq!(format!("{}", LinkType::Issue), "issue");
        assert_eq!(format!("{}", LinkType::Tms), "tms");
        assert_eq!(format!("{}", LinkType::Default), "link");
    }

    #[test]
    fn test_parameter_mode_display() {
        assert_eq!(format!("{}", ParameterMode::Default), "default");
        assert_eq!(format!("{}", ParameterMode::Hidden), "hidden");
        assert_eq!(format!("{}", ParameterMode::Masked), "masked");
    }

    #[test]
    fn test_content_type_display() {
        assert_eq!(format!("{}", ContentType::Json), "application/json");
        assert_eq!(format!("{}", ContentType::Png), "image/png");
    }

    #[test]
    fn test_label_name_display() {
        assert_eq!(format!("{}", LabelName::Epic), "epic");
        assert_eq!(format!("{}", LabelName::AllureId), "AS_ID");
    }
}
