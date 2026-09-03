//! Tiny deterministic cross-repository integration map.
//!
//! Local mode depends only on `weavatrix-scan` and `weavatrix-parse`. It never
//! opens a network connection or executes repository code.

#![forbid(unsafe_code)]

mod aliases;
mod cli;
mod detector;
mod discover;
mod error;
mod model;
mod normalize;
mod render;
mod resolve;
mod source;
mod tokens;
mod write;

pub use cli::{Command, OutputMode, parse, usage};
pub use discover::{RepoRef, Scope, folder_scope, target_scope};
pub use error::{Error, Result};
pub use model::{
    ApiDirection, ApiObservation, ApiProtocol, ApiRelation, DatabaseEngine, DatabaseObservation,
    DatabaseRelation, KafkaObservation, KafkaRelation, KafkaRole, RepoId, RepoInventory,
    RepositoryMap,
};
pub use render::render;
pub use write::FILENAME;

use std::io::{self, Write};

/// Crate version compiled into the binary.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Process output for the CLI and integration tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliOutput {
    /// Exit code.
    pub code: i32,
    /// Stdout body.
    pub stdout: String,
    /// Stderr body.
    pub stderr: String,
}

/// Runs the CLI against already-split argv (no binary name).
#[must_use]
pub fn run(args: &[String]) -> CliOutput {
    match parse(args) {
        Ok(Command::Help) => CliOutput {
            code: 0,
            stdout: format!("{}\n", usage()),
            stderr: String::new(),
        },
        Ok(Command::Version) => CliOutput {
            code: 0,
            stdout: format!("weavatrix-md {VERSION}\n"),
            stderr: String::new(),
        },
        Ok(command) => match execute(&command) {
            Ok(output) => output,
            Err(error) => CliOutput {
                code: 2,
                stdout: String::new(),
                stderr: format!("weavatrix-md: {error}\n"),
            },
        },
        Err(error) => CliOutput {
            code: 2,
            stdout: String::new(),
            stderr: format!("weavatrix-md: {error}\n{}", usage()),
        },
    }
}

fn execute(command: &Command) -> Result<CliOutput> {
    let (scope, output) = match command {
        Command::Help | Command::Version => unreachable!(),
        Command::Target { path, output } => (discover::target_scope(path.as_deref())?, *output),
        Command::Folder { folder, output } => (discover::folder_scope(folder)?, *output),
    };
    generate(&scope, output)
}

/// Analyzes every peer once, then writes or checks each target.
///
/// # Errors
///
/// Scan, resolution, or file IO fails.
pub fn generate(scope: &Scope, output: OutputMode) -> Result<CliOutput> {
    let mut inventories = Vec::new();
    for peer in &scope.peers {
        inventories.push(source::inventory(peer)?);
    }
    let names: Vec<String> = scope.targets.iter().map(|repo| repo.name.clone()).collect();
    let maps = resolve::resolve(&inventories, &names);
    let mut connections = 0_usize;
    let mut stale = false;
    let mut stdout = String::new();
    for (target, map) in scope.targets.iter().zip(maps.iter()) {
        let markdown = render(map);
        connections += map.connection_count();
        match output {
            OutputMode::Stdout => stdout.push_str(&markdown),
            OutputMode::Write => write::atomic_write(&target.root, &markdown)?,
            OutputMode::Check => {
                let existing = write::read_existing(&target.root)?;
                if existing.as_deref() != Some(markdown.as_str()) {
                    stale = true;
                }
            }
        }
    }
    let scanned = scope.peers.len();
    let summary = match output {
        OutputMode::Write => format!(
            "{scanned} repositories scanned, {connections} connections proven, {FILENAME} updated\n"
        ),
        OutputMode::Check if stale => {
            format!("{FILENAME} is stale; run weavatrix-md to regenerate\n")
        }
        OutputMode::Check => format!("{FILENAME} is up to date\n"),
        OutputMode::Stdout => String::new(),
    };
    let (code, stderr) = match output {
        OutputMode::Check if stale => (1, summary),
        OutputMode::Stdout => (0, String::new()),
        _ => (0, summary),
    };
    Ok(CliOutput {
        code,
        stdout,
        stderr,
    })
}

/// Writes CLI output to the real stdout/stderr.
///
/// # Errors
///
/// Stdout/stderr writes fail.
pub fn emit(output: &CliOutput) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();
    stdout.write_all(output.stdout.as_bytes())?;
    stderr.write_all(output.stderr.as_bytes())?;
    Ok(())
}
