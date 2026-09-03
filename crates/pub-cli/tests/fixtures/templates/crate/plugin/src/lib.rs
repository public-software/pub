//! `{{CRATE}}` — the `{{COMPONENT}}` plugin of [`{{REPO}}`](https://github.com/{{ORG}}/{{REPO}}).
//!
//! A plugin is a WebAssembly component implementing the `component` world of `{{WIT_NAMESPACE}}:core`
//! (the `interfaces` repository); the host is the `plugin-runtime` repository. Until the
//! wasm32-wasip2 build and the generated bindings land there, this crate compiles on the host as a
//! cdylib + rlib and exposes its [`manifest`], which is what the host reads first.
//!
//! ```
//! assert_eq!({{CRATE_IDENT}}::manifest().world, "{{WIT_NAMESPACE}}:core/component@0.1.0");
//! ```

#![forbid(unsafe_code)]

/// The world this plugin implements, as `{{WIT_NAMESPACE}}:core` names it.
pub const WORLD: &str = "{{WIT_NAMESPACE}}:core/component@0.1.0";

/// What the host reads before it instantiates the plugin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// The plugin's name: the crate name.
    pub name: &'static str,
    /// The plugin's version: the crate version.
    pub version: &'static str,
    /// The WIT world the plugin implements.
    pub world: &'static str,
}

/// The plugin's manifest.
pub fn manifest() -> Manifest {
    Manifest {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        world: WORLD,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_names_the_crate_and_the_world() {
        let manifest = manifest();
        assert_eq!(manifest.name, "{{CRATE}}");
        assert_eq!(manifest.world, WORLD);
    }

    #[test]
    fn the_world_is_a_versioned_interface_of_the_org_namespace() {
        let (package, version) = WORLD.split_once('@').expect("a versioned world");
        assert!(package.starts_with("{{WIT_NAMESPACE}}:core/"), "{package}");
        assert_eq!(version.split('.').count(), 3, "{version}");
    }
}
