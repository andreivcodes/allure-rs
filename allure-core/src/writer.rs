//! File writer for Allure test results, containers, and attachments.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::enums::ContentType;
use crate::model::{Attachment, Category, TestResult, TestResultContainer};

/// Default directory for Allure results.
pub const DEFAULT_RESULTS_DIR: &str = "allure-results";

/// Writer for Allure test result files.
#[derive(Debug, Clone)]
pub struct AllureWriter {
    results_dir: PathBuf,
}

impl AllureWriter {
    /// Creates a new writer with the default results directory.
    pub fn new() -> Self {
        Self::with_results_dir(DEFAULT_RESULTS_DIR)
    }

    /// Creates a new writer with a custom results directory.
    pub fn with_results_dir(path: impl AsRef<Path>) -> Self {
        Self {
            results_dir: path.as_ref().to_path_buf(),
        }
    }

    /// Returns the results directory path.
    pub fn results_dir(&self) -> &Path {
        &self.results_dir
    }

    /// Initializes the results directory, optionally cleaning it first.
    pub fn init(&self, clean: bool) -> io::Result<()> {
        if clean && self.results_dir.exists() {
            fs::remove_dir_all(&self.results_dir)?;
        }
        fs::create_dir_all(&self.results_dir)?;
        Ok(())
    }

    /// Ensures the results directory exists.
    fn ensure_dir(&self) -> io::Result<()> {
        if !self.results_dir.exists() {
            fs::create_dir_all(&self.results_dir)?;
        }
        Ok(())
    }

    /// Writes a test result to a JSON file.
    pub fn write_test_result(&self, result: &TestResult) -> io::Result<PathBuf> {
        self.ensure_dir()?;
        let filename = format!("{}-result.json", result.uuid);
        let path = self.results_dir.join(&filename);
        let json = serde_json::to_string_pretty(result)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&path, json)?;
        Ok(path)
    }

    /// Writes a container to a JSON file.
    pub fn write_container(&self, container: &TestResultContainer) -> io::Result<PathBuf> {
        self.ensure_dir()?;
        let filename = format!("{}-container.json", container.uuid);
        let path = self.results_dir.join(&filename);
        let json = serde_json::to_string_pretty(container)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&path, json)?;
        Ok(path)
    }

    /// Writes a text attachment and returns the Attachment reference.
    pub fn write_text_attachment(
        &self,
        name: impl Into<String>,
        content: impl AsRef<str>,
    ) -> io::Result<Attachment> {
        self.ensure_dir()?;
        let uuid = uuid::Uuid::new_v4().to_string();
        let filename = format!("{}-attachment.txt", uuid);
        let path = self.results_dir.join(&filename);
        fs::write(&path, content.as_ref())?;
        Ok(Attachment::new(
            name,
            filename,
            Some(ContentType::Text.as_mime().to_string()),
        ))
    }

    /// Writes a JSON attachment and returns the Attachment reference.
    pub fn write_json_attachment<T: serde::Serialize>(
        &self,
        name: impl Into<String>,
        value: &T,
    ) -> io::Result<Attachment> {
        self.ensure_dir()?;
        let uuid = uuid::Uuid::new_v4().to_string();
        let filename = format!("{}-attachment.json", uuid);
        let path = self.results_dir.join(&filename);
        let json = serde_json::to_string_pretty(value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&path, json)?;
        Ok(Attachment::new(
            name,
            filename,
            Some(ContentType::Json.as_mime().to_string()),
        ))
    }

    /// Writes a binary attachment and returns the Attachment reference.
    pub fn write_binary_attachment(
        &self,
        name: impl Into<String>,
        content: &[u8],
        content_type: ContentType,
    ) -> io::Result<Attachment> {
        self.ensure_dir()?;
        let uuid = uuid::Uuid::new_v4().to_string();
        let filename = format!("{}-attachment.{}", uuid, content_type.extension());
        let path = self.results_dir.join(&filename);
        fs::write(&path, content)?;
        Ok(Attachment::new(
            name,
            filename,
            Some(content_type.as_mime().to_string()),
        ))
    }

    /// Writes a binary attachment with a custom MIME type.
    pub fn write_binary_attachment_with_mime(
        &self,
        name: impl Into<String>,
        content: &[u8],
        mime_type: impl Into<String>,
        extension: impl AsRef<str>,
    ) -> io::Result<Attachment> {
        self.ensure_dir()?;
        let uuid = uuid::Uuid::new_v4().to_string();
        let filename = format!("{}-attachment.{}", uuid, extension.as_ref());
        let path = self.results_dir.join(&filename);
        fs::write(&path, content)?;
        Ok(Attachment::new(name, filename, Some(mime_type.into())))
    }

    /// Copies a file as an attachment and returns the Attachment reference.
    pub fn copy_file_attachment(
        &self,
        name: impl Into<String>,
        source_path: impl AsRef<Path>,
        content_type: Option<ContentType>,
    ) -> io::Result<Attachment> {
        self.ensure_dir()?;
        let source = source_path.as_ref();
        let extension = source
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("bin");

        let uuid = uuid::Uuid::new_v4().to_string();
        let filename = format!("{}-attachment.{}", uuid, extension);
        let dest_path = self.results_dir.join(&filename);
        fs::copy(source, &dest_path)?;

        let mime = content_type
            .map(|ct| ct.as_mime().to_string())
            .or_else(|| guess_mime_type(extension));

        Ok(Attachment::new(name, filename, mime))
    }

    /// Writes the environment.properties file.
    ///
    /// Keys and values are escaped according to the Java Properties file format.
    pub fn write_environment(&self, properties: &[(String, String)]) -> io::Result<PathBuf> {
        self.ensure_dir()?;
        let path = self.results_dir.join("environment.properties");
        let mut file = File::create(&path)?;
        for (key, value) in properties {
            let escaped_key = escape_property_value(key);
            let escaped_value = escape_property_value(value);
            writeln!(file, "{}={}", escaped_key, escaped_value)?;
        }
        Ok(path)
    }

    /// Writes the categories.json file.
    pub fn write_categories(&self, categories: &[Category]) -> io::Result<PathBuf> {
        self.ensure_dir()?;
        let path = self.results_dir.join("categories.json");
        let json = serde_json::to_string_pretty(categories)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(&path, json)?;
        Ok(path)
    }
}

impl Default for AllureWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Escapes a string for use in a Java Properties file.
///
/// Order of operations matters:
/// 1. Escape backslashes first (\ -> \\)
/// 2. Then escape newlines (\n -> \\n)
/// 3. Then escape carriage returns (\r -> \\r)
/// 4. Then escape equals signs (= -> \=)
fn escape_property_value(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('=', "\\=")
}

/// Guesses the MIME type from a file extension.
fn guess_mime_type(extension: &str) -> Option<String> {
    match extension.to_lowercase().as_str() {
        "txt" => Some("text/plain".to_string()),
        "json" => Some("application/json".to_string()),
        "xml" => Some("application/xml".to_string()),
        "html" | "htm" => Some("text/html".to_string()),
        "css" => Some("text/css".to_string()),
        "csv" => Some("text/csv".to_string()),
        "png" => Some("image/png".to_string()),
        "jpg" | "jpeg" => Some("image/jpeg".to_string()),
        "gif" => Some("image/gif".to_string()),
        "svg" => Some("image/svg+xml".to_string()),
        "webp" => Some("image/webp".to_string()),
        "mp4" => Some("video/mp4".to_string()),
        "webm" => Some("video/webm".to_string()),
        "pdf" => Some("application/pdf".to_string()),
        "zip" => Some("application/zip".to_string()),
        "log" => Some("text/plain".to_string()),
        _ => None,
    }
}

/// Generates a new UUID v4 string.
pub fn generate_uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Computes the history ID for a test based on its full name and parameters.
pub fn compute_history_id(full_name: &str, parameters: &[crate::model::Parameter]) -> String {
    use md5::{Digest, Md5};

    let mut hasher = Md5::new();
    hasher.update(full_name.as_bytes());

    for param in parameters {
        // Skip excluded parameters
        if param.excluded.unwrap_or(false) {
            continue;
        }
        hasher.update(param.name.as_bytes());
        hasher.update(param.value.as_bytes());
    }

    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enums::Status;
    use crate::model::Parameter;
    use std::env;

    fn temp_dir() -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!("allure-test-{}", uuid::Uuid::new_v4()));
        path
    }

    #[test]
    fn test_writer_init() {
        let dir = temp_dir();
        let writer = AllureWriter::with_results_dir(&dir);
        writer.init(true).unwrap();
        assert!(dir.exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_write_test_result() {
        let dir = temp_dir();
        let writer = AllureWriter::with_results_dir(&dir);
        writer.init(true).unwrap();

        let mut result = TestResult::new("test-123".to_string(), "My Test".to_string());
        result.pass();

        let path = writer.write_test_result(&result).unwrap();
        assert!(path.exists());
        assert!(path.to_string_lossy().contains("test-123-result.json"));

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"uuid\": \"test-123\""));
        assert!(content.contains("\"status\": \"passed\""));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_write_text_attachment() {
        let dir = temp_dir();
        let writer = AllureWriter::with_results_dir(&dir);
        writer.init(true).unwrap();

        let attachment = writer
            .write_text_attachment("Log", "Test log content")
            .unwrap();
        assert_eq!(attachment.name, "Log");
        assert!(attachment.source.ends_with(".txt"));
        assert_eq!(attachment.r#type, Some("text/plain".to_string()));

        let path = dir.join(&attachment.source);
        assert!(path.exists());
        assert_eq!(fs::read_to_string(&path).unwrap(), "Test log content");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_write_json_attachment() {
        let dir = temp_dir();
        let writer = AllureWriter::with_results_dir(&dir);
        writer.init(true).unwrap();

        #[derive(serde::Serialize)]
        struct Data {
            foo: String,
            bar: i32,
        }

        let data = Data {
            foo: "hello".to_string(),
            bar: 42,
        };

        let attachment = writer.write_json_attachment("Response", &data).unwrap();
        assert_eq!(attachment.r#type, Some("application/json".to_string()));

        let path = dir.join(&attachment.source);
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"foo\": \"hello\""));
        assert!(content.contains("\"bar\": 42"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_write_binary_attachment() {
        let dir = temp_dir();
        let writer = AllureWriter::with_results_dir(&dir);
        writer.init(true).unwrap();

        let png_data = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic bytes
        let attachment = writer
            .write_binary_attachment("Screenshot", &png_data, ContentType::Png)
            .unwrap();
        assert!(attachment.source.ends_with(".png"));
        assert_eq!(attachment.r#type, Some("image/png".to_string()));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_write_environment() {
        let dir = temp_dir();
        let writer = AllureWriter::with_results_dir(&dir);
        writer.init(true).unwrap();

        let env = vec![
            ("os".to_string(), "linux".to_string()),
            ("rust_version".to_string(), "1.75.0".to_string()),
        ];

        let path = writer.write_environment(&env).unwrap();
        assert!(path.exists());

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("os=linux"));
        assert!(content.contains("rust_version=1.75.0"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_write_categories() {
        let dir = temp_dir();
        let writer = AllureWriter::with_results_dir(&dir);
        writer.init(true).unwrap();

        let categories = vec![
            Category::new("Infrastructure Issues")
                .with_status(Status::Broken)
                .with_message_regex(".*timeout.*"),
            Category::new("Product Defects").with_status(Status::Failed),
        ];

        let path = writer.write_categories(&categories).unwrap();
        assert!(path.exists());

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("Infrastructure Issues"));
        assert!(content.contains("Product Defects"));
        assert!(content.contains("timeout"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_compute_history_id() {
        let params = vec![Parameter::new("a", "1"), Parameter::new("b", "2")];

        let id1 = compute_history_id("test::my_test", &params);
        let id2 = compute_history_id("test::my_test", &params);
        assert_eq!(id1, id2);

        // Different name should produce different ID
        let id3 = compute_history_id("test::other_test", &params);
        assert_ne!(id1, id3);

        // Excluded parameters should not affect the ID
        let params_with_excluded = vec![
            Parameter::new("a", "1"),
            Parameter::new("b", "2"),
            Parameter::excluded("timestamp", "12345"),
        ];
        let id4 = compute_history_id("test::my_test", &params_with_excluded);
        assert_eq!(id1, id4);
    }

    #[test]
    fn test_generate_uuid() {
        let uuid1 = generate_uuid();
        let uuid2 = generate_uuid();
        assert_ne!(uuid1, uuid2);
        assert_eq!(uuid1.len(), 36); // UUID v4 format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    }
}
