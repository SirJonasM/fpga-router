use std::path::PathBuf;

pub fn get_test_data_path(filename: &str) -> PathBuf {
    // This finds the project root regardless of where the test is running from
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(root).parent().unwrap().join("tests/data").join(filename)
}
