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
