use std::path::PathBuf;

#[must_use]
/// returns the full path of a test ressource
/// # Panics
/// Panincs if the test directory is missing.
pub fn get_test_data_path(filename: &str) -> PathBuf {
    // This finds the project root regardless of where the test is running from
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(root).parent().unwrap().join("tests/data").join(filename)
}
