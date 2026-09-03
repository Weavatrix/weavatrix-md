//! Scan → bounded read → parse → discard source.

use crate::detector::{detect_api, detect_database, detect_kafka};
use crate::discover::RepoRef;
use crate::error::Result;
use crate::model::{RepoId, RepoInventory};
use crate::{aliases, tokens};
use std::fs;
use std::path::Path;
use weavatrix_parse::{Language, extract};
use weavatrix_scan::{ScanOptions, Scanner};

const MAX_FILE_BYTES: u64 = 2_000_000;

const EXTENSIONS: &[&str] = &[
    "js",
    "jsx",
    "mjs",
    "cjs",
    "ts",
    "tsx",
    "mts",
    "cts",
    "rs",
    "py",
    "go",
    "java",
    "cs",
    "proto",
    "graphql",
    "gql",
    "json",
    "yml",
    "yaml",
    "toml",
    "env",
    "example",
    "sample",
    "properties",
    "xml",
    "tf",
    "sql",
    "swift",
];

/// Builds a compact inventory for one repository and drops all source.
///
/// # Errors
///
/// Repository scan fails.
pub fn inventory(repo: &RepoRef) -> Result<RepoInventory> {
    let mut options = ScanOptions::default()
        .with_extensions(EXTENSIONS.iter().copied())
        .selected_files_only();
    options.hash_file_contents = false;
    options.max_file_bytes = MAX_FILE_BYTES;
    options.parallelism = 2;
    let report = Scanner::new(&repo.root).options(options).scan()?;
    let mut inventory = RepoInventory {
        repo: RepoId {
            name: repo.name.clone(),
            root: repo.root.clone(),
        },
        aliases: aliases::repo_aliases(repo),
        api: Vec::new(),
        kafka: Vec::new(),
        databases: Vec::new(),
    };
    for file in &report.files {
        let relative = file.relative.replace('\\', "/");
        if skip_path(&relative) {
            continue;
        }
        let Ok(source) = fs::read_to_string(&file.absolute) else {
            continue;
        };
        if source.len() as u64 > MAX_FILE_BYTES || looks_binary(&source) {
            continue;
        }
        let language = language_for(&relative, &file.absolute);
        inspect_file(&mut inventory, &relative, &source, language);
    }
    inventory.api.sort();
    inventory.api.dedup();
    inventory.kafka.sort();
    inventory.kafka.dedup();
    inventory.databases.sort();
    inventory.databases.dedup();
    Ok(inventory)
}

fn inspect_file(
    inventory: &mut RepoInventory,
    relative: &str,
    source: &str,
    language: Option<Language>,
) {
    let facts = language.map(|item| extract(source, item));
    let stream = language.map(|item| tokens::Stream::new(source, item));
    let bindings = stream
        .as_ref()
        .map_or_else(Vec::new, tokens::Stream::bindings);
    detect_kafka(
        inventory,
        relative,
        source,
        facts.as_ref(),
        stream.as_ref(),
        &bindings,
    );
    detect_database(
        inventory,
        relative,
        source,
        facts.as_ref(),
        stream.as_ref(),
        &bindings,
    );
    detect_api(
        inventory,
        relative,
        source,
        facts.as_ref(),
        stream.as_ref(),
        &bindings,
    );
}

fn skip_path(relative: &str) -> bool {
    let path = relative.to_ascii_lowercase();
    path.contains("/test/")
        || path.contains("/tests/")
        || path.contains("/__tests__/")
        || path.contains("/testdata/")
        || path.contains(".test.")
        || path.contains(".spec.")
        || path.ends_with("_test.go")
        || path.ends_with("_test.rs")
        || path.ends_with(".md")
        || path.ends_with(".mdx")
        || path.ends_with(".rst")
        || path.ends_with(".html")
        || path.ends_with(".css")
        || path.ends_with(".scss")
}

fn looks_binary(source: &str) -> bool {
    source.as_bytes().iter().take(8192).any(|byte| *byte == 0)
}

fn language_for(relative: &str, absolute: &Path) -> Option<Language> {
    let name = absolute
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(relative);
    if name.ends_with(".env.example")
        || name.ends_with(".env.sample")
        || name.ends_with(".env.template")
        || name == ".env.example"
        || name == ".env.sample"
    {
        return Some(Language::Bash);
    }
    let extension = relative.rsplit_once('.')?.1;
    match extension.to_ascii_lowercase().as_str() {
        "json" | "toml" => Some(Language::JavaScript),
        "env" | "example" | "sample" | "properties" => Some(Language::Bash),
        other => Language::from_extension(other),
    }
}
