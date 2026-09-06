//! Stdio MCP host. Same binary as the CLI: `weavatrix-md mcp`.

use crate::{OutputMode, VERSION, folder_scope, generate, target_scope};
use mcport::{ConcurrentMcpServer, RuntimeConfig, ToolReply, Value, json};
use std::time::Duration;

/// Serves MCP on stdin/stdout until the client disconnects.
///
/// # Errors
///
/// Transport or handler failures.
pub fn serve() -> Result<(), String> {
    ConcurrentMcpServer::new("weavatrix-md", VERSION)
        .instructions(
            "Generate WEAVATRIX.md, a tiny cross-repository integration map \
             (Database, Redis, Vault, Kafka, API). Local, offline, no writes \
             except the generated Markdown file.",
        )
        .tool(
            "weavatrix_md",
            "Analyze repositories and write or return WEAVATRIX.md.",
            json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Target repository. Default: current Git root."
                    },
                    "folder": {
                        "type": "string",
                        "description": "Scan every Git repository under this folder."
                    },
                    "check": {
                        "type": "boolean",
                        "description": "Do not write; fail if WEAVATRIX.md is stale."
                    },
                    "stdout": {
                        "type": "boolean",
                        "description": "Return Markdown instead of writing the file."
                    }
                },
                "additionalProperties": false
            }),
            |_, arguments| generate_tool(&arguments),
        )
        .serve(RuntimeConfig {
            max_in_flight: 1,
            queue_depth: 8,
            output_queue_depth: 8,
            handler_deadline: Some(Duration::from_secs(120)),
            ..RuntimeConfig::default()
        })
        .map_err(|error| error.to_string())
}

fn generate_tool(arguments: &Value) -> ToolReply {
    let path = arguments.get("path").and_then(Value::as_str);
    let folder = arguments.get("folder").and_then(Value::as_str);
    let check = arguments
        .get("check")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let stdout = arguments
        .get("stdout")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if check && stdout {
        return ToolReply::error("use only one of check and stdout");
    }
    let output = if check {
        OutputMode::Check
    } else if stdout {
        OutputMode::Stdout
    } else {
        OutputMode::Write
    };
    let scope = if let Some(folder) = folder {
        folder_scope(folder)
    } else {
        target_scope(path)
    };
    match scope.and_then(|scope| generate(&scope, output)) {
        Ok(result) => ToolReply::structured(json!({
            "code": result.code,
            "stdout": result.stdout,
            "stderr": result.stderr
        })),
        Err(error) => ToolReply::error(error.to_string()),
    }
}
