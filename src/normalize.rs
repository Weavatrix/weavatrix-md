//! String normalization used by aliases, routes, hosts, and topics.

/// Lowercase hyphenated form plus original lowercase.
#[must_use]
pub fn alias_candidates(raw: &str) -> Vec<String> {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let stripped = trimmed
        .trim_start_matches('@')
        .rsplit('/')
        .next()
        .unwrap_or(trimmed);
    push_unique(&mut out, trimmed.to_ascii_lowercase());
    push_unique(&mut out, stripped.to_ascii_lowercase());
    push_unique(&mut out, hyphenate(stripped));
    if trimmed.contains('/') {
        push_unique(&mut out, hyphenate(trimmed));
    }
    out
}

/// Canonical hyphenated lowercase identity.
#[must_use]
pub fn hyphenate(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('@')
        .replace('_', "-")
        .to_ascii_lowercase()
}

/// Host aliases: full host, first label, k8s service name.
#[must_use]
pub fn host_aliases(host: &str) -> Vec<String> {
    let host = host.trim().trim_matches(['[', ']']).to_ascii_lowercase();
    if host.is_empty() {
        return Vec::new();
    }
    let mut out = vec![host.clone(), hyphenate(&host)];
    if let Some((first, rest)) = host.split_once('.') {
        push_unique(&mut out, first.to_owned());
        push_unique(&mut out, hyphenate(first));
        if rest.contains("svc") {
            push_unique(&mut out, first.to_owned());
        }
    }
    out
}

/// True for loopback hosts that do not prove a shared instance.
#[must_use]
pub fn is_localhost(host: &str) -> bool {
    matches!(
        host.to_ascii_lowercase().as_str(),
        "localhost" | "127.0.0.1" | "0.0.0.0" | "::1" | "[::1]"
    )
}

/// HTTP route identity: lowercase, no trailing slash, `{id}`/`:id` kept.
#[must_use]
pub fn normalize_route(route: &str) -> String {
    let mut path = route.trim().to_owned();
    if let Some(query) = path.find('?') {
        path.truncate(query);
    }
    if let Some(hash) = path.find('#') {
        path.truncate(hash);
    }
    if let Some(scheme) = path.find("://")
        && let Some(slash) = path[scheme + 3..].find('/')
    {
        path = path[scheme + 3 + slash..].to_owned();
    }
    if !path.starts_with('/') {
        path.insert(0, '/');
    }
    while path.len() > 1 && path.ends_with('/') {
        path.pop();
    }
    path.to_ascii_lowercase()
}

/// True for routes that must never unique-match across repositories.
#[must_use]
pub fn is_generic_route(route: &str) -> bool {
    matches!(
        normalize_route(route).as_str(),
        "/" | "/health"
            | "/healthz"
            | "/ready"
            | "/readiness"
            | "/live"
            | "/liveness"
            | "/metrics"
            | "/status"
            | "/version"
            | "/ping"
            | "/favicon.ico"
    )
}

/// Generic Kafka topics that need a cluster hint.
#[must_use]
pub fn is_generic_topic(topic: &str) -> bool {
    matches!(
        topic.to_ascii_lowercase().as_str(),
        "events"
            | "event"
            | "logs"
            | "log"
            | "metrics"
            | "metric"
            | "test"
            | "testing"
            | "debug"
            | "default"
            | "topic"
            | "dlq"
            | "dead-letter"
            | "messages"
            | "message"
    )
}

fn push_unique(out: &mut Vec<String>, value: String) {
    if !value.is_empty() && !out.iter().any(|item| item == &value) {
        out.push(value);
    }
}
