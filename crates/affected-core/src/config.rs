use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

use crate::types::Ecosystem;

#[derive(Debug, Deserialize, Default)]
pub struct Config {
    pub test: Option<TestConfig>,
    pub ignore: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
pub struct TestConfig {
    pub cargo: Option<String>,
    pub npm: Option<String>,
    pub go: Option<String>,
    pub python: Option<String>,
}

impl Config {
    /// Load config from `.affected.toml` in the project root, or return defaults.
    pub fn load(root: &Path) -> Result<Self> {
        let config_path = root.join(".affected.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    /// Get a custom test command for a given ecosystem and package.
    /// Replaces `{package}` placeholder with the actual package name.
    pub fn test_command_for(&self, ecosystem: Ecosystem, package: &str) -> Option<Vec<String>> {
        let template = match &self.test {
            Some(tc) => match ecosystem {
                Ecosystem::Cargo => tc.cargo.as_deref(),
                Ecosystem::Npm => tc.npm.as_deref(),
                Ecosystem::Go => tc.go.as_deref(),
                Ecosystem::Python => tc.python.as_deref(),
            },
            None => None,
        }?;

        let expanded = template.replace("{package}", package);
        Some(expanded.split_whitespace().map(String::from).collect())
    }

    /// Check if a file path matches any ignore patterns.
    pub fn is_ignored(&self, path: &str) -> bool {
        match &self.ignore {
            Some(patterns) => patterns.iter().any(|pat| {
                glob::Pattern::new(pat)
                    .map(|p| p.matches(path))
                    .unwrap_or(false)
            }),
            None => false,
        }
    }
}
