//! Fail-closed errors. Messages never include analyzed source or secrets.

use std::fmt;
use std::io;
use std::path::Path;

/// Recoverable CLI/library failure.
#[derive(Debug)]
pub struct Error {
    message: String,
}

/// Result alias for this crate.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(crate) fn path(action: &str, path: &Path) -> Self {
        Self::new(format!("{action} {}", path.display()))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::new(error.to_string())
    }
}

impl From<weavatrix_scan::Error> for Error {
    fn from(error: weavatrix_scan::Error) -> Self {
        Self::new(error.to_string())
    }
}
