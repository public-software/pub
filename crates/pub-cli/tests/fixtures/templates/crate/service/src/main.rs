//! `{{DAEMON_PREFIX}}-{{COMPONENT}}` — the daemon around [`Service`].

#![forbid(unsafe_code)]

use std::process::ExitCode;

use {{CRATE_IDENT}}::Service;

const NAME: &str = "{{DAEMON_PREFIX}}-{{COMPONENT}}";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match args.as_slice() {
        ["--health"] => {
            let health = Service::default().health();
            println!("{health}");
            ExitCode::from(health.exit_code())
        }
        ["--version"] => {
            println!("{NAME} {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        [] => {
            eprintln!("{NAME}: serving is not implemented yet; see README.md");
            ExitCode::from(2)
        }
        [other, ..] => {
            eprintln!("{NAME}: unknown argument `{other}`; try --health or --version");
            ExitCode::FAILURE
        }
    }
}
