use anyhow::{Context, Result};
use serde::Deserialize;

use crate::config::PathKey;

/// The known paths, embedded at compile time.
const PATHS_JSON: &str = include_str!("../paths.json");

/// A single known path from `paths.json`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct KnownPath {
    pub(crate) group: String,
    pub(crate) title: String,
    pub(crate) path: String,
}

impl KnownPath {
    /// Whether this entry is the one `key` refers to.
    fn matches(&self, key: &PathKey) -> bool {
        self.group.eq_ignore_ascii_case(&key.group) && self.title.eq_ignore_ascii_case(&key.title)
    }

    /// The key that names this entry, spelled as `paths.json` spells it.
    pub(crate) fn key(&self) -> PathKey {
        PathKey {
            group: self.group.clone(),
            title: self.title.clone(),
        }
    }
}

/// Every path dotfig knows how to back up.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Registry {
    paths: Vec<KnownPath>,
}

impl Registry {
    /// Parse the embedded `paths.json`.
    ///
    /// # Errors
    ///
    /// Returns an error if `paths.json` is not valid, which means the binary was built with a broken registry.
    pub(crate) fn load() -> Result<Self> {
        serde_json::from_str(PATHS_JSON).context("Could not parse the built in paths.json")
    }

    /// Look up the entry `key` refers to, if there is one.
    pub(crate) fn get(&self, key: &PathKey) -> Option<&KnownPath> {
        self.paths.iter().find(|known| known.matches(key))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        Registry::load().unwrap()
    }

    fn key(raw: &str) -> PathKey {
        raw.parse().unwrap()
    }

    #[test]
    fn embedded_registry_parses() {
        assert!(!registry().paths.is_empty(), "paths.json should not be empty");
    }

    #[test]
    fn entries_are_well_formed() {
        for known in &registry().paths {
            let name = format!("{}:{}", known.group, known.title);

            assert_eq!(known.group.trim(), known.group, "{name} group has surrounding whitespace");
            assert_eq!(known.title.trim(), known.title, "{name} title has surrounding whitespace");
            assert!(!known.group.is_empty(), "{name} has an empty group");
            assert!(!known.title.is_empty(), "{name} has an empty title");
            assert!(!known.path.trim().is_empty(), "{name} has an empty path");
        }
    }

    #[test]
    fn entries_are_addressable() {
        // A `:` in either half would make the entry impossible to name in a config file.
        for known in &registry().paths {
            let name = format!("{}:{}", known.group, known.title);

            assert!(!known.group.contains(':'), "{name} group contains a `:`");
            assert!(!known.title.contains(':'), "{name} title contains a `:`");

            let found = registry().get(&key(&name)).map(|found| found.path.clone());

            assert_eq!(found.as_deref(), Some(known.path.as_str()), "{name} does not resolve to itself");
        }
    }

    #[test]
    fn keys_are_unique() {
        let registry = registry();

        for (index, known) in registry.paths.iter().enumerate() {
            let duplicate = registry
                .paths
                .iter()
                .skip(index + 1)
                .any(|other| other.group.eq_ignore_ascii_case(&known.group) && other.title.eq_ignore_ascii_case(&known.title));

            assert!(!duplicate, "{}:{} appears more than once", known.group, known.title);
        }
    }

    #[test]
    fn paths_are_absolute_or_home_relative() {
        for known in &registry().paths {
            assert!(
                known.path.starts_with("~/") || known.path.starts_with('/'),
                "{}:{} path `{}` is neither absolute nor home relative",
                known.group,
                known.title,
                known.path
            );
        }
    }

    #[test]
    fn get_finds_a_known_path() {
        assert_eq!(
            registry().get(&key("Zed:Settings")).map(|known| known.path.as_str()),
            Some("~/.config/zed/settings.json")
        );
    }

    #[test]
    fn get_ignores_case() {
        let registry = registry();
        let expected = registry.get(&key("Ghostty:Config")).map(|known| known.path.as_str());

        assert!(expected.is_some());
        assert_eq!(registry.get(&key("ghostty:config")).map(|known| known.path.as_str()), expected);
        assert_eq!(registry.get(&key("GHOSTTY:CONFIG")).map(|known| known.path.as_str()), expected);
    }

    #[test]
    fn get_ignores_surrounding_whitespace() {
        let registry = registry();

        assert_eq!(
            registry.get(&key("  Zsh : Config  ")).map(|known| known.path.as_str()),
            registry.get(&key("Zsh:Config")).map(|known| known.path.as_str())
        );
    }

    #[test]
    fn get_returns_none_for_an_unknown_key() {
        let registry = registry();

        assert!(registry.get(&key("Nope:Missing")).is_none());
        assert!(
            registry.get(&key("Zed:Nothing")).is_none(),
            "a known group with an unknown title should not match"
        );
        assert!(
            registry.get(&key("Nothing:Settings")).is_none(),
            "an unknown group with a known title should not match"
        );
    }
}
