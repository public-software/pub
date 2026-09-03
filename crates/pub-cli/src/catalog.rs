//! The catalog model: `catalog.toml` as data, plus the rules its JSON Schema states.
//!
//! The rules are spelled out here rather than read from the schema so the binary has no
//! schema dependency; `catalog.schema.json` in the catalog repository is the reference and
//! the two are kept in step by the bootstrap kit's tests.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The dependency rings, outermost last.
pub const RINGS: [&str; 5] = ["spine", "platform", "system", "domain", "standards"];
/// Highest stack layer; layers are `L0`..`L18`, or `all` for the spine.
pub const TOP_LAYER: u32 = 18;
/// Where `catalog.toml` is looked for when no path is given, in order.
pub const SEARCH_PATHS: [&str; 3] = [
    "catalog/catalog.toml",
    "catalog.toml",
    "config/catalog.toml",
];
/// The only catalog format version this binary understands.
pub const VERSION: u32 = 1;
/// The organization's community-health repository; the one name outside the pattern.
pub const DOTGITHUB: &str = ".github";

/// The whole file.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    /// The `[catalog]` table.
    pub catalog: Meta,
    /// Every `[[repo]]`, in file order.
    pub repo: Vec<Repo>,
}

/// The `[catalog]` table.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Meta {
    /// Format version; see [`VERSION`].
    pub version: u32,
    /// The GitHub organization every repository belongs to.
    pub org: String,
}

/// One repository. Field order is the order of the JSON view.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Repo {
    /// GitHub repository name.
    pub name: String,
    /// Dependency ring; one of [`RINGS`].
    pub ring: String,
    /// One sentence: what the repository is for.
    pub purpose: String,
    /// Planned crates, apps, services or documents, separated by `·`.
    pub contents: String,
    /// Stack layers served: `L0`..`L18`, or `all`.
    pub layers: Vec<String>,
    /// The wave in which work starts; 1 or more.
    pub wave: u32,
}

/// The catalog file to use: the explicit path, else the first of [`SEARCH_PATHS`] that exists.
pub fn locate(explicit: Option<&Path>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return Ok(path.to_path_buf());
    }
    SEARCH_PATHS
        .iter()
        .map(PathBuf::from)
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| {
            format!(
                "no catalog found: pass --catalog, or run where one of {} exists",
                SEARCH_PATHS.join(", ")
            )
        })
}

impl Catalog {
    /// Read and parse `path`; a parse error names the file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        Self::parse(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// Parse catalog TOML.
    pub fn parse(text: &str) -> Result<Self, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }

    /// Every problem the rules find, as `where: problem` lines; empty when the catalog is valid.
    pub fn validate(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if self.catalog.version != VERSION {
            problems.push(format!(
                "catalog.catalog.version: {} is not {VERSION}",
                self.catalog.version
            ));
        }
        if !is_login(&self.catalog.org) {
            problems.push(format!(
                "catalog.catalog.org: {:?} is not a GitHub login",
                self.catalog.org
            ));
        }
        if self.repo.is_empty() {
            problems.push("catalog.repo: fewer than 1 items".to_string());
        }
        for (i, repo) in self.repo.iter().enumerate() {
            repo.check(&format!("catalog.repo[{i}]"), &mut problems);
            if let Some(first) = self.repo[..i]
                .iter()
                .position(|earlier| earlier.name == repo.name)
            {
                problems.push(format!(
                    "catalog.repo[{i}].name: {:?} is already repo[{first}]",
                    repo.name
                ));
            }
        }
        problems
    }

    /// Repositories in `ring`, in file order.
    pub fn in_ring<'a>(&'a self, ring: &'a str) -> impl Iterator<Item = &'a Repo> + 'a {
        self.repo.iter().filter(move |repo| repo.ring == ring)
    }
}

impl Repo {
    fn check(&self, at: &str, problems: &mut Vec<String>) {
        if !is_repo_name(&self.name) {
            problems.push(format!(
                "{at}.name: {:?} is not a repository name (lowercase, digits, hyphens)",
                self.name
            ));
        }
        if !RINGS.contains(&self.ring.as_str()) {
            problems.push(format!(
                "{at}.ring: {:?} is not one of {RINGS:?}",
                self.ring
            ));
        }
        if self.wave < 1 {
            problems.push(format!("{at}.wave: {} is below 1", self.wave));
        }
        if self.layers.is_empty() {
            problems.push(format!("{at}.layers: fewer than 1 items"));
        }
        for (j, layer) in self.layers.iter().enumerate() {
            if !is_layer(layer) {
                problems.push(format!(
                    "{at}.layers[{j}]: {layer:?} is not L0..L{TOP_LAYER} or \"all\""
                ));
            }
            if self.layers[..j].contains(layer) {
                problems.push(format!("{at}.layers: repeated items"));
            }
        }
        if self.purpose.is_empty() {
            problems.push(format!("{at}.purpose: shorter than 1"));
        }
        if self.contents.is_empty() {
            problems.push(format!("{at}.contents: shorter than 1"));
        }
    }
}

/// `.github`, or `[a-z0-9][a-z0-9-]{0,99}`.
pub fn is_repo_name(name: &str) -> bool {
    if name == DOTGITHUB {
        return true;
    }
    let bytes = name.as_bytes();
    let Some((first, rest)) = bytes.split_first() else {
        return false;
    };
    let plain = |b: &u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    bytes.len() <= 100 && plain(first) && rest.iter().all(|b| plain(b) || *b == b'-')
}

/// A GitHub login: `[a-z0-9]([a-z0-9-]{0,37}[a-z0-9])?`.
pub fn is_login(login: &str) -> bool {
    let bytes = login.as_bytes();
    let edge_ok = |b: &u8| b.is_ascii_lowercase() || b.is_ascii_digit();
    match bytes {
        [] => false,
        [only] => edge_ok(only),
        [first, middle @ .., last] => {
            bytes.len() <= 39
                && edge_ok(first)
                && edge_ok(last)
                && middle.iter().all(|b| edge_ok(b) || *b == b'-')
        }
    }
}

/// `all`, or `L<n>` with `0 <= n <= 18` and no leading zero.
pub fn is_layer(layer: &str) -> bool {
    if layer == "all" {
        return true;
    }
    let Some(number) = layer.strip_prefix('L') else {
        return false;
    };
    if number.len() > 1 && number.starts_with('0') {
        return false;
    }
    number.parse::<u32>().is_ok_and(|n| n <= TOP_LAYER)
}

/// A small valid catalog for tests across modules.
#[cfg(test)]
pub(crate) const FIXTURE: &str = r#"
[catalog]
version = 1
org     = "public-software"

[[repo]]
name     = "catalog"
ring     = "spine"
wave     = 1
layers   = ["L18"]
purpose  = "Machine-readable ledger."
contents = "catalog.toml · schema"

[[repo]]
name     = "kernel"
ring     = "system"
wave     = 1
layers   = ["L3", "L4"]
purpose  = "Kernel hardening, scheduler, driver ABI."
contents = "pub-kernel · pub-libc"

[[repo]]
name     = ".github"
ring     = "spine"
wave     = 1
layers   = ["all"]
purpose  = "Org profile."
contents = "profile/README"
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_is_valid() {
        let cat = Catalog::parse(FIXTURE).unwrap();
        assert_eq!(cat.repo.len(), 3);
        assert_eq!(cat.catalog.org, "public-software");
        assert!(cat.validate().is_empty(), "{:?}", cat.validate());
    }

    #[test]
    fn wrong_ring_names_the_field() {
        let cat = Catalog::parse(&FIXTURE.replace("ring     = \"system\"", "ring     = \"sysem\""))
            .unwrap();
        let problems = cat.validate();
        assert_eq!(problems.len(), 1);
        assert!(
            problems[0].starts_with("catalog.repo[1].ring:"),
            "{problems:?}"
        );
    }

    #[test]
    fn duplicate_name_is_reported_once_with_both_positions() {
        let cat =
            Catalog::parse(&FIXTURE.replace("name     = \"kernel\"", "name     = \"catalog\""))
                .unwrap();
        let problems = cat.validate();
        assert_eq!(
            problems,
            vec!["catalog.repo[1].name: \"catalog\" is already repo[0]".to_string()]
        );
    }

    #[test]
    fn wave_zero_and_bad_layers_fail() {
        let text = FIXTURE.replace(
            "wave     = 1\nlayers   = [\"L3\", \"L4\"]",
            "wave     = 0\nlayers   = [\"L3\", \"L3\", \"L19\"]",
        );
        let problems = Catalog::parse(&text).unwrap().validate();
        assert!(
            problems
                .iter()
                .any(|p| p == "catalog.repo[1].wave: 0 is below 1"),
            "{problems:?}"
        );
        assert!(
            problems
                .iter()
                .any(|p| p == "catalog.repo[1].layers: repeated items"),
            "{problems:?}"
        );
        assert!(
            problems
                .iter()
                .any(|p| p.starts_with("catalog.repo[1].layers[2]:")),
            "{problems:?}"
        );
    }

    #[test]
    fn unknown_field_and_missing_field_are_parse_errors() {
        assert!(
            Catalog::parse(&FIXTURE.replace(
                "wave     = 1\nlayers   = [\"L18\"]",
                "wave     = 1\ntier = \"x\"\nlayers   = [\"L18\"]"
            ))
            .is_err()
        );
        assert!(Catalog::parse(&FIXTURE.replace("purpose  = \"Org profile.\"\n", "")).is_err());
    }

    #[test]
    fn names_layers_logins() {
        assert!(is_repo_name("3d") && is_repo_name("design-system") && is_repo_name(".github"));
        assert!(
            !is_repo_name("Kernel")
                && !is_repo_name("-x")
                && !is_repo_name("")
                && !is_repo_name("a b")
        );
        assert!(is_layer("L0") && is_layer("L18") && is_layer("all"));
        assert!(!is_layer("L19") && !is_layer("L01") && !is_layer("l3") && !is_layer("L"));
        assert!(is_login("public-software") && is_login("a") && is_login("a1"));
        assert!(!is_login("-a") && !is_login("a-") && !is_login("") && !is_login("Public"));
    }

    #[test]
    fn locate_prefers_the_explicit_path() {
        let explicit = Path::new("/nowhere/catalog.toml");
        assert_eq!(locate(Some(explicit)).unwrap(), explicit);
    }
}
