//! Secret literals must never appear in Markdown, inventory display, or errors.

mod support;

use support::{generate_folder, read_md, repo, workspace};

const SECRETS: &[&str] = &["super-secret", "p@ssw0rd", "token-xyz", "alice", "p%40ss"];

#[test]
fn credentials_are_stripped_from_markdown_and_errors() {
    let space = workspace("secrets");
    repo(
        &space.root,
        "orders-api",
        &[(
            ".env.example",
            "DATABASE_URL=postgres://alice:super-secret@db.internal:5432/orders?sslmode=require\n\
             MONGO_URI=mongodb://alice:p%40ss@mongo.internal:27017/orders\n\
             SQLSERVER=Server=sql.internal;Database=app;User Id=sa;Password=p@ssw0rd;\n\
             API_URL=https://user-service/users?token=token-xyz\n",
        )],
    );
    repo(
        &space.root,
        "billing-worker",
        &[(
            ".env.example",
            "DATABASE_URL=postgres://bob:super-secret@db.internal:5432/orders\n",
        )],
    );
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let markdown = read_md(&space.root.join("orders-api"));
    let haystacks = [
        markdown.as_str(),
        output.stdout.as_str(),
        output.stderr.as_str(),
    ];
    for secret in SECRETS {
        for haystack in haystacks {
            assert!(
                !haystack.contains(secret),
                "secret {secret:?} leaked in {haystack:?}"
            );
        }
    }
    assert!(!markdown.contains("postgres://"), "{markdown}");
    assert!(!markdown.contains("mongodb://"), "{markdown}");
    assert!(markdown.contains("PostgreSQL / `orders`"), "{markdown}");
}

#[test]
fn ignored_dotenv_is_not_read() {
    let space = workspace("dotenv");
    repo(
        &space.root,
        "alpha",
        &[
            (
                ".env",
                "DATABASE_URL=postgres://alice:super-secret@db.internal:5432/orders\n",
            ),
            ("src/main.rs", "fn main() {}\n"),
        ],
    );
    repo(&space.root, "beta", &[("src/main.rs", "fn main() {}\n")]);
    let output = generate_folder(&space.root);
    assert_eq!(output.code, 0, "{}", output.stderr);
    let md = read_md(&space.root.join("alpha"));
    assert!(!md.contains("## Database"), "{md}");
    assert!(!output.stderr.contains("super-secret"), "{}", output.stderr);
}
