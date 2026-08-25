# Dotfig

Dotfig is a very simple opinionated CLI tool built in
[Rust](https://www.rust-lang.org/) to back up and restore your dotfiles or other
configuration files.

## Usage

Clone the repository:

```sh
git clone https://github.com/bobbymannino/dotfig.git
```

Build and install Dotfig from the repository:

```sh
cargo install --path .
```

Dotfig uses `dotfig.json` in the current directory by default. If it does not
exist, Dotfig creates it when you run a command.

List every configuration file Dotfig knows about:

```sh
dotfig --list --all
```

Add files to your configuration using their `Group:Title` names, then review the
configured paths:

```sh
dotfig --add "Zed:Settings"
dotfig --add "Git:Config"
dotfig --list
```

Back up the configured files:

```sh
dotfig --backup
```

Restore the live files from the backup:

```sh
dotfig --restore
```

> [!WARNING]
> Restoring replaces each configured live file with its backed-up copy.

Remove a file from the configuration with:

```sh
dotfig --remove "Git:Config"
```

By default, backups are stored in a `backups` directory next to the config file.
Set `savePath` to use another location:

```json
{
  "paths": ["Zed:Settings", "Git:Config"],
  "savePath": "~/Documents/dotfig"
}
```

Use `--config` (or `-c`) to work with a different config file:

```sh
dotfig --config ~/dotfiles/dotfig.json --backup
```

Run `dotfig --help` to see all available options.
