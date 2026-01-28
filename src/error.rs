//! Custom error types for testlist.

use std::path::PathBuf;
use thiserror::Error;

/// Custom error type for testlist operations.
#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse RON file: {0}")]
    Parse(#[from] ron::error::SpannedError),

    #[error("Failed to serialize RON: {0}")]
    Serialize(#[from] ron::Error),

    #[error("Testlist file not found: {0}")]
    TestlistNotFound(PathBuf),

    #[error("Invalid test ID: {0}")]
    InvalidTestId(String),

    #[error("Results file not found: {0}")]
    ResultsNotFound(PathBuf),
}

/// Result type alias using our custom Error.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = Error::Io(io_err);
        let display = format!("{}", err);
        assert!(display.contains("IO error"));
    }

    #[test]
    fn test_error_display_testlist_not_found() {
        let err = Error::TestlistNotFound(PathBuf::from("/path/to/test.ron"));
        let display = format!("{}", err);
        assert!(display.contains("Testlist file not found"));
        assert!(display.contains("/path/to/test.ron"));
    }

    #[test]
    fn test_error_display_invalid_test_id() {
        let err = Error::InvalidTestId("bad-id".to_string());
        let display = format!("{}", err);
        assert!(display.contains("Invalid test ID"));
        assert!(display.contains("bad-id"));
    }

    #[test]
    fn test_error_display_results_not_found() {
        let err = Error::ResultsNotFound(PathBuf::from("/results.ron"));
        let display = format!("{}", err);
        assert!(display.contains("Results file not found"));
    }

    #[test]
    fn test_io_error_from() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }
}
