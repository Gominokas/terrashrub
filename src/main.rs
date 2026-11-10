//! tesh — terrashrub: an environment-diff layer for Terraform/OpenTofu.
//!
//! Git manages history. Terraform executes. tesh manages environment
//! differences: `land` is the standard configuration, `shrub`s are
//! difference units, and an `env` is land + shrubs, generated into
//! `output/<env>/`. See `docs/design.md` (design v0.2).

mod build;
mod commands;
mod config;
mod error;
mod project;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use error::Result;

#[derive(Parser)]
#[command(name = "tesh", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new env (design v0.2 §7.1)
    Newenv {
        name: String,
        /// Copy shrubs and context from an existing env
        #[arg(long = "from")]
        from: Option<String>,
    },
    /// Create a new shrub (design v0.2 §7.2)
    Newshrub {
        name: String,
        /// Shrub type, e.g. feature, experiment, migration, hotfix, override
        #[arg(long = "type")]
        shrub_type: Option<String>,
        #[arg(long)]
        description: Option<String>,
    },
    /// Manage envs
    Env {
        #[command(subcommand)]
        command: EnvCommand,
    },
    /// Build an env into output/<env>/ (design v0.2 §7.4)
    Build { env: String },
    /// Build and run terraform plan (design v0.2 §7.5)
    Plan { env: String },
    /// Build and run terraform apply (design v0.2 §7.6)
    Apply { env: String },
    /// Compare two envs (design v0.2 §7.7)
    Diff { env_a: String, env_b: String },
    /// Explain where a value comes from (design v0.2 §7.8)
    Explain { env: String, path: String },
    /// Promote a shrub into land (design v0.2 §7.9)
    Promote {
        #[command(subcommand)]
        command: PromoteCommand,
    },
    /// Check the project for problems (design v0.2 §7.10)
    Doctor,
}

#[derive(Subcommand)]
enum EnvCommand {
    /// Add a shrub to an env (design v0.2 §7.3)
    AddShrub { env: String, shrub: String },
}

#[derive(Subcommand)]
enum PromoteCommand {
    /// Promote the named shrub
    Shrub {
        name: String,
        #[arg(long, default_value = "land")]
        to: String,
    },
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let root = PathBuf::from(".");

    match &cli.command {
        Command::Newenv { name, from } => {
            println!("{}", commands::newenv(&root, name, from.as_deref())?);
        }
        Command::Newshrub {
            name,
            shrub_type,
            description,
        } => {
            println!(
                "{}",
                commands::newshrub(
                    &root,
                    name,
                    shrub_type.as_deref(),
                    description.as_deref()
                )?
            );
        }
        Command::Env { command } => match command {
            EnvCommand::AddShrub { env, shrub } => {
                println!("{}", commands::add_shrub(&root, env, shrub)?);
            }
        },
        Command::Build { env } => {
            let project = project::Project::load(&root)?;
            let built = build::build(&project, env)?;
            for file in &built {
                println!("{} <- {}", file.name, file.sources.join(", "));
            }
            println!(
                "built env {env:?}: {} file(s) into {}",
                built.len(),
                project.output_dir().join(env).display()
            );
        }
        Command::Plan { .. } => {
            return Err(commands::not_yet_implemented("plan", "§7.5"));
        }
        Command::Apply { .. } => {
            return Err(commands::not_yet_implemented("apply", "§7.6"));
        }
        Command::Diff { .. } => {
            return Err(commands::not_yet_implemented("diff", "§7.7"));
        }
        Command::Explain { .. } => {
            return Err(commands::not_yet_implemented("explain", "§7.8"));
        }
        Command::Promote { .. } => {
            return Err(commands::not_yet_implemented("promote", "§7.9"));
        }
        Command::Doctor => {
            commands::doctor(&root)?;
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        let msg = e.to_string();
        if !msg.is_empty() {
            eprintln!("error: {msg}");
        }
        std::process::exit(1);
    }
}
