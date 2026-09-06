//! Expected edges for the ground-truth corpus.

mod support;

use support::{generate_folder, read_md, repo, workspace};

#[test]
fn kafka_producer_consumer_pair() {
    let space = workspace("kafka-pair");
    repo(
        &space.root,
        "producer-a",
        &[(
            "src/index.ts",
            r#"
import { Kafka } from "kafkajs";
const kafka = new Kafka({ brokers: ["broker.internal:9092"] });
const producer = kafka.producer();
await producer.send({ topic: "orders.created", messages: [] });
"#,
        )],
    );
    repo(
        &space.root,
        "consumer-b",
        &[(
            "src/index.ts",
            r#"
import { Kafka } from "kafkajs";
const kafka = new Kafka({ brokers: ["broker.internal:9092"] });
const consumer = kafka.consumer({ groupId: "billing" });
await consumer.subscribe({ topic: "orders.created" });
"#,
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let producer = read_md(&space.root.join("producer-a"));
    let consumer = read_md(&space.root.join("consumer-b"));
    assert!(producer.contains("## Kafka"), "{producer}");
    assert!(producer.contains("`orders.created`"), "{producer}");
    assert!(producer.contains("produces → `consumer-b`"), "{producer}");
    assert!(consumer.contains("consumes ← `producer-a`"), "{consumer}");
}

#[test]
fn postgres_shared_identity() {
    let space = workspace("postgres-shared");
    repo(
        &space.root,
        "orders-api",
        &[(
            ".env.example",
            "DATABASE_URL=postgres://alice:secret@db.internal:5432/orders\n",
        )],
    );
    repo(
        &space.root,
        "billing-worker",
        &[(
            ".env.example",
            "DATABASE_URL=postgres://bob:other-secret@db.internal:5432/orders\n",
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let orders = read_md(&space.root.join("orders-api"));
    assert!(orders.contains("## Database"), "{orders}");
    assert!(orders.contains("PostgreSQL / `orders`"), "{orders}");
    assert!(orders.contains("`billing-worker`"), "{orders}");
    assert!(!orders.contains("secret"), "{orders}");
    assert!(!orders.contains("alice"), "{orders}");
}

#[test]
fn http_host_match() {
    let space = workspace("http-host");
    repo(
        &space.root,
        "user-service",
        &[(
            "src/server.ts",
            r#"
import express from "express";
const app = express();
app.get("/users", (_req, res) => res.json([]));
"#,
        )],
    );
    repo(
        &space.root,
        "checkout-web",
        &[(
            "src/client.ts",
            r#"
export async function loadUsers() {
  return fetch("http://user-service:8080/users");
}
"#,
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let web = read_md(&space.root.join("checkout-web"));
    let api = read_md(&space.root.join("user-service"));
    assert!(web.contains("calls → `user-service`"), "{web}");
    assert!(api.contains("called by ← `checkout-web`"), "{api}");
}

#[test]
fn mixed_three_repos() {
    let space = workspace("mixed");
    repo(
        &space.root,
        "payments-api",
        &[
            (
                "src/server.ts",
                r#"
import express from "express";
import { Kafka } from "kafkajs";
const app = express();
app.post("/charge", () => undefined);
const producer = new Kafka({ brokers: ["kafka.internal:9092"] }).producer();
await producer.send({ topic: "payment.completed", messages: [] });
"#,
            ),
            (
                ".env.example",
                "DATABASE_URL=postgres://u:p@db.internal:5432/payments\n",
            ),
        ],
    );
    repo(
        &space.root,
        "payment-worker",
        &[
            (
                "src/worker.ts",
                r#"
import { Kafka } from "kafkajs";
const consumer = new Kafka({ brokers: ["kafka.internal:9092"] }).consumer({ groupId: "w" });
await consumer.subscribe({ topic: "payment.completed" });
"#,
            ),
            (
                ".env.example",
                "DATABASE_URL=postgres://w:p@db.internal:5432/payments\n",
            ),
        ],
    );
    repo(
        &space.root,
        "checkout-web",
        &[(
            "src/client.ts",
            r#"
export const charge = () => fetch("http://payments-api:3000/charge", { method: "POST" });
"#,
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let api = read_md(&space.root.join("payments-api"));
    assert!(api.contains("PostgreSQL / `payments`"), "{api}");
    assert!(api.contains("`payment-worker`"), "{api}");
    assert!(api.contains("produces → `payment-worker`"), "{api}");
    assert!(api.contains("called by ← `checkout-web`"), "{api}");
}

#[test]
fn empty_standalone_repo() {
    let space = workspace("empty");
    repo(
        &space.root,
        "standalone-tool",
        &[("src/main.rs", "fn main() {}\n")],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let md = read_md(&space.root.join("standalone-tool"));
    assert_eq!(
        md,
        "# Weavatrix\n\nRepository: `standalone-tool`\n\nNo cross-repository integrations were proven in the selected scope.\n"
    );
}

#[test]
fn output_is_byte_identical_across_runs() {
    let space = workspace("idempotent");
    repo(
        &space.root,
        "producer-a",
        &[(
            "src/index.ts",
            r#"
import { Kafka } from "kafkajs";
const producer = new Kafka({ brokers: ["k:9092"] }).producer();
await producer.send({ topic: "orders.created", messages: [] });
"#,
        )],
    );
    repo(
        &space.root,
        "consumer-b",
        &[(
            "src/index.ts",
            r#"
import { Kafka } from "kafkajs";
const consumer = new Kafka({ brokers: ["k:9092"] }).consumer({ groupId: "g" });
await consumer.subscribe({ topic: "orders.created" });
"#,
        )],
    );
    let first = generate_folder(&space.root);
    assert_eq!(first.code, 0, "{}", first.stderr);
    let left = read_md(&space.root.join("producer-a"));
    let second = generate_folder(&space.root);
    assert_eq!(second.code, 0, "{}", second.stderr);
    let right = read_md(&space.root.join("producer-a"));
    assert_eq!(left, right);
}

#[test]
fn check_detects_stale_file() {
    let space = workspace("check");
    let path = repo(
        &space.root,
        "standalone-tool",
        &[("src/main.rs", "fn main() {}\n")],
    );
    let write = generate_folder(&space.root);
    assert_eq!(write.code, 0, "{}", write.stderr);
    std::fs::write(path.join("WEAVATRIX.md"), "# stale\n").unwrap();
    let check = weavatrix_md::run(&[
        "--folder".to_owned(),
        space.root.to_string_lossy().into_owned(),
        "--check".to_owned(),
    ]);
    assert_eq!(check.code, 1, "{}", check.stderr);
}

#[test]
fn go_flag_string_kafka_pair() {
    let space = workspace("go-flags");
    repo(
        &space.root,
        "log-service",
        &[(
            "service/service.go",
            r#"
package service
import (
    "flag"
    "github.com/segmentio/kafka-go"
)
var flagTopicIn = flag.String("ls_kafka_topic_in", "logs", "kafka in topic")
var flagTopicOut = flag.String("ls_kafka_topic_out", "events2notify", "kafka out topic")
func run() {
    _ = kafka.NewReader(kafka.ReaderConfig{Topic: *flagTopicIn})
    _ = kafka.NewWriter(kafka.WriterConfig{Topic: *flagTopicOut})
}
"#,
        )],
    );
    repo(
        &space.root,
        "notifier",
        &[(
            "infra/kafka/kafka.go",
            r#"
package kafka
import (
    "flag"
    "github.com/segmentio/kafka-go"
)
var flagEventsTopic = flag.String("eh_notifier_kafka_topic", "events2notify", "kafka in topic")
func consume() {
    _ = kafka.NewReader(kafka.ReaderConfig{Topic: *flagEventsTopic})
}
"#,
        )],
    );
    repo(
        &space.root,
        "logger",
        &[(
            "logsender/sender.go",
            r#"
package logsender
import (
    "flag"
    "github.com/segmentio/kafka-go"
)
var flagKafkaTopic = flag.String("logs_topic", "logs", "kafka topic name for logs")
func send() {
    _ = kafka.NewWriter(kafka.WriterConfig{Topic: *flagKafkaTopic})
}
"#,
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let logs = read_md(&space.root.join("log-service"));
    assert!(logs.contains("`events2notify`"), "{logs}");
    assert!(logs.contains("produces → `notifier`"), "{logs}");
    assert!(logs.contains("`logs`"), "{logs}");
    assert!(logs.contains("consumes ← `logger`"), "{logs}");
    let notifier = read_md(&space.root.join("notifier"));
    assert!(notifier.contains("consumes ← `log-service`"), "{notifier}");
}

#[test]
fn namsral_flag_events2notify() {
    let space = workspace("namsral");
    repo(
        &space.root,
        "log-service",
        &[(
            "service/service.go",
            "package service\nimport \"github.com/namsral/flag\"\nvar flagTopicOut = flag.String(\"ls_kafka_topic_out\", \"events2notify\", \"kafka out topic\")\n",
        )],
    );
    repo(
        &space.root,
        "notifier",
        &[(
            "infra/kafka/kafka.go",
            "package kafka\nimport (\n\t\"github.com/namsral/flag\"\n\t\"github.com/segmentio/kafka-go\"\n)\nvar (\n\tflagEventsTopic = flag.String(\"eh_notifier_kafka_topic\", \"events2notify\", \"kafka in topic\")\n)\nfunc CreateEventsEngine() { _ = kafka.NewReader(kafka.ReaderConfig{}) }\n",
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let logs = read_md(&space.root.join("log-service"));
    let notifier = read_md(&space.root.join("notifier"));
    assert!(logs.contains("`events2notify`"), "log-service:\n{logs}");
    assert!(
        logs.contains("produces → `notifier`"),
        "log-service:\n{logs}"
    );
    assert!(
        notifier.contains("consumes ← `log-service`"),
        "notifier:\n{notifier}"
    );
}

#[test]
fn redis_and_vault_shared_identity() {
    let space = workspace("stores");
    repo(
        &space.root,
        "api",
        &[(
            ".env.example",
            "REDIS_URL=redis://cache.internal:6379/0\nVAULT_ADDR=https://vault.internal:8200\n",
        )],
    );
    repo(
        &space.root,
        "worker",
        &[(
            ".env.example",
            "REDIS_URL=redis://cache.internal:6379/0\nVAULT_ADDR=https://vault.internal:8200\n",
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let md = read_md(&space.root.join("api"));
    assert!(md.contains("## Redis"), "{md}");
    assert!(
        md.contains("Redis / `0`") || md.contains("Redis / `cache.internal`"),
        "{md}"
    );
    assert!(md.contains("`worker`"), "{md}");
    assert!(md.contains("## Vault"), "{md}");
    assert!(
        md.contains("`vault.internal`") || md.contains("Vault /"),
        "{md}"
    );
}

#[test]
fn bgp_speaker_does_not_drop_events2notify() {
    let log_service = std::fs::read_to_string(
        r"C:\Users\SergiiZiborov\Documents\GitHub\log-service\service\service.go",
    );
    let notifier = std::fs::read_to_string(
        r"C:\Users\SergiiZiborov\Documents\GitHub\notifier\infra\kafka\kafka.go",
    );
    let bgp = std::fs::read_to_string(
        r"C:\Users\SergiiZiborov\Documents\GitHub\bgp-speaker\kafkareader\kafkareader.go",
    );
    let (Ok(log_service), Ok(notifier), Ok(bgp)) = (log_service, notifier, bgp) else {
        return;
    };
    let space = workspace("bgp-mix");
    repo(
        &space.root,
        "log-service",
        &[("service/service.go", log_service.as_str())],
    );
    repo(
        &space.root,
        "notifier",
        &[("infra/kafka/kafka.go", notifier.as_str())],
    );
    repo(
        &space.root,
        "bgp-speaker",
        &[("kafkareader/kafkareader.go", bgp.as_str())],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let logs = read_md(&space.root.join("log-service"));
    let bgp_md = read_md(&space.root.join("bgp-speaker"));
    assert!(
        logs.contains("`events2notify`"),
        "log-service lost events2notify:\n{logs}\nbgp:\n{bgp_md}"
    );
}
