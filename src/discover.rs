//! Git-root discovery. Local mode never spawns `git`.

use crate::error::{Error, Result};
use std::fs;
use std::path::{Path, PathBuf};

const SKIP_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".venv",
    "__pycache__",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "target",
    "vendor",
];

/// One repository root with a stable display name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepoRef {
    /// Canonical filesystem root.
    pub root: PathBuf,
    /// Short name used in Markdown unless a collision forces a longer form.
    pub name: String,
}

/// Target repositories and the peer set used for matching.
#[derive(Debug, Clone)]
pub struct Scope {
    /// Repositories that receive a `WEAVATRIX.md`.
    pub targets: Vec<RepoRef>,
    /// Repositories whose evidence participates in matching.
    pub peers: Vec<RepoRef>,
}

/// Resolves the current directory or an explicit path into a sibling-peer scope.
///
/// # Errors
///
/// The path is missing or cannot be canonicalized.
pub fn target_scope(path: Option<&str>) -> Result<Scope> {
    let start = match path {
        Some(value) => PathBuf::from(value),
        None => std::env::current_dir()?,
    };
    if !start.exists() {
        return Err(Error::path("path not found:", &start));
    }
    let root = git_root(&start).unwrap_or_else(|| canonicalize(&start));
    let mut peers = sibling_git_roots(&root);
    if !peers.iter().any(|repo| repo.root == root) {
        peers.push(repo_ref(root.clone()));
    }
    peers.sort();
    peers.dedup();
    assign_unique_names(&mut peers);
    let target = peers
        .iter()
        .find(|repo| repo.root == root)
        .cloned()
        .ok_or_else(|| Error::new("failed to name target repository"))?;
    Ok(Scope {
        targets: vec![target],
        peers,
    })
}

/// Recursively finds Git repositories under `folder`.
///
/// Nested directories inside a Git root are not separate repositories.
///
/// # Errors
///
/// The folder is missing or unreadable.
pub fn folder_scope(folder: &str) -> Result<Scope> {
    let root = PathBuf::from(folder);
    if !root.is_dir() {
        return Err(Error::path("folder not found:", &root));
    }
    let mut found = Vec::new();
    collect_git_roots(&canonicalize(&root), &mut found);
    found.sort();
    found.dedup();
    if found.is_empty() {
        return Err(Error::new(
            "no Git repositories found under the given folder",
        ));
    }
    assign_unique_names(&mut found);
    Ok(Scope {
        targets: found.clone(),
        peers: found,
    })
}

fn sibling_git_roots(root: &Path) -> Vec<RepoRef> {
    let Some(parent) = root.parent() else {
        return vec![repo_ref(root.to_path_buf())];
    };
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(parent) else {
        return vec![repo_ref(root.to_path_buf())];
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && is_git_root(&path) {
            found.push(repo_ref(canonicalize(&path)));
        }
    }
    found
}

fn collect_git_roots(dir: &Path, out: &mut Vec<RepoRef>) {
    if is_git_root(dir) {
        out.push(repo_ref(dir.to_path_buf()));
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut children: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    children.sort();
    for child in children {
        let name = child
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if SKIP_DIRS.contains(&name) || name.starts_with('.') {
            continue;
        }
        collect_git_roots(&child, out);
    }
}

/// Walks up from `start` until a Git root is found.
#[must_use]
pub fn git_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_file() {
        start
            .parent()
            .map_or_else(|| start.to_path_buf(), Path::to_path_buf)
    } else {
        start.to_path_buf()
    };
    loop {
        if is_git_root(&current) {
            return Some(canonicalize(&current));
        }
        if !current.pop() {
            return None;
        }
    }
}

fn is_git_root(path: &Path) -> bool {
    path.join(".git").exists()
}

fn repo_ref(root: PathBuf) -> RepoRef {
    let name = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("repository")
        .to_owned();
    RepoRef { root, name }
}

fn assign_unique_names(repos: &mut [RepoRef]) {
    let mut seen = std::collections::BTreeMap::<String, usize>::new();
    for repo in repos.iter() {
        *seen.entry(repo.name.clone()).or_insert(0) += 1;
    }
    for repo in repos.iter_mut() {
        if seen.get(&repo.name).copied().unwrap_or(0) > 1 {
            let parent = repo
                .root
                .parent()
                .and_then(Path::file_name)
                .and_then(|value| value.to_str())
                .unwrap_or("repo");
            repo.name = format!("{parent}/{}", repo.name);
        }
    }
}

fn canonicalize(path: &Path) -> PathBuf {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    strip_verbatim(canonical)
}

fn strip_verbatim(path: PathBuf) -> PathBuf {
    let Some(value) = path.to_str() else {
        return path;
    };
    if let Some(stripped) = value.strip_prefix(r"\\?\") {
        if let Some(unc) = stripped.strip_prefix("UNC\\") {
            return PathBuf::from(format!(r"\\{unc}"));
        }
        return PathBuf::from(stripped);
    }
    path
}
