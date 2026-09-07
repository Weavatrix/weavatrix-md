//! Database identity from sanitized DSNs. Credentials never leave this module.

use super::{env_values, resolve_binding};
use crate::model::{DatabaseEngine, DatabaseObservation, RepoInventory};
use crate::normalize;
use crate::tokens::Stream;
use weavatrix_parse::Facts;

const CONNECT_CALLS: &[&str] = &[
    "connect",
    "createpool",
    "create_engine",
    "createengine",
    "getconnection",
    "mongodb",
    "mongoclient",
    "mongoose",
    "pool",
    "npgsqlconnection",
    "pgpooloptions",
];

pub(crate) fn detect_database(
    inventory: &mut RepoInventory,
    relative: &str,
    source: &str,
    facts: Option<&Facts>,
    stream: Option<&Stream<'_>>,
    bindings: &[(String, String)],
) {
    if is_ignored_secret_env(relative) {
        return;
    }
    if let Some(facts) = facts {
        for call in facts.calls() {
            if !is_connect_call(&call.name, call.receiver.as_deref()) {
                continue;
            }
            for argument in &call.string_arguments {
                consider(inventory, argument);
            }
            for name in &call.name_arguments {
                if let Some(value) = resolve_binding(bindings, name) {
                    consider(inventory, &value);
                }
            }
        }
        for reference in &facts.references {
            for argument in &reference.string_arguments {
                consider(inventory, argument);
            }
        }
    }
    if let Some(stream) = stream {
        for (_, value) in stream.properties(&[
            "connectionstring",
            "connection_string",
            "url",
            "uri",
            "dsn",
            "databaseurl",
            "database_url",
            "mongodburi",
            "mongo_uri",
        ]) {
            let resolved = resolve_binding(bindings, &value).unwrap_or(value);
            consider(inventory, &resolved);
        }
        for (_, value) in stream.bindings() {
            consider(inventory, &value);
        }
        collect_flag_store(inventory, stream);
        let mut index = 0;
        while index < stream.len() {
            if let Some(literal) = stream.string(index) {
                consider(inventory, &literal);
            }
            index += 1;
        }
    }
    if is_env_file(relative) {
        for value in env_values(source) {
            consider(inventory, &value);
        }
    }
    scan_unquoted_dsns(inventory, source);
}

fn is_connect_call(name: &str, receiver: Option<&str>) -> bool {
    let name_l = name.to_ascii_lowercase();
    if CONNECT_CALLS.contains(&name_l.as_str()) {
        return true;
    }
    if let Some(receiver) = receiver {
        let receiver = receiver.to_ascii_lowercase();
        return CONNECT_CALLS.iter().any(|item| receiver.contains(item));
    }
    false
}

fn consider(inventory: &mut RepoInventory, raw: &str) {
    if let Some(observation) = parse_identity(raw) {
        inventory.databases.push(observation);
    }
}

fn scan_unquoted_dsns(inventory: &mut RepoInventory, source: &str) {
    for line in source.lines() {
        if let Some(offset) = dsn_offset(line) {
            let rest = &line[offset..];
            let token = rest
                .split(|character: char| {
                    character.is_ascii_whitespace()
                        || matches!(character, '"' | '\'' | '`' | ')' | ']' | '}' | ',')
                })
                .next()
                .unwrap_or(rest);
            consider(inventory, token);
        }
        if let Some(observation) = parse_sql_server(line) {
            inventory.databases.push(observation);
        }
        if let Some(observation) = parse_named_addr(line) {
            inventory.databases.push(observation);
        }
    }
}

fn dsn_offset(line: &str) -> Option<usize> {
    const SCHEMES: &[&str] = &[
        "postgres://",
        "postgresql://",
        "mysql://",
        "mariadb://",
        "mongodb://",
        "mongodb+srv://",
        "clickhouse://",
        "mssql://",
        "sqlserver://",
        "jdbc:postgresql:",
        "jdbc:mysql:",
        "jdbc:sqlserver:",
        "sqlite://",
        "redis://",
        "rediss://",
        "vault://",
    ];
    let lower = line.to_ascii_lowercase();
    SCHEMES.iter().find_map(|scheme| lower.find(scheme))
}

/// Parses a DSN into a sanitized identity. The raw string is not retained.
#[must_use]
pub fn parse_identity(raw: &str) -> Option<DatabaseObservation> {
    let trimmed = raw.trim().trim_matches(['"', '\'', '`']);
    if trimmed.is_empty()
        || trimmed.contains("${")
        || trimmed.contains('$') && trimmed.contains('{')
    {
        return None;
    }
    if let Some(observation) = parse_sql_server(trimmed) {
        return Some(observation);
    }
    let lower = trimmed.to_ascii_lowercase();
    let (engine, rest) = engine_from_scheme(&lower, trimmed)?;
    let rest = rest.trim_start_matches("//");
    let without_query = rest.split('?').next().unwrap_or(rest);
    let (authority, database) = split_authority_db(without_query);
    let host = host_from_authority(authority)?;
    if host_is_unresolved_env(&host) {
        return None;
    }
    let localhost = normalize::is_localhost(&host);
    Some(DatabaseObservation {
        engine,
        host: Some(host),
        database,
        localhost,
    })
}

fn engine_from_scheme(lower: &str, original: &str) -> Option<(DatabaseEngine, String)> {
    let pairs = [
        ("postgres://", DatabaseEngine::Postgres),
        ("postgresql://", DatabaseEngine::Postgres),
        ("jdbc:postgresql:", DatabaseEngine::Postgres),
        ("mysql://", DatabaseEngine::Mysql),
        ("mariadb://", DatabaseEngine::Mysql),
        ("jdbc:mysql:", DatabaseEngine::Mysql),
        ("mongodb+srv://", DatabaseEngine::Mongo),
        ("mongodb://", DatabaseEngine::Mongo),
        ("clickhouse://", DatabaseEngine::Clickhouse),
        ("mssql://", DatabaseEngine::SqlServer),
        ("sqlserver://", DatabaseEngine::SqlServer),
        ("jdbc:sqlserver:", DatabaseEngine::SqlServer),
        ("sqlite://", DatabaseEngine::Sqlite),
        ("rediss://", DatabaseEngine::Redis),
        ("redis://", DatabaseEngine::Redis),
        ("vault://", DatabaseEngine::Vault),
    ];
    for (scheme, engine) in pairs {
        if let Some(rest) = lower.strip_prefix(scheme) {
            let original_rest = original.get(scheme.len()..).unwrap_or(rest);
            return Some((engine, original_rest.to_owned()));
        }
    }
    None
}

fn split_authority_db(rest: &str) -> (&str, Option<String>) {
    if let Some((authority, db)) = rest.split_once('/') {
        let database = db
            .split(['?', ';'])
            .next()
            .unwrap_or(db)
            .trim()
            .trim_start_matches('/');
        let database = (!database.is_empty()).then(|| database.to_owned());
        (authority, database)
    } else {
        (rest, None)
    }
}

fn host_from_authority(authority: &str) -> Option<String> {
    let without_user = authority.rsplit('@').next().unwrap_or(authority);
    let host = if without_user.starts_with('[') {
        let end = without_user.find(']')?;
        without_user[1..end].to_owned()
    } else {
        without_user
            .split(':')
            .next()
            .unwrap_or(without_user)
            .to_owned()
    };
    let host = host.trim().to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

fn parse_sql_server(raw: &str) -> Option<DatabaseObservation> {
    if !raw.to_ascii_lowercase().contains("server=")
        || !raw.to_ascii_lowercase().contains("database=")
    {
        return None;
    }
    let mut host = None;
    let mut database = None;
    for part in raw.split(';') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim().trim_matches(['"', '\'']);
        if key == "server" || key == "data source" {
            let value = value.rsplit('\\').next().unwrap_or(value);
            host = Some(
                value
                    .split(',')
                    .next()
                    .unwrap_or(value)
                    .trim()
                    .to_ascii_lowercase(),
            );
        }
        if key == "database" || key == "initial catalog" {
            database = Some(value.to_owned());
        }
    }
    let host = host.filter(|value| !value.is_empty())?;
    let localhost = normalize::is_localhost(&host);
    Some(DatabaseObservation {
        engine: DatabaseEngine::SqlServer,
        host: Some(host),
        database,
        localhost,
    })
}

fn parse_named_addr(line: &str) -> Option<DatabaseObservation> {
    let trimmed = line.trim();
    let (engine, value) =
        if let Some(value) = env_assignment(trimmed, &["VAULT_ADDR", "VAULT_AGENT_ADDR"]) {
            (DatabaseEngine::Vault, value)
        } else {
            let value = env_assignment(trimmed, &["REDIS_URL", "REDIS_ADDR", "REDIS_HOST"])?;
            (DatabaseEngine::Redis, value)
        };
    if let Some(parsed) = parse_identity(&value) {
        return Some(parsed);
    }
    let host = host_from_url_or_addr(&value)?;
    if host_is_unresolved_env(&host) {
        return None;
    }
    Some(DatabaseObservation {
        engine,
        host: Some(host.clone()),
        database: None,
        localhost: normalize::is_localhost(&host),
    })
}

fn host_from_url_or_addr(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(trimmed);
    let hostport = rest.split('/').next().unwrap_or(rest);
    let host = hostport
        .split(':')
        .next()
        .unwrap_or(hostport)
        .trim()
        .trim_matches(['[', ']'])
        .to_ascii_lowercase();
    (!host.is_empty() && !host_is_unresolved_env(&host)).then_some(host)
}

fn env_assignment(line: &str, keys: &[&str]) -> Option<String> {
    let stripped = line.trim_start_matches("export ").trim();
    let (key, value) = stripped.split_once('=')?;
    let key = key.trim().trim_matches(['"', '\'']);
    if !keys.iter().any(|item| key.eq_ignore_ascii_case(item)) {
        return None;
    }
    Some(crate::tokens::unquote(value.trim()))
}

fn host_is_unresolved_env(host: &str) -> bool {
    host.contains('$') || host.starts_with('%') || host.contains('{')
}

fn is_env_file(relative: &str) -> bool {
    let name = relative
        .rsplit('/')
        .next()
        .unwrap_or(relative)
        .to_ascii_lowercase();
    name.contains(".env")
        || name.ends_with(".properties")
        || name.ends_with(".example")
        || matches!(
            name.as_str(),
            "readme" | "readme.md" | "readme.rst" | "readme.txt"
        )
}

fn is_ignored_secret_env(relative: &str) -> bool {
    let name = relative.rsplit('/').next().unwrap_or(relative);
    name == ".env" || name.ends_with(".env.local") || name.ends_with(".env.secret")
}

/// Go `flag.String("redis_ipport", "10.0.0.1:6379", …)` / `mongodb_uri` defaults.
fn collect_flag_store(inventory: &mut RepoInventory, stream: &Stream<'_>) {
    let mut index = 0;
    while index + 5 < stream.len() {
        let Some(receiver) = stream.ident(index) else {
            index += 1;
            continue;
        };
        if !(receiver.eq_ignore_ascii_case("flag") || receiver.contains("Flag"))
            || !stream.is_punct(index + 1, ".")
        {
            index += 1;
            continue;
        }
        let Some(call) = stream.ident(index + 2) else {
            index += 1;
            continue;
        };
        if call != "String" && call != "StringVar" {
            index += 1;
            continue;
        }
        let mut strings = Vec::new();
        let mut scan = index + 3;
        while scan < stream.len() && strings.len() < 3 {
            if let Some(literal) = stream.string(scan) {
                strings.push(literal);
            }
            if stream.is_punct(scan, ")") {
                break;
            }
            scan += 1;
        }
        let flag_name = strings.first().map_or("", String::as_str);
        let value = strings.get(1).map_or("", String::as_str);
        let help = strings.get(2).map_or("", String::as_str);
        let name_l = flag_name.to_ascii_lowercase();
        let help_l = help.to_ascii_lowercase();
        if name_l.contains("mongo") || help_l.contains("mongo") {
            if value.contains("://") {
                consider(inventory, value);
            } else if !value.is_empty() && !value.starts_with(':') {
                consider(inventory, &format!("mongodb://{value}"));
            }
        } else if name_l.contains("redis") || help_l.contains("redis") {
            if value.contains("://") {
                consider(inventory, value);
            } else if let Some(observation) = parse_redis_addr(value) {
                inventory.databases.push(observation);
            }
        }
        index += 1;
    }
}

fn parse_redis_addr(raw: &str) -> Option<DatabaseObservation> {
    let trimmed = raw.trim().trim_matches(['"', '\'']);
    if trimmed.is_empty() || trimmed == ":6379" {
        return None;
    }
    let hostport = trimmed.split(',').next()?.trim();
    let host = hostport.split(':').next()?.trim();
    if host.is_empty() || host_is_unresolved_env(host) {
        return None;
    }
    let localhost = normalize::is_localhost(host);
    let database = hostport
        .split(':')
        .nth(1)
        .map(str::trim)
        .filter(|port| !port.is_empty())
        .map(ToOwned::to_owned);
    Some(DatabaseObservation {
        engine: DatabaseEngine::Redis,
        host: Some(host.to_ascii_lowercase()),
        database,
        localhost,
    })
}
