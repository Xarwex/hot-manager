use std::{collections::HashMap, path::PathBuf, time::Duration};

use notify::{
    Config, EventHandler, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
    event::{AccessKind, AccessMode},
};

use crate::hotlink::HotlinkedFile;
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// This function uses an inotify watcher to monitor the hotlinked files and relink them every time
/// they are changed.
///
/// We need to do that because hot reloading monitors the change to some underlying config file,
/// but if that file is a symlink, changes from the symlinked file are not propagated to the
/// symlink. Relinking will recreate the symlink, resulting in a message that will trigger hot
/// reloading.
///
/// The shutdown channel handle is here in order to guarantee that the underlying watch task gets
/// dropped before the program exits. Otherwise the hotlinked files may be dropped after the
/// program exists which will not call their `drop` implementation bringing them back to the
/// original state.
pub fn relinker(
    hotlinks: HashMap<PathBuf, HotlinkedFile>,
    shutdown_channel_tx: std::sync::mpsc::Sender<()>,
) -> notify::INotifyWatcher {
    let directories = get_directories_to_watch(hotlinks.keys().cloned());
    let mut watcher = RecommendedWatcher::new(
        Relinker {
            hotlinks,
            shutdown_channel_tx,
        },
        Config::default().with_poll_interval(POLL_INTERVAL),
    )
    .unwrap();

    crate::output::print_list("Watching directories", &directories);
    directories
        .iter()
        .for_each(|path| watcher.watch(path, RecursiveMode::Recursive).unwrap());
    watcher
}

struct Relinker {
    hotlinks: HashMap<PathBuf, HotlinkedFile>,
    shutdown_channel_tx: std::sync::mpsc::Sender<()>,
}

impl Drop for Relinker {
    fn drop(&mut self) {
        // Explicitly clear the hotlinks so that we're sure they are dropped here.
        self.hotlinks.clear();
        // Let the rx know that we've dropped the hotlinks.
        self.shutdown_channel_tx.send(()).unwrap()
    }
}

impl EventHandler for Relinker {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        match event {
            Ok(event) => {
                if let EventKind::Access(AccessKind::Close(AccessMode::Write)) = event.kind {
                    for file_path in event.paths.iter() {
                        if let Some(hotlinked_file) = self.hotlinks.get(file_path)
                            && let Err(e) = hotlinked_file.relink()
                        {
                            tracing::error!(
                                "Got an error {e} while trying to relink {file_path:?}"
                            )
                        }
                    }
                }
            }
            Err(e) => tracing::error!("Got error {e} from the relinker"),
        }
    }
}

/// This function, given a list of files, will figure out a set of shortest path contained within that list such that there are no two
/// path `p1` and `p2` such that `p2` is a prefix of `p1` where `pi` is parent directory of a file
/// passed in the list.
///
/// TLDR; good enough set of directories that we can watch for file changes
fn get_directories_to_watch(files: impl Iterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut directories = files
        .map(|config_path| config_path.parent().unwrap().to_path_buf())
        .collect::<Vec<_>>();

    directories.sort();

    let mut res = vec![];
    for directory in directories {
        if let Some(current) = res.last() {
            if !directory.starts_with(current) {
                res.push(directory);
            }
        } else {
            res.push(directory);
        }
    }
    res
}
