//! HTTP/gRPC/GraphQL observations. Relative `/health` is not a peer by itself.

use super::resolve_binding;
use crate::model::{ApiDirection, ApiObservation, ApiProtocol, RepoInventory};
use crate::normalize;
use crate::tokens::Stream;
use weavatrix_parse::{ContractKind, Facts};

const SERVER_CALLS: &[&str] = &[
    "get",
    "post",
    "put",
    "patch",
    "delete",
    "head",
    "options",
    "Get",
    "Post",
    "Put",
    "Patch",
    "Delete",
    "GET",
    "POST",
    "PUT",
    "PATCH",
    "DELETE",
    "GetMapping",
    "PostMapping",
    "PutMapping",
    "PatchMapping",
    "DeleteMapping",
    "RequestMapping",
    "HandleFunc",
    "Handle",
    "route",
    "Route",
    "router",
    "Methods",
];
const CLIENT_CALLS: &[&str] = &[
    "fetch",
    "axios",
    "request",
    "requests",
    "httpx",
    "got",
    "ky",
    "superagent",
    "GetAsync",
    "PostAsync",
    "PutAsync",
    "SendAsync",
    "getForObject",
    "exchange",
    "URLSession",
    "reqwest",
    "ureq",
];
const SERVER_RECEIVERS: &[&str] = &[
    "app", "router", "route", "server", "api", "r", "e", "mux", "chi", "gin", "echo", "http",
];

pub(crate) fn detect_api(
    inventory: &mut RepoInventory,
    _relative: &str,
    source: &str,
    facts: Option<&Facts>,
    stream: Option<&Stream<'_>>,
    bindings: &[(String, String)],
) {
    if let Some(facts) = facts {
        for call in facts.calls() {
            observe_call(
                inventory,
                &call.name,
                call.receiver.as_deref(),
                &call.string_arguments,
                bindings,
            );
        }
        for contract in &facts.contracts {
            match &contract.kind {
                ContractKind::ProtobufService | ContractKind::ProtobufRpc { .. } => {
                    inventory.api.push(ApiObservation {
                        direction: ApiDirection::Exposes,
                        protocol: ApiProtocol::Grpc,
                        method: None,
                        resource: contract.owner.clone().map_or_else(
                            || contract.name.clone(),
                            |owner| format!("{owner}.{}", contract.name),
                        ),
                        host_hint: None,
                    });
                }
                ContractKind::GraphqlOperation(_) | ContractKind::GraphqlCall(_) => {
                    if let Some(host) = host_from_source(source, bindings) {
                        inventory.api.push(ApiObservation {
                            direction: ApiDirection::Calls,
                            protocol: ApiProtocol::Graphql,
                            method: None,
                            resource: contract.name.clone(),
                            host_hint: Some(host),
                        });
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(stream) = stream {
        let mut index = 0;
        while index < stream.len() {
            if let Some(literal) = stream.string(index) {
                observe_url(inventory, &literal, bindings);
            }
            index += 1;
        }
    }
    for (_, value) in bindings {
        observe_url(inventory, value, bindings);
    }
}

fn observe_call(
    inventory: &mut RepoInventory,
    name: &str,
    receiver: Option<&str>,
    strings: &[String],
    bindings: &[(String, String)],
) {
    for raw in strings {
        let value = resolve_binding(bindings, raw).unwrap_or_else(|| raw.clone());
        observe_url(inventory, &value, bindings);
        if let Some(route) = http_route(&value)
            && let Some((direction, method)) = classify_http(name, receiver, &value)
        {
            inventory.api.push(ApiObservation {
                direction,
                protocol: ApiProtocol::Http,
                method,
                resource: route,
                host_hint: url_host(&value),
            });
        }
    }
    if CLIENT_CALLS.contains(&name) {
        for raw in strings {
            let value = resolve_binding(bindings, raw).unwrap_or_else(|| raw.clone());
            if let Some(host) = url_host(&value) {
                inventory.api.push(ApiObservation {
                    direction: ApiDirection::Calls,
                    protocol: ApiProtocol::Http,
                    method: http_method(name),
                    resource: http_route(&value).unwrap_or_else(|| "/".to_owned()),
                    host_hint: Some(host),
                });
            }
        }
    }
}

fn classify_http(
    name: &str,
    receiver: Option<&str>,
    value: &str,
) -> Option<(ApiDirection, Option<String>)> {
    if url_host(value).is_some() || CLIENT_CALLS.contains(&name) {
        return Some((ApiDirection::Calls, http_method(name)));
    }
    let server_name = SERVER_CALLS.contains(&name);
    let server_receiver = receiver.is_some_and(|item| {
        SERVER_RECEIVERS
            .iter()
            .any(|known| item.eq_ignore_ascii_case(known))
    });
    if server_name
        && (server_receiver || is_mapping(name) || receiver.is_none() && is_mapping(name))
    {
        return Some((ApiDirection::Exposes, http_method(name)));
    }
    if server_name && server_receiver {
        return Some((ApiDirection::Exposes, http_method(name)));
    }
    if is_mapping(name) {
        return Some((ApiDirection::Exposes, http_method(name)));
    }
    None
}

fn is_mapping(name: &str) -> bool {
    name.ends_with("Mapping") || name == "HandleFunc" || name == "Handle"
}

fn http_method(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let method = lower
        .strip_suffix("mapping")
        .unwrap_or(&lower)
        .strip_suffix("async")
        .unwrap_or(&lower);
    match method {
        "get" | "post" | "put" | "patch" | "delete" | "head" | "options" => {
            Some(method.to_ascii_uppercase())
        }
        _ => None,
    }
}

fn http_route(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.contains("://") {
        return Some(normalize::normalize_route(trimmed));
    }
    if trimmed.starts_with('/') && !trimmed.contains(' ') && trimmed.len() < 256 {
        return Some(normalize::normalize_route(trimmed));
    }
    None
}

fn observe_url(inventory: &mut RepoInventory, raw: &str, _bindings: &[(String, String)]) {
    let Some(host) = url_host(raw) else {
        return;
    };
    if normalize::is_localhost(&host) {
        return;
    }
    if looks_like_database_url(raw) || looks_like_kafka_url(raw) {
        return;
    }
    inventory.api.push(ApiObservation {
        direction: ApiDirection::Calls,
        protocol: ApiProtocol::Http,
        method: None,
        resource: http_route(raw).unwrap_or_else(|| "/".to_owned()),
        host_hint: Some(host),
    });
}

fn url_host(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))?;
    let hostport = rest.split('/').next().unwrap_or(rest);
    let host = hostport.split(':').next().unwrap_or(hostport).trim();
    if host.is_empty() || host.contains('$') || host.contains('{') {
        return None;
    }
    Some(host.to_ascii_lowercase())
}

fn looks_like_database_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("postgres://")
        || lower.contains("postgresql://")
        || lower.contains("mongodb://")
        || lower.contains("mysql://")
        || lower.contains("clickhouse://")
        || lower.contains("sqlserver://")
}

fn looks_like_kafka_url(value: &str) -> bool {
    value.to_ascii_lowercase().contains("kafka://")
}

fn host_from_source(source: &str, bindings: &[(String, String)]) -> Option<String> {
    for (_, value) in bindings {
        if let Some(host) = url_host(value) {
            return Some(host);
        }
    }
    source.lines().find_map(url_host)
}
