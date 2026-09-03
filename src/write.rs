//! Atomic replacement of `WEAVATRIX.md`.

use crate::error::{Error, Result};
use std::fs;
use std::path::Path;

/// Generated file name.
pub const FILENAME: &str = "WEAVATRIX.md";

/// Writes `contents` next to `root` via a sibling temp file, then rename.
///
/// # Errors
///
/// Filesystem write or rename fails.
pub fn atomic_write(root: &Path, contents: &str) -> Result<()> {
    let destination = root.join(FILENAME);
    let temp = root.join(".WEAVATRIX.md.tmp");
    fs::write(&temp, contents).map_err(|_| Error::path("failed to write", &temp))?;
    if destination.exists() {
        fs::remove_file(&destination)
            .map_err(|_| Error::path("failed to replace", &destination))?;
    }
    fs::rename(&temp, &destination).map_err(|_| {
        let _ = fs::remove_file(&temp);
        Error::path("failed to replace", &destination)
    })?;
    Ok(())
}

/// Reads the committed file if it exists.
///
/// # Errors
///
/// The file exists but cannot be read.
pub fn read_existing(root: &Path) -> Result<Option<String>> {
    let path = root.join(FILENAME);
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(fs::read_to_string(path)?))
}
