//! `pub new` as a contributor runs it: the built binary, a fixture copy of the crate skeletons
//! (`tests/fixtures/templates/crate/<kind>`, placeholders intact) and throwaway repositories.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

/// The five crate kinds the skeleton set carries.
const KINDS: [&str; 5] = ["lib", "app", "service", "plugin", "spec"];

/// The fixture templates checkout: `crate/<kind>/` under it.
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/templates")
}

/// A fresh directory under the system temp directory, removed on drop.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("pub-cli-{tag}-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).expect("a scratch directory");
        Scratch { dir }
    }

    /// A repository named `kernel`: a Cargo workspace with a `CATALOG.toml` `[repo]` table.
    fn repo() -> Self {
        let scratch = Scratch::new("repo");
        write(
            &scratch.dir.join("Cargo.toml"),
            "[workspace]\nresolver = \"3\"\nmembers = [\"crates/*\"]\n",
        );
        write(
            &scratch.dir.join("CATALOG.toml"),
            "[repo]\nname = \"kernel\"\nring = \"system\"\n",
        );
        scratch
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.join(rel)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn write(path: &Path, content: impl AsRef<[u8]>) {
    fs::create_dir_all(path.parent().expect("a parent")).expect("the parent directory");
    fs::write(path, content).expect("the file is written");
}

/// `pub new <args…> --dir <repo> --templates <templates>`.
fn pub_new(repo: &Scratch, templates: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pub"))
        .arg("new")
        .args(args)
        .arg("--dir")
        .arg(&repo.dir)
        .arg("--templates")
        .arg(templates)
        .output()
        .expect("the binary runs")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Every file under `root`, as sorted paths relative to it.
fn files_under(root: &Path) -> Vec<String> {
    fn walk(dir: &Path, root: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).expect("a readable directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                walk(&path, root, out);
            } else {
                let rel = path.strip_prefix(root).expect("under the root");
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// The first `{{NAME}}` (upper-case letters and underscores) left in `text`, if any.
fn surviving_placeholder(text: &str) -> Option<&str> {
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("}}") {
            let name = &after[..end];
            if !name.is_empty() && name.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
                return Some(name);
            }
        }
        rest = after;
    }
    None
}

#[test]
fn lib_renders_the_crate_and_prints_its_component_entry() {
    let repo = Scratch::repo();
    let out = pub_new(&repo, &fixtures(), &["lib", "sched"]);
    assert!(out.status.success(), "{out:?}");

    let manifest = fs::read_to_string(repo.path("crates/pub-kernel-sched/Cargo.toml")).unwrap();
    assert!(
        manifest.contains("name = \"pub-kernel-sched\""),
        "{manifest}"
    );
    let lib = fs::read_to_string(repo.path("crates/pub-kernel-sched/src/lib.rs")).unwrap();
    assert!(lib.contains("pub_kernel_sched::NAME"), "{lib}");
    assert!(lib.contains("github.com/public-software/kernel"), "{lib}");

    let printed = stdout(&out);
    assert!(printed.contains("[[component]]"), "{printed}");
    assert!(
        printed.contains("crate     = \"pub-kernel-sched\""),
        "{printed}"
    );
    assert!(printed.contains("kind      = \"lib\""), "{printed}");
    assert!(printed.contains("crates/pub-kernel-sched"), "{printed}");
}

#[test]
fn every_kind_renders_the_skeleton_file_set_and_leaves_no_placeholder() {
    let repo = Scratch::repo();
    for kind in KINDS {
        let component = format!("x-{kind}");
        let out = pub_new(&repo, &fixtures(), &[kind, &component]);
        assert!(out.status.success(), "{kind}: {out:?}");
        let rendered = repo.path(&format!("crates/pub-kernel-{component}"));
        assert_eq!(
            files_under(&rendered),
            files_under(&fixtures().join("crate").join(kind)),
            "{kind}: the rendered tree mirrors the skeleton"
        );
        for rel in files_under(&rendered) {
            let text = fs::read_to_string(rendered.join(&rel)).unwrap();
            assert_eq!(
                surviving_placeholder(&text),
                None,
                "{kind}/{rel} keeps a placeholder"
            );
            assert!(text.ends_with('\n'), "{kind}/{rel} ends with a newline");
            assert!(
                !text.ends_with("\n\n"),
                "{kind}/{rel} ends with one newline"
            );
        }
    }
}

#[test]
fn app_names_the_binary_after_the_component_and_service_after_the_daemon_prefix() {
    let repo = Scratch::repo();
    assert!(
        pub_new(&repo, &fixtures(), &["app", "ctl"])
            .status
            .success()
    );
    assert!(
        pub_new(&repo, &fixtures(), &["service", "sched"])
            .status
            .success()
    );
    let app = fs::read_to_string(repo.path("crates/pub-kernel-ctl/Cargo.toml")).unwrap();
    assert!(app.contains("name = \"ctl\""), "{app}");
    let service = fs::read_to_string(repo.path("crates/pub-kernel-sched/Cargo.toml")).unwrap();
    assert!(service.contains("name = \"pubd-sched\""), "{service}");
    let main = fs::read_to_string(repo.path("crates/pub-kernel-sched/src/main.rs")).unwrap();
    assert!(main.contains("use pub_kernel_sched::Service;"), "{main}");
}

#[test]
fn an_existing_path_is_refused_and_left_alone() {
    let repo = Scratch::repo();
    let marker = repo.path("crates/pub-kernel-sched/KEEP");
    write(&marker, "mine\n");
    let out = pub_new(&repo, &fixtures(), &["lib", "sched"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(stderr(&out).contains("exists"), "{}", stderr(&out));
    assert!(marker.exists(), "the marker survives");
    assert!(
        !repo.path("crates/pub-kernel-sched/Cargo.toml").exists(),
        "nothing was written next to it"
    );
}

#[test]
fn a_component_with_an_uppercase_letter_is_refused() {
    let repo = Scratch::repo();
    let out = pub_new(&repo, &fixtures(), &["lib", "Sched"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(stderr(&out).contains("lowercase"), "{}", stderr(&out));
    assert!(!repo.path("crates").join("pub-kernel-Sched").exists());
}

#[test]
fn an_unknown_kind_is_refused_naming_the_five() {
    let repo = Scratch::repo();
    let out = pub_new(&repo, &fixtures(), &["widget", "sched"]);
    assert!(!out.status.success(), "{out:?}");
    let err = stderr(&out);
    for kind in KINDS {
        assert!(err.contains(kind), "{err}");
    }
}

#[test]
fn outside_a_repository_it_says_what_is_missing() {
    let bare = Scratch::new("bare");
    let out = pub_new(&bare, &fixtures(), &["lib", "sched"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(stderr(&out).contains("CATALOG.toml"), "{}", stderr(&out));
}

#[test]
fn text_is_normalized_to_one_trailing_newline_and_binaries_are_copied_byte_for_byte() {
    let templates = Scratch::new("templates");
    let kind = templates.path("crate/lib");
    write(&kind.join("README.md"), "# {{CRATE}}\n\n\n");
    write(&kind.join("src/lib.rs"), "//! {{CRATE_IDENT}} of {{REPO}}");
    write(&kind.join("logo.png"), b"\x89PNG {{CRATE}}\n\n");
    write(&kind.join(".DS_Store"), "finder noise");
    let repo = Scratch::repo();
    let out = pub_new(&repo, &templates.dir, &["lib", "sched"]);
    assert!(out.status.success(), "{out:?}");
    let rendered = repo.path("crates/pub-kernel-sched");
    assert_eq!(
        fs::read_to_string(rendered.join("README.md")).unwrap(),
        "# pub-kernel-sched\n"
    );
    assert_eq!(
        fs::read_to_string(rendered.join("src/lib.rs")).unwrap(),
        "//! pub_kernel_sched of kernel\n"
    );
    assert_eq!(
        fs::read(rendered.join("logo.png")).unwrap(),
        b"\x89PNG {{CRATE}}\n\n"
    );
    assert_eq!(
        files_under(&rendered),
        vec!["README.md", "logo.png", "src/lib.rs"],
        ".DS_Store is skipped"
    );
}

#[test]
fn a_placeholder_the_cli_does_not_know_is_refused_before_anything_is_written() {
    let templates = Scratch::new("templates");
    write(
        &templates.path("crate/lib/Cargo.toml"),
        "name = \"{{CRATE}}\"\nowner = \"{{MAINT_TEAM}}\"\n",
    );
    let repo = Scratch::repo();
    let out = pub_new(&repo, &templates.dir, &["lib", "sched"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(stderr(&out).contains("MAINT_TEAM"), "{}", stderr(&out));
    assert!(!repo.path("crates/pub-kernel-sched").exists());
}

#[test]
fn a_templates_path_without_the_kind_is_refused_naming_the_path() {
    let templates = Scratch::new("templates");
    let repo = Scratch::repo();
    let out = pub_new(&repo, &templates.dir, &["lib", "sched"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(stderr(&out).contains("crate/lib"), "{}", stderr(&out));
}
