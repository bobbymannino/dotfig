use std::path::PathBuf;

use anyhow::{Context, Result, bail};

/// Check if a file exists, returning its absolute path.
///
/// # Errors
///
/// Returns an error if the path does not exist, cannot be resolved, or points
/// at something other than a file.
pub fn does_file_exist(path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(path)
        .canonicalize()
        .with_context(|| format!("Could not resolve {path}"))?;

    if !path.is_file() {
        bail!("{} is not a file", path.display());
    }

    Ok(path)
}

/// Expand a leading `~/` into the user's home directory.
///
/// # Errors
///
/// Returns an error if the path starts with `~/` and the home directory cannot be found.
pub fn expand_home(path: &str) -> Result<PathBuf> {
    let Some(rest) = path.strip_prefix("~/") else {
        return Ok(PathBuf::from(path));
    };

    let home = std::env::home_dir().context("Could not find your home directory")?;

    Ok(home.join(rest))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn absolute_paths_are_left_alone() {
        assert_eq!(expand_home("/etc/hosts").unwrap(), PathBuf::from("/etc/hosts"));
    }

    #[test]
    fn a_leading_tilde_becomes_the_home_directory() {
        let home = std::env::home_dir().unwrap();

        assert_eq!(expand_home("~/.zshrc").unwrap(), home.join(".zshrc"));
    }

    #[test]
    fn a_tilde_elsewhere_is_not_expanded() {
        assert_eq!(expand_home("/tmp/~/file").unwrap(), PathBuf::from("/tmp/~/file"));
    }
}
