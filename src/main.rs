//! `weavatrix-md` binary.

#![forbid(unsafe_code)]

use std::process::ExitCode;
use weavatrix_md::{emit, run};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|item| item == "mcp") {
        return run_mcp(&args[1..]);
    }
    let output = run(&args);
    let _ = emit(&output);
    ExitCode::from(u8::try_from(output.code).unwrap_or(1))
}

fn run_mcp(args: &[String]) -> ExitCode {
    if args.iter().any(|item| item == "--help" || item == "-h") {
        println!(
            "weavatrix-md mcp — generate WEAVATRIX.md for coding agents\n\nUsage:\n  weavatrix-md mcp"
        );
        return ExitCode::SUCCESS;
    }
    match weavatrix_md::serve_mcp() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("weavatrix-md mcp: {error}");
            ExitCode::FAILURE
        }
    }
}
