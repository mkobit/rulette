use crate::{CompilationGraph, DiagnosticSeverity, PackageId};
use anyhow::{bail, Result};
use std::collections::{BTreeMap, BTreeSet};

/// Selects an exact, deterministic union of graph packages without changing
/// any package content.
///
/// An empty selector list retains the complete validated graph.
/// Otherwise every requested ID must exist, repeated IDs are deduplicated,
/// packages remain in their `BTreeMap` identifier order, and diagnostics are
/// limited to selected package diagnostics plus package-independent warnings.
pub fn select_packages(
    graph: &CompilationGraph,
    identifiers: &[PackageId],
) -> Result<CompilationGraph> {
    graph.validate()?;
    if identifiers.is_empty() {
        return Ok(graph.clone());
    }

    let mut selected_ids = BTreeSet::new();
    for identifier in identifiers {
        if !graph.packages.contains_key(identifier) {
            bail!("unknown package ID `{}`", identifier.as_str());
        }
        selected_ids.insert(identifier.clone());
    }

    let packages: BTreeMap<_, _> = graph
        .packages
        .iter()
        .filter(|(identifier, _)| selected_ids.contains(*identifier))
        .map(|(identifier, package)| (identifier.clone(), package.clone()))
        .collect();
    let diagnostics = graph
        .diagnostics
        .iter()
        .filter(|diagnostic| match &diagnostic.package_id {
            Some(identifier) => selected_ids.contains(identifier),
            None => diagnostic.severity == DiagnosticSeverity::Warning,
        })
        .cloned()
        .collect();
    let selected = CompilationGraph {
        graph_version: graph.graph_version.clone(),
        packages,
        diagnostics,
    };
    selected.validate()?;
    Ok(selected)
}

#[cfg(test)]
mod tests {
    use super::select_packages;
    use crate::{
        CompilationGraph, DiagnosticSeverity, GraphDiagnostic, Package, PackageKind, PackageRoot,
        Resource, ResourceContent, ResourcePath, SemanticIdentity, SemanticItem, SourceProvenance,
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn rule(name: &str, body: &str) -> Package {
        let primary_path = ResourcePath::parse("AGENTS.md").unwrap();
        let opaque_path = ResourcePath::parse("scripts/check.sh").unwrap();
        let mut resources = BTreeMap::new();
        resources.insert(
            primary_path.clone(),
            Resource::primary_instruction(
                primary_path.clone(),
                ResourceContent::Text(body.to_owned()),
                false,
            ),
        );
        resources.insert(
            opaque_path.clone(),
            Resource::opaque(opaque_path, ResourceContent::Bytes(vec![0, 1, 2, 3]), true),
        );
        Package::new(
            PackageKind::Rule,
            SemanticIdentity::parse(format!("rule:{name}")).unwrap(),
            SourceProvenance::new("codex", format!("fixtures/{name}/AGENTS.md")).unwrap(),
            PackageRoot::parse(format!("fixtures/{name}")).unwrap(),
            SemanticItem::Rule {
                primary_instruction: primary_path,
                description: None,
                activation: None,
                frontend_payload: None,
            },
            resources,
            None,
        )
        .unwrap()
    }

    fn unsupported(name: &str) -> Package {
        let resource_path = ResourcePath::parse("reviewer.md").unwrap();
        let mut resources = BTreeMap::new();
        resources.insert(
            resource_path.clone(),
            Resource::opaque(
                resource_path,
                ResourceContent::Bytes(b"native agent".to_vec()),
                false,
            ),
        );
        Package::new(
            PackageKind::Unsupported,
            SemanticIdentity::parse(format!("unsupported:agent/{name}")).unwrap(),
            SourceProvenance::new("claude", format!("fixtures/{name}/reviewer.md")).unwrap(),
            PackageRoot::parse(format!("fixtures/{name}")).unwrap(),
            SemanticItem::Unsupported {
                native_kind: "agent".to_owned(),
            },
            resources,
            None,
        )
        .unwrap()
    }

    fn graph_with_diagnostics() -> (CompilationGraph, Package, Package) {
        let first = rule("first", "first rule\n");
        let second = unsupported("second");
        let mut graph = CompilationGraph::new([first.clone(), second.clone()]).unwrap();
        graph.diagnostics.insert(
            0,
            GraphDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "a-global-warning".to_owned(),
                message: "input contains an unrecognized file".to_owned(),
                package_id: None,
            },
        );
        graph.diagnostics.insert(
            1,
            GraphDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "b-package-warning".to_owned(),
                message: "selected rule has a source warning".to_owned(),
                package_id: Some(first.id().clone()),
            },
        );
        graph.validate().unwrap();
        (graph, first, second)
    }

    #[test]
    fn no_selectors_retain_the_validated_graph_byte_for_byte() {
        let (graph, _, _) = graph_with_diagnostics();

        let selected = select_packages(&graph, &[]).unwrap();

        assert_eq!(selected, graph);
        assert_eq!(
            selected.to_canonical_json().unwrap(),
            graph.to_canonical_json().unwrap()
        );
    }

    #[test]
    fn exact_selectors_form_a_deterministic_union_without_mutating_packages() {
        let (graph, first, second) = graph_with_diagnostics();

        let selected = select_packages(
            &graph,
            &[second.id().clone(), first.id().clone(), second.id().clone()],
        )
        .unwrap();

        let expected_ids: BTreeSet<_> = [first.id().clone(), second.id().clone()]
            .into_iter()
            .collect();
        let actual_ids: BTreeSet<_> = selected.packages.keys().cloned().collect();
        assert_eq!(actual_ids, expected_ids);
        assert_eq!(
            selected.packages.get(first.id()),
            graph.packages.get(first.id())
        );
        assert_eq!(
            selected.packages.get(second.id()),
            graph.packages.get(second.id())
        );
    }

    #[test]
    fn unknown_selector_fails_without_altering_the_input_graph() {
        let (graph, _, _) = graph_with_diagnostics();
        let unknown = rule("unknown", "unknown rule\n").id().clone();
        let original = graph.clone();

        let error = select_packages(&graph, std::slice::from_ref(&unknown)).unwrap_err();

        assert!(error.to_string().contains(unknown.as_str()));
        assert_eq!(graph, original);
    }

    #[test]
    fn selection_retains_only_global_and_selected_package_diagnostics_in_stable_order() {
        let (graph, first, second) = graph_with_diagnostics();

        let selected = select_packages(&graph, &[first.id().clone()]).unwrap();

        assert_eq!(selected.packages.len(), 1);
        assert!(selected.packages.contains_key(first.id()));
        assert!(!selected.packages.contains_key(second.id()));
        assert_eq!(selected.diagnostics.len(), 2);
        assert_eq!(selected.diagnostics[0], graph.diagnostics[0]);
        assert_eq!(selected.diagnostics[1], graph.diagnostics[1]);
        assert_eq!(
            selected
                .packages
                .get(first.id())
                .unwrap()
                .resources
                .get(&ResourcePath::parse("scripts/check.sh").unwrap()),
            graph
                .packages
                .get(first.id())
                .unwrap()
                .resources
                .get(&ResourcePath::parse("scripts/check.sh").unwrap())
        );
    }
}
