//! Error types for Allure operations.
//!
//! This module provides a unified error type for all Allure operations,
//! including I/O errors, serialization errors, and configuration issues.

use thiserror::Error;

/// Result type alias for Allure operations.
pub type AllureResult<T> = Result<T, AllureError>;

/// Errors that can occur during Allure operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum AllureError {
    /// I/O error occurred while reading or writing files.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Configuration error with a descriptive message.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// No active test context is available.
    ///
    /// This error occurs when trying to add metadata or steps
    /// outside of a test function wrapped with `#[allure_test]`.
    #[error("No active test context - ensure you are inside an #[allure_test] function")]
    NoActiveContext,

    /// Invalid attachment error.
    #[error("Invalid attachment: {0}")]
    InvalidAttachment(String),

    /// Invalid parameter value.
    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),
}

impl AllureError {
    /// Creates a new configuration error.
    pub fn configuration(message: impl Into<String>) -> Self {
        AllureError::Configuration(message.into())
    }

    /// Creates a new invalid attachment error.
    pub fn invalid_attachment(message: impl Into<String>) -> Self {
        AllureError::InvalidAttachment(message.into())
    }

    /// Creates a new invalid parameter error.
    pub fn invalid_parameter(message: impl Into<String>) -> Self {
        AllureError::InvalidParameter(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let io_err = AllureError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(io_err.to_string().contains("I/O error"));

        let config_err = AllureError::configuration("invalid path");
        assert_eq!(config_err.to_string(), "Configuration error: invalid path");

        let no_context = AllureError::NoActiveContext;
        assert!(no_context.to_string().contains("No active test context"));
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let allure_err: AllureError = io_err.into();
        assert!(matches!(allure_err, AllureError::Io(_)));
    }

    #[test]
    fn test_error_constructors() {
        let attach_err = AllureError::invalid_attachment("file too large");
        assert!(matches!(attach_err, AllureError::InvalidAttachment(_)));

        let param_err = AllureError::invalid_parameter("empty name");
        assert!(matches!(param_err, AllureError::InvalidParameter(_)));
    }
}
