use std::path::Path;

use glob::glob;

/// This function will detect any leftover `.hotlink` files and bring them back to the original
/// state.
pub fn fix_bad_state(config_dir: impl AsRef<Path>) {
    let hotlink_glob = config_dir.as_ref().join("**/*.hotlink");
    for path in glob(hotlink_glob.to_str().unwrap())
        .unwrap()
        .filter_map(Result::ok)
        .collect::<Vec<_>>()
    {
        // This just removes the .hotlink extension according to the docs
        let original_path = path.with_extension("");
        crate::output::print_status(&format!(
            "Unscrewing {} -> {}",
            path.display(),
            original_path.display()
        ));

        // First we need to remove the original file
        std::fs::remove_file(&original_path).unwrap();
        // Now we need to rename the file to the original path
        std::fs::rename(&path, &original_path).unwrap();
    }
}
