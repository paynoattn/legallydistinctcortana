//! Checks `cargo metadata` against the dependency rules in `arch.toml`.
//!
//! - Every workspace crate must be declared, and every declared crate must exist.
//! - A crate may depend (normal, dev, or build) only on the workspace crates in its `allow`.
//! - No crate may depend on a crate listed in `no_dependents`.
//! - A crate may not depend on a crate in its `ban`, directly or through normal/build
//!   dependencies. Transitive checks use Cargo's workspace-wide feature resolution, so
//!   they can over-report but never miss.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::Outcome;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rules {
    #[serde(default)]
    no_dependents: Vec<String>,
    crates: BTreeMap<String, CrateRule>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CrateRule {
    #[serde(default)]
    allow: Vec<String>,
    #[serde(default)]
    ban: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    resolve: Resolve,
}

#[derive(Debug, Deserialize)]
struct Package {
    id: String,
    name: String,
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct Dependency {
    name: String,
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Debug, Deserialize)]
struct Node {
    id: String,
    deps: Vec<NodeDep>,
}

#[derive(Debug, Deserialize)]
struct NodeDep {
    pkg: String,
    dep_kinds: Vec<DepKind>,
}

#[derive(Debug, Deserialize)]
struct DepKind {
    kind: Option<String>,
}

pub(crate) fn run(root: &Path, rules_path: &Path) -> Outcome {
    let rules_text =
        fs::read_to_string(rules_path).map_err(|e| format!("{}: {e}", rules_path.display()))?;
    let rules: Rules =
        toml::from_str(&rules_text).map_err(|e| format!("{}: {e}", rules_path.display()))?;
    let metadata = cargo_metadata(root)?;
    let violations = check(&metadata, &rules);
    if violations.is_empty() {
        print_graph(&metadata);
        println!("arch: ok, no dependency rule violations");
        return Ok(());
    }
    for violation in &violations {
        println!("  {violation}");
    }
    Err(format!(
        "arch: FAILED with {} violation(s). Fix the dependency (invert it behind a port in \
         tasks-application, or move the code), not the rules file. See the architecture-guard skill.",
        violations.len()
    ))
}

fn cargo_metadata(root: &Path) -> Result<Metadata, String> {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot run cargo metadata: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    serde_json::from_slice(&output.stdout).map_err(|e| format!("cannot parse cargo metadata: {e}"))
}

fn check(metadata: &Metadata, rules: &Rules) -> Vec<String> {
    let checker = Checker::new(metadata, rules);
    let mut violations = checker.declaration_violations();
    for package in &checker.members {
        if let Some(rule) = rules.crates.get(&package.name) {
            violations.extend(checker.direct_violations(package, rule));
            violations.extend(checker.transitive_violations(package, rule));
        }
    }
    violations
}

struct Checker<'a> {
    rules: &'a Rules,
    members: Vec<&'a Package>,
    member_names: BTreeSet<&'a str>,
    graph: Graph<'a>,
}

impl<'a> Checker<'a> {
    fn new(metadata: &'a Metadata, rules: &'a Rules) -> Self {
        let members = member_packages(metadata);
        let member_names = members.iter().map(|p| p.name.as_str()).collect();
        Self {
            rules,
            members,
            member_names,
            graph: Graph::new(metadata),
        }
    }

    fn declaration_violations(&self) -> Vec<String> {
        let declared: BTreeSet<&str> = self.rules.crates.keys().map(String::as_str).collect();
        let undeclared = self
            .member_names
            .difference(&declared)
            .map(|name| format!("{name}: not declared in arch.toml"));
        let unknown = declared
            .difference(&self.member_names)
            .map(|name| format!("{name}: declared in arch.toml but not a workspace crate"));
        undeclared.chain(unknown).collect()
    }

    fn direct_violations(&self, package: &Package, rule: &CrateRule) -> Vec<String> {
        package
            .dependencies
            .iter()
            .filter_map(|dep| {
                let kind = dep.kind.as_deref().unwrap_or("normal");
                self.direct_violation(&dep.name, rule)
                    .map(|why| format!("{} -> {} ({kind}): {why}", package.name, dep.name))
            })
            .collect()
    }

    fn direct_violation(&self, dep: &str, rule: &CrateRule) -> Option<String> {
        if self.rules.no_dependents.iter().any(|name| name == dep) {
            return Some(format!("no crate may depend on {dep}"));
        }
        if rule.ban.iter().any(|name| name == dep) {
            return Some("banned for this crate".to_owned());
        }
        let undeclared =
            self.member_names.contains(dep) && !rule.allow.iter().any(|name| name == dep);
        undeclared.then(|| format!("not in allow list {:?}", rule.allow))
    }

    fn transitive_violations(&self, package: &Package, rule: &CrateRule) -> Vec<String> {
        let mut violations = Vec::new();
        for direct in self.graph.runtime_deps(&package.id) {
            let via = self.graph.name(direct);
            if rule.ban.iter().any(|name| name == via) {
                continue;
            }
            let reachable = self.graph.reachable_names(direct);
            let banned = rule
                .ban
                .iter()
                .filter(|name| reachable.contains(name.as_str()) && !self.bans(via, name));
            violations.extend(banned.map(|banned| {
                format!(
                    "{} -> {via} -> ... -> {banned}: banned for this crate",
                    package.name
                )
            }));
        }
        violations
    }

    /// Whether workspace crate `owner` bans `name` itself, in which case a path through
    /// `owner` is reported as `owner`'s violation instead.
    fn bans(&self, owner: &str, name: &str) -> bool {
        self.rules
            .crates
            .get(owner)
            .is_some_and(|rule| rule.ban.iter().any(|b| b == name))
    }
}

fn member_packages(metadata: &Metadata) -> Vec<&Package> {
    let ids: HashSet<&str> = metadata
        .workspace_members
        .iter()
        .map(String::as_str)
        .collect();
    metadata
        .packages
        .iter()
        .filter(|p| ids.contains(p.id.as_str()))
        .collect()
}

/// The resolved dependency graph, restricted to normal and build edges.
struct Graph<'a> {
    names: HashMap<&'a str, &'a str>,
    edges: HashMap<&'a str, Vec<&'a str>>,
}

impl<'a> Graph<'a> {
    fn new(metadata: &'a Metadata) -> Self {
        let names = metadata
            .packages
            .iter()
            .map(|p| (p.id.as_str(), p.name.as_str()))
            .collect();
        let edges = metadata
            .resolve
            .nodes
            .iter()
            .map(|node| {
                (
                    node.id.as_str(),
                    node.deps
                        .iter()
                        .filter(|d| is_runtime(d))
                        .map(|d| d.pkg.as_str())
                        .collect(),
                )
            })
            .collect();
        Self { names, edges }
    }

    fn name(&self, id: &'a str) -> &'a str {
        self.names.get(id).copied().unwrap_or(id)
    }

    fn runtime_deps(&self, id: &str) -> Vec<&'a str> {
        self.edges.get(id).cloned().unwrap_or_default()
    }

    fn reachable_names(&self, start: &'a str) -> HashSet<&'a str> {
        let mut seen = HashSet::from([start]);
        let mut pending = vec![start];
        while let Some(id) = pending.pop() {
            pending.extend(
                self.runtime_deps(id)
                    .into_iter()
                    .filter(|dep| seen.insert(dep)),
            );
        }
        seen.into_iter().map(|id| self.name(id)).collect()
    }
}

fn is_runtime(dep: &NodeDep) -> bool {
    dep.dep_kinds
        .iter()
        .any(|k| k.kind.as_deref() != Some("dev"))
}

fn print_graph(metadata: &Metadata) {
    let members = member_packages(metadata);
    let names: BTreeSet<&str> = members.iter().map(|p| p.name.as_str()).collect();
    for package in members {
        let deps: BTreeSet<&str> = package
            .dependencies
            .iter()
            .map(|d| d.name.as_str())
            .filter(|d| names.contains(d))
            .collect();
        println!("  {} -> {deps:?}", package.name);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const RULES: &str = r#"
        no_dependents = ["app"]
        [crates.domain]
        ban = ["tokio"]
        [crates.application]
        allow = ["domain"]
        ban = ["tokio"]
        [crates.app]
        allow = ["domain", "application"]
    "#;

    /// `(dependency name, kind)`; kind `None` is a normal dependency.
    type Deps<'a> = &'a [(&'a str, Option<&'a str>)];

    fn metadata(packages: &[(&str, Deps<'_>)]) -> Metadata {
        let members = ["domain", "application", "app"];
        let package_json: Vec<_> = packages
            .iter()
            .map(|(name, deps)| {
                let deps: Vec<_> = deps
                    .iter()
                    .map(|(dep, kind)| json!({"name": dep, "kind": kind}))
                    .collect();
                json!({"id": name, "name": name, "dependencies": deps})
            })
            .collect();
        let nodes: Vec<_> = packages
            .iter()
            .map(|(name, deps)| {
                let deps: Vec<_> = deps
                    .iter()
                    .map(|(dep, kind)| json!({"pkg": dep, "dep_kinds": [{"kind": kind}]}))
                    .collect();
                json!({"id": name, "deps": deps})
            })
            .collect();
        serde_json::from_value(json!({
            "packages": package_json,
            "workspace_members": members.iter().filter(|m| packages.iter().any(|(n, _)| n == *m)).collect::<Vec<_>>(),
            "resolve": {"nodes": nodes},
        }))
        .unwrap()
    }

    fn violations(packages: &[(&str, Deps<'_>)]) -> Vec<String> {
        check(&metadata(packages), &toml::from_str(RULES).unwrap())
    }

    #[test]
    fn allowed_graph_passes() {
        let ok = violations(&[
            ("domain", &[("thiserror", None)]),
            ("application", &[("domain", None)]),
            (
                "app",
                &[("application", None), ("domain", None), ("tokio", None)],
            ),
            ("thiserror", &[]),
            ("tokio", &[]),
        ]);
        assert_eq!(ok, Vec::<String>::new());
    }

    #[test]
    fn wrong_way_workspace_dependency_fails() {
        let found = violations(&[
            ("domain", &[("application", None)]),
            ("application", &[]),
            ("app", &[]),
        ]);
        assert_eq!(
            found,
            vec!["domain -> application (normal): not in allow list []".to_owned()]
        );
    }

    #[test]
    fn dev_dependencies_are_checked_too() {
        let found = violations(&[
            ("domain", &[]),
            ("application", &[("app", Some("dev"))]),
            ("app", &[]),
        ]);
        assert_eq!(
            found,
            vec!["application -> app (dev): no crate may depend on app".to_owned()]
        );
    }

    #[test]
    fn banned_direct_dependency_fails() {
        let found = violations(&[
            ("domain", &[("tokio", None)]),
            ("application", &[]),
            ("app", &[]),
            ("tokio", &[]),
        ]);
        assert_eq!(
            found,
            vec!["domain -> tokio (normal): banned for this crate".to_owned()]
        );
    }

    #[test]
    fn banned_transitive_dependency_fails_but_dev_only_paths_do_not() {
        let found = violations(&[
            ("domain", &[("helper", None), ("test-helper", Some("dev"))]),
            ("application", &[]),
            ("app", &[]),
            ("helper", &[("tokio", None)]),
            ("test-helper", &[("tokio", None)]),
            ("tokio", &[]),
        ]);
        assert_eq!(
            found,
            vec!["domain -> helper -> ... -> tokio: banned for this crate".to_owned()]
        );
    }

    #[test]
    fn a_ban_is_reported_once_where_it_is_introduced() {
        let found = violations(&[
            ("domain", &[("tokio", None)]),
            ("application", &[("domain", None)]),
            ("app", &[]),
            ("tokio", &[("mio", None)]),
            ("mio", &[]),
        ]);
        assert_eq!(
            found,
            vec!["domain -> tokio (normal): banned for this crate".to_owned()]
        );
    }

    #[test]
    fn every_crate_must_be_declared() {
        let rules: Rules = toml::from_str("[crates.domain]\n[crates.ghost]").unwrap();
        let found = check(&metadata(&[("domain", &[]), ("app", &[])]), &rules);
        assert_eq!(
            found,
            vec![
                "app: not declared in arch.toml".to_owned(),
                "ghost: declared in arch.toml but not a workspace crate".to_owned()
            ]
        );
    }

    #[test]
    fn rules_file_rejects_unknown_keys() {
        assert!(
            toml::from_str::<Rules>("[crates.domain]\nallowed = []").is_err(),
            "typo in key"
        );
    }
}
