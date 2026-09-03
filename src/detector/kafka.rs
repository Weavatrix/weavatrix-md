//! Kafka producer/consumer observations. Topic must be a proven literal.

use super::{has_kafka_marker, looks_like_topic, resolve_binding};
use crate::model::{KafkaObservation, KafkaRole, RepoInventory};
use crate::tokens::Stream;
use weavatrix_parse::Facts;

const PRODUCER_CALLS: &[&str] = &[
    "send",
    "produce",
    "Produce",
    "ProduceSync",
    "ProduceAsync",
    "NewWriter",
    "KafkaProducer",
    "KafkaTemplate",
];
const CONSUMER_CALLS: &[&str] = &[
    "subscribe",
    "Subscribe",
    "KafkaListener",
    "NewReader",
    "Consume",
    "KafkaConsumer",
];
const PRODUCER_RECEIVERS: &[&str] = &["producer", "writer", "template"];
const CONSUMER_RECEIVERS: &[&str] = &["consumer", "reader", "listener"];
const TOPIC_KEYS: &[&str] = &["topic", "topics"];
const CLUSTER_KEYS: &[&str] = &[
    "bootstrapservers",
    "bootstrap_servers",
    "brokers",
    "broker",
    "bootstrap.servers",
];

pub(crate) fn detect_kafka(
    inventory: &mut RepoInventory,
    _relative: &str,
    source: &str,
    facts: Option<&Facts>,
    stream: Option<&Stream<'_>>,
    bindings: &[(String, String)],
) {
    if !has_kafka_marker(source) {
        return;
    }
    let cluster = cluster_hint(source, stream, bindings);
    if let Some(facts) = facts {
        for call in facts.calls() {
            let role = kafka_role(&call.name, call.receiver.as_deref());
            for argument in &call.string_arguments {
                let topic = resolve_literal(argument, bindings);
                if looks_like_topic(&topic)
                    && let Some(role) = role
                {
                    push(inventory, role, topic, cluster.clone());
                }
            }
            for name in &call.name_arguments {
                if let Some(topic) = resolve_binding(bindings, name)
                    && looks_like_topic(&topic)
                    && let Some(role) = role
                {
                    push(inventory, role, topic, cluster.clone());
                }
            }
        }
    }
    if let Some(stream) = stream {
        let mut index = 0;
        while index + 2 < stream.len() {
            if let Some(name) = stream.ident(index)
                && TOPIC_KEYS.iter().any(|key| name.eq_ignore_ascii_case(key))
            {
                let mut cursor = index + 1;
                if stream.is_punct(cursor, ":") || stream.is_punct(cursor, "=") {
                    cursor += 1;
                }
                if stream.is_punct(cursor, "[") {
                    cursor += 1;
                }
                let literal = stream
                    .string(cursor)
                    .or_else(|| stream.ident(cursor).map(ToOwned::to_owned));
                if let Some(value) = literal {
                    let topic = resolve_literal(&value, bindings);
                    if looks_like_topic(&topic)
                        && let Some(role) = role_near(stream, index)
                    {
                        push(inventory, role, topic, cluster.clone());
                    }
                }
            }
            index += 1;
        }
    }
}

fn kafka_role(name: &str, receiver: Option<&str>) -> Option<KafkaRole> {
    if CONSUMER_CALLS.contains(&name) {
        return Some(KafkaRole::Consumer);
    }
    if PRODUCER_CALLS.contains(&name) {
        if name.eq_ignore_ascii_case("send") {
            let receiver = receiver.unwrap_or_default().to_ascii_lowercase();
            if PRODUCER_RECEIVERS
                .iter()
                .any(|item| receiver.contains(item))
                || receiver.is_empty()
            {
                return Some(KafkaRole::Producer);
            }
            return None;
        }
        return Some(KafkaRole::Producer);
    }
    if let Some(receiver) = receiver {
        let receiver = receiver.to_ascii_lowercase();
        if CONSUMER_RECEIVERS
            .iter()
            .any(|item| receiver.contains(item))
            && name.eq_ignore_ascii_case("subscribe")
        {
            return Some(KafkaRole::Consumer);
        }
    }
    None
}

fn role_near(stream: &Stream<'_>, index: usize) -> Option<KafkaRole> {
    let start = index.saturating_sub(24);
    let end = (index + 12).min(stream.len());
    let mut producer = false;
    let mut consumer = false;
    for cursor in start..end {
        let Some(name) = stream.ident(cursor) else {
            continue;
        };
        let lower = name.to_ascii_lowercase();
        if PRODUCER_RECEIVERS.iter().any(|item| lower.contains(item))
            || PRODUCER_CALLS
                .iter()
                .any(|item| item.eq_ignore_ascii_case(name))
        {
            producer = true;
        }
        if CONSUMER_RECEIVERS.iter().any(|item| lower.contains(item))
            || CONSUMER_CALLS
                .iter()
                .any(|item| item.eq_ignore_ascii_case(name))
        {
            consumer = true;
        }
    }
    match (producer, consumer) {
        (true, false) => Some(KafkaRole::Producer),
        (false, true) => Some(KafkaRole::Consumer),
        (true, true) | (false, false) => None,
    }
}

fn cluster_hint(
    source: &str,
    stream: Option<&Stream<'_>>,
    bindings: &[(String, String)],
) -> Option<String> {
    if let Some(stream) = stream {
        for (key, value) in stream.properties(CLUSTER_KEYS) {
            let _ = key;
            let resolved = resolve_literal(&value, bindings);
            if let Some(host) = broker_host(&resolved) {
                return Some(host);
            }
        }
    }
    for (_, value) in bindings {
        if let Some(host) = broker_host(value) {
            return Some(host);
        }
    }
    for line in source.lines() {
        let lower = line.to_ascii_lowercase();
        if CLUSTER_KEYS.iter().any(|key| lower.contains(key))
            && let Some(host) = broker_host(line)
        {
            return Some(host);
        }
    }
    None
}

fn broker_host(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches(['"', '\'']);
    let candidate = trimmed
        .split([',', ';', ' '])
        .map(str::trim)
        .find(|part| part.contains(':') || part.contains('.'))?;
    if candidate.contains("://") {
        return None;
    }
    let host = candidate.split(':').next()?.to_ascii_lowercase();
    if host.is_empty() || crate::normalize::is_localhost(&host) {
        return None;
    }
    Some(host)
}

fn resolve_literal(value: &str, bindings: &[(String, String)]) -> String {
    resolve_binding(bindings, value).unwrap_or_else(|| value.trim().to_owned())
}

fn push(inventory: &mut RepoInventory, role: KafkaRole, topic: String, cluster: Option<String>) {
    inventory.kafka.push(KafkaObservation {
        role,
        topic,
        cluster,
    });
}
