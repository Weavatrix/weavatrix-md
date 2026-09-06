//! Deterministic `WEAVATRIX.md` projection. Empty sections are omitted.

use crate::model::RepositoryMap;

/// Renders one repository map.
#[must_use]
pub fn render(map: &RepositoryMap) -> String {
    let mut out = String::new();
    out.push_str("# Weavatrix\n\n");
    out.push_str("Repository: `");
    out.push_str(&map.repository);
    out.push_str("`\n");
    let mut wrote = false;
    for section in ["Database", "Redis", "Vault"] {
        let items: Vec<_> = map
            .databases
            .iter()
            .filter(|item| engine_section(&item.engine) == section)
            .collect();
        if items.is_empty() {
            continue;
        }
        wrote = true;
        out.push_str("\n## ");
        out.push_str(section);
        out.push_str("\n\n");
        for item in items {
            out.push_str("- ");
            out.push_str(&item.engine);
            out.push_str(" / `");
            out.push_str(&item.database);
            out.push_str("`\n");
            for peer in &item.peers {
                out.push_str("  - `");
                out.push_str(peer);
                out.push_str("`\n");
            }
        }
    }
    if !map.kafka.is_empty() {
        wrote = true;
        out.push_str("\n## Kafka\n\n");
        for (index, item) in map.kafka.iter().enumerate() {
            if index > 0 {
                out.push('\n');
            }
            out.push_str("- `");
            out.push_str(&item.topic);
            out.push_str("`\n");
            for peer in &item.produces {
                out.push_str("  - produces → `");
                out.push_str(peer);
                out.push_str("`\n");
            }
            for peer in &item.consumes {
                out.push_str("  - consumes ← `");
                out.push_str(peer);
                out.push_str("`\n");
            }
        }
    }
    if !map.api.outgoing.is_empty() || !map.api.incoming.is_empty() {
        wrote = true;
        out.push_str("\n## API\n\n");
        for peer in &map.api.outgoing {
            out.push_str("- calls → `");
            out.push_str(peer);
            out.push_str("`\n");
        }
        for peer in &map.api.incoming {
            out.push_str("- called by ← `");
            out.push_str(peer);
            out.push_str("`\n");
        }
    }
    if !wrote {
        out.push_str("\nNo cross-repository integrations were proven in the selected scope.\n");
    }
    out
}

fn engine_section(label: &str) -> &'static str {
    match label {
        "Redis" => "Redis",
        "Vault" => "Vault",
        _ => "Database",
    }
}
