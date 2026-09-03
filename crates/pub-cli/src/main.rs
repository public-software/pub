//! `pub` — the Public Software organization CLI.
//!
//! Version 0 knows the catalog: `pub catalog validate` checks `catalog.toml` against its rules,
//! `pub catalog render readme|json` prints a view of a valid catalog, `pub catalog sync` makes
//! every repository on GitHub match it (description, homepage, topics, custom properties and,
//! with `--labels`, the label set), reading before every write. `pub new <kind> <component>`
//! renders a crate from the skeleton set into the current repository.

mod catalog;
mod gh;
mod new;
mod render;
mod sync;

use std::fs;
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
    /// A new crate from the skeleton set: `crates/pub-<repo>-<component>` in this repository
    New {
        /// The crate kind
        kind: new::Kind,
        /// The component name: lowercase words joined by hyphens
        component: String,
        /// The repository (or a directory inside it); default: the current directory
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
        /// A templates checkout (`crate/<kind>` under it) or the bootstrap kit; default: a shallow
        /// clone of the templates repository
        #[arg(long, value_name = "PATH")]
        templates: Option<PathBuf>,
        /// The ref of the templates repository to clone
        #[arg(long = "ref", value_name = "REF", default_value = new::TEMPLATES_REF)]
        reference: String,
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
    /// Make every repository on GitHub match the catalog (read-then-diff, through `gh`)
    Sync {
        /// Also converge this label set (a JSON array of {name, color, description})
        #[arg(long, value_name = "FILE")]
        labels: Option<PathBuf>,
        /// Only these repositories (repeatable)
        #[arg(long = "repo", value_name = "NAME")]
        repos: Vec<String>,
        /// Print the writes instead of sending them
        #[arg(long)]
        dry_run: bool,
        /// Repositories worked on at once
        #[arg(long, default_value_t = 8)]
        jobs: usize,
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
        Command::New {
            kind,
            component,
            dir,
            templates,
            reference,
        } => {
            let opts = new::Options {
                kind,
                component,
                dir,
                templates,
                reference: Some(reference),
            };
            match new::run(&opts) {
                Ok(out) => {
                    print!("{out}");
                    ExitCode::SUCCESS
                }
                Err(problem) => {
                    eprintln!("pub new: {problem}");
                    ExitCode::FAILURE
                }
            }
        }
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
        CatalogAction::Sync {
            labels,
            repos,
            dry_run,
            jobs,
        } => run_sync(&cat, labels, repos, dry_run, jobs),
    }
}

fn run_sync(
    cat: &Catalog,
    labels: Option<PathBuf>,
    only: Vec<String>,
    dry_run: bool,
    jobs: usize,
) -> ExitCode {
    let labels = match labels.map(read_labels) {
        Some(Ok(set)) => Some(set),
        Some(Err(problem)) => {
            eprintln!("{problem}");
            return ExitCode::FAILURE;
        }
        None => None,
    };
    let opts = sync::Options {
        dry_run,
        jobs,
        only,
        labels,
    };
    let outcomes = sync::run(cat, &opts);
    let (mut writes, mut failures) = (0usize, 0usize);
    for outcome in &outcomes {
        writes += outcome.writes.len();
        if dry_run {
            for w in &outcome.writes {
                println!("  $ {} {} — {}", w.method, w.path, w.what);
            }
        }
        match (&outcome.error, outcome.writes.len()) {
            (Some(error), _) => {
                failures += 1;
                eprintln!("  ✗ {}: {error}", outcome.name);
            }
            (None, 0) => println!("  · {} (up to date)", outcome.name),
            (None, n) if dry_run => println!("  → {} ({n} writes planned)", outcome.name),
            (None, n) => println!("  ✓ {} ({n} changes)", outcome.name),
        }
    }
    println!(
        "  {} repositories, {writes} writes{}, {failures} failures",
        outcomes.len(),
        if dry_run { " planned" } else { "" }
    );
    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn read_labels(path: PathBuf) -> Result<Vec<sync::Label>, String> {
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}
