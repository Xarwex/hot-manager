use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use snafu::{Whatever, whatever};
use walkdir::DirEntry;

/// This function detects all of the relevant files that we can monitor in the `config_dir` passed.
///
/// `exclude_set` is a set of paths that we explicitly don't want to monitor.
/// `detect_symlinks` indicates that we should also detect the symlinks present
/// `detect_files` indicates that we should detect normal files present.
///
/// Note that at least one one of the `detect` options must be set to `true` in order for the
/// function to do anything.
pub(crate) fn detect_config_paths(
    config_dir: impl AsRef<Path>,
    exclude_set: &HashSet<PathBuf>,
    detect_symlinks: bool,
    detect_files: bool,
) -> Result<HashSet<PathBuf>, Whatever> {
    let config_dir = config_dir.as_ref();
    if !config_dir.is_dir() {
        whatever!("{config_dir:?} is not a directory!");
    }
    let walk_dir = walkdir::WalkDir::new(config_dir)
        .follow_root_links(true)
        .into_iter()
        .filter_map(|e| e.inspect_err(|err| tracing::warn!("{err}")).ok())
        .filter(|e| {
            let metadata = e.metadata().unwrap();
            !exclude_set.contains(e.path())
                && ((detect_symlinks
                    && metadata.is_symlink()
                    && std::fs::read_link(e.path()).is_ok())
                    || (detect_files && metadata.is_file()))
        });

    Ok(walk_dir.map(DirEntry::into_path).collect())
}

/// This function returns a mapping from config paths to the paths we're supposed to match onto in
/// the source of the config paths (the config directory in the dotfiles)
pub(crate) fn match_config_to_dotfiles(
    config_paths: &HashSet<PathBuf>,
    dotfiles_paths: HashSet<PathBuf>,
    dotfiles_config_dir: impl AsRef<Path>,
) -> Result<HashMap<PathBuf, PathBuf>, Whatever> {
    config_paths
        .iter()
        .chain(dotfiles_paths.iter())
        .try_for_each(|e| {
            if e.is_relative() {
                whatever!("{e:?} is supposed to be absolute!")
            };
            Ok(())
        })?;

    // This is now a map from key to the actual path
    // Don't make this a hashmap as we will need to modify the keys and also drop any duplicated
    // matches.
    let mut dotfiles_paths = dotfiles_paths
        .into_iter()
        .map(|entry| {
            (
                entry
                    .strip_prefix(dotfiles_config_dir.as_ref())
                    .unwrap()
                    .to_path_buf(),
                entry,
            )
        })
        .collect::<Vec<_>>();

    let mut skip_components = 0;
    let mut config_path_to_dotfiles_config_path = Vec::new();
    // This is guaranteed to terminate since from a certain point, `get_path_suffix` returns None
    while !dotfiles_paths.is_empty() {
        // Create new, reduced keys
        dotfiles_paths = dotfiles_paths
            .into_iter()
            .filter_map(|(key, value)| Some((get_path_suffix(key, skip_components)?, value)))
            .collect();

        tracing::debug!(
            "Dotfiles paths with skip_components = {skip_components} are {dotfiles_paths:#?}"
        );

        // Check for reduced key clashes
        let mut seen_keys = HashSet::new();
        let mut clashing_keys = HashSet::new();

        for (key, _) in dotfiles_paths.iter() {
            if seen_keys.insert(key) {
                clashing_keys.insert(key.clone());
            }
        }

        dotfiles_paths.retain(|(key, _)| clashing_keys.contains(key));

        // Perform the actual mapping
        // Retain only the unmapped dotfiles_paths - they will be used in subsequent iterations.
        dotfiles_paths.retain(|(key, value)| {
            let mut keep = true;
            config_paths.iter().for_each(|config_path| {
                if config_path.ends_with(key) {
                    config_path_to_dotfiles_config_path.push((config_path.clone(), value.clone()));
                    keep = false;
                }
            });
            keep
        });

        // If the paths are not empty yet, then increment the skip counter
        skip_components += 1;
    }

    // Now we need to remove duplicate mappings from either side
    let mut seen_keys = HashSet::new();
    let mut duplicate_keys = HashSet::new();
    let mut seen_values = HashSet::new();
    let mut duplicate_values = HashSet::new();

    for (key, value) in config_path_to_dotfiles_config_path.iter() {
        if !seen_keys.insert(key) {
            duplicate_keys.insert(key.clone());
        }
        if !seen_values.insert(value) {
            duplicate_values.insert(value.clone());
        }
    }

    tracing::info!("List of duplicate config paths: {duplicate_keys:#?}");
    tracing::info!("List of duplicate dotfiles config paths: {duplicate_values:#?}");
    tracing::info!("Removing entries that contain duplicates");

    config_path_to_dotfiles_config_path
        .retain(|(key, value)| !duplicate_keys.contains(key) && !duplicate_values.contains(value));

    Ok(HashMap::from_iter(config_path_to_dotfiles_config_path))
}

/// Given a path, skips the first `skip_components` components and returns the result if there is
/// any.
fn get_path_suffix(path: impl AsRef<Path>, skip_components: usize) -> Option<PathBuf> {
    let path: PathBuf = path.as_ref().components().skip(skip_components).collect();
    path.components().next()?;
    Some(path)
}
