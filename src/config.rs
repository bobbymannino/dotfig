use std::{
    fmt, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize, Serializer};

use crate::utils::expand_home;

/// A reference to an entry in `paths.json`, written as `"Group:Title"`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct PathKey {
    pub(crate) group: String,
    pub(crate) title: String,
}

impl FromStr for PathKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let Some((group, title)) = s.split_once(':') else {
            bail!("`{s}` is not a valid path, expected `Group:Title`");
        };

        let (group, title) = (group.trim(), title.trim());

        if group.is_empty() || title.is_empty() {
            bail!("`{s}` is not a valid path, expected `Group:Title`");
        }

        Ok(Self {
            group: group.to_owned(),
            title: title.to_owned(),
        })
    }
}

impl TryFrom<String> for PathKey {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self> {
        value.parse()
    }
}

impl fmt::Display for PathKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.group, self.title)
    }
}

impl Serialize for PathKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl PathKey {
    /// Whether both keys name the same entry, ignoring case as lookups do.
    pub(crate) fn matches(&self, other: &Self) -> bool {
        self.group.eq_ignore_ascii_case(&other.group) && self.title.eq_ignore_ascii_case(&other.title)
    }
}

/// The contents of the config file.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Config {
    /// The paths to back up, each referencing an entry in `paths.json`.
    #[serde(default)]
    pub(crate) paths: Vec<PathKey>,

    /// Where to keep the backups, defaulting to `backups` next to the config file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) save_path: Option<String>,
}

impl Config {
    /// Read and parse the config file at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, is not valid JSON, or
    /// contains a path that is not in `Group:Title` form.
    pub(crate) fn from_file(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path).with_context(|| format!("Could not read {}", path.display()))?;

        serde_json::from_str(&contents).with_context(|| format!("Could not parse {}", path.display()))
    }

    /// Write the config back to `path`, replacing what is there.
    ///
    /// # Errors
    ///
    /// Returns an error if the config cannot be serialised or the file cannot be written.
    pub(crate) fn to_file(&self, path: &Path) -> Result<()> {
        let mut contents = serde_json::to_string_pretty(self).context("Could not serialise the config")?;
        contents.push('\n');

        fs::write(path, contents).with_context(|| format!("Could not write {}", path.display()))
    }

    /// Where backups are kept for this config.
    ///
    /// A relative `save_path` is resolved against the config file's directory, not the working directory, so the same
    /// config behaves the same wherever dotfig is run from.
    ///
    /// # Errors
    ///
    /// Returns an error if `save_path` is empty or starts with `~/` and the home directory cannot be found.
    pub(crate) fn backups_dir(&self, config_path: &Path) -> Result<PathBuf> {
        let base = config_path.parent().unwrap_or_else(|| Path::new(""));

        let Some(save_path) = &self.save_path else {
            return Ok(base.join("backups"));
        };

        if save_path.trim().is_empty() {
            bail!("`save_path` is empty, remove it to use the default");
        }

        let save_path = expand_home(save_path)?;

        Ok(if save_path.is_absolute() { save_path } else { base.join(save_path) })
    }

    /// Add `key`, reporting whether it was not already there.
    pub(crate) fn add(&mut self, key: PathKey) -> bool {
        if self.paths.iter().any(|existing| existing.matches(&key)) {
            return false;
        }

        self.paths.push(key);

        true
    }

    /// Remove `key`, reporting whether it was there to remove.
    pub(crate) fn remove(&mut self, key: &PathKey) -> bool {
        let before = self.paths.len();

        self.paths.retain(|existing| !existing.matches(key));

        self.paths.len() != before
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn config(save_path: Option<&str>) -> Config {
        Config {
            paths: Vec::new(),
            save_path: save_path.map(str::to_owned),
        }
    }

    #[test]
    fn backups_default_to_sitting_beside_the_config() {
        assert_eq!(
            config(None).backups_dir(Path::new("/home/bob/dotfiles/dotfig.json")).unwrap(),
            PathBuf::from("/home/bob/dotfiles/backups")
        );
    }

    #[test]
    fn a_bare_config_name_keeps_backups_in_the_working_directory() {
        assert_eq!(
            config(None).backups_dir(Path::new("dotfig.json")).unwrap(),
            PathBuf::from("backups")
        );
    }

    #[test]
    fn a_relative_save_path_is_resolved_against_the_config() {
        assert_eq!(
            config(Some("saved/here"))
                .backups_dir(Path::new("/home/bob/dotfiles/dotfig.json"))
                .unwrap(),
            PathBuf::from("/home/bob/dotfiles/saved/here")
        );
    }

    #[test]
    fn an_absolute_save_path_is_used_as_is() {
        assert_eq!(
            config(Some("/mnt/backups"))
                .backups_dir(Path::new("/home/bob/dotfiles/dotfig.json"))
                .unwrap(),
            PathBuf::from("/mnt/backups")
        );
    }

    #[test]
    fn a_save_path_can_start_at_home() {
        let home = std::env::home_dir().unwrap();

        assert_eq!(
            config(Some("~/Dropbox/dotfiles"))
                .backups_dir(Path::new("/anywhere/dotfig.json"))
                .unwrap(),
            home.join("Dropbox/dotfiles")
        );
    }

    #[test]
    fn an_empty_save_path_is_rejected() {
        assert!(config(Some("   ")).backups_dir(Path::new("dotfig.json")).is_err());
    }

    #[test]
    fn a_config_without_a_save_path_does_not_grow_one() {
        let json = serde_json::to_string(&config(None)).unwrap();

        assert_eq!(json, r#"{"paths":[]}"#);
    }

    #[test]
    fn a_save_path_survives_a_round_trip() {
        let json = serde_json::to_string(&config(Some("~/Dropbox/dotfiles"))).unwrap();
        let parsed: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.save_path.as_deref(), Some("~/Dropbox/dotfiles"));
    }
}
