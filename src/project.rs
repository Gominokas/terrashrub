//! A loaded tesh project: the root config plus every shrub and env
//! found under `shrubs/` and `envs/` (design v0.2 §4).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::{EnvConfig, ProjectConfig, ShrubConfig, read_toml};
use crate::error::{Error, Result};

#[derive(Debug)]
pub struct Shrub {
    pub config: ShrubConfig,
    pub dir: PathBuf,
}

#[derive(Debug)]
pub struct Env {
    pub config: EnvConfig,
    pub dir: PathBuf,
}

#[derive(Debug)]
pub struct Project {
    pub root: PathBuf,
    pub config: ProjectConfig,
    pub shrubs: BTreeMap<String, Shrub>,
    pub envs: BTreeMap<String, Env>,
}

impl Project {
    pub fn land_dir(&self) -> PathBuf {
        self.root.join("land")
    }

    pub fn shrubs_dir(&self) -> PathBuf {
        self.root.join("shrubs")
    }

    pub fn envs_dir(&self) -> PathBuf {
        self.root.join("envs")
    }

    pub fn output_dir(&self) -> PathBuf {
        self.root.join(&self.config.output_dir)
    }

    /// Load a project from its root directory (the one holding
    /// `tesh.toml`). Parse errors are hard errors; consistency issues
    /// are reported by [`Project::validate`] instead, so `doctor` can
    /// list them all at once.
    pub fn load(root: &Path) -> Result<Project> {
        let config_path = root.join("tesh.toml");
        if !config_path.is_file() {
            return Err(Error::new(format!(
                "no tesh.toml found in {} (run tesh from a project root)",
                root.display()
            )));
        }
        let config: ProjectConfig = read_toml(&config_path)?;

        let mut shrubs = BTreeMap::new();
        for dir in subdirs(&root.join("shrubs"))? {
            let cfg_path = dir.join("shrub.toml");
            if !cfg_path.is_file() {
                continue;
            }
            let cfg: ShrubConfig = read_toml(&cfg_path)?;
            let key = dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            shrubs.insert(key, Shrub { config: cfg, dir });
        }

        let mut envs = BTreeMap::new();
        for dir in subdirs(&root.join("envs"))? {
            let cfg_path = dir.join("env.toml");
            if !cfg_path.is_file() {
                continue;
            }
            let cfg: EnvConfig = read_toml(&cfg_path)?;
            let key = dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            envs.insert(key, Env { config: cfg, dir });
        }

        Ok(Project {
            root: root.to_path_buf(),
            config,
            shrubs,
            envs,
        })
    }

    /// Consistency problems (errors) and notes (informational), for
    /// `tesh doctor` (design v0.2 §7.10).
    pub fn validate(&self) -> (Vec<String>, Vec<String>) {
        let mut problems = Vec::new();
        let mut notes = Vec::new();

        if self.config.name.is_empty() {
            problems.push(
                "tesh.toml: `name` is empty (design v0.2 §5.1 expects a project name)"
                    .to_string(),
            );
        }
        // Design v0.2 §8.3: remote state only.
        if self.config.defaults.state_mode != "remote_only" {
            problems.push(format!(
                "tesh.toml: defaults.state_mode is {:?}, but only \"remote_only\" is allowed (design v0.2 §8.3)",
                self.config.defaults.state_mode
            ));
        }

        for (dir_name, shrub) in &self.shrubs {
            if shrub.config.name != *dir_name {
                problems.push(format!(
                    "shrubs/{dir_name}/shrub.toml: name is {:?} but the directory is {dir_name:?}",
                    shrub.config.name
                ));
            }
        }

        let mut used_shrubs: Vec<&str> = Vec::new();
        for (dir_name, env) in &self.envs {
            if env.config.name != *dir_name {
                problems.push(format!(
                    "envs/{dir_name}/env.toml: name is {:?} but the directory is {dir_name:?}",
                    env.config.name
                ));
            }
            for shrub_name in &env.config.shrubs {
                used_shrubs.push(shrub_name.as_str());
                if !self.shrubs.contains_key(shrub_name) {
                    problems.push(format!(
                        "env {dir_name:?}: shrub {shrub_name:?} is listed in env.toml but shrubs/{shrub_name}/ does not exist"
                    ));
                }
            }
            // Design v0.2 §8.1: the execution context must be explicit.
            let missing = env.config.context.missing_fields();
            if !missing.is_empty() {
                problems.push(format!(
                    "env {dir_name:?}: context is incomplete, missing: {}",
                    missing.join(", ")
                ));
            }
        }

        for name in self.shrubs.keys() {
            if !used_shrubs.contains(&name.as_str()) {
                notes.push(format!("shrub {name:?} is not used by any env"));
            }
        }

        (problems, notes)
    }
}

/// Subdirectories of `dir`, sorted by path. A missing directory counts
/// as empty.
fn subdirs(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| Error::new(format!("cannot list {}: {e}", dir.display())))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    Ok(dirs)
}

/// `.tf` files directly inside `dir`, sorted by file name.
pub fn sorted_tf_files(dir: &Path) -> Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| Error::new(format!("cannot list {}: {e}", dir.display())))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "tf"))
        .collect();
    files.sort();
    Ok(files)
}
