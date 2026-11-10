//! Configuration file models (design v0.2 §5).
//!
//! - `tesh.toml` at the project root
//! - `shrubs/<name>/shrub.toml` per shrub
//! - `envs/<name>/env.toml` per env

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// `tesh.toml` (design v0.2 §5.1).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_terraform_binary")]
    pub terraform_binary: String,
    #[serde(default = "default_output_dir")]
    pub output_dir: String,
    #[serde(default)]
    pub defaults: Defaults,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Defaults {
    #[serde(default = "default_true")]
    pub strict: bool,
    #[serde(default = "default_state_mode")]
    pub state_mode: String,
}

impl Default for Defaults {
    fn default() -> Self {
        Defaults {
            strict: true,
            state_mode: default_state_mode(),
        }
    }
}

fn default_terraform_binary() -> String {
    "terraform".to_string()
}

fn default_output_dir() -> String {
    "output".to_string()
}

fn default_true() -> bool {
    true
}

fn default_state_mode() -> String {
    "remote_only".to_string()
}

/// `shrubs/<name>/shrub.toml` (design v0.2 §5.2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShrubConfig {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// One of the design's example types (`experiment`, `feature`,
    /// `migration`, `hotfix`, `override`). The design lists them as
    /// examples, so any string is accepted.
    #[serde(default = "default_shrub_type", rename = "type")]
    pub shrub_type: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, toml::Value>,
}

fn default_shrub_type() -> String {
    "feature".to_string()
}

/// `envs/<name>/env.toml` (design v0.2 §5.3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvConfig {
    pub name: String,
    #[serde(default = "default_base")]
    pub base: String,
    /// Shrubs applied to this env, in application order (design v0.2 §6.1).
    #[serde(default)]
    pub shrubs: Vec<String>,
    #[serde(default)]
    pub context: EnvContext,
    #[serde(default)]
    pub metadata: BTreeMap<String, toml::Value>,
}

fn default_base() -> String {
    "land".to_string()
}

/// Execution context of an env (design v0.2 §8.1): every field must be
/// stated explicitly before the env can be built.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EnvContext {
    #[serde(default)]
    pub provider: String,
    #[serde(default)]
    pub account: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub backend: String,
}

impl EnvContext {
    /// Names of the context fields that are still empty.
    pub fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.provider.is_empty() {
            missing.push("provider");
        }
        if self.account.is_empty() {
            missing.push("account");
        }
        if self.region.is_empty() {
            missing.push("region");
        }
        if self.backend.is_empty() {
            missing.push("backend");
        }
        missing
    }

    /// Whether every context field is stated (design v0.2 §8.1).
    /// Kept as the boolean companion of `missing_fields`; used by tests.
    #[allow(dead_code)]
    pub fn is_complete(&self) -> bool {
        self.missing_fields().is_empty()
    }
}

/// Read and parse a TOML file, with the path in the error message.
pub fn read_toml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| Error::new(format!("cannot read {}: {e}", path.display())))?;
    toml::from_str(&text)
        .map_err(|e| Error::new(format!("cannot parse {}: {e}", path.display())))
}

/// Serialize a value as pretty TOML and write it to a file.
pub fn write_toml<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let text = toml::to_string_pretty(value)
        .map_err(|e| Error::new(format!("cannot serialize {}: {e}", path.display())))?;
    std::fs::write(path, text)
        .map_err(|e| Error::new(format!("cannot write {}: {e}", path.display())))?;
    Ok(())
}

/// A name usable for an env or shrub directory: ASCII alphanumerics,
/// `-` and `_`, starting with an alphanumeric. Keeps names safe to use
/// as a single path component.
pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_design_example_tesh_toml() {
        // design v0.2 §5.1
        let cfg: ProjectConfig = toml::from_str(
            r#"
name = "sample-platform"
terraform_binary = "terraform"
output_dir = "output"

[defaults]
strict = true
state_mode = "remote_only"
"#,
        )
        .unwrap();
        assert_eq!(cfg.name, "sample-platform");
        assert_eq!(cfg.terraform_binary, "terraform");
        assert_eq!(cfg.output_dir, "output");
        assert!(cfg.defaults.strict);
        assert_eq!(cfg.defaults.state_mode, "remote_only");
    }

    #[test]
    fn parses_empty_tesh_toml_to_defaults() {
        let cfg: ProjectConfig = toml::from_str("").unwrap();
        assert_eq!(cfg.terraform_binary, "terraform");
        assert_eq!(cfg.output_dir, "output");
        assert!(cfg.defaults.strict);
        assert_eq!(cfg.defaults.state_mode, "remote_only");
    }

    #[test]
    fn parses_design_example_shrub_toml() {
        // design v0.2 §5.2
        let cfg: ShrubConfig = toml::from_str(
            r#"
name = "waf"
description = "WAF v2 rules for production"
type = "feature"

[metadata]
owner = "gomi"
purpose = "Security hardening"
"#,
        )
        .unwrap();
        assert_eq!(cfg.name, "waf");
        assert_eq!(cfg.shrub_type, "feature");
        assert_eq!(cfg.metadata["owner"].as_str(), Some("gomi"));
    }

    #[test]
    fn parses_design_example_env_toml() {
        // design v0.2 §5.3
        let cfg: EnvConfig = toml::from_str(
            r#"
name = "prod"
base = "land"
shrubs = ["waf", "cloudfront", "monitoring"]

[context]
provider = "aws"
account = "production"
region = "ap-northeast-1"
backend = "s3://my-tfstate/prod"

[metadata]
purpose = "Production environment"
"#,
        )
        .unwrap();
        assert_eq!(cfg.name, "prod");
        assert_eq!(cfg.base, "land");
        assert_eq!(cfg.shrubs, ["waf", "cloudfront", "monitoring"]);
        assert!(cfg.context.is_complete());
    }

    #[test]
    fn context_missing_fields_are_listed() {
        let ctx = EnvContext {
            provider: "aws".to_string(),
            ..Default::default()
        };
        assert_eq!(ctx.missing_fields(), ["account", "region", "backend"]);
        assert!(!ctx.is_complete());
    }

    #[test]
    fn name_validation() {
        assert!(is_valid_name("dev"));
        assert!(is_valid_name("test-v2"));
        assert!(is_valid_name("prod_1"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("-dev"));
        assert!(!is_valid_name("a/b"));
        assert!(!is_valid_name(".."));
        assert!(!is_valid_name("a b"));
    }
}
