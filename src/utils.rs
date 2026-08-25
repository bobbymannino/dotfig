use std::path::PathBuf;

use anyhow::{Context, Result, bail};

/// Check if a file exists, returning its absolute path.
///
/// # Errors
///
/// Returns an error if the path does not exist, cannot be resolved, or points
/// at something other than a file.
pub(crate) fn does_file_exist(path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(path)
        .canonicalize()
        .with_context(|| format!("Could not resolve {path}"))?;

    if !path.is_file() {
        bail!("{} is not a file", path.display());
    }

    Ok(path)
}
