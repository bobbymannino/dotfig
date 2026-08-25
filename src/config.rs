use std::{fmt, fs, path::Path, str::FromStr};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize, Serializer};

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
