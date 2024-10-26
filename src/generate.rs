use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::DirectoryNode;

const CYAN: &str = "\x1B[36m";
const MAGENTA: &str = "\x1B[35m";
const YELLOW: &str = "\x1B[33m";
const RED: &str = "\x1B[31m";

fn determine_color(path: &Path) -> String {
    if path.is_symlink() {
        YELLOW // Symlinks
    } else if path.is_dir() {
        CYAN // Directories
    } else {
        MAGENTA // Regular files
    }
    .to_string()
}

static COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn build_directory_tree(dir: &str) -> DirectoryNode {
    let path = PathBuf::from(dir);

    if !path.is_dir() {
        return DirectoryNode {
            path: path.clone(),
            children: Vec::new(),
            matched: false,
            color: determine_color(&path),
            error: Some(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Error: '{dir}' is not a directory."),
            )),
        };
    }

    let children = match fs::read_dir(&path) {
        Ok(entries) => entries.filter_map(Result::ok),
        Err(e) => {
            return DirectoryNode {
                path,
                children: Vec::new(),
                matched: false,
                color: RED.to_string(),
                error: Some(e),
            };
        }
    }
    .map(|entry| {
        let count = COUNT.fetch_add(1, Ordering::SeqCst) + 1;
        if count % 100_000 == 0 {
            println!("Count: {count} {}\r", entry.path().display());
        }

        if entry
            .file_type()
            .expect("Failed to get file type for entry")
            .is_dir()
        {
            build_directory_tree(
                entry
                    .path()
                    .to_str()
                    .expect("Failed to convert path to string"),
            )
        } else {
            DirectoryNode {
                color: determine_color(&entry.path()),
                path: entry.path(),
                children: Vec::new(),
                matched: false,
                error: None,
            }
        }
    })
    .collect();

    DirectoryNode {
        path: path.clone(),
        children,
        matched: false,
        color: determine_color(&path),
        error: None,
    }
}
