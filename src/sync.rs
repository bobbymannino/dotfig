use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};

use crate::paths::KnownPath;

/// What happened to a single path.
#[derive(Debug)]
pub(crate) enum Outcome {
    /// The file was copied to this destination.
    Copied(PathBuf),
    /// There was nothing to copy at this source.
    Missing(PathBuf),
}

/// Expand a leading `~/` into the user's home directory.
///
/// # Errors
///
/// Returns an error if the path starts with `~/` and the home directory cannot be found.
pub(crate) fn expand_home(path: &str) -> Result<PathBuf> {
    let Some(rest) = path.strip_prefix("~/") else {
        return Ok(PathBuf::from(path));
    };

    let home = std::env::home_dir().context("Could not find your home directory")?;

    Ok(home.join(rest))
}

/// Where `known` lives inside the backup directory.
pub(crate) fn backup_path(root: &Path, known: &KnownPath) -> PathBuf {
    root.join(&known.group).join(&known.title)
}

/// Copy the live file into the backup directory.
///
/// # Errors
///
/// Returns an error if the home directory cannot be found, the source is not a file, or the copy fails.
pub(crate) fn backup(root: &Path, known: &KnownPath) -> Result<Outcome> {
    copy(&expand_home(&known.path)?, &backup_path(root, known))
}

/// Copy the backed up file back over the live one.
///
/// # Errors
///
/// Returns an error if the home directory cannot be found, the backup is not a file, or the copy fails.
pub(crate) fn restore(root: &Path, known: &KnownPath) -> Result<Outcome> {
    copy(&backup_path(root, known), &expand_home(&known.path)?)
}

/// Copy `from` to `to`, creating any directories `to` needs.
fn copy(from: &Path, to: &Path) -> Result<Outcome> {
    if !from.exists() {
        return Ok(Outcome::Missing(from.to_path_buf()));
    }

    if !from.is_file() {
        bail!("{} is not a file", from.display());
    }

    if let Some(parent) = to.parent() {
        fs::create_dir_all(parent).with_context(|| format!("Could not create {}", parent.display()))?;
    }

    fs::copy(from, to).with_context(|| format!("Could not copy {} to {}", from.display(), to.display()))?;

    Ok(Outcome::Copied(to.to_path_buf()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn known(group: &str, title: &str, path: &str) -> KnownPath {
        KnownPath {
            group: group.to_owned(),
            title: title.to_owned(),
            path: path.to_owned(),
        }
    }

    #[test]
    fn backups_are_grouped_by_group_then_title() {
        assert_eq!(
            backup_path(Path::new("/tmp/backups"), &known("Zed", "Key Map", "~/.config/zed/keymap.json")),
            PathBuf::from("/tmp/backups/Zed/Key Map")
        );
    }

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

    #[test]
    fn copying_reports_a_missing_source() {
        let from = Path::new("/nonexistent/dotfig/source");
        let to = Path::new("/nonexistent/dotfig/dest");

        let outcome = copy(from, to).unwrap();

        assert!(matches!(&outcome, Outcome::Missing(path) if path == from), "{outcome:?}");
        assert!(!to.exists(), "a missing source should not create a destination");
    }
}
