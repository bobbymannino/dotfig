mod config;
mod paths;
mod sync;
mod utils;

use std::{path::PathBuf, process::ExitCode};

use clap::Parser;
use tracing::{error, info, warn};

/// A simple CLI tool for creating and restoring backups for your dot files.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    #[clap(short, long, default_value = "dotfig.json")]
    config: String,

    /// Back up the configured paths
    #[clap(short, long, group = "action")]
    backup: bool,

    /// Restore the configured paths from their backups
    #[clap(short, long, group = "action")]
    restore: bool,
}

/// What the user asked dotfig to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    Backup,
    Restore,
}

impl Args {
    /// The requested action, if one was given.
    ///
    /// Clap guarantees `backup` and `restore` are never both set.
    fn action(&self) -> Option<Action> {
        match (self.backup, self.restore) {
            (true, _) => Some(Action::Backup),
            (_, true) => Some(Action::Restore),
            _ => None,
        }
    }
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_ansi(console::colors_enabled())
        .with_target(false)
        .init();

    let args = Args::parse();

    let Some(action) = args.action() else {
        warn!("Nothing to do, pass --backup or --restore");
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

    let config = match config::Config::from_file(&config_path) {
        Ok(config) => config,
        Err(err) => {
            error!("{err:#}");
            return ExitCode::FAILURE;
        }
    };

    if config.paths.is_empty() {
        info!("There are no paths loaded so nothing will be restored or backed up");
        return ExitCode::SUCCESS;
    }

    let registry = match paths::Registry::load() {
        Ok(registry) => registry,
        Err(err) => {
            error!("{err:#}");
            return ExitCode::FAILURE;
        }
    };

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

    let backups = config_path
        .parent()
        .map_or_else(|| PathBuf::from("backups"), |parent| parent.join("backups"));

    match action {
        Action::Backup => info!("Backing up {} path(s) to {}", resolved.len(), backups.display()),
        Action::Restore => info!("Restoring {} path(s) from {}", resolved.len(), backups.display()),
    }

    let mut copied = 0_usize;
    let mut missing = 0_usize;
    let mut failed = 0_usize;

    for known in &resolved {
        let result = match action {
            Action::Backup => sync::backup(&backups, known),
            Action::Restore => sync::restore(&backups, known),
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

    if failed > 0 { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
