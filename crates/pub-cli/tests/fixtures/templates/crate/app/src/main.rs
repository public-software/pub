//! `{{CRATE}}` — the `{{COMPONENT}}` command of [`{{REPO}}`](https://github.com/{{ORG}}/{{REPO}}).
//!
//! `main` does the I/O; [`run`] does the work and is what the unit tests call.

#![forbid(unsafe_code)]

use std::process::ExitCode;

/// What `--version` prints.
const VERSION_LINE: &str = concat!("{{COMPONENT}} ", env!("CARGO_PKG_VERSION"));

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(out) => {
            print!("{out}");
            ExitCode::SUCCESS
        }
        Err(problem) => {
            eprintln!("{{COMPONENT}}: {problem}");
            ExitCode::FAILURE
        }
    }
}

/// Runs the command on `args` and returns what it prints; free of I/O so tests call it directly.
fn run(args: &[String]) -> Result<String, String> {
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        [] | ["--version"] => Ok(format!("{VERSION_LINE}\n")),
        [other, ..] => Err(format!("unknown argument `{other}`; try --version")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|a| (*a).to_owned()).collect()
    }

    #[test]
    fn no_arguments_prints_the_version_line() {
        assert_eq!(run(&args(&[])).unwrap(), format!("{VERSION_LINE}\n"));
    }

    #[test]
    fn version_names_the_command() {
        assert!(run(&args(&["--version"])).unwrap().starts_with("{{COMPONENT}} "));
    }

    #[test]
    fn an_unknown_argument_is_an_error_naming_it() {
        let problem = run(&args(&["--bogus"])).unwrap_err();
        assert!(problem.contains("--bogus"), "{problem}");
    }
}
