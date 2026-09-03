//! `weavatrix-md` binary.

#![forbid(unsafe_code)]

use std::process::ExitCode;
use weavatrix_md::{emit, run};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let output = run(&args);
    let _ = emit(&output);
    ExitCode::from(u8::try_from(output.code).unwrap_or(1))
}
