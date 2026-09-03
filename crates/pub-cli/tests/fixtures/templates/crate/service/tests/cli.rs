//! The daemon as an operator runs it.

use std::process::Command;

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_{{DAEMON_PREFIX}}-{{COMPONENT}}"))
}

#[test]
fn health_answers_ok_and_exits_zero() {
    let out = command().arg("--health").output().expect("the daemon runs");
    assert!(out.status.success(), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "ok");
}

#[test]
fn version_names_the_daemon() {
    let out = command()
        .arg("--version")
        .output()
        .expect("the daemon runs");
    assert!(out.status.success(), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).starts_with("{{DAEMON_PREFIX}}-{{COMPONENT}} "));
}
