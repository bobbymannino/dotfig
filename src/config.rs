use std::{fmt, fs, path::Path, str::FromStr};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

/// A reference to an entry in `paths.json`, written as `"Group:Title"`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct PathKey {
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

/// The contents of the config file.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    /// The paths to back up, each referencing an entry in `paths.json`.
    #[serde(default)]
    pub(crate) paths: Vec<PathKey>,
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
}
