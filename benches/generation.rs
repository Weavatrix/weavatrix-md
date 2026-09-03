//! Generation overhead over synthetic sibling repositories.

use std::fs;
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;
use weavatrix_md::{OutputMode, folder_scope, generate};

fn main() {
    println!("statistic=median runs=11 warmups=2");
    for count in [1_usize, 5, 20] {
        let workspace = temp_workspace(count);
        let scope = folder_scope(workspace.to_str().expect("utf8")).expect("scope");
        let measure = || {
            let output = generate(&scope, OutputMode::Stdout).expect("generate");
            black_box(output);
        };
        let median = median_ms(measure);
        println!("repos={count} median_ms={median:.3}");
        let _ = fs::remove_dir_all(&workspace);
    }
}

fn temp_workspace(count: usize) -> std::path::PathBuf {
    let root =
        std::env::temp_dir().join(format!("weavatrix-md-bench-{}-{count}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("workspace");
    for index in 0..count {
        let repo = root.join(format!("service-{index}"));
        write_repo(&repo, index, count);
    }
    root
}

fn write_repo(root: &Path, index: usize, count: usize) {
    fs::create_dir_all(root.join("src")).expect("src");
    fs::create_dir_all(root.join(".git")).expect("git");
    fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").expect("head");
    let peer = (index + 1) % count.max(1);
    let source = format!(
        "import {{ Kafka }} from \"kafkajs\";\n\
         import express from \"express\";\n\
         const app = express();\n\
         app.get(\"/items\", () => undefined);\n\
         const producer = new Kafka({{ brokers: [\"kafka.internal:9092\"] }}).producer();\n\
         await producer.send({{ topic: \"stream.{index}\", messages: [] }});\n\
         const consumer = new Kafka({{ brokers: [\"kafka.internal:9092\"] }}).consumer({{ groupId: \"g\" }});\n\
         await consumer.subscribe({{ topic: \"stream.{peer}\" }});\n\
         export const call = () => fetch(\"http://service-{peer}:8080/items\");\n"
    );
    fs::write(root.join("src/index.ts"), source).expect("source");
    fs::write(
        root.join(".env.example"),
        "DATABASE_URL=postgres://u:p@db.internal:5432/shared\n",
    )
    .expect("env");
}

fn median_ms(mut operation: impl FnMut()) -> f64 {
    for _ in 0..2 {
        operation();
    }
    let mut samples = Vec::new();
    for _ in 0..11 {
        let started = Instant::now();
        operation();
        samples.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}
