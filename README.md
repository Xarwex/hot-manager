# Hot manager 🥵
This tool enables temporary hot-reloading in your `home-manager` config.

Hot manager is a tool that enables users to quickly prototype on `.config` files that contain symlinks.
The main use of this tool is to work with a `home-manager` setup, where configuration files are symlinked into an immutable store.

This program temporarily changes symlinks from `.config` to nix store (or any other target for that matter) to point to the files that are present in some other folder (either your dotfiles or a temporary folder for configs that you define directly in nix files, or even both).

As an example consider the following dotfiles structure:

```
 dotfiles
 ├─ config
 │  ├─ yazi
 │  │  └─ init.lua
 │  ├─ television
 │  │  └─ config.toml
 │  ├─ starship.toml
  │  ...
 └─ home-manager
    ├─ atuin.nix
    ...
```

You define `yazi`, `tv` and `starship` as standalone config files and `atuin` is defined in nix. Hot manager will point the config symlinks to the `yazi/init.lua`, `television/config.toml` and `starship.toml` and create a temporary folder where you will be able to access atuin config files as well. At this point, modifying any of these files will have an instant effect in your `.config` directory rather than having to initiate a rebuild, so you can hack away at your `starship.toml` making proper use of the fact that it will be updated instantly rather than only after `nixos rebuild switch` or modify `atuin` to your liking before reflecting these changes in your `atuin.nix`.

Ironically, `hot-manager` itself does not support hot reloading. It is, simply put, too hot to handle.

## Table of contents

- [Installation](#installation)
- [Quick start](#quick-start)
- [CLI reference](#cli-reference)
- [Config file](#config-file)
- [How it works](#how-it-works)
- [License](#license)

## Installation

### Nix (recommended)

Run directly from the flake:

```sh
nix run github:Xarwex/hot-manager -- --help
```

Or add it to your system flake:

```nix
inputs.hot-manager.url = "github:Xarwex/hot-manager";
```

Once you do that, use the default package from the flake.

### Cargo

You're probably using nix but you can of course go the cargo way.

```sh
cargo install --git https://github.com/Xarwex/hot-manager
```

## Quick start

With a home-manager setup that symlinks config files into `~/.config`:

```sh
# Basic usage — watches ~/.config and autodetects symlinks
hot-manager

# Point at your dotfiles directory for automatic source matching
hot-manager --dotfiles-config-dir ~/dotfiles/config
# or even
hot-manager --dotfiles-config-dir ~/dotfiles/
# since automatching works on common suffixes

# Use a config file for explicit mappings
hot-manager --hotmanager-config-path ~/dotfiles/hot-manager.toml
```

Note: a config file is only loaded when you pass `--hotmanager-config-path`.
If you don't, an empty default config is used (no file is auto-discovered).
The temporary mappings folder is created in the system temp dir by default,
or inside your dotfiles config dir when `--dotfiles-config-dir` is given.

For files that are not matched against your dotfiles - there will be a folder with all of the so called "temporary mappings". This will be a folder with a name in a format `hot-manager.<random_characters>`. All of the temporary config files will be there, you can edit them normally and save to update `.config` accordingly. They will disappear once the program exits.

While the program is running you can either edit config files in your dotfiles (say `television/config.toml`) or the config files in the temporary directory (`hot-manager.<random_characters>/atuin.toml`) and it will be like editing `.config` itself!

## CLI reference

All flags are optional:

| Flag | Short | Description |
|------|-------|-------------|
| `--config-dir <PATH>` | | Target config directory to repoint. Defaults to `~/.config`. |
| `--dotfiles-config-dir <PATH>` | `-d` | Folder containing your config sources, used for automatic source matching. |
| `--hotmanager-config-path <PATH>` | | Path to the TOML config file (see [Config file](#config-file)). |
| `--fix` | | Restore any leftover `.hotlink` state and exit. |
| `--help` | `-h` | Print help. |

Additional behavior worth knowing:

- **Auto-recovery**: on every startup the program scans `config_dir/**/*.hotlink`
  and restores any leftover backups to their original paths, so a crash that left
  stale `.hotlink` files is self-healing. `--fix` runs this step and then exits.
- **Watching**: the watcher runs in poll mode with a ~1 second interval and
  watches directories (not individual files). Editing a file triggers a
  `relink` when the file handle is closed after a write.
- **Temporary mappings folder**: named `hot-manager.<random_characters>`. It is
  created in the system temp dir, or inside `--dotfiles-config-dir` when that
  flag is given, and is removed when the program exits.

## Config file

The config file is a TOML file with the following fields:

```toml
# Explicit mappings: symlinked config path -> source path.
# Keys are resolved relative to the config dir (~/.config by default,
# or --config-dir), values relative to the config file's own directory,
# unless they are absolute.
[mappings]
"alacritty/alacritty.toml" = "alacritty/alacritty.toml"
"hyprland/hyprland.conf" = "hyprland/hyprland.conf"
"/absolute/path/to/conf" = "/absolute/path/to/source"

# Glob patterns to exclude from detection.
# Patterns are matched against the config directory.
exclude = [
    "**/*.dll",
    "heroic/**/*",
    "sops-nix/**/*",
]

# Skip the default set of ignore globs (Windows binaries,
# heroic launcher, obs-studio plugin configs, etc.)
no_default_exclude = false

# Disable automatic detection of symlinks.
# Only explicit [mappings] will be used.
no_automappings = false

# Disable temporary mappings that is mappings that are not listed under [mappings] and are not autodetected, but are still valid config files.
no_tmp_mappings = false
```

### Default excludes

By default, hot-manager ignores:

- Windows-specific files: `**/*.dll`, `**/*.exe`, `**/*.so*`, `**/*.drv`, `**/*.com`, `**/*.cpl`, `**/*.ocx`
- Heroic launcher: `heroic/**/*`
- OBS Studio plugin configs: `obs-studio/plugin_config/**/*`
- Steam singleton files: `steam/**/Singleton*`
- sops-nix secrets: `sops-nix/**/*`

## How it works

### Matching
We match a dotfiles source file to a `.config` symlink by reducing the dotfiles path from the left (dropping leading components) and finding a reduced suffix that the `.config` path ends with. As we relax the prefix, more dotfiles paths may match the same `.config` path. When a single `.config` path ends up matching multiple reduced dotfiles suffixes (an ambiguous match), the mapping for that `.config` path is dropped.

### Modifying
Once we match files (or don't - then they are artificially matched against the temporary directory), we repin the symlinks from `.config` -> nix store to `.config` -> matched paths and save the original nix store paths under `.hotlink` extension in the original location, which we revert to after the program exits.
Once the file under the symlink is edited and the file handle is closed after a write, we touch the `.config` symlink itself (not its target) using `touch --no-create --no-dereference`. Normally the symlink's metadata is not updated when the underlying file changes, so we trigger that manually in order to signal to some running program that it may want to hot reload the file.


## Contributing

I am accepting contributions, but do raise an issue first to discuss :) Note that this is a small side project, but I will do my best to engage with any willing contributors in my spare time.

## License

MIT — see [LICENSE](LICENSE).
