use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use glob::glob;
use serde::Deserialize;

const DEFAULT_IGNORE_GLOBS: [&str; 11] = [
    // Ignore Windows specific files in case someone puts there proton stuff in.
    "**/*.dll",
    "**/*.exe",
    "**/*.so*",
    "**/*.drv",
    "**/*.com",
    "**/*.cpl",
    "**/*.ocx",
    // Heroic launcher holds a looot of proton files.
    "heroic/**/*",
    "obs-studio/plugin_config/**/*",
    "steam/**/Singleton*",
    "sops-nix/**/*",
];

#[derive(Debug)]
pub struct Config {
    /// Set of files that should be ignored derived from the passed glob expressions.
    pub exclude_set: HashSet<PathBuf>,

    /// This is a mapping from the symlinked config file to the source of the config file.
    /// This mappings is only valid if the paths are absolute.
    pub mappings: HashMap<PathBuf, PathBuf>,

    /// Whether to skip creating temporary mappings for orphaned config files.
    pub no_tmp_mappings: bool,
}

/// Raw config is a 1:1 mapping from the config file to a struct.
///
/// Raw config can be parsed into the that the program uses one.
#[derive(Debug, Default, Deserialize)]
struct RawConfig {
    /// List of glob expressions that should be ignored.
    #[serde(default)]
    exclude: Vec<String>,
    /// This is a mapping from the symlinked config file to the source of the config file.
    ///
    /// This mapping can be either relative to the config dir and hot-manager config dir respectively
    /// or they can be absolute.
    #[serde(default)]
    mappings: HashMap<PathBuf, PathBuf>,
    /// Exclude the default iglobs.
    #[serde(default)]
    no_default_exclude: bool,
    /// No automappings
    #[serde(default)]
    no_automappings: bool,
    /// Do not create temporary mappings for orphaned config files
    #[serde(default)]
    no_tmp_mappings: bool,
}

impl Config {
    pub fn from_file(path: impl AsRef<Path>, config_dir: impl AsRef<Path>) -> Self {
        let file_str =
            std::fs::read_to_string(path.as_ref()).expect("Failed to read the config file");
        let raw_config: RawConfig =
            toml::from_str(&file_str).expect("Failed to parse the config file");

        let mappings = to_absolute_paths(
            raw_config.mappings,
            path.as_ref().parent().unwrap(),
            config_dir.as_ref(),
        );

        Self::new(
            config_dir,
            mappings,
            raw_config.exclude,
            raw_config.no_default_exclude,
            raw_config.no_automappings,
            raw_config.no_tmp_mappings,
        )
    }

    pub fn new(
        config_dir: impl AsRef<Path>,
        mappings: HashMap<PathBuf, PathBuf>,
        iglobs: Vec<String>,
        no_default_exclude: bool,
        no_automappings: bool,
        no_tmp_mappings: bool,
    ) -> Self {
        let mut exclude_set = HashSet::new();
        let mut iglobs = iglobs
            .iter()
            .map(|glob_str| glob_str.as_str())
            .collect::<Vec<_>>();

        if !no_default_exclude {
            iglobs.extend(DEFAULT_IGNORE_GLOBS);
        }
        if no_automappings {
            iglobs.push("**/*");
        }
        for glob_str in iglobs {
            let paths = glob(config_dir.as_ref().join(glob_str).to_str().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .collect::<Vec<_>>();
            exclude_set.extend(paths);
        }

        Config {
            exclude_set,
            mappings,
            no_tmp_mappings,
        }
    }
}

fn to_absolute_paths(
    mappings: HashMap<PathBuf, PathBuf>,
    hotmanager_config_path: impl AsRef<Path>,
    config_dir: impl AsRef<Path>,
) -> HashMap<PathBuf, PathBuf> {
    mappings
        .into_iter()
        .map(|(key, value)| {
            let key = if key.is_relative() {
                config_dir.as_ref().join(key)
            } else {
                key
            };
            let value = if value.is_relative() {
                hotmanager_config_path.as_ref().join(value)
            } else {
                value
            };
            (key, value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_to_absolute_paths() {
        let hotmanager_config_path = Path::new("/tmp/hotmanager");
        let config_dir = Path::new("/home/user/.config");

        let mut mappings = HashMap::new();
        mappings.insert(
            PathBuf::from("relative_key"),
            PathBuf::from("relative_value"),
        );
        mappings.insert(PathBuf::from("/abs/key"), PathBuf::from("/abs/value"));

        let absolute_mappings = to_absolute_paths(mappings, hotmanager_config_path, config_dir);

        let expected_key1 = config_dir.join("relative_key");
        let expected_value1 = hotmanager_config_path.join("relative_value");
        assert_eq!(
            absolute_mappings.get(&expected_key1),
            Some(&expected_value1)
        );

        let expected_key2 = PathBuf::from("/abs/key");
        let expected_value2 = PathBuf::from("/abs/value");
        assert_eq!(
            absolute_mappings.get(&expected_key2),
            Some(&expected_value2)
        );
    }

    #[test]
    fn test_config_from_file_valid() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config_dir");
        fs::create_dir(&config_dir).unwrap();
        let hotmanager_config_dir = dir.path().join("hotmanager_config");
        fs::create_dir(&hotmanager_config_dir).unwrap();
        let config_path = hotmanager_config_dir.join("manager.toml");

        let toml_content = r#"
no_default_exclude = true
no_automappings = true

[mappings]
"app/conf" = "dotfiles/app.conf"
"/abs/path/conf" = "/abs/dotfiles/conf"
"#;
        fs::write(&config_path, toml_content).unwrap();

        let config = Config::from_file(&config_path, &config_dir);

        let expected_key1 = config_dir.join("app/conf");
        let expected_value1 = hotmanager_config_dir.join("dotfiles/app.conf");
        assert_eq!(config.mappings.get(&expected_key1), Some(&expected_value1));

        let expected_key2 = PathBuf::from("/abs/path/conf");
        let expected_value2 = PathBuf::from("/abs/dotfiles/conf");
        assert_eq!(config.mappings.get(&expected_key2), Some(&expected_value2));

        assert!(config.exclude_set.is_empty());
    }

    #[test]
    #[should_panic(expected = "Failed to read the config file")]
    fn test_config_from_file_not_found() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config_dir");
        fs::create_dir(&config_dir).unwrap();
        let config_path = dir.path().join("non_existent.toml");
        Config::from_file(config_path, &config_dir);
    }

    #[test]
    #[should_panic(expected = "Failed to parse the config file")]
    fn test_config_from_file_malformed() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path().join("config_dir");
        fs::create_dir(&config_dir).unwrap();
        let config_path = dir.path().join("malformed.toml");

        let toml_content = "this is not valid toml";
        fs::write(&config_path, toml_content).unwrap();

        Config::from_file(config_path, &config_dir);
    }

    #[test]
    fn test_config_new_globs() {
        let dir = tempdir().unwrap();
        let config_dir = dir.path();
        fs::write(config_dir.join("test.dll"), "").unwrap();
        fs::write(config_dir.join("another.txt"), "").unwrap();

        let config = Config::new(
            config_dir,
            HashMap::new(),
            vec!["**/*.txt".to_string()],
            false,
            false,
            false,
        );

        // It should contain the default globs + the custom one
        assert!(config.exclude_set.contains(&config_dir.join("test.dll")));
        assert!(config.exclude_set.contains(&config_dir.join("another.txt")));
    }
}
