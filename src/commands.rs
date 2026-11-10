//! Command implementations other than `build` (design v0.2 §7).

use std::path::Path;

use crate::config::{EnvConfig, ShrubConfig, is_valid_name, read_toml, write_toml};
use crate::error::{Error, Result};
use crate::project::Project;

fn require_valid_name(kind: &str, name: &str) -> Result<()> {
    if is_valid_name(name) {
        Ok(())
    } else {
        Err(Error::new(format!(
            "invalid {kind} name {name:?}: use ASCII letters, digits, '-' and '_', starting with a letter or digit"
        )))
    }
}

/// `tesh newenv <name> [--from <env>]` (design v0.2 §7.1).
pub fn newenv(root: &Path, name: &str, from: Option<&str>) -> Result<String> {
    require_valid_name("env", name)?;
    let project = Project::load(root)?;

    let env_dir = project.envs_dir().join(name);
    if env_dir.exists() {
        return Err(Error::new(format!("env {name:?} already exists")));
    }

    let mut config = EnvConfig {
        name: name.to_string(),
        base: "land".to_string(),
        shrubs: Vec::new(),
        context: Default::default(),
        metadata: Default::default(),
    };
    if let Some(from_name) = from {
        let source = project.envs.get(from_name).ok_or_else(|| {
            Error::new(format!(
                "--from env {from_name:?} not found (no envs/{from_name}/env.toml)"
            ))
        })?;
        config.shrubs = source.config.shrubs.clone();
        config.context = source.config.context.clone();
        config.base = source.config.base.clone();
    }

    std::fs::create_dir_all(&env_dir)
        .map_err(|e| Error::new(format!("cannot create {}: {e}", env_dir.display())))?;
    let path = env_dir.join("env.toml");
    write_toml(&path, &config)?;
    Ok(format!(
        "created env {name:?} at {}",
        path.display()
    ))
}

/// `tesh newshrub <name> [--type <type>] [--description <text>]`
/// (design v0.2 §7.2).
pub fn newshrub(
    root: &Path,
    name: &str,
    shrub_type: Option<&str>,
    description: Option<&str>,
) -> Result<String> {
    require_valid_name("shrub", name)?;
    let project = Project::load(root)?;

    let shrub_dir = project.shrubs_dir().join(name);
    if shrub_dir.exists() {
        return Err(Error::new(format!("shrub {name:?} already exists")));
    }

    let config = ShrubConfig {
        name: name.to_string(),
        description: description.unwrap_or_default().to_string(),
        shrub_type: shrub_type.unwrap_or("feature").to_string(),
        metadata: Default::default(),
    };

    std::fs::create_dir_all(&shrub_dir)
        .map_err(|e| Error::new(format!("cannot create {}: {e}", shrub_dir.display())))?;
    let path = shrub_dir.join("shrub.toml");
    write_toml(&path, &config)?;
    Ok(format!(
        "created shrub {name:?} at {}",
        path.display()
    ))
}

/// `tesh env add-shrub <env> <shrub>` (design v0.2 §7.3). Appends to
/// the env's ordered shrub list (design v0.2 §6.1).
pub fn add_shrub(root: &Path, env_name: &str, shrub_name: &str) -> Result<String> {
    let project = Project::load(root)?;
    let env = project.envs.get(env_name).ok_or_else(|| {
        Error::new(format!(
            "env {env_name:?} not found (no envs/{env_name}/env.toml)"
        ))
    })?;
    if !project.shrubs.contains_key(shrub_name) {
        return Err(Error::new(format!(
            "shrub {shrub_name:?} not found (no shrubs/{shrub_name}/shrub.toml)"
        )));
    }
    if env.config.shrubs.iter().any(|s| s == shrub_name) {
        return Err(Error::new(format!(
            "env {env_name:?} already has shrub {shrub_name:?}"
        )));
    }

    let path = env.dir.join("env.toml");
    let mut config: EnvConfig = read_toml(&path)?;
    config.shrubs.push(shrub_name.to_string());
    write_toml(&path, &config)?;
    Ok(format!(
        "added shrub {shrub_name:?} to env {env_name:?} (position {})",
        config.shrubs.len()
    ))
}

/// `tesh doctor` (design v0.2 §7.10): report every consistency
/// problem found in the project. Returns Ok(true) when healthy;
/// prints problems and returns Err(silent) otherwise, so the CLI
/// exits non-zero without a duplicated message.
pub fn doctor(root: &Path) -> Result<()> {
    let project = Project::load(root)?;
    let (problems, notes) = project.validate();
    for problem in &problems {
        println!("problem: {problem}");
    }
    for note in &notes {
        println!("note: {note}");
    }
    if problems.is_empty() {
        println!(
            "ok: no problems found ({} envs, {} shrubs)",
            project.envs.len(),
            project.shrubs.len()
        );
        Ok(())
    } else {
        println!("{} problem(s) found", problems.len());
        Err(Error::silent())
    }
}

/// Placeholder for commands whose design is settled in v0.2 §7 but
/// which are not implemented yet.
pub fn not_yet_implemented(command: &str, section: &str) -> Error {
    Error::new(format!(
        "`tesh {command}` is not yet implemented (design v0.2 {section})"
    ))
}
