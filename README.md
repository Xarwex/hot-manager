# Hot manager
Hot manager is a tool that enables users to quickly prototype on `.config` files that contain symlinks.
The main use of this tool is to work with a `home-manager` setup, where configuration files are symlinked into an immutable store.
This enables rapid prototyping which can leverage hot reloading, which is often useless when paired with a standard nix setup.

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
hot-manager --dotfiles-config-dir ~/dotfiles/.config

# Use a config file for explicit mappings
hot-manager --hotmanager-config-path ~/dotfiles/hot-manager.toml
```

For files that are not matched against your dotfiles - there will be a folder with all of the so called "temporary mappings". This will be a folder with a name in a format `hot-manager.<random_characters>`. All of the temporary config files will be there, you can edit them normally and they will be hot reloaded as well!

## Config file

The config file is a TOML file with the following fields:

```toml
# Explicit mappings: symlinked config path -> source path.
# Paths are resolved relative to their config file's directory (keys)
# or the config file's directory (values), unless they are absolute.
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

- Windows-specific files: `*.dll`, `*.exe`, `*.so*`, `*.drv`, `*.com`, `*.cpl`, `*.ocx`
- Heroic launcher: `heroic/**/*`
- OBS Studio plugin configs: `obs-studio/plugin_config/**/*`
- Steam singleton files: `steam/**/Singleton*`
- sops-nix secrets: `sops-nix/**/*`

## License

MIT — see [LICENSE](LICENSE).
