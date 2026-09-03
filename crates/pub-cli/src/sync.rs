//! `pub catalog sync`: the GitHub state every repository should have, derived from the catalog,
//! and the smallest set of writes that gets there. Every write is preceded by a read, so a
//! converged organization sees reads only. The formulas are the bootstrap kit's (steps 05 and
//! 07), so the kit and the CLI agree on what "converged" means.

use std::collections::{BTreeSet, VecDeque};
use std::sync::Mutex;
use std::thread;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::catalog::{Catalog, DOTGITHUB, Meta, Repo};
use crate::gh;

/// GitHub caps repository descriptions at 350 characters.
pub const DESCRIPTION_MAX: usize = 350;
/// Every repository starts in this tier (custom property `tier`).
pub const TIER: &str = "incubating";
/// Every repository starts with this readiness (custom property `readiness`).
pub const READINESS: &str = "none";
/// Labels GitHub creates on a new repository; deleted when the label set does not use them.
pub const GITHUB_DEFAULT_LABELS: [&str; 9] = [
    "bug",
    "documentation",
    "duplicate",
    "enhancement",
    "good first issue",
    "help wanted",
    "invalid",
    "question",
    "wontfix",
];

/// A label, as the label file and GitHub describe it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Label {
    /// Label name.
    pub name: String,
    /// Six hex digits, with or without `#`.
    pub color: String,
    /// Description; GitHub answers `null` for an empty one.
    #[serde(default)]
    pub description: Option<String>,
}

impl Label {
    fn same_as(&self, other: &Label) -> bool {
        self.name == other.name
            && normalize_color(&self.color) == normalize_color(&other.color)
            && self.description.clone().unwrap_or_default()
                == other.description.clone().unwrap_or_default()
    }
}

fn normalize_color(color: &str) -> String {
    color.trim_start_matches('#').to_ascii_lowercase()
}

/// One planned API write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Write {
    /// HTTP method.
    pub method: &'static str,
    /// API path.
    pub path: String,
    /// JSON body, when the method takes one.
    pub body: Option<Value>,
    /// What it changes, for people.
    pub what: String,
}

/// What `sync` does or would do.
#[derive(Debug, Default)]
pub struct Options {
    /// Plan only; print the writes instead of sending them.
    pub dry_run: bool,
    /// Repositories worked on at once.
    pub jobs: usize,
    /// Only these repositories (all when empty).
    pub only: Vec<String>,
    /// The label set to converge, when given.
    pub labels: Option<Vec<Label>>,
}

/// The result for one repository.
#[derive(Debug)]
pub struct Outcome {
    /// Repository name.
    pub name: String,
    /// Writes planned (dry run) or applied.
    pub writes: Vec<Write>,
    /// Why the repository did not converge, when it did not.
    pub error: Option<String>,
}

/// `<first layer> · <purpose> · wave <n>`, cut to [`DESCRIPTION_MAX`] characters.
pub fn description(repo: &Repo) -> String {
    let full = format!(
        "{} · {} · wave {}",
        repo.layers.first().map(String::as_str).unwrap_or(""),
        repo.purpose,
        repo.wave
    );
    full.chars().take(DESCRIPTION_MAX).collect()
}

/// `<site>/<name>`.
pub fn homepage(meta: &Meta, repo: &Repo) -> String {
    format!("{}/{}", meta.site.trim_end_matches('/'), repo.name)
}

/// The organization, `rust`, the ring, the wave and every layer (lowercase), sorted and unique.
pub fn topics(meta: &Meta, repo: &Repo) -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    set.insert(meta.org.clone());
    set.insert("rust".to_string());
    set.insert(format!("ring-{}", repo.ring));
    set.insert(format!("wave-{}", repo.wave));
    for layer in &repo.layers {
        set.insert(format!("layer-{}", layer.to_ascii_lowercase()));
    }
    set.into_iter().collect()
}

/// The custom property values the catalog decides.
pub fn properties(repo: &Repo) -> Vec<Value> {
    vec![
        json!({"property_name": "ring", "value": repo.ring}),
        json!({"property_name": "wave", "value": repo.wave.to_string()}),
        json!({"property_name": "tier", "value": TIER}),
        json!({"property_name": "readiness", "value": READINESS}),
        json!({"property_name": "maintainer_team", "value": format!("maint-{}", repo.name)}),
        json!({"property_name": "layers", "value": repo.layers}),
    ]
}

/// `PATCH /repos/{org}/{name}` when description or homepage differ.
pub fn plan_settings(current: &Value, meta: &Meta, repo: &Repo) -> Option<Write> {
    let want_description = description(repo);
    let want_homepage = homepage(meta, repo);
    let same = current["description"].as_str().unwrap_or("") == want_description
        && current["homepage"].as_str().unwrap_or("") == want_homepage;
    (!same).then(|| Write {
        method: "PATCH",
        path: format!("/repos/{}/{}", meta.org, repo.name),
        body: Some(json!({"description": want_description, "homepage": want_homepage})),
        what: "description and homepage".to_string(),
    })
}

/// `PUT /repos/{org}/{name}/topics` when the topic set differs.
pub fn plan_topics(current: &Value, meta: &Meta, repo: &Repo) -> Option<Write> {
    let have: BTreeSet<String> = current["topics"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let want = topics(meta, repo);
    let same = have.iter().eq(want.iter());
    (!same).then(|| Write {
        method: "PUT",
        path: format!("/repos/{}/{}/topics", meta.org, repo.name),
        body: Some(json!({"names": want})),
        what: "topics".to_string(),
    })
}

/// True when every wanted property holds its value; arrays compare as sets (GitHub stores
/// multi_select values sorted).
pub fn properties_satisfied(current: &[Value], wanted: &[Value]) -> bool {
    wanted.iter().all(|w| {
        current.iter().any(|c| {
            c["property_name"] == w["property_name"] && same_value(&c["value"], &w["value"])
        })
    })
}

fn same_value(a: &Value, b: &Value) -> bool {
    match (a.as_array(), b.as_array()) {
        (Some(x), Some(y)) => {
            let sx: BTreeSet<String> = x.iter().map(Value::to_string).collect();
            let sy: BTreeSet<String> = y.iter().map(Value::to_string).collect();
            sx == sy
        }
        _ => a == b,
    }
}

/// `PATCH /repos/{org}/{name}/properties/values` when a property is missing or differs.
pub fn plan_properties(current: &[Value], meta: &Meta, repo: &Repo) -> Option<Write> {
    let want = properties(repo);
    (!properties_satisfied(current, &want)).then(|| Write {
        method: "PATCH",
        path: format!("/repos/{}/{}/properties/values", meta.org, repo.name),
        body: Some(json!({"properties": want})),
        what: "custom properties".to_string(),
    })
}

/// Deletes GitHub defaults the set does not use; creates or updates labels that are missing or
/// differ in colour or description. Labels the set does not know are left alone.
pub fn plan_labels(current: &[Label], wanted: &[Label], meta: &Meta, repo: &Repo) -> Vec<Write> {
    let base = format!("/repos/{}/{}/labels", meta.org, repo.name);
    let mut writes = Vec::new();
    for label in current {
        let is_default = GITHUB_DEFAULT_LABELS.contains(&label.name.as_str());
        if is_default && !wanted.iter().any(|w| w.name == label.name) {
            writes.push(Write {
                method: "DELETE",
                path: format!("{base}/{}", gh::segment(&label.name)),
                body: None,
                what: format!("delete label {}", label.name),
            });
        }
    }
    for want in wanted {
        match current.iter().find(|c| c.name == want.name) {
            Some(have) if have.same_as(want) => {}
            Some(_) => writes.push(Write {
                method: "PATCH",
                path: format!("{base}/{}", gh::segment(&want.name)),
                body: Some(label_body(want)),
                what: format!("update label {}", want.name),
            }),
            None => writes.push(Write {
                method: "POST",
                path: base.clone(),
                body: Some(label_body(want)),
                what: format!("create label {}", want.name),
            }),
        }
    }
    writes
}

fn label_body(label: &Label) -> Value {
    json!({
        "name": label.name,
        "color": normalize_color(&label.color),
        "description": label.description.clone().unwrap_or_default(),
    })
}

/// Read, plan and (unless `dry_run`) apply for one repository.
pub fn sync_repo(meta: &Meta, repo: &Repo, labels: Option<&[Label]>, dry_run: bool) -> Outcome {
    let name = repo.name.clone();
    match plan_repo(meta, repo, labels) {
        Err(error) => Outcome {
            name,
            writes: Vec::new(),
            error: Some(error),
        },
        Ok(writes) => {
            let mut error = None;
            if !dry_run {
                for write in &writes {
                    if let Err(e) = gh::api(write.method, &write.path, write.body.as_ref()) {
                        error = Some(e);
                        break;
                    }
                }
            }
            Outcome {
                name,
                writes,
                error,
            }
        }
    }
}

fn plan_repo(meta: &Meta, repo: &Repo, labels: Option<&[Label]>) -> Result<Vec<Write>, String> {
    let base = format!("/repos/{}/{}", meta.org, repo.name);
    let current = gh::get(&base)?.ok_or_else(|| {
        "does not exist on GitHub — the bootstrap kit creates repositories".to_string()
    })?;
    let mut writes = Vec::new();
    writes.extend(plan_settings(&current, meta, repo));
    writes.extend(plan_topics(&current, meta, repo));
    let props = gh::get(&format!("{base}/properties/values"))?
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default();
    writes.extend(plan_properties(&props, meta, repo));
    if let Some(wanted) = labels {
        let have: Vec<Label> = match gh::get(&format!("{base}/labels?per_page=100"))? {
            Some(value) => serde_json::from_value(value)
                .map_err(|e| format!("labels of {}: {e}", repo.name))?,
            None => Vec::new(),
        };
        writes.extend(plan_labels(&have, wanted, meta, repo));
    }
    Ok(writes)
}

/// Every catalog repository except `.github`, `jobs` at a time, results in catalog order.
pub fn run(cat: &Catalog, opts: &Options) -> Vec<Outcome> {
    let targets: Vec<&Repo> = cat
        .repo
        .iter()
        .filter(|r| r.name != DOTGITHUB && (opts.only.is_empty() || opts.only.contains(&r.name)))
        .collect();
    let queue: Mutex<VecDeque<(usize, &Repo)>> =
        Mutex::new(targets.iter().copied().enumerate().collect());
    let results: Mutex<Vec<(usize, Outcome)>> = Mutex::new(Vec::with_capacity(targets.len()));
    let workers = opts.jobs.clamp(1, targets.len().max(1));
    thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let next = queue.lock().map(|mut q| q.pop_front()).unwrap_or(None);
                    let Some((i, repo)) = next else { break };
                    let outcome =
                        sync_repo(&cat.catalog, repo, opts.labels.as_deref(), opts.dry_run);
                    if let Ok(mut r) = results.lock() {
                        r.push((i, outcome));
                    }
                }
            });
        }
    });
    let mut results = results.into_inner().unwrap_or_default();
    results.sort_by_key(|(i, _)| *i);
    results.into_iter().map(|(_, o)| o).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::FIXTURE;

    fn fixture() -> Catalog {
        Catalog::parse(FIXTURE).unwrap()
    }

    fn kernel(cat: &Catalog) -> &Repo {
        cat.repo.iter().find(|r| r.name == "kernel").unwrap()
    }

    fn label(name: &str, color: &str, description: &str) -> Label {
        Label {
            name: name.into(),
            color: color.into(),
            description: Some(description.into()),
        }
    }

    #[test]
    fn description_follows_the_kit_formula_and_is_cut_at_350_chars() {
        let cat = fixture();
        assert_eq!(
            description(kernel(&cat)),
            "L3 · Kernel hardening, scheduler, driver ABI. · wave 1"
        );
        let mut long = kernel(&cat).clone();
        long.purpose = "é".repeat(400);
        assert_eq!(description(&long).chars().count(), DESCRIPTION_MAX);
    }

    #[test]
    fn homepage_and_topics() {
        let cat = fixture();
        assert_eq!(
            homepage(&cat.catalog, kernel(&cat)),
            "https://publicsoftware.dev/kernel"
        );
        assert_eq!(
            topics(&cat.catalog, kernel(&cat)),
            [
                "layer-l3",
                "layer-l4",
                "public-software",
                "ring-system",
                "rust",
                "wave-1"
            ]
        );
    }

    #[test]
    fn settings_plan_only_when_they_differ() {
        let cat = fixture();
        let repo = kernel(&cat);
        let converged = json!({"description": description(repo), "homepage": "https://publicsoftware.dev/kernel"});
        assert!(plan_settings(&converged, &cat.catalog, repo).is_none());
        let fresh = json!({"description": null, "homepage": null});
        let write = plan_settings(&fresh, &cat.catalog, repo).unwrap();
        assert_eq!(
            (write.method, write.path.as_str()),
            ("PATCH", "/repos/public-software/kernel")
        );
    }

    #[test]
    fn topics_plan_ignores_order_and_notices_a_missing_topic() {
        let cat = fixture();
        let repo = kernel(&cat);
        let shuffled = json!({"topics": ["wave-1", "rust", "ring-system", "public-software", "layer-l4", "layer-l3"]});
        assert!(plan_topics(&shuffled, &cat.catalog, repo).is_none());
        let short = json!({"topics": ["rust"]});
        assert_eq!(
            plan_topics(&short, &cat.catalog, repo).unwrap().method,
            "PUT"
        );
    }

    #[test]
    fn properties_compare_arrays_as_sets_like_github_stores_them() {
        let cat = fixture();
        let repo = kernel(&cat);
        let mut github = properties(repo);
        github[5] = json!({"property_name": "layers", "value": ["L4", "L3"]});
        assert!(plan_properties(&github, &cat.catalog, repo).is_none());
        github[5] = json!({"property_name": "layers", "value": ["L3"]});
        assert!(plan_properties(&github, &cat.catalog, repo).is_some());
        assert!(plan_properties(&[], &cat.catalog, repo).is_some());
    }

    #[test]
    fn label_plan_matches_the_kit() {
        let cat = fixture();
        let repo = kernel(&cat);
        let current = vec![
            label("bug", "d73a4a", ""),
            label("kind/bug", "4a5563", "Something is wrong"),
            label("stale", "000000", "old"),
        ];
        let wanted = vec![
            label("kind/bug", "4A5563", "Something is wrong"),
            label("stale", "#FFFFFF", "old"),
            label("new", "111111", "n"),
        ];
        let plan = plan_labels(&current, &wanted, &cat.catalog, repo);
        let whats: Vec<&str> = plan.iter().map(|w| w.what.as_str()).collect();
        assert_eq!(
            whats,
            ["delete label bug", "update label stale", "create label new"]
        );
        assert_eq!(plan[0].path, "/repos/public-software/kernel/labels/bug");
        assert_eq!(plan[1].method, "PATCH");
        assert_eq!(plan[2].body.as_ref().unwrap()["color"], "111111");
    }

    #[test]
    fn null_descriptions_from_github_equal_empty_ones() {
        let github = Label {
            name: "x".into(),
            color: "AAAAAA".into(),
            description: None,
        };
        let file = label("x", "#aaaaaa", "");
        assert!(github.same_as(&file));
    }
}
