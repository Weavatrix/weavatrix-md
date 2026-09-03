//! CLI surface.

mod support;

use support::{repo, workspace};
use weavatrix_md::{VERSION, run};

#[test]
fn help_and_version() {
    let help = run(&["--help".to_owned()]);
    assert_eq!(help.code, 0);
    assert!(help.stdout.contains("weavatrix-md"));
    assert!(help.stdout.contains("--folder"));
    let version = run(&["--version".to_owned()]);
    assert_eq!(version.code, 0);
    assert!(version.stdout.contains(VERSION));
}

#[test]
fn unknown_flag_fails() {
    let output = run(&["--wat".to_owned()]);
    assert_eq!(output.code, 2);
    assert!(output.stderr.contains("unknown flag"));
}

#[test]
fn stdout_prints_without_writing() {
    let space = workspace("stdout");
    let path = repo(
        &space.root,
        "standalone-tool",
        &[("src/main.rs", "fn main() {}\n")],
    );
    let output = run(&[path.to_string_lossy().into_owned(), "--stdout".to_owned()]);
    assert_eq!(output.code, 0, "{}", output.stderr);
    assert!(output.stdout.contains("# Weavatrix"));
    assert!(!path.join("WEAVATRIX.md").exists());
}

#[test]
fn folder_and_path_cannot_combine() {
    let output = run(&[".".to_owned(), "--folder".to_owned(), ".".to_owned()]);
    assert_eq!(output.code, 2);
}
