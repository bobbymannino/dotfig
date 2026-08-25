mod config;
mod paths;
mod sync;
mod utils;

use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use anyhow::{Result, bail};
use clap::{CommandFactory, Parser, error::ErrorKind};
use tracing::{error, info, warn};

use crate::{
    config::{Config, PathKey},
    paths::Registry,
};

/// A simple CLI tool for creating and restoring backups for your dot files.
#[derive(Parser, Debug)]
#[command(version, about)]
// The bools are command line flags, collapsing them into an enum would hide them from clap.
#[allow(clippy::struct_excessive_bools)]
struct Args {
    #[clap(short, long, default_value = "dotfig.json")]
    config: String,

    /// Back up the configured paths
    #[clap(short, long, group = "action")]
    backup: bool,

    /// Restore the configured paths from their backups
    #[clap(short, long, group = "action")]
    restore: bool,

    /// List the paths in your config
    #[clap(short, long, group = "action")]
    list: bool,

    /// Add a path to your config, as `Group:Title`
    #[clap(short, long, group = "action", value_name = "PATH")]
    add: Option<String>,

    /// Remove a path from your config, as `Group:Title`
    #[clap(long, group = "action", value_name = "PATH")]
    remove: Option<String>,

    /// With --list, list every path dotfig knows about instead of your config
    ///
    /// `requires` is no use here, clap counts a `SetTrue` flag as always present, so this is checked by hand.
    #[clap(long)]
    all: bool,
}

/// What the user asked dotfig to do.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Backup,
    Restore,
    /// List the config, or the whole registry when `all` is set.
    List {
        all: bool,
    },
    Add(String),
    Remove(String),
}

impl Args {
    /// The requested action, if one was given.
    ///
    /// Clap guarantees at most one of these is set.
    fn action(&self) -> Option<Action> {
        if self.backup {
            return Some(Action::Backup);
        }

        if self.restore {
            return Some(Action::Restore);
        }

        if self.list {
            return Some(Action::List { all: self.all });
        }

        if let Some(raw) = &self.add {
            return Some(Action::Add(raw.clone()));
        }

        self.remove.clone().map(Action::Remove)
    }
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_ansi(console::colors_enabled())
        .with_target(false)
        .init();

    let args = Args::parse();

    if args.all && !args.list {
        Args::command()
            .error(ErrorKind::MissingRequiredArgument, "--all can only be used with --list")
            .exit();
    }

    let Some(action) = args.action() else {
        warn!("Nothing to do, pass --backup, --restore, --list, --add or --remove");
        return ExitCode::FAILURE;
    };

    let config_path = if let Ok(path) = utils::does_file_exist(&args.config) {
        path
    } else {
        warn!("Config file {} does not exist, creating now...", &args.config);
        if let Err(err) = std::fs::write(&args.config, "{}") {
            error!("Failed to create config file: {}", err);
            return ExitCode::FAILURE;
        }
        info!("Config file created successfully");
        PathBuf::from(&args.config)
    };

    match run(&action, &config_path) {
        Ok(code) => code,
        Err(err) => {
            error!("{err:#}");
            ExitCode::FAILURE
        }
    }
}

/// Carry out `action` against the config at `config_path`.
fn run(action: &Action, config_path: &Path) -> Result<ExitCode> {
    let mut config = Config::from_file(config_path)?;
    let registry = Registry::load()?;

    match action {
        Action::Add(raw) => add(&mut config, &registry, raw, config_path),
        Action::Remove(raw) => remove(&mut config, raw, config_path),
        Action::List { all: true } => Ok(list_all(&config, &registry)),
        Action::List { all: false } => list(&config, &registry, config_path),
        Action::Backup | Action::Restore => transfer(action, &config, &registry, config_path),
    }
}

/// Add a path to the config.
fn add(config: &mut Config, registry: &Registry, raw: &str, config_path: &Path) -> Result<ExitCode> {
    let key: PathKey = raw.parse()?;

    let Some(known) = registry.get(&key) else {
        bail!("Unknown path `{key}`, it is not one of the paths dotfig knows about");
    };

    // Store the spelling paths.json uses, whatever case was typed.
    if !config.add(known.key()) {
        info!("{} is already in your config", known.key());

        return Ok(ExitCode::SUCCESS);
    }

    config.to_file(config_path)?;

    info!("Added {} -> {}", known.key(), known.path);

    Ok(ExitCode::SUCCESS)
}

/// Remove a path from the config.
///
/// Unknown paths can still be removed, so a config that outlived a `paths.json` entry can be cleaned up.
fn remove(config: &mut Config, raw: &str, config_path: &Path) -> Result<ExitCode> {
    let key: PathKey = raw.parse()?;

    if !config.remove(&key) {
        warn!("{key} is not in your config, nothing to remove");

        return Ok(ExitCode::SUCCESS);
    }

    config.to_file(config_path)?;

    info!("Removed {key}");

    Ok(ExitCode::SUCCESS)
}

/// List the paths in the config.
fn list(config: &Config, registry: &Registry, config_path: &Path) -> Result<ExitCode> {
    info!("Backups are kept in {}", config.backups_dir(config_path)?.display());

    if config.paths.is_empty() {
        info!("No paths configured, add one with --add Group:Title");

        return Ok(ExitCode::SUCCESS);
    }

    let mut unknown = 0_usize;

    info!("{} path(s) configured", config.paths.len());

    for key in &config.paths {
        if let Some(known) = registry.get(key) {
            info!("  {key} -> {}", known.path);
        } else {
            warn!("  {key} is not one of the paths dotfig knows about");
            unknown += 1;
        }
    }

    if unknown > 0 {
        warn!("{unknown} path(s) cannot be backed up, remove them with --remove");
    }

    Ok(ExitCode::SUCCESS)
}

/// List every path in `paths.json`, marking the ones already configured.
fn list_all(config: &Config, registry: &Registry) -> ExitCode {
    let known_paths = registry.all();

    info!("{} path(s) available, add one with --add Group:Title", known_paths.len());

    for known in known_paths {
        let key = known.key();

        if config.paths.iter().any(|configured| configured.matches(&key)) {
            info!("  {key} -> {} (in your config)", known.path);
        } else {
            info!("  {key} -> {}", known.path);
        }
    }

    ExitCode::SUCCESS
}

/// Back up or restore every configured path.
fn transfer(action: &Action, config: &Config, registry: &Registry, config_path: &Path) -> Result<ExitCode> {
    if config.paths.is_empty() {
        info!("There are no paths loaded so nothing will be restored or backed up");

        return Ok(ExitCode::SUCCESS);
    }

    let mut resolved = Vec::with_capacity(config.paths.len());
    let mut invalid = 0_usize;

    for key in &config.paths {
        if let Some(known) = registry.get(key) {
            resolved.push(known);
        } else {
            warn!("Unknown path `{key}`, skipping");
            invalid += 1;
        }
    }

    if invalid > 0 {
        warn!("Skipped {invalid} unknown path(s), check them against paths.json");
    }

    info!("Resolved {} path(s) from {}", resolved.len(), config_path.display());

    let backups = config.backups_dir(config_path)?;

    let restoring = action == &Action::Restore;

    if restoring {
        info!("Restoring {} path(s) from {}", resolved.len(), backups.display());
    } else {
        info!("Backing up {} path(s) to {}", resolved.len(), backups.display());
    }

    let mut copied = 0_usize;
    let mut missing = 0_usize;
    let mut failed = 0_usize;

    for known in &resolved {
        let result = if restoring {
            sync::restore(&backups, known)
        } else {
            sync::backup(&backups, known)
        };

        match result {
            Ok(sync::Outcome::Copied(to)) => {
                info!("  {}:{} -> {}", known.group, known.title, to.display());
                copied += 1;
            }
            Ok(sync::Outcome::Missing(from)) => {
                warn!("  {}:{} has nothing at {}, skipping", known.group, known.title, from.display());
                missing += 1;
            }
            Err(err) => {
                error!("  {}:{} failed: {err:#}", known.group, known.title);
                failed += 1;
            }
        }
    }

    info!("{copied} copied, {missing} missing, {failed} failed");

    Ok(if failed > 0 { ExitCode::FAILURE } else { ExitCode::SUCCESS })
}
