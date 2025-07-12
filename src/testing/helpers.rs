//! Test helper functions for common testing patterns

use anyhow::Result;
use std::fs;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

/// Create a temporary file with given content for testing
pub fn create_temp_file(content: &str) -> Result<NamedTempFile> {
    let mut temp_file = NamedTempFile::new()?;
    use std::io::Write;
    write!(temp_file, "{content}")?;
    temp_file.flush()?;
    Ok(temp_file)
}

/// Create a temporary directory with test files
pub fn create_temp_dir_with_files(files: &[(&str, &str)]) -> Result<TempDir> {
    let temp_dir = TempDir::new()?;

    for (filename, content) in files {
        let file_path = temp_dir.path().join(filename);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, content)?;
    }

    Ok(temp_dir)
}

/// Assert that an error contains a specific message
pub fn assert_error_contains<T, E>(result: Result<T, E>, expected_message: &str)
where
    E: std::fmt::Display,
{
    match result {
        Ok(_) => panic!("Expected error but got Ok"),
        Err(e) => {
            let error_string = e.to_string();
            assert!(
                error_string.contains(expected_message),
                "Error '{error_string}' does not contain expected message '{expected_message}'"
            );
        }
    }
}

/// Create a test file path that doesn't exist (for error testing)
pub fn nonexistent_file_path() -> String {
    "/nonexistent/path/to/file.yml".to_string()
}

/// Generate a large string for stress testing
pub fn generate_large_string(size: usize) -> String {
    "a".repeat(size)
}

/// Create malformed YAML content for error testing
pub fn malformed_yaml_scenarios() -> Vec<(&'static str, &'static str)> {
    vec![
        ("unclosed_bracket", "key: [unclosed"),
        ("invalid_indentation", "key:\n  value\n invalid"),
        ("duplicate_key", "key: value1\nkey: value2"),
        ("invalid_character", "key: value\x00"),
        ("unterminated_string", r#"key: "unterminated"#),
    ]
}

/// Helper for testing property-based scenarios
pub fn run_property_test<F>(test_fn: F, iterations: usize)
where
    F: Fn(usize) -> Result<()>,
{
    for i in 0..iterations {
        if let Err(e) = test_fn(i) {
            panic!("Property test failed on iteration {i}: {e}");
        }
    }
}

/// Assert that a closure doesn't panic with any input
pub fn assert_no_panic<F, T>(test_fn: F, inputs: Vec<T>)
where
    F: Fn(T),
    T: std::fmt::Debug + Clone,
{
    for input in inputs {
        let input_clone = input.clone();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            test_fn(input);
        }));

        if result.is_err() {
            panic!("Function panicked with input: {input_clone:?}");
        }
    }
}

/// Helper to verify file permissions (for security testing)
#[cfg(unix)]
pub fn check_file_permissions(path: &Path) -> Result<u32> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = fs::metadata(path)?;
    let permissions = metadata.permissions();
    Ok(permissions.mode())
}

/// Helper to verify file permissions (for security testing)
/// Windows version - returns 0 as permissions concept differs
#[cfg(windows)]
pub fn check_file_permissions(path: &Path) -> Result<u32> {
    let _metadata = fs::metadata(path)?; // Verify file exists
    Ok(0) // Windows permissions work differently, return 0 as placeholder
}

/// Utility for measuring test execution time
pub struct TestTimer {
    start: std::time::Instant,
}

impl TestTimer {
    pub fn new() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }

    pub fn elapsed_ms(&self) -> u128 {
        self.start.elapsed().as_millis()
    }

    pub fn assert_under_duration(&self, max_ms: u128) {
        let elapsed = self.elapsed_ms();
        assert!(
            elapsed <= max_ms,
            "Test took {elapsed}ms, expected under {max_ms}ms"
        );
    }
}

impl Default for TestTimer {
    fn default() -> Self {
        Self::new()
    }
}
