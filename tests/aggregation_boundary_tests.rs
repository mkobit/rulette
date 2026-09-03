use rulette::inputs::{ArtifactObservation, InputOrigin, ObservationProvenance};
use rulette::{
    aggregate, AggregationCandidate, AggregationRequest, CompilationGraph, DecoderSelection,
    NativeFrontend, Package, Resource, ResourceContent, ResourcePath, SemanticIdentity,
    SourceProvenance,
};

fn package() -> Package {
    Package::rule(
        SemanticIdentity::parse("rule:repository-guidance").unwrap(),
        SourceProvenance::new("codex", "original/packages.tar").unwrap(),
        Resource::primary_instruction(
            ResourcePath::parse("AGENTS.md").unwrap(),
            ResourceContent::Text("Use the graph compiler.\n".to_owned()),
            false,
        ),
    )
    .unwrap()
}

#[test]
fn aggregation_entry_point_is_usable_with_parser_owned_decoder_selection() {
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
fn aggregation_candidate_preserves_package_provenance_and_keeps_outer_identity_transient() {
    let mut provenance = SourceProvenance::new("codex", "original/packages.tar").unwrap();
    provenance.archive_member = Some(ResourcePath::parse("members/AGENTS.md").unwrap());
    let mut package = package();
    package.provenance = provenance;
    let observation = ArtifactObservation::new(
        b"snapshot".to_vec(),
        "graph.json",
        false,
        InputOrigin::Tar,
        "snapshots/generated.tar",
        Some(ResourcePath::parse("graph.json").unwrap()),
    )
    .unwrap();

    let request = AggregationRequest::new(
        vec![observation.clone()],
        DecoderSelection::Native(NativeFrontend::Codex),
    );
    let candidate = AggregationCandidate::new(package.clone(), &observation.provenance).unwrap();

    assert_eq!(request.observations(), &[observation]);
    assert_eq!(
        request.decoder(),
        DecoderSelection::Native(NativeFrontend::Codex)
    );
    assert_eq!(candidate.package(), &package);
    assert_eq!(
        candidate.outer_input_identity().as_str(),
        "snapshots/generated.tar"
    );
    assert_eq!(
        candidate.package().provenance.archive_member,
        Some(ResourcePath::parse("members/AGENTS.md").unwrap())
    );
    assert!(!CompilationGraph::new([candidate.package().clone()])
        .unwrap()
        .to_canonical_json()
        .unwrap()
        .contains("snapshots/generated.tar"));
    assert!(!CompilationGraph::new([candidate.package().clone()])
        .unwrap()
        .to_toml()
        .unwrap()
        .contains("snapshots/generated.tar"));
}

#[test]
fn aggregation_candidate_rejects_unsafe_or_non_normalized_outer_input_labels() {
    for input_label in ["/tmp/generated.tar", "snapshots/./generated.tar"] {
        let provenance = ObservationProvenance {
            input_label: input_label.to_owned(),
            archive_member: None,
        };

        assert!(AggregationCandidate::new(package(), &provenance).is_err());
    }
}

#[test]
fn aggregation_candidate_accepts_stdin_as_outer_identity() {
    let provenance = ObservationProvenance {
        input_label: "stdin".to_owned(),
        archive_member: None,
    };

    let candidate = AggregationCandidate::new(package(), &provenance).unwrap();

    assert_eq!(candidate.outer_input_identity().as_str(), "stdin");
}
