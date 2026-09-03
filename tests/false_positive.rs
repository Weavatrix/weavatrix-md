//! Ambiguous evidence must not become a committed edge.

mod support;

use support::{generate_folder, read_md, repo, workspace};

#[test]
fn ambiguous_health_path_is_omitted() {
    let space = workspace("health");
    repo(
        &space.root,
        "payments",
        &[(
            "src/server.ts",
            "import express from 'express'; const app = express(); app.get('/health', () => undefined);\n",
        )],
    );
    repo(
        &space.root,
        "orders",
        &[(
            "src/server.ts",
            "import express from 'express'; const app = express(); app.get('/health', () => undefined);\n",
        )],
    );
    repo(
        &space.root,
        "frontend",
        &[(
            "src/client.ts",
            "export const ping = () => fetch('/health');\n",
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    for name in ["payments", "orders", "frontend"] {
        let md = read_md(&space.root.join(name));
        assert!(!md.contains("## API"), "{name}: {md}");
        assert!(
            md.contains("No cross-repository integrations were proven"),
            "{name}: {md}"
        );
    }
}

#[test]
fn kafka_same_role_has_no_peer() {
    let space = workspace("kafka-same");
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
        "producer-b",
        &[(
            "src/index.ts",
            r#"
import { Kafka } from "kafkajs";
const producer = new Kafka({ brokers: ["k:9092"] }).producer();
await producer.send({ topic: "orders.created", messages: [] });
"#,
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let md = read_md(&space.root.join("producer-a"));
    assert!(!md.contains("## Kafka"), "{md}");
}

#[test]
fn kafka_cluster_conflict_is_omitted() {
    let space = workspace("kafka-cluster");
    repo(
        &space.root,
        "alpha",
        &[(
            "src/index.ts",
            r#"
import { Kafka } from "kafkajs";
const producer = new Kafka({ brokers: ["cluster-a:9092"] }).producer();
await producer.send({ topic: "events", messages: [] });
"#,
        )],
    );
    repo(
        &space.root,
        "beta",
        &[(
            "src/index.ts",
            r#"
import { Kafka } from "kafkajs";
const consumer = new Kafka({ brokers: ["cluster-b:9092"] }).consumer({ groupId: "g" });
await consumer.subscribe({ topic: "events" });
"#,
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let md = read_md(&space.root.join("alpha"));
    assert!(!md.contains("## Kafka"), "{md}");
}

#[test]
fn table_name_similarity_is_not_a_database_edge() {
    let space = workspace("tables");
    repo(
        &space.root,
        "alpha",
        &[("src/db.sql", "CREATE TABLE users (id int);\n")],
    );
    repo(
        &space.root,
        "beta",
        &[("src/db.sql", "CREATE TABLE users (id int);\n")],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let md = read_md(&space.root.join("alpha"));
    assert!(!md.contains("## Database"), "{md}");
}

#[test]
fn localhost_database_is_not_shared() {
    let space = workspace("localhost");
    repo(
        &space.root,
        "alpha",
        &[(
            ".env.example",
            "DATABASE_URL=postgres://u:p@localhost:5432/app\n",
        )],
    );
    repo(
        &space.root,
        "beta",
        &[(
            ".env.example",
            "DATABASE_URL=postgres://u:p@localhost:5432/app\n",
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let md = read_md(&space.root.join("alpha"));
    assert!(!md.contains("## Database"), "{md}");
}

#[test]
fn comment_and_docs_kafka_topic_is_ignored() {
    let space = workspace("docs");
    repo(
        &space.root,
        "alpha",
        &[(
            "src/index.ts",
            "// producer.send({ topic: \"orders.created\" })\nexport const ok = true;\n",
        )],
    );
    repo(
        &space.root,
        "beta",
        &[(
            "src/index.ts",
            "/* consumer.subscribe({ topic: \"orders.created\" }) */\nexport const ok = true;\n",
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let md = read_md(&space.root.join("alpha"));
    assert!(!md.contains("## Kafka"), "{md}");
}

#[test]
fn different_database_names_on_same_host_are_separate() {
    let space = workspace("db-names");
    repo(
        &space.root,
        "alpha",
        &[(
            ".env.example",
            "DATABASE_URL=postgres://u:p@db.internal:5432/orders\n",
        )],
    );
    repo(
        &space.root,
        "beta",
        &[(
            ".env.example",
            "DATABASE_URL=postgres://u:p@db.internal:5432/payments\n",
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let md = read_md(&space.root.join("alpha"));
    assert!(!md.contains("## Database"), "{md}");
}
