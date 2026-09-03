//! `{{CRATE}}` — the `{{COMPONENT}}` library of [`{{REPO}}`](https://github.com/{{ORG}}/{{REPO}}).
//!
//! Replace this paragraph with what the crate is for and the specification it implements
//! (listed in the repository's `PROVENANCE.md`).
//!
//! ```
//! assert_eq!({{CRATE_IDENT}}::NAME, "{{CRATE}}");
//! ```

#![forbid(unsafe_code)]

/// The crate's name, as `CATALOG.toml` and crates.io know it.
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// The crate's version, as Cargo knows it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_follows_the_naming_rule() {
        assert_eq!(NAME, "{{CRATE}}");
        assert!(NAME.starts_with("{{CRATE_PREFIX}}-{{REPO}}-"));
    }

    #[test]
    fn version_is_semver_shaped() {
        assert_eq!(VERSION.split('.').count(), 3, "{VERSION}");
    }
}
