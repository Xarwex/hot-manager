use std::{io, path::PathBuf, process::ExitStatus};

use snafu::Snafu;

const HOTLINK: &str = "hotlink";

#[derive(Debug)]
pub(crate) struct HotlinkedFile {
    /// Path to the file that is currently symlinked to the original_file_path
    config_path: PathBuf,
    /// Original file path that is now a symlink to the config_path
    original_file_path: PathBuf,
    /// Path to the actual file that is now backed
    original_file_backed_path: PathBuf,
}

impl HotlinkedFile {
    /// This function re-creates the symlink from the config path to the original file path.
    ///
    /// Since the link points to an inode, there is a chance the original file is recreated,
    /// therefore there is a need to create the symlink again, in case the original one is broken.
    pub(crate) fn relink(&self) -> Result<(), HotlinkError> {
        crate::output::print_relink(&self.config_path, &self.original_file_path);

        // We could use some native implementation of touch but this gets the job done for now.
        // Alternatively we could use a crate that modifies the metadata.
        let exit_status = std::process::Command::new("touch")
            .arg(&self.original_file_path)
            .arg("--no-create")
            .arg("--no-dereference") // We want to change the symlink not the
            // underlying file
            .status()?;

        if !exit_status.success() {
            return Err(HotlinkError::TouchFailed { exit_status });
        }

        Ok(())
    }
}

impl Drop for HotlinkedFile {
    fn drop(&mut self) {
        assert_eq!(
            self.original_file_backed_path.extension().unwrap(),
            HOTLINK,
            "The final file extension should be {HOTLINK}"
        );
        let original_name = self.original_file_backed_path.with_extension("");
        tracing::debug!(
            "Renaming {:?} -> {original_name:?}",
            self.original_file_backed_path
        );
        std::fs::rename(&self.original_file_backed_path, &original_name).unwrap()
    }
}

#[derive(Debug, Snafu)]
pub(crate) enum HotlinkError {
    #[snafu(display("touch returned exit status {exit_status:?}"))]
    TouchFailed {
        exit_status: ExitStatus,
    },
    #[snafu(transparent)]
    Io {
        source: io::Error,
    },
    #[snafu(display("{path:?} already exists!"))]
    HotlinkedFileExists {
        path: PathBuf,
    },
    OriginalIsNotAFile {
        path: PathBuf,
    },
    NewIsNotAFile {
        path: PathBuf,
    },
    NewIsNotAbsolute {
        path: PathBuf,
    },
}

pub(crate) fn hotlink(
    original_file_path: PathBuf,
    config_path: PathBuf,
) -> Result<HotlinkedFile, HotlinkError> {
    // move the source path to source_path.hotlinked
    // new path is linked into the source path instead
    // keep both paths in a struct

    if !original_file_path.is_file() {
        return Err(HotlinkError::OriginalIsNotAFile {
            path: original_file_path,
        });
    }

    if !config_path.is_file() {
        return Err(HotlinkError::NewIsNotAFile { path: config_path });
    }

    if config_path.is_relative() {
        return Err(HotlinkError::NewIsNotAbsolute { path: config_path });
    }

    let original_file_backed_path = {
        let file_path = original_file_path
            .file_name()
            .expect("We expect the file to have a name")
            .to_str()
            .unwrap()
            .to_string();
        original_file_path.with_file_name(format!("{file_path}.{HOTLINK}"))
    };

    if original_file_backed_path.exists() {
        return Err(HotlinkError::HotlinkedFileExists {
            path: original_file_backed_path,
        });
    }

    tracing::debug!("Renaming {original_file_path:?} -> {original_file_backed_path:?}");
    std::fs::rename(&original_file_path, &original_file_backed_path)?;

    tracing::debug!("Symlinking {:?} -> {original_file_path:?}", config_path);
    std::os::unix::fs::symlink(&config_path, &original_file_path)?;

    Ok(HotlinkedFile {
        config_path,
        original_file_path,
        original_file_backed_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_hotlink_lifecycle() {
        let dir = tempdir().unwrap();
        let original_file_path = dir.path().join("original.conf");
        let config_path = dir.path().join("config.conf");

        fs::write(&original_file_path, "original content").unwrap();
        fs::write(&config_path, "new content").unwrap();

        let hotlinked_file = hotlink(original_file_path.clone(), config_path.clone()).unwrap();

        assert!(fs::symlink_metadata(&original_file_path)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_to_string(&original_file_path).unwrap(), "new content");
        let backup_path = dir.path().join("original.conf.hotlink");
        assert!(backup_path.exists());
        assert_eq!(fs::read_to_string(&backup_path).unwrap(), "original content");

        drop(hotlinked_file);

        assert!(!original_file_path.is_symlink());
        assert!(original_file_path.is_file());
        assert_eq!(fs::read_to_string(&original_file_path).unwrap(), "original content");
        assert!(!backup_path.exists());
    }

    #[test]
    fn test_hotlink_original_not_a_file() {
        let dir = tempdir().unwrap();
        let original_file_path = dir.path().join("original.conf");
        let config_path = dir.path().join("config.conf");
        fs::write(&config_path, "new content").unwrap();

        let result = hotlink(original_file_path, config_path);
        assert!(matches!(result, Err(HotlinkError::OriginalIsNotAFile { .. })));
    }
    
    #[test]
    fn test_hotlink_new_not_a_file() {
        let dir = tempdir().unwrap();
        let original_file_path = dir.path().join("original.conf");
        fs::write(&original_file_path, "original content").unwrap();
        let config_path = dir.path().join("config.conf");

        let result = hotlink(original_file_path, config_path);
        assert!(matches!(result, Err(HotlinkError::NewIsNotAFile { .. })));
    }

    #[test]
    fn test_hotlink_already_exists() {
        let dir = tempdir().unwrap();
        let original_file_path = dir.path().join("original.conf");
        let config_path = dir.path().join("config.conf");
        let backup_path = dir.path().join("original.conf.hotlink");

        fs::write(&original_file_path, "original content").unwrap();
        fs::write(&config_path, "new content").unwrap();
        fs::write(&backup_path, "backup content").unwrap();

        let result = hotlink(original_file_path, config_path);
        assert!(matches!(result, Err(HotlinkError::HotlinkedFileExists { .. })));
    }

    #[test]
    #[should_panic]
    fn test_drop_panics_if_backup_is_gone() {
        let dir = tempdir().unwrap();
        let original_file_path = dir.path().join("original.conf");
        let config_path = dir.path().join("config.conf");

        fs::write(&original_file_path, "original content").unwrap();
        fs::write(&config_path, "new content").unwrap();

        let hotlinked_file = hotlink(original_file_path.clone(), config_path.clone()).unwrap();
        let backup_path = dir.path().join("original.conf.hotlink");
        assert!(backup_path.exists());

        fs::remove_file(backup_path).unwrap();

        drop(hotlinked_file); // This should panic
    }

    #[test]
    fn test_relink() {
        let dir = tempdir().unwrap();
        let original_file_path = dir.path().join("original.conf");
        let config_path = dir.path().join("config.conf");

        fs::write(&original_file_path, "original content").unwrap();
        fs::write(&config_path, "new content").unwrap();

        let hotlinked_file = hotlink(original_file_path.clone(), config_path.clone()).unwrap();

        let mtime_before = fs::symlink_metadata(&original_file_path).unwrap().modified().unwrap();
        
        // We need to sleep a bit to make sure the modification time is different.
        std::thread::sleep(std::time::Duration::from_millis(10));
        
        hotlinked_file.relink().unwrap();

        let mtime_after = fs::symlink_metadata(&original_file_path).unwrap().modified().unwrap();

        assert!(mtime_after > mtime_before);
    }
}