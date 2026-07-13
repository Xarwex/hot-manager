use clap::Parser;
use config::Config;
use fix::fix_bad_state;
use hotlink::hotlink;
use snafu::Whatever;
use std::{
    collections::{HashMap, HashSet},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    str::FromStr,
};
use watcher::relinker;

use detect::{detect_config_paths, match_config_to_dotfiles};

mod config;
mod detect;
mod fix;
mod hotlink;
mod output;
mod watcher;

const HOME_DIR: &str = env!("HOME");

// These args should only enable very basic usage of the tool for now.
// Handle the elaborate cases through the config file instead
#[derive(Debug, Parser)]
struct Args {
    /// Path to the config files, defaults to ~/.config
    #[arg(long, default_value = PathBuf::from_str(HOME_DIR).unwrap().join(".config").into_os_string())]
    config_dir: PathBuf,

    /// Path to the folder which contains all of the config sources
    #[arg(long, short)]
    dotfiles_config_dir: Option<PathBuf>,

    /// Path to the hot-manager config file
    #[arg(long)]
    hotmanager_config_path: Option<PathBuf>,

    /// Fix the hotlinked paths to the original state.
    ///
    /// This should be used if the program was closed unexpectedly and there is some leftover bad
    /// state.
    /// Note that the fix function will also run during the normal run of the program, so only
    /// specify this flag if you don't want to run the rest of the tool.
    #[arg(long)]
    fix: bool,
}

fn main() -> Result<(), Whatever> {
    // Exit handler
    let (exit_tx, exit_rx) = std::sync::mpsc::channel();
    ctrlc::set_handler(move || exit_tx.send(()).unwrap()).unwrap();

    let args = Args::parse();

    tracing_subscriber::fmt::init();

    // This function is cheap enough that we can run it every time.
    fix_bad_state(&args.config_dir);
    if args.fix {
        tracing::info!("Fix flag specified - exiting");
        return Ok(());
    }

    let tmp_dir = if let Some(ref dotfiles_config_dir) = args.dotfiles_config_dir {
        tempfile::TempDir::with_prefix_in("hot-manager.", dotfiles_config_dir)
    } else {
        tempfile::TempDir::with_prefix("hot-manager.")
    }
    .unwrap();

    let hotmanager_config = args
        .hotmanager_config_path
        .map(|hotmanager_config_path| {
            config::Config::from_file(hotmanager_config_path, &args.config_dir)
        })
        .unwrap_or(Config::new(
            &args.config_dir,
            Default::default(),
            Default::default(),
            false,
            false,
            false,
        ));

    output::print_mappings("Config", &hotmanager_config.mappings);

    // Only detect symlinks - actual files can be dealt with normally
    let autodetected_config_paths = detect_config_paths(
        &args.config_dir,
        &hotmanager_config.exclude_set,
        true,
        false,
    )
    .unwrap();

    let autodetected_mappings = if let Some(ref dotfiles_config_dir) = args.dotfiles_config_dir {
        // Do not detect symlinks since the watcher will not deal with them properly
        let autodetected_dotfiles_paths =
            detect_config_paths(dotfiles_config_dir, &HashSet::default(), false, true).unwrap();

        match_config_to_dotfiles(
            &autodetected_config_paths,
            autodetected_dotfiles_paths,
            dotfiles_config_dir,
        )
        .unwrap()
    } else {
        Default::default()
    };

    output::print_mappings("Autodetected", &autodetected_mappings);

    let mappings = {
        // Merge autodetected with config mappings, giving priority to the config ones.
        let mut mappings = autodetected_mappings;
        for (k, v) in hotmanager_config.mappings {
            mappings.insert(k, v);
        }

        // Get the orphaned configs in, and place them in the temporary directory
        let tmp_mappings = if hotmanager_config.no_tmp_mappings {
            HashMap::new()
        } else {
            let mapped_paths = mappings.keys().cloned().collect::<HashSet<_>>();

            autodetected_config_paths
                .difference(&mapped_paths)
                .filter_map(|path| {
                    let tmp_file = tmp_dir
                        .path()
                        .join(path.strip_prefix(&args.config_dir).unwrap());
                    std::fs::create_dir_all(tmp_file.parent().unwrap()).unwrap();
                    std::fs::copy(path, &tmp_file)
                        .inspect_err(|e| {
                            tracing::warn!(
                                "Encountered {e} when trying to copy {path:?} -> {tmp_file:?}"
                            )
                        })
                        .ok()?;
                    std::fs::set_permissions(&tmp_file, std::fs::Permissions::from_mode(0o755))
                        .unwrap();
                    Some((path.clone(), tmp_file))
                })
                .collect::<HashMap<_, _>>()
        };

        output::print_mappings("Tmp", &tmp_mappings);
        mappings.extend(tmp_mappings);
        mappings
    };

    let hotlinks = mappings
        .into_iter()
        .map(|(config_path, scratch_config_path)| {
            (
                scratch_config_path.clone(),
                hotlink(config_path, scratch_config_path).unwrap(),
            )
        })
        .collect::<HashMap<_, _>>();

    let (relinker_drop_tx, relinker_drop_rx) = std::sync::mpsc::channel();
    let relinker = relinker(hotlinks, relinker_drop_tx);

    output::print_status("Watching for changes... (Ctrl+C to stop)");
    exit_rx.recv().unwrap();
    drop(relinker);
    // We need to wait for the message that the drop has completed since there seems to be some funky business happenning
    // to the hotlinked files otherwise
    relinker_drop_rx.recv().unwrap();
    tracing::info!("Relinker dropped!");

    Ok(())
}
