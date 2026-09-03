//! Views of a valid catalog.

use crate::catalog::{Catalog, RINGS, Repo};

/// The heading each ring's table carries in the organization README.
pub const RING_TITLES: [(&str, &str); 5] = [
    (
        "spine",
        "Spine — defines, assembles and documents everything else",
    ),
    (
        "platform",
        "Platform ring — the only cross-ring dependencies",
    ),
    (
        "system",
        "System ring — toolchain, silicon, kernel, base, infrastructure, media, platform shells",
    ),
    ("domain", "Domain ring — the products"),
    ("standards", "Specs & content"),
];

/// The ring tables the organization README embeds between its catalog markers: one `###`
/// section per ring in [`RINGS`] order, rows in catalog order.
pub fn readme(cat: &Catalog) -> String {
    let mut out = String::new();
    for ring in RINGS {
        let title = RING_TITLES
            .iter()
            .find(|(r, _)| *r == ring)
            .map(|(_, t)| *t)
            .unwrap_or(ring);
        out.push_str(&format!(
            "\n### {title}\n\n| Repository | Purpose | Layers | Starts |\n|---|---|---|---|\n"
        ));
        for repo in cat.in_ring(ring) {
            out.push_str(&row(&cat.catalog.org, repo));
            out.push('\n');
        }
    }
    out
}

fn row(org: &str, repo: &Repo) -> String {
    format!(
        "| [{name}](https://github.com/{org}/{name}) | {purpose} | {layers} | wave {wave} |",
        name = repo.name,
        purpose = repo.purpose,
        layers = repo.layers.join(", "),
        wave = repo.wave,
    )
}

/// The repositories as a JSON array, fields in catalog order (name, ring, purpose, contents, layers, wave).
pub fn json(cat: &Catalog) -> Result<String, String> {
    serde_json::to_string(&cat.repo).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::FIXTURE;

    #[test]
    fn readme_tables_match_the_kit_renderer_shape() {
        let cat = Catalog::parse(FIXTURE).unwrap();
        let text = readme(&cat);
        let expected_spine = "\n### Spine — defines, assembles and documents everything else\n\n\
| Repository | Purpose | Layers | Starts |\n|---|---|---|---|\n\
| [catalog](https://github.com/public-software/catalog) | Machine-readable ledger. | L18 | wave 1 |\n\
| [.github](https://github.com/public-software/.github) | Org profile. | all | wave 1 |\n\
\n### Platform ring — the only cross-ring dependencies\n\n| Repository | Purpose | Layers | Starts |\n|---|---|---|---|\n";
        assert!(text.starts_with(expected_spine), "{text}");
        assert!(text.contains("| [kernel](https://github.com/public-software/kernel) | Kernel hardening, scheduler, driver ABI. | L3, L4 | wave 1 |\n"));
        assert!(text.ends_with("\n### Specs & content\n\n| Repository | Purpose | Layers | Starts |\n|---|---|---|---|\n"));
        assert_eq!(text.matches("\n### ").count(), RINGS.len());
    }

    #[test]
    fn json_keeps_field_order() {
        let cat = Catalog::parse(FIXTURE).unwrap();
        let text = json(&cat).unwrap();
        assert!(text.starts_with(r#"[{"name":"catalog","ring":"spine","purpose":"Machine-readable ledger.","contents":"catalog.toml · schema","layers":["L18"],"wave":1}"#), "{text}");
    }
}
