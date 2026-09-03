//! `pub new <kind> <component>` — a crate from the skeleton set, rendered into
//! `crates/<CRATE_PREFIX>-<repo>-<component>/` of the current repository.
//!
//! The skeleton set (`crate/<kind>/` in the `templates` repository) is plain files with `{{NAME}}`
//! placeholders; the templates repository's README is the contract. Text is substituted and written
//! with exactly one trailing newline, a file that is not text is copied byte for byte, `.DS_Store`
//! is skipped. Everything is rendered in memory first, so an existing path, a malformed component or
//! a placeholder this command does not know refuses the run before a byte is written.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::ValueEnum;

/// The organization's names, as the bootstrap kit's `config/org.env` states them.
pub mod org {
    /// The GitHub organization.
    pub const ORG: &str = "public-software";
    /// The organization's display name.
    pub const ORG_DISPLAY_NAME: &str = "Public Software";
    /// The organization's site.
    pub const ORG_URL: &str = "https://publicsoftware.dev";
    /// The org CLI binary.
    pub const CLI: &str = "pub";
    /// Crates are `<CRATE_PREFIX>-<repo>-<component>`.
    pub const CRATE_PREFIX: &str = "pub";
    /// A service's binary is `<DAEMON_PREFIX>-<component>`.
    pub const DAEMON_PREFIX: &str = "pubd";
    /// WIT packages are `<WIT_NAMESPACE>:<name>@<version>`.
    pub const WIT_NAMESPACE: &str = "public";
    /// The minimum supported Rust version.
    pub const MSRV: &str = "1.90";
    /// The Rust edition.
    pub const EDITION: &str = "2024";
    /// The licence of code.
    pub const LICENSE_SPDX: &str = "Apache-2.0 OR MIT";
}

/// The templates repository, cloned when no `--templates` path is given.
pub const TEMPLATES_URL: &str = "https://github.com/public-software/templates";
/// The ref of the templates repository the clone pins; a release tag once the repository tags one.
pub const TEMPLATES_REF: &str = "main";

/// Files with these extensions are copied byte for byte instead of substituted.
const BINARY_EXTENSIONS: [&str; 9] = [
    "png", "jpg", "jpeg", "gif", "ico", "webp", "woff", "woff2", "pdf",
];

/// A crate kind of the skeleton set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "lower")]
pub enum Kind {
    /// A library crate.
    Lib,
    /// A command; the binary is the component name.
    App,
    /// A daemon; the binary is `<DAEMON_PREFIX>-<component>`.
    Service,
    /// A `<WIT_NAMESPACE>:core` component (cdylib + rlib).
    Plugin,
    /// A specification with its conformance cases.
    Spec,
}

impl Kind {
    /// The kind as the skeleton directory and `CATALOG.toml` name it.
    pub fn name(self) -> &'static str {
        match self {
            Kind::Lib => "lib",
            Kind::App => "app",
            Kind::Service => "service",
            Kind::Plugin => "plugin",
            Kind::Spec => "spec",
        }
    }
}

/// What `pub new` was asked to do.
#[derive(Debug)]
pub struct Options {
    /// The crate kind.
    pub kind: Kind,
    /// The component name: lowercase words joined by hyphens.
    pub component: String,
    /// The repository, or a directory inside it; default: the current directory.
    pub dir: Option<PathBuf>,
    /// A templates checkout (`crate/<kind>/` under it) or the bootstrap kit
    /// (`templates/skeleton/crate/<kind>/`); default: a shallow clone of [`TEMPLATES_URL`].
    pub templates: Option<PathBuf>,
    /// The ref to clone; default: [`TEMPLATES_REF`].
    pub reference: Option<String>,
}

/// Runs the command and returns what it prints.
pub fn run(opts: &Options) -> Result<String, String> {
    if !component_is_well_formed(&opts.component) {
        return Err(format!(
            "`{}` is not a component name: lowercase words joined by hyphens (a-z, 0-9, -)",
            opts.component
        ));
    }
    let start = match &opts.dir {
        Some(dir) => dir.clone(),
        None => std::env::current_dir().map_err(|e| format!("current directory: {e}"))?,
    };
    let repo = Repo::locate(&start)?;
    let skeleton = Skeleton::resolve(
        opts.kind,
        opts.templates.as_deref(),
        opts.reference.as_deref(),
    )?;
    let rendered = render(opts.kind, &repo, &opts.component, &skeleton.dir)?;
    rendered.write()?;
    let mut out = format!(
        "  ✓ {} rendered into {} (from {})\n\n  next:\n  1. add this entry to {}:\n\n",
        rendered.crate_name,
        rendered.dir.display(),
        skeleton.dir.display(),
        repo.root.join("CATALOG.toml").display()
    );
    match rendered.component_entry() {
        Some(entry) => out.push_str(&entry),
        None => out.push_str("  (the skeleton's README carries no [[component]] entry)\n"),
    }
    out.push_str(&format!(
        "\n  2. add a Status row to {}\n  3. {} check\n",
        repo.root.join("README.md").display(),
        org::CLI
    ));
    Ok(out)
}

/// A component is lowercase words joined by hyphens: `sched`, `boot-hello`.
pub fn component_is_well_formed(component: &str) -> bool {
    !component.is_empty()
        && component.split('-').all(|word| {
            !word.is_empty()
                && word
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// The repository a crate is rendered into.
#[derive(Debug, PartialEq, Eq)]
pub struct Repo {
    /// The directory holding `CATALOG.toml` and `Cargo.toml`.
    pub root: PathBuf,
    /// `[repo] name` of its `CATALOG.toml`.
    pub name: String,
}

impl Repo {
    /// Finds the repository `start` lies in: the first ancestor (itself included) with a
    /// `CATALOG.toml` carrying `[repo] name`, which must have a `Cargo.toml` beside it.
    pub fn locate(start: &Path) -> Result<Repo, String> {
        let start = fs::canonicalize(start).map_err(|e| format!("{}: {e}", start.display()))?;
        let mut dir = Some(start.as_path());
        while let Some(candidate) = dir {
            let catalog = candidate.join("CATALOG.toml");
            if catalog.is_file() {
                let name = repo_name(&catalog)?;
                if !candidate.join("Cargo.toml").is_file() {
                    return Err(format!(
                        "{} has no Cargo.toml beside it; a crate needs a workspace to land in",
                        catalog.display()
                    ));
                }
                return Ok(Repo {
                    root: candidate.to_path_buf(),
                    name,
                });
            }
            dir = candidate.parent();
        }
        Err(format!(
            "{} is not inside a repository: no CATALOG.toml with a [repo] table in it or above it (--dir names one)",
            start.display()
        ))
    }
}

/// `[repo] name` of a `CATALOG.toml`.
fn repo_name(catalog: &Path) -> Result<String, String> {
    let text = fs::read_to_string(catalog).map_err(|e| format!("{}: {e}", catalog.display()))?;
    let table: toml::Table =
        toml::from_str(&text).map_err(|e| format!("{}: {e}", catalog.display()))?;
    table
        .get("repo")
        .and_then(|repo| repo.get("name"))
        .and_then(|name| name.as_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("{}: no [repo] name", catalog.display()))
}

/// The skeleton directory of one kind, and the clone it came from when it was fetched.
#[derive(Debug)]
pub struct Skeleton {
    /// `…/crate/<kind>`.
    pub dir: PathBuf,
    /// A temporary clone to remove when done.
    clone: Option<PathBuf>,
}

impl Skeleton {
    /// Finds `crate/<kind>/` under `templates` (a templates checkout, or the bootstrap kit's
    /// `templates/skeleton/`), or clones the templates repository at `reference` when no path is given.
    pub fn resolve(
        kind: Kind,
        templates: Option<&Path>,
        reference: Option<&str>,
    ) -> Result<Skeleton, String> {
        let rel = Path::new("crate").join(kind.name());
        if let Some(root) = templates {
            for candidate in [root.join(&rel), root.join("templates/skeleton").join(&rel)] {
                if candidate.is_dir() {
                    return Ok(Skeleton {
                        dir: candidate,
                        clone: None,
                    });
                }
            }
            return Err(format!(
                "{} carries no {}: pass a templates checkout or the bootstrap kit",
                root.display(),
                rel.display()
            ));
        }
        let reference = reference.unwrap_or(TEMPLATES_REF);
        let clone = std::env::temp_dir().join(format!("pub-new-{}", std::process::id()));
        let _ = fs::remove_dir_all(&clone);
        let out = Command::new("git")
            .args([
                "clone",
                "--quiet",
                "--depth",
                "1",
                "--branch",
                reference,
                TEMPLATES_URL,
            ])
            .arg(&clone)
            .output()
            .map_err(|e| format!("git: {e} (is git installed?)"))?;
        if !out.status.success() {
            let _ = fs::remove_dir_all(&clone);
            return Err(format!(
                "git clone {TEMPLATES_URL} at {reference} failed: {}\n  (a local checkout works too: --templates <path>)",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        let dir = clone.join(&rel);
        if !dir.is_dir() {
            let _ = fs::remove_dir_all(&clone);
            return Err(format!(
                "{TEMPLATES_URL} at {reference} carries no {} yet: pass --templates <path> to a checkout that does",
                rel.display()
            ));
        }
        Ok(Skeleton {
            dir,
            clone: Some(clone),
        })
    }
}

impl Drop for Skeleton {
    fn drop(&mut self) {
        if let Some(clone) = &self.clone {
            let _ = fs::remove_dir_all(clone);
        }
    }
}

/// A crate rendered in memory, ready to write.
#[derive(Debug)]
pub struct Rendered {
    /// `<CRATE_PREFIX>-<repo>-<component>`.
    pub crate_name: String,
    /// `crates/<crate_name>` of the repository.
    pub dir: PathBuf,
    /// Every file, relative to `dir`, with its bytes.
    pub files: Vec<(PathBuf, Vec<u8>)>,
}

impl Rendered {
    /// Writes every file under `dir`.
    pub fn write(&self) -> Result<(), String> {
        for (rel, bytes) in &self.files {
            let path = self.dir.join(rel);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
            }
            fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        Ok(())
    }

    /// The `[[component]]` entry the rendered README carries in its ```` ```toml ```` block.
    pub fn component_entry(&self) -> Option<String> {
        let readme = self
            .files
            .iter()
            .find(|(rel, _)| rel == Path::new("README.md"))?;
        component_entry(&String::from_utf8_lossy(&readme.1))
    }
}

/// The first fenced ```` ```toml ```` block of `readme` that holds a `[[component]]` table.
pub fn component_entry(readme: &str) -> Option<String> {
    let mut rest = readme;
    while let Some(start) = rest.find("```toml\n") {
        let body = &rest[start + "```toml\n".len()..];
        let end = body.find("\n```")?;
        let block = &body[..end];
        if block.contains("[[component]]") {
            return Some(format!("{block}\n"));
        }
        rest = &body[end..];
    }
    None
}

/// Renders `skeleton` (`crate/<kind>/`) for `component` of `repo`, in memory. Refuses a path that
/// exists and a placeholder the vocabulary does not cover.
pub fn render(
    kind: Kind,
    repo: &Repo,
    component: &str,
    skeleton: &Path,
) -> Result<Rendered, String> {
    let crate_name = format!("{}-{}-{component}", org::CRATE_PREFIX, repo.name);
    let dir = repo.root.join("crates").join(&crate_name);
    if dir.exists() {
        return Err(format!("{} exists", dir.display()));
    }
    let vars = vars(kind, &repo.name, component, &crate_name);
    let mut files = Vec::new();
    let mut survivors = Vec::new();
    for rel in files_under(skeleton)? {
        let path = skeleton.join(&rel);
        let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let bytes = if is_binary(&rel) {
            bytes
        } else {
            let text = String::from_utf8(bytes).map_err(|_| {
                format!(
                    "{}: not UTF-8 text (an unknown binary extension?)",
                    path.display()
                )
            })?;
            let text = normalize(&substitute(&text, &vars));
            if let Some(name) = surviving_placeholder(&text) {
                survivors.push(format!("{{{{{name}}}}} in {}", rel.display()));
            }
            text.into_bytes()
        };
        files.push((rel, bytes));
    }
    if !survivors.is_empty() {
        return Err(format!(
            "the skeleton knows a placeholder this command does not: {} (nothing written)",
            survivors.join(", ")
        ));
    }
    Ok(Rendered {
        crate_name,
        dir,
        files,
    })
}

/// The placeholder vocabulary: the organization's names plus what the invocation determines.
fn vars(kind: Kind, repo: &str, component: &str, crate_name: &str) -> Vec<(&'static str, String)> {
    vec![
        ("ORG", org::ORG.to_owned()),
        ("ORG_DISPLAY_NAME", org::ORG_DISPLAY_NAME.to_owned()),
        ("ORG_URL", org::ORG_URL.to_owned()),
        ("CLI", org::CLI.to_owned()),
        ("CRATE_PREFIX", org::CRATE_PREFIX.to_owned()),
        ("DAEMON_PREFIX", org::DAEMON_PREFIX.to_owned()),
        ("WIT_NAMESPACE", org::WIT_NAMESPACE.to_owned()),
        ("MSRV", org::MSRV.to_owned()),
        ("EDITION", org::EDITION.to_owned()),
        ("LICENSE_SPDX", org::LICENSE_SPDX.to_owned()),
        ("KIND", kind.name().to_owned()),
        ("REPO", repo.to_owned()),
        ("COMPONENT", component.to_owned()),
        ("COMPONENT_IDENT", component.replace('-', "_")),
        ("CRATE", crate_name.to_owned()),
        ("CRATE_IDENT", crate_name.replace('-', "_")),
    ]
}

/// Replaces every `{{NAME}}` of `vars` in `text`, verbatim.
pub fn substitute(text: &str, vars: &[(&str, String)]) -> String {
    let mut out = text.to_owned();
    for (name, value) in vars {
        out = out.replace(&format!("{{{{{name}}}}}"), value);
    }
    out
}

/// Exactly one trailing newline, the way the shell reference (`$(cat)` then `printf '%s\n'`) writes it.
pub fn normalize(text: &str) -> String {
    format!("{}\n", text.trim_end_matches('\n'))
}

/// The first `{{NAME}}` (upper-case letters and underscores) left in `text`.
pub fn surviving_placeholder(text: &str) -> Option<&str> {
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            let name = &after[..end];
            if !name.is_empty() && name.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') {
                return Some(name);
            }
        }
        rest = after;
    }
    None
}

/// Whether a skeleton file is copied byte for byte.
fn is_binary(rel: &Path) -> bool {
    rel.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| BINARY_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

/// Every file under `root` as sorted relative paths, `.DS_Store` left out.
fn files_under(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for entry in entries {
            let path = entry.map_err(|e| format!("{}: {e}", dir.display()))?.path();
            if path.is_dir() {
                walk(&path, root, out)?;
            } else if path.file_name().is_some_and(|name| name != ".DS_Store") {
                let rel = path
                    .strip_prefix(root)
                    .map_err(|e| format!("{}: {e}", path.display()))?;
                out.push(rel.to_path_buf());
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pub-new-unit-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_component_is_lowercase_words_joined_by_hyphens() {
        for good in ["sched", "boot-hello", "x11", "a-b-c"] {
            assert!(component_is_well_formed(good), "{good}");
        }
        for bad in [
            "",
            "Sched",
            "boot_hello",
            "-sched",
            "sched-",
            "a--b",
            "sché",
        ] {
            assert!(!component_is_well_formed(bad), "{bad}");
        }
    }

    #[test]
    fn the_vocabulary_derives_the_identifiers_from_the_names() {
        let vars = vars(
            Kind::Service,
            "kernel",
            "boot-hello",
            "pub-kernel-boot-hello",
        );
        let get = |name: &str| vars.iter().find(|(n, _)| *n == name).unwrap().1.clone();
        assert_eq!(get("KIND"), "service");
        assert_eq!(get("COMPONENT_IDENT"), "boot_hello");
        assert_eq!(get("CRATE_IDENT"), "pub_kernel_boot_hello");
        assert_eq!(get("DAEMON_PREFIX"), "pubd");
        assert_eq!(vars.len(), 16);
    }

    #[test]
    fn substitution_is_verbatim_and_leaves_other_braces_alone() {
        let vars = vars(Kind::Lib, "kernel", "sched", "pub-kernel-sched");
        let text = "{{CRATE}} ${{ github.ref }} {{CRATE_IDENT}}::NAME {{lower}}";
        assert_eq!(
            substitute(text, &vars),
            "pub-kernel-sched ${{ github.ref }} pub_kernel_sched::NAME {{lower}}"
        );
        assert_eq!(surviving_placeholder(&substitute(text, &vars)), None);
        assert_eq!(
            surviving_placeholder("a {{MAINT_TEAM}} b"),
            Some("MAINT_TEAM")
        );
    }

    #[test]
    fn normalization_leaves_exactly_one_trailing_newline() {
        assert_eq!(normalize("x"), "x\n");
        assert_eq!(normalize("x\n"), "x\n");
        assert_eq!(normalize("x\n\n\n"), "x\n");
        assert_eq!(normalize("x\n\ny\n"), "x\n\ny\n");
    }

    #[test]
    fn the_component_entry_is_the_toml_block_with_the_table() {
        let readme = "# c\n\n```sh\ncargo test\n```\n\nEntry:\n\n```toml\n[[component]]\ncrate = \"pub-k-c\"\nkind  = \"lib\"\n```\n";
        assert_eq!(
            component_entry(readme).as_deref(),
            Some("[[component]]\ncrate = \"pub-k-c\"\nkind  = \"lib\"\n")
        );
        assert_eq!(component_entry("```toml\nname = 1\n```\n"), None);
    }

    #[test]
    fn a_repository_is_found_from_a_directory_inside_it() {
        let root = scratch("repo");
        fs::write(root.join("Cargo.toml"), "[workspace]\n").unwrap();
        fs::write(root.join("CATALOG.toml"), "[repo]\nname = \"kernel\"\n").unwrap();
        let inside = root.join("crates/pub-kernel-sched/src");
        fs::create_dir_all(&inside).unwrap();
        let repo = Repo::locate(&inside).unwrap();
        assert_eq!(repo.name, "kernel");
        assert_eq!(repo.root, fs::canonicalize(&root).unwrap());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_catalog_without_a_workspace_beside_it_is_refused() {
        let root = scratch("no-cargo");
        fs::write(root.join("CATALOG.toml"), "[repo]\nname = \"kernel\"\n").unwrap();
        let problem = Repo::locate(&root).unwrap_err();
        assert!(problem.contains("Cargo.toml"), "{problem}");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn binaries_are_told_by_extension() {
        assert!(is_binary(Path::new("brand/logo.PNG")));
        assert!(!is_binary(Path::new("src/lib.rs")));
        assert!(!is_binary(Path::new("Cargo.toml")));
    }
}
