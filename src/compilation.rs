//! Library-owned compilation coordination before target backend work.

use crate::{
    aggregate, lower, pipeline, AggregationRequest, CompilationGraph, LoweringOptions,
    LoweringPlan, NativeTarget, PackageId,
};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

/// One source aggregation request and its exact package selections.
///
/// Target backend resolution and lowering intentionally do not belong here.
/// Callers can perform target syntax parsing before source I/O, but must wait
/// for this coordinator to return a fully aggregated, collision-free graph
/// before resolving a backend or producing artifacts.
pub struct CompilationRequest {
    aggregation: AggregationRequest,
    selectors: Vec<String>,
}

impl CompilationRequest {
    pub fn new(aggregation: AggregationRequest, selectors: Vec<String>) -> Self {
        Self {
            aggregation,
            selectors,
        }
    }
}

/// Aggregates every source, validates aggregate collisions, then selects the
/// requested exact package IDs from the complete validated graph.
pub fn compile(request: CompilationRequest) -> Result<CompilationGraph> {
    let graph = aggregate(request.aggregation)?;
    let selectors = resolve_package_ids(&graph, &request.selectors)?;
    pipeline::select_packages(&graph, &selectors)
}

/// Lowers each selected backend exactly once from a complete validated graph.
///
/// Publication scopes intentionally are not part of the key: project and user
/// publication of one backend share the same immutable backend artifact set.
pub fn lower_unique_targets(
    graph: &CompilationGraph,
    targets: impl IntoIterator<Item = NativeTarget>,
    options: LoweringOptions,
) -> Result<BTreeMap<NativeTarget, LoweringPlan>> {
    lower_unique_targets_with(graph, targets, options, |graph, target, options| {
        lower(graph, target, options)
    })
}

fn lower_unique_targets_with(
    graph: &CompilationGraph,
    targets: impl IntoIterator<Item = NativeTarget>,
    options: LoweringOptions,
    mut lower_backend: impl FnMut(
        &CompilationGraph,
        NativeTarget,
        LoweringOptions,
    ) -> Result<LoweringPlan>,
) -> Result<BTreeMap<NativeTarget, LoweringPlan>> {
    targets
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|target| lower_backend(graph, target, options).map(|plan| (target, plan)))
        .collect()
}

fn resolve_package_ids(graph: &CompilationGraph, selectors: &[String]) -> Result<Vec<PackageId>> {
    selectors
        .iter()
        .map(|selector| {
            graph
                .packages
                .keys()
                .find(|identifier| identifier.as_str() == selector)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("unknown package ID `{selector}`"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{lower_unique_targets, lower_unique_targets_with};
    use crate::{
        CompilationGraph, LoweringOptions, NativeTarget, Package, Resource, ResourceContent,
        ResourcePath, SemanticIdentity, SourceProvenance,
    };

    #[test]
    fn lower_unique_targets_deduplicates_repeated_backends() {
        let package = Package::rule(
            SemanticIdentity::parse("rule:repository-guidance").unwrap(),
            SourceProvenance::new("codex", "AGENTS.md").unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("AGENTS.md").unwrap(),
                ResourceContent::Text("Follow the repository guidance.\n".to_owned()),
                false,
            ),
        )
        .unwrap();
        let graph = CompilationGraph::new([package]).unwrap();

        let lowerings = lower_unique_targets(
            &graph,
            [NativeTarget::Codex, NativeTarget::Codex],
            LoweringOptions::strict(),
        )
        .unwrap();

        assert_eq!(lowerings.len(), 1);
        assert!(lowerings.contains_key(&NativeTarget::Codex));
    }

    #[test]
    fn repeated_targets_invoke_lowering_once() {
        let package = Package::rule(
            SemanticIdentity::parse("rule:repository-guidance").unwrap(),
            SourceProvenance::new("codex", "AGENTS.md").unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("AGENTS.md").unwrap(),
                ResourceContent::Text("Follow the repository guidance.\n".to_owned()),
                false,
            ),
        )
        .unwrap();
        let graph = CompilationGraph::new([package]).unwrap();
        let mut invocations = 0;

        lower_unique_targets_with(
            &graph,
            [NativeTarget::Codex, NativeTarget::Codex],
            LoweringOptions::strict(),
            |graph, target, options| {
                invocations += 1;
                crate::lower(graph, target, options)
            },
        )
        .unwrap();

        assert_eq!(invocations, 1);
    }
}
