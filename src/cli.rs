//! Argv parser. No clap: four command shapes plus two output flags.

use crate::error::{Error, Result};

/// How Markdown should be delivered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Atomically replace `WEAVATRIX.md` in each target root.
    Write,
    /// Compare generated Markdown with the committed file.
    Check,
    /// Print Markdown for a single target.
    Stdout,
}

/// Parsed invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Show usage.
    Help,
    /// Show crate version.
    Version,
    /// Current or explicit repository; peers are sibling Git roots.
    Target {
        /// Repository path; `None` means the current directory.
        path: Option<String>,
        /// Delivery mode.
        output: OutputMode,
    },
    /// Every Git repository under `folder` is both target and peer.
    Folder {
        /// Root to walk for Git repositories.
        folder: String,
        /// Delivery mode. `--stdout` is rejected for folder runs.
        output: OutputMode,
    },
}

/// Parses argv after the binary name.
///
/// # Errors
///
/// Unknown flags, missing values, or incompatible flag combinations.
pub fn parse(args: &[String]) -> Result<Command> {
    let mut path = None;
    let mut folder = None;
    let mut check = false;
    let mut stdout = false;
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "-h" | "--help" => return Ok(Command::Help),
            "-V" | "--version" => return Ok(Command::Version),
            "--check" => check = true,
            "--stdout" => stdout = true,
            "--folder" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| Error::new("--folder requires a directory"))?;
                folder = Some(value.clone());
            }
            flag if flag.starts_with('-') => {
                return Err(Error::new(format!("unknown flag: {flag}")));
            }
            _ if path.is_none() => path = Some(arg.clone()),
            _ => return Err(Error::new("unexpected extra argument")),
        }
        index += 1;
    }
    if check && stdout {
        return Err(Error::new("use only one of --check and --stdout"));
    }
    let output = if check {
        OutputMode::Check
    } else if stdout {
        OutputMode::Stdout
    } else {
        OutputMode::Write
    };
    if let Some(folder) = folder {
        if path.is_some() {
            return Err(Error::new("PATH and --folder cannot be combined"));
        }
        if output == OutputMode::Stdout {
            return Err(Error::new("--stdout requires a single target repository"));
        }
        return Ok(Command::Folder { folder, output });
    }
    Ok(Command::Target { path, output })
}

/// Usage text.
#[must_use]
pub fn usage() -> &'static str {
    "weavatrix-md — tiny cross-repository integration map

Usage:
  weavatrix-md [PATH]
  weavatrix-md --folder DIR
  weavatrix-md mcp
  weavatrix-md --help
  weavatrix-md --version

Options:
  --folder DIR   Find Git repositories under DIR and write WEAVATRIX.md in each
  --check        Exit 1 if a committed WEAVATRIX.md is stale
  --stdout       Print Markdown instead of writing the file
  -h, --help     Show this help
  -V, --version  Show version

Default scope is the current Git repository plus sibling Git repositories
in the parent directory. Missing edges are omitted; false edges are not.
"
}
