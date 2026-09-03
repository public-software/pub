//! The binary as a user runs it.

use std::process::Command;

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_{{COMPONENT}}"))
}

#[test]
fn version_exits_zero_and_names_the_command() {
    let out = command()
        .arg("--version")
        .output()
        .expect("the binary runs");
    assert!(out.status.success(), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("{{COMPONENT}} "));
}

#[test]
fn an_unknown_argument_exits_one_and_explains_on_stderr() {
    let out = command().arg("--bogus").output().expect("the binary runs");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown argument"));
}
