//! Kafka producer/consumer observations. Topic must be a proven literal.

use super::{has_kafka_marker, looks_like_topic, resolve_binding};
use crate::model::{KafkaObservation, KafkaRole, RepoInventory};
use crate::tokens::Stream;
use weavatrix_parse::Facts;

const PRODUCER_CALLS: &[&str] = &[
    "send",
    "sendMessage",
    "sendMessages",
    "produce",
    "Produce",
    "ProduceSync",
    "ProduceAsync",
    "NewWriter",
    "KafkaProducer",
    "KafkaTemplate",
    "WriteMessages",
    "WriteMessage",
];
const CONSUMER_CALLS: &[&str] = &[
    "subscribe",
    "Subscribe",
    "KafkaListener",
    "NewReader",
    "Consume",
    "KafkaConsumer",
    "ReadMessage",
    "FetchMessage",
    "initConsumerForATopic",
    "initConsumer",
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
    let mut local = bindings.to_vec();
    if let Some(facts) = facts {
        for call in facts.calls() {
            collect_flag_topic(inventory, &mut local, call, cluster.clone());
            let role = kafka_role(&call.name, call.receiver.as_deref());
            for argument in &call.string_arguments {
                let topic = resolve_literal(argument, &local);
                if looks_like_topic(&topic)
                    && let Some(role) = role
                {
                    push(inventory, role, topic, cluster.clone());
                }
            }
            for name in &call.name_arguments {
                if let Some(topic) = resolve_binding(&local, name)
                    && looks_like_topic(&topic)
                    && let Some(role) = role
                {
                    push(inventory, role, topic, cluster.clone());
                }
            }
        }
    }
    if let Some(stream) = stream {
        collect_flag_assignments(stream, &mut local, inventory, cluster.as_deref());
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
                let topic = if let Some(literal) = stream.string(cursor) {
                    Some(resolve_literal(&literal, &local))
                } else if let Some(ident) = stream.ident(cursor) {
                    resolve_binding(&local, ident)
                } else {
                    None
                };
                if let Some(topic) = topic
                    && looks_like_topic(&topic)
                    && let Some(role) = role_near(stream, index)
                {
                    push(inventory, role, topic, cluster.clone());
                }
            }
            index += 1;
        }
    }
}

fn collect_flag_topic(
    inventory: &mut RepoInventory,
    bindings: &mut Vec<(String, String)>,
    call: &weavatrix_parse::Reference,
    cluster: Option<String>,
) {
    if !is_flag_string(&call.name, call.receiver.as_deref()) {
        return;
    }
    let flag_name = call.string_arguments.first().map_or("", String::as_str);
    let topic = call.string_arguments.get(1).map_or("", String::as_str);
    let help = call.string_arguments.get(2).map_or("", String::as_str);
    if !looks_like_topic(topic) {
        return;
    }
    if let Some(owner) = call.owner.as_deref() {
        bindings.push((owner.to_owned(), topic.to_owned()));
    }
    for name in &call.name_arguments {
        if looks_like_ident(name) {
            bindings.push((name.clone(), topic.to_owned()));
        }
    }
    if let Some(role) = kafka_flag_role(flag_name, help, topic) {
        push(inventory, role, topic.to_owned(), cluster);
    }
}

fn is_flag_string(name: &str, receiver: Option<&str>) -> bool {
    matches!(name, "String" | "StringVar")
        && receiver.is_some_and(|item| item.eq_ignore_ascii_case("flag") || item.contains("Flag"))
}

fn looks_like_ident(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(char::is_alphabetic)
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

fn collect_flag_assignments(
    stream: &Stream<'_>,
    bindings: &mut Vec<(String, String)>,
    inventory: &mut RepoInventory,
    cluster: Option<&str>,
) {
    let mut index = 0;
    while index + 5 < stream.len() {
        let Some(lhs) = stream.ident(index) else {
            index += 1;
            continue;
        };
        let mut cursor = index + 1;
        if stream.is_punct(cursor, ":") && stream.is_punct(cursor + 1, "=") {
            cursor += 2;
        } else if stream.is_punct(cursor, "=") {
            cursor += 1;
        } else {
            index += 1;
            continue;
        }
        let Some(receiver) = stream.ident(cursor) else {
            index += 1;
            continue;
        };
        if !(receiver.eq_ignore_ascii_case("flag") || receiver.contains("Flag"))
            || !stream.is_punct(cursor + 1, ".")
        {
            index += 1;
            continue;
        }
        let Some(call) = stream.ident(cursor + 2) else {
            index += 1;
            continue;
        };
        if call != "String" && call != "StringVar" {
            index += 1;
            continue;
        }
        let mut strings = Vec::new();
        let mut scan = cursor + 3;
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
        let topic = strings.get(1).map_or("", String::as_str);
        let help = strings.get(2).map_or("", String::as_str);
        if looks_like_topic(topic) {
            bindings.push((lhs.to_owned(), topic.to_owned()));
            if let Some(role) = kafka_flag_role(flag_name, help, topic) {
                push(
                    inventory,
                    role,
                    topic.to_owned(),
                    cluster.map(ToOwned::to_owned),
                );
            }
        }
        index += 1;
    }
}

fn kafka_flag_role(flag_name: &str, help: &str, topic: &str) -> Option<KafkaRole> {
    let name = flag_name.to_ascii_lowercase();
    let help = help.to_ascii_lowercase();
    let topic = topic.to_ascii_lowercase();
    if !(name.contains("kafka")
        || name.contains("topic")
        || help.contains("kafka")
        || help.contains("topic"))
    {
        return None;
    }
    // Prefer the topic default and flag name over help text (help is often wrong).
    if topic.ends_with("_out")
        || topic.ends_with(".out")
        || name.contains("topic_out")
        || name.ends_with("_out")
        || name.contains("_out_")
        || name.contains("topic_controller")
        || name.contains("producer")
        || name.contains("notify_topic")
        || name.contains("response")
    {
        return Some(KafkaRole::Producer);
    }
    if topic.ends_with("_in")
        || topic.ends_with(".in")
        || name.contains("topic_in")
        || name.ends_with("_in")
        || name.contains("_in_")
    {
        return Some(KafkaRole::Consumer);
    }
    if help.contains("out topic") || help.contains("producer") || help.contains("write topic") {
        return Some(KafkaRole::Producer);
    }
    if help.contains("in topic") || help.contains("consumer") || help.contains("read topic") {
        return Some(KafkaRole::Consumer);
    }
    if name.contains("logs_topic") || name == "logs_topic" || topic == "logs" {
        return Some(KafkaRole::Producer);
    }
    if name.contains("kafka_topic") && !name.contains("out") {
        return Some(KafkaRole::Consumer);
    }
    None
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
        // sendMessage(topic, …) / sendMessages(topic, …) — topic is the first arg.
        if name.eq_ignore_ascii_case("sendMessage") || name.eq_ignore_ascii_case("sendMessages") {
            return Some(KafkaRole::Producer);
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
    _source: &str,
    stream: Option<&Stream<'_>>,
    bindings: &[(String, String)],
) -> Option<String> {
    if let Some(stream) = stream {
        for (key, value) in stream.properties(CLUSTER_KEYS) {
            let _ = key;
            if looks_like_ident(&value) {
                continue;
            }
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
    None
}

fn broker_host(value: &str) -> Option<String> {
    let trimmed = value.trim().trim_matches(['"', '\'']);
    if trimmed.contains('(') || trimmed.contains("messages.") {
        return None;
    }
    let candidate = trimmed
        .split([',', ';', ' '])
        .map(str::trim)
        .find(|part| part.contains(':'))?;
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
