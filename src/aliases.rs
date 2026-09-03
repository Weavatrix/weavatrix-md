//! Cheap repository aliases. No manifest graph.

use crate::discover::RepoRef;
use crate::normalize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Alias set for one repository.
#[must_use]
pub fn repo_aliases(repo: &RepoRef) -> BTreeSet<String> {
    let mut aliases = BTreeSet::new();
    insert(&mut aliases, &repo.name);
    insert_path_segments(&mut aliases, &repo.root);
    if let Some(remote) = git_remote_slug(&repo.root) {
        insert(&mut aliases, &remote);
        if let Some((_, name)) = remote.rsplit_once('/') {
            insert(&mut aliases, name);
        }
    }
    if let Some(name) = json_name(&repo.root.join("package.json")) {
        insert(&mut aliases, &name);
    }
    if let Some(name) = toml_package_name(&repo.root.join("Cargo.toml"), "[package]") {
        insert(&mut aliases, &name);
    }
    if let Some(name) = go_module_name(&repo.root.join("go.mod")) {
        insert(&mut aliases, &name);
    }
    if let Some(name) = toml_package_name(&repo.root.join("pyproject.toml"), "[project]") {
        insert(&mut aliases, &name);
    }
    for service in compose_services(&repo.root) {
        insert(&mut aliases, &service);
    }
    aliases
}

fn insert(aliases: &mut BTreeSet<String>, raw: &str) {
    for candidate in normalize::alias_candidates(raw) {
        aliases.insert(candidate);
    }
}

fn insert_path_segments(aliases: &mut BTreeSet<String>, root: &Path) {
    if let Some(name) = root.file_name().and_then(|value| value.to_str()) {
        insert(aliases, name);
    }
}

fn git_remote_slug(root: &Path) -> Option<String> {
    let git = root.join(".git");
    let config_path = if git.is_file() {
        let text = fs::read_to_string(&git).ok()?;
        let gitdir = text
            .lines()
            .find_map(|line| line.strip_prefix("gitdir:"))?
            .trim();
        let resolved = if Path::new(gitdir).is_absolute() {
            Path::new(gitdir).to_path_buf()
        } else {
            root.join(gitdir)
        };
        resolved.join("config")
    } else {
        git.join("config")
    };
    let text = fs::read_to_string(config_path).ok()?;
    let mut in_origin = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_origin = trimmed.eq_ignore_ascii_case("[remote \"origin\"]");
            continue;
        }
        if in_origin && let Some(url) = trimmed.strip_prefix("url") {
            let value = url.trim().trim_start_matches('=').trim();
            return slug_from_url(value);
        }
    }
    None
}

fn slug_from_url(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches(".git");
    if let Some(rest) = trimmed.strip_prefix("git@") {
        let rest = rest.replacen(':', "/", 1);
        return first_two_segments(&rest);
    }
    for prefix in ["https://", "http://", "ssh://git@", "ssh://", "git://"] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return first_two_segments(rest);
        }
    }
    first_two_segments(trimmed)
}

fn first_two_segments(value: &str) -> Option<String> {
    let mut parts = value.split('/').filter(|part| !part.is_empty());
    let host_or_owner = parts.next()?;
    let owner = if host_or_owner.contains('.') {
        parts.next()?
    } else {
        host_or_owner
    };
    let name = parts.next()?;
    Some(format!("{owner}/{name}"))
}

fn json_name(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let marker = "\"name\"";
    let start = text.find(marker)?;
    let after = text.get(start + marker.len()..)?;
    let after = after.trim_start().trim_start_matches(':').trim_start();
    quoted(after)
}

fn toml_package_name(path: &Path, section: &str) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    let section_start = text.find(section)?;
    let rest = text.get(section_start..)?;
    let end = rest[section.len()..]
        .find("\n[")
        .map_or(rest.len(), |index| section.len() + index);
    let body = rest.get(..end)?;
    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(value) = trimmed.strip_prefix("name") {
            let value = value.trim().trim_start_matches('=').trim();
            if let Some(quoted) = quoted(value) {
                return Some(quoted);
            }
            if !value.is_empty() {
                return Some(value.trim_matches('"').to_owned());
            }
        }
    }
    None
}

fn go_module_name(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        if let Some(module) = line.trim().strip_prefix("module ") {
            let module = module.trim();
            let name = module.rsplit('/').next().unwrap_or(module);
            return Some(name.to_owned());
        }
    }
    None
}

fn compose_services(root: &Path) -> Vec<String> {
    let candidates = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];
    for name in candidates {
        let path = root.join(name);
        if let Ok(text) = fs::read_to_string(path)
            && let Some(services) = parse_compose_services(&text)
        {
            return services;
        }
    }
    Vec::new()
}

fn parse_compose_services(text: &str) -> Option<Vec<String>> {
    let mut services = Vec::new();
    let mut in_services = false;
    for line in text.lines() {
        if line.starts_with("services:") {
            in_services = true;
            continue;
        }
        if in_services && !line.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
            break;
        }
        if in_services {
            let trimmed = line.trim();
            if let Some(name) = trimmed.strip_suffix(':')
                && !name.is_empty()
                && !name.contains(' ')
                && line.chars().take_while(char::is_ascii_whitespace).count() == 2
            {
                services.push(name.to_owned());
            }
        }
    }
    (!services.is_empty()).then_some(services)
}

fn quoted(value: &str) -> Option<String> {
    let value = value.trim();
    let bytes = value.as_bytes();
    if bytes.len() < 2 {
        return None;
    }
    let quote = bytes[0];
    if !matches!(quote, b'"' | b'\'') {
        return None;
    }
    let rest = &value[1..];
    let end = rest.find(quote as char)?;
    Some(rest[..end].to_owned())
}
