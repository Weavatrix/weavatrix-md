//! Three tiny detectors. Precision over recall.

mod api;
mod database;
mod kafka;

pub(crate) use api::detect_api;
pub(crate) use database::detect_database;
pub(crate) use kafka::detect_kafka;

use crate::tokens;

const KAFKA_MARKERS: &[&str] = &[
    "kafka",
    "kafkajs",
    "kafkalistener",
    "kafkaproducer",
    "kafkaconsumer",
    "kafkatemplate",
    "aiokafka",
    "confluent",
    "sarama",
    "segmentio",
    "franz-go",
    "rdkafka",
    "spring-kafka",
];

pub(crate) fn has_kafka_marker(source: &str) -> bool {
    let lower = source.to_ascii_lowercase();
    KAFKA_MARKERS.iter().any(|marker| lower.contains(marker))
}

pub(crate) fn resolve_binding(bindings: &[(String, String)], name: &str) -> Option<String> {
    bindings
        .iter()
        .rev()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.clone())
}

pub(crate) fn looks_like_topic(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() < 2 || trimmed.len() > 200 {
        return false;
    }
    if trimmed.starts_with('/') || trimmed.contains("://") || trimmed.contains('\\') {
        return false;
    }
    if trimmed.contains(' ') || trimmed.contains('\n') {
        return false;
    }
    if matches!(
        trimmed.to_ascii_lowercase().as_str(),
        "topic" | "topics" | "name" | "key" | "value" | "data" | "msg" | "message"
    ) {
        return false;
    }
    trimmed
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
}

pub(crate) fn env_values(source: &str) -> Vec<String> {
    let mut values = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((_, value)) = trimmed.split_once('=') {
            let value = tokens::unquote(value.trim());
            if !value.is_empty() {
                values.push(value);
            }
        }
    }
    values
}
