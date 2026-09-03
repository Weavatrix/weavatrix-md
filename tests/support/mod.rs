//! Shared fixture helpers.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicU64 = AtomicU64::new(1);

pub struct Workspace {
    pub root: PathBuf,
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn workspace(label: &str) -> Workspace {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |item| item.as_nanos());
    let id = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "weavatrix-md-{label}-{}-{id}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("workspace");
    Workspace { root }
}

pub fn repo(root: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
    let path = root.join(name);
    for (relative, contents) in files {
        let file = path.join(relative);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).expect("parent");
        }
        fs::write(file, contents).expect("write");
    }
    fs::create_dir_all(path.join(".git")).expect("git");
    fs::write(path.join(".git/HEAD"), "ref: refs/heads/main\n").expect("head");
    path
}

pub fn read_md(repo: &Path) -> String {
    fs::read_to_string(repo.join("WEAVATRIX.md")).expect("WEAVATRIX.md")
}

pub fn generate_folder(root: &Path) -> weavatrix_md::CliOutput {
    weavatrix_md::run(&["--folder".to_owned(), root.to_string_lossy().into_owned()])
}
