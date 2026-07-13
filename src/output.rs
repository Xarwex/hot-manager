use colored::Colorize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn home_shorten(path: &Path) -> String {
    let s = path.display().to_string();
    if let Ok(home) = std::env::var("HOME") {
        if s == home {
            return "~".to_string();
        }
        if s.starts_with(&format!("{home}/")) {
            return format!("~{}", &s[home.len()..]);
        }
    }
    s
}

pub fn print_heading(label: &str) {
    println!("\n{}", label.bold());
}

pub fn print_mappings(mapping_type: &str, mappings: &HashMap<PathBuf, PathBuf>) {
    if mappings.is_empty() {
        return;
    }
    let mut entries: Vec<_> = mappings.iter().collect();
    entries.sort_by_key(|(k, _)| k.display().to_string());

    let label = format!("{mapping_type} mappings ({}):", entries.len());
    print_heading(&label);

    let max_src = entries
        .iter()
        .map(|(k, _)| home_shorten(k).len())
        .max()
        .unwrap_or(0);

    for (src, dst) in entries {
        let src_str = home_shorten(src);
        let dst_str = home_shorten(dst);
        println!(
            "  {:width$}  {}  {}",
            src_str,
            "→".cyan(),
            dst_str.green(),
            width = max_src,
        );
    }
}

pub fn print_list(label: &str, items: &[PathBuf]) {
    if items.is_empty() {
        return;
    }
    print_heading(&format!("{} ({}):", label, items.len()));
    for item in items {
        println!("  {}", home_shorten(item));
    }
}

pub fn print_relink(from: &Path, to: &Path) {
    println!(
        "  {}  {}  {}",
        home_shorten(from),
        "→".cyan(),
        home_shorten(to).green(),
    );
}

pub fn print_status(msg: &str) {
    println!("{} {}", "|>".green().bold(), msg);
}
