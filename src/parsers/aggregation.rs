use crate::inputs::{ArtifactObservation, ObservationProvenance};
use crate::ir::graph::{
    CompilationGraph, GraphDiagnostic, Package, PackageId, ResourcePath, SemanticIdentity,
    SourceProvenance,
};
use crate::parsers::frontend::{compile_graph, DecoderSelection};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

/// A validated package paired with the explicit input that supplied it.
///
/// The outer input identity is transient diagnostic context.
/// It is deliberately not part of `Package` or `CompilationGraph` serialization.
#[derive(Clone, Debug, PartialEq)]
pub struct AggregationCandidate {
    package: Package,
    outer_input_identity: OuterInputIdentity,
}

impl AggregationCandidate {
    pub fn new(package: Package, provenance: &ObservationProvenance) -> Result<Self> {
        package.validate()?;
        Ok(Self {
            package,
            outer_input_identity: OuterInputIdentity::from_observation_provenance(provenance)?,
        })
    }

    pub fn package(&self) -> &Package {
        &self.package
    }

    fn into_package(self) -> Package {
        self.package
    }

    pub fn outer_input_identity(&self) -> &OuterInputIdentity {
        &self.outer_input_identity
    }
}

/// One pre-construction aggregate collision key.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AggregateCollisionKey {
    SemanticIdentity(SemanticIdentity),
    PackageId(PackageId),
}

impl Display for AggregateCollisionKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SemanticIdentity(identity) => {
                write!(formatter, "semantic identity `{}`", identity.as_str())
            }
            Self::PackageId(package_id) => {
                write!(formatter, "package ID `{}`", package_id.as_str())
            }
        }
    }
}

/// One candidate participating in an aggregate collision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregateCollisionCandidate {
    pub package_id: PackageId,
    pub provenance: SourceProvenance,
    pub outer_input_identity: OuterInputIdentity,
}

/// A complete collision group for one independently indexed identity key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregateCollisionGroup {
    pub key: AggregateCollisionKey,
    pub candidates: Vec<AggregateCollisionCandidate>,
}

/// All aggregate collisions discovered before graph construction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregationCollisionError {
    groups: Vec<AggregateCollisionGroup>,
}

impl AggregationCollisionError {
    pub fn groups(&self) -> &[AggregateCollisionGroup] {
        &self.groups
    }
}

impl Display for AggregationCollisionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("aggregate package collisions:\n")?;
        for group in &self.groups {
            writeln!(formatter, "- {}:", group.key)?;
            for candidate in &group.candidates {
                write!(
                    formatter,
                    "  - package ID `{}`; provenance `{}:{}`",
                    candidate.package_id.as_str(),
                    candidate.provenance.frontend,
                    candidate.provenance.input_label
                )?;
                if let Some(member) = &candidate.provenance.archive_member {
                    write!(formatter, "; archive member `{}`", member.as_str())?;
                }
                writeln!(
                    formatter,
                    "; outer input `{}`",
                    candidate.outer_input_identity.as_str()
                )?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for AggregationCollisionError {}

/// A content-safe identity for one explicit outer input.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct OuterInputIdentity(String);

impl OuterInputIdentity {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn from_observation_provenance(provenance: &ObservationProvenance) -> Result<Self> {
        if provenance.input_label != "stdin" {
            ResourcePath::parse(provenance.input_label.clone())
                .context("outer input identity must be a normalized content-safe input label")?;
        }
        Ok(Self(provenance.input_label.clone()))
    }
}

/// Library-owned input to later decoder selection and aggregation stages.
///
/// It contains observed content and a parser-owned decoder selection only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AggregationRequest {
    observations: Vec<ArtifactObservation>,
    decoder: DecoderSelection,
}

impl AggregationRequest {
    pub fn new(observations: Vec<ArtifactObservation>, decoder: DecoderSelection) -> Self {
        Self {
            observations,
            decoder,
        }
    }

    pub fn observations(&self) -> &[ArtifactObservation] {
        &self.observations
    }

    pub fn decoder(&self) -> DecoderSelection {
        self.decoder
    }
}

/// Decodes one library-owned aggregation request into the current graph model.
pub fn aggregate(request: AggregationRequest) -> Result<CompilationGraph> {
    compile_graph(request.observations(), request.decoder())
}

/// Constructs one graph from collision-free aggregation candidates and decoded diagnostics.
pub(crate) fn construct_graph(
    candidates: Vec<AggregationCandidate>,
    diagnostics: Vec<GraphDiagnostic>,
) -> Result<CompilationGraph> {
    if let Some(error) = collect_collisions(&candidates) {
        return Err(error.into());
    }
    CompilationGraph::from_parts(
        candidates
            .into_iter()
            .map(AggregationCandidate::into_package),
        diagnostics,
    )
}

fn collect_collisions(candidates: &[AggregationCandidate]) -> Option<AggregationCollisionError> {
    let mut semantic_indexes = BTreeMap::<SemanticIdentity, Vec<usize>>::new();
    let mut package_id_indexes = BTreeMap::<PackageId, Vec<usize>>::new();
    for (index, candidate) in candidates.iter().enumerate() {
        semantic_indexes
            .entry(candidate.package.semantic_identity.clone())
            .or_default()
            .push(index);
        package_id_indexes
            .entry(candidate.package.id.clone())
            .or_default()
            .push(index);
    }

    let semantic_groups = semantic_indexes.into_iter().filter_map(|(key, indexes)| {
        (indexes.len() > 1).then(|| {
            collision_group(
                AggregateCollisionKey::SemanticIdentity(key),
                indexes,
                candidates,
            )
        })
    });
    let package_id_groups = package_id_indexes.into_iter().filter_map(|(key, indexes)| {
        (indexes.len() > 1)
            .then(|| collision_group(AggregateCollisionKey::PackageId(key), indexes, candidates))
    });
    let groups = semantic_groups.chain(package_id_groups).collect::<Vec<_>>();

    (!groups.is_empty()).then_some(AggregationCollisionError { groups })
}

fn collision_group(
    key: AggregateCollisionKey,
    indexes: Vec<usize>,
    candidates: &[AggregationCandidate],
) -> AggregateCollisionGroup {
    let mut candidates = indexes
        .into_iter()
        .map(|index| {
            let candidate = &candidates[index];
            AggregateCollisionCandidate {
                package_id: candidate.package.id.clone(),
                provenance: candidate.package.provenance.clone(),
                outer_input_identity: candidate.outer_input_identity.clone(),
            }
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        (
            &left.outer_input_identity,
            &left.package_id,
            &left.provenance.frontend,
            &left.provenance.input_label,
            left.provenance.archive_member.as_ref(),
        )
            .cmp(&(
                &right.outer_input_identity,
                &right.package_id,
                &right.provenance.frontend,
                &right.provenance.input_label,
                right.provenance.archive_member.as_ref(),
            ))
    });
    AggregateCollisionGroup { key, candidates }
}

#[cfg(test)]
mod tests {
    use super::{aggregate, construct_graph, AggregationCandidate, AggregationRequest};
    use crate::inputs::{ArtifactObservation, InputOrigin, ObservationProvenance};
    use crate::ir::graph::{
        CompilationGraph, Package, Resource, ResourceContent, ResourcePath, SemanticIdentity,
        SourceProvenance,
    };
    use crate::parsers::frontend::{DecoderSelection, NativeFrontend};

    fn package() -> Package {
        Package::rule(
            SemanticIdentity::parse("rule:repository-guidance").unwrap(),
            SourceProvenance::new("codex", "original/AGENTS.md").unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("AGENTS.md").unwrap(),
                ResourceContent::Text("Use the graph compiler.\n".to_owned()),
                false,
            ),
        )
        .unwrap()
    }

    fn candidate(package: Package, input_label: &str) -> AggregationCandidate {
        AggregationCandidate::new(
            package,
            &ObservationProvenance {
                input_label: input_label.to_owned(),
                archive_member: None,
            },
        )
        .unwrap()
    }

    #[test]
    fn construct_graph_reports_every_semantic_and_package_id_collision_stably() {
        let duplicate = package();
        let changed_content = Package::rule(
            SemanticIdentity::parse("rule:repository-guidance").unwrap(),
            SourceProvenance::new("codex", "original/changed/AGENTS.md").unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("AGENTS.md").unwrap(),
                ResourceContent::Text("Use a different graph compiler.\n".to_owned()),
                false,
            ),
        )
        .unwrap();

        let error = construct_graph(
            vec![
                candidate(changed_content, "snapshots/second"),
                candidate(duplicate.clone(), "snapshots/third"),
                candidate(duplicate, "snapshots/first"),
                candidate(package(), "snapshots/fourth"),
            ],
            Vec::new(),
        )
        .unwrap_err();
        let rendered = error.to_string();

        assert!(rendered.contains("semantic identity `rule:repository-guidance`"));
        assert!(rendered.contains("package ID `pkg_"));
        assert!(rendered.contains("outer input `snapshots/first`"));
        assert!(rendered.contains("outer input `snapshots/second`"));
        assert!(rendered.contains("outer input `snapshots/third`"));
        assert!(rendered.contains("outer input `snapshots/fourth`"));
        assert!(rendered.contains("provenance `codex:original/AGENTS.md`"));
        assert!(rendered.contains("provenance `codex:original/changed/AGENTS.md`"));
    }

    #[test]
    fn native_candidates_keep_their_outer_input_identity_for_collision_diagnostics() {
        let observations = [
            ArtifactObservation::new(
                b"Use the graph compiler.\n".to_vec(),
                "AGENTS.md",
                false,
                InputOrigin::Filesystem,
                "snapshots/first",
                None,
            )
            .unwrap(),
            ArtifactObservation::new(
                b"Use the graph compiler.\n".to_vec(),
                "AGENTS.md",
                false,
                InputOrigin::Filesystem,
                "snapshots/second",
                None,
            )
            .unwrap(),
        ];

        let error = aggregate(AggregationRequest::new(
            observations.to_vec(),
            DecoderSelection::Native(NativeFrontend::Codex),
        ))
        .unwrap_err();
        let rendered = error.to_string();

        assert!(rendered.contains("outer input `snapshots/first`"));
        assert!(rendered.contains("outer input `snapshots/second`"));
    }

    #[test]
    fn collision_groups_and_members_are_stable_across_candidate_permutations() {
        let repository = package();
        let changed_repository = Package::rule(
            SemanticIdentity::parse("rule:repository-guidance").unwrap(),
            SourceProvenance::new("codex", "original/changed/AGENTS.md").unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("AGENTS.md").unwrap(),
                ResourceContent::Text("Use a different graph compiler.\n".to_owned()),
                false,
            ),
        )
        .unwrap();
        let alternate = Package::rule(
            SemanticIdentity::parse("rule:alternate-guidance").unwrap(),
            SourceProvenance::new("codex", "original/alternate/AGENTS.md").unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("AGENTS.md").unwrap(),
                ResourceContent::Text("Use the alternate graph compiler.\n".to_owned()),
                false,
            ),
        )
        .unwrap();
        let forward = vec![
            candidate(repository.clone(), "snapshots/repository-b"),
            candidate(alternate.clone(), "snapshots/alternate-b"),
            candidate(changed_repository, "snapshots/repository-c"),
            candidate(repository, "snapshots/repository-a"),
            candidate(alternate, "snapshots/alternate-a"),
        ];
        let reverse = forward.iter().cloned().rev().collect::<Vec<_>>();

        let forward_error = construct_graph(forward, Vec::new()).unwrap_err();
        let reverse_error = construct_graph(reverse, Vec::new()).unwrap_err();
        let collision = forward_error
            .downcast_ref::<super::AggregationCollisionError>()
            .unwrap();

        assert_eq!(forward_error.to_string(), reverse_error.to_string());
        assert_eq!(collision.groups().len(), 4);
        assert!(matches!(
            collision.groups()[0].key,
            super::AggregateCollisionKey::SemanticIdentity(_)
        ));
        assert!(matches!(
            collision.groups()[1].key,
            super::AggregateCollisionKey::SemanticIdentity(_)
        ));
        assert!(matches!(
            collision.groups()[2].key,
            super::AggregateCollisionKey::PackageId(_)
        ));
        assert!(matches!(
            collision.groups()[3].key,
            super::AggregateCollisionKey::PackageId(_)
        ));
        for group in collision.groups() {
            assert!(group
                .candidates
                .windows(2)
                .all(|pair| { pair[0].outer_input_identity <= pair[1].outer_input_identity }));
        }
    }

    #[test]
    fn candidate_rejects_unsafe_outer_input_identity() {
        let provenance = ObservationProvenance {
            input_label: "/tmp/generated.tar".to_owned(),
            archive_member: None,
        };

        assert!(AggregationCandidate::new(package(), &provenance).is_err());
    }

    #[test]
    fn candidate_accepts_stdin_outer_input_identity() {
        let provenance = ObservationProvenance {
            input_label: "stdin".to_owned(),
            archive_member: None,
        };

        let candidate = AggregationCandidate::new(package(), &provenance).unwrap();

        assert_eq!(candidate.outer_input_identity().as_str(), "stdin");
    }

    #[test]
    fn aggregate_delegates_to_the_selected_native_frontend() {
        let observation = ArtifactObservation::new(
            b"Use the graph compiler.\n".to_vec(),
            "AGENTS.md",
            false,
            InputOrigin::Filesystem,
            "snapshots/codex",
            None,
        )
        .unwrap();

        let graph = aggregate(AggregationRequest::new(
            vec![observation],
            DecoderSelection::Native(NativeFrontend::Codex),
        ))
        .unwrap();

        assert_eq!(graph.packages.len(), 1);
    }

    #[test]
    fn aggregate_rejects_interchange_diagnostics_for_missing_packages() {
        let graph = CompilationGraph::new([package()]).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(&graph.to_canonical_json().unwrap()).unwrap();
        value["diagnostics"] = serde_json::json!([{
            "severity": "warning",
            "code": "missing-package",
            "message": "diagnostic package is absent",
            "package_id": format!("pkg_{}", "0".repeat(64)),
        }]);
        let observation = ArtifactObservation::new(
            serde_json::to_vec(&value).unwrap(),
            "graph.json",
            false,
            InputOrigin::Filesystem,
            "snapshots/graph.json",
            None,
        )
        .unwrap();

        assert!(aggregate(AggregationRequest::new(
            vec![observation],
            DecoderSelection::GraphJson,
        ))
        .is_err());
    }
}
