//! `pub` — the Public Software organization CLI.
//!
//! Version 0 knows the catalog: `pub catalog validate` checks `catalog.toml` against its rules,
//! `pub catalog render readme|json` prints a view of a valid catalog.

mod catalog;
mod render;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

use crate::catalog::Catalog;

#[derive(Parser)]
#[command(name = "pub", version, about = "The Public Software organization CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// The catalog: every repository of the organization, from catalog.toml
    Catalog {
        /// Path to catalog.toml; default: the first of catalog/catalog.toml, catalog.toml,
        /// config/catalog.toml that exists
        #[arg(long, global = true, value_name = "PATH")]
        catalog: Option<PathBuf>,
        #[command(subcommand)]
        action: CatalogAction,
    },
}

#[derive(Subcommand)]
enum CatalogAction {
    /// Check the catalog against its rules; every problem goes to stderr
    Validate,
    /// Print a view of a valid catalog
    Render {
        #[command(subcommand)]
        view: View,
    },
}

#[derive(Subcommand)]
enum View {
    /// The ring tables the organization README embeds
    Readme,
    /// The repositories as a JSON array
    Json,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Catalog { catalog, action } => run_catalog(catalog, action),
    }
}

fn run_catalog(path: Option<PathBuf>, action: CatalogAction) -> ExitCode {
    let loaded = catalog::locate(path.as_deref()).and_then(|p| Catalog::load(&p));
    let cat = match loaded {
        Ok(cat) => cat,
        Err(problem) => {
            eprintln!("{problem}");
            return ExitCode::FAILURE;
        }
    };
    let problems = cat.validate();
    for problem in &problems {
        eprintln!("{problem}");
    }
    if !problems.is_empty() {
        return ExitCode::FAILURE;
    }
    match action {
        CatalogAction::Validate => ExitCode::SUCCESS,
        CatalogAction::Render { view: View::Readme } => {
            print!("{}", render::readme(&cat));
            ExitCode::SUCCESS
        }
        CatalogAction::Render { view: View::Json } => match render::json(&cat) {
            Ok(text) => {
                println!("{text}");
                ExitCode::SUCCESS
            }
            Err(problem) => {
                eprintln!("{problem}");
                ExitCode::FAILURE
            }
        },
    }
}
