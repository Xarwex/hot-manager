# Hot manager
Hot manager is a tool that enables users to quickly prototype on `.config` files that contain symlinks.
The main use of this tool is to work with a `home-manager` setup, where configuration files are symlinked into an immutable store.
This enables rapid prototyping which can leverage hot reloading, which is often useless when paired with a standard nix setup.

This program temporarily changes symlinks from `.config` to nix store (or any other target for that matter) to point to the files that are present in some other folder (either your dotfiles or/and a temporary folder for configs that you define directly in nix files).

As an example consider the following dotfiles structure:

```
 dotfiles
 ├─ config
 │  ├─ yazi
 │  │  └─ init.lua
 │  ├─ television
 │  │  └─ config.toml
 │  ├─ starship.toml
 |  ...
 └─ home-manager
    ├─ atuin.nix
    ...
```

You define `yazi`, `tv` and `starship` as standalone config files and `atuin` is defined in nix. Hot manager will point the config symlinks to the `yazi/init.lua`, `television/config.toml` and `starship.toml` and create a temporary folder where we you will be able to access atuin config files as well. At this point, modifying any of these files will have an instant effect in your `.config` directory rather than having to initiate a rebuild, so you can hack away at your `starship.toml` making proper use of the fact that it will be updated instantly rather than only after `nixos rebuild switch` or modify `atuin` to your liking before reflecting these changes in your `atuin.nix`.

Ironically, `hot-manager` itself does not support hot reloading.

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
# Default config path is .config/hot-manager.toml
```

For files that are not matched against your dotfiles - there will be a folder with all of the so called "temporary mappings". This will be a folder with a name in a format `hot-manager.<random_characters>`. All of the temporary config files will be there, you can edit them normally and save to update `.config` accordingly. They will disappear once the program exits.

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

## Contributing

I am accepting contributions, but do raise an issue first to discuss :) Note that this is a small side project, but I will do my best to engage with any willing contributors in my spare time.

## License

MIT — see [LICENSE](LICENSE).
