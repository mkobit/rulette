#![cfg(any(target_os = "linux", target_os = "android"))]

use rulette::publication::{
    apply_plan, check_plan, check_sources, mapping_for, parse_plan_with_expected_digest, stage,
    ApplyOptions, AuthorizedRoot, DestinationState, PlanOperationRequest, PublicationScope,
    ScopedAcceptedLoss, ScopedLowering, SourceCheckRequest, StageRequest, StageRoot,
};
use rulette::{
    lower, CompilationGraph, LoweringOptions, NativeTarget, Package, Resource, ResourceContent,
    ResourcePath, SemanticIdentity, SourceProvenance,
};
use std::fs;
use std::path::{Path, PathBuf};

struct Fixture {
    _temporary: tempfile::TempDir,
    root: PathBuf,
    stage_dir: PathBuf,
    staged: rulette::publication::StagedPublication,
}

fn fixture() -> Fixture {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    let stage_dir = temporary.path().join("stage");
    fs::create_dir(&root).unwrap();
    let package = Package::rule(
        SemanticIdentity::parse("rule:repository-guidance").unwrap(),
        SourceProvenance::new("opencode", "rules/repository-guidance.md").unwrap(),
        Resource::primary_instruction(
            ResourcePath::parse("RULE.md").unwrap(),
            ResourceContent::Text("# Repository guidance\n".to_owned()),
            false,
        ),
    )
    .unwrap();
    let graph = CompilationGraph::new([package]).unwrap();
    let lowering = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();
    let staged = stage(StageRequest {
        graph: &graph,
        lowerings: vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        roots: vec![StageRoot {
            target: NativeTarget::OpenCode,
            scope: PublicationScope::Project,
            path: &root,
        }],
        accepted_losses: vec![],
        stage_dir: &stage_dir,
    })
    .unwrap();

    Fixture {
        _temporary: temporary,
        root,
        stage_dir,
        staged,
    }
}

fn request<'a>(fixture: &'a Fixture) -> PlanOperationRequest<'a> {
    PlanOperationRequest {
        stage_dir: &fixture.stage_dir,
        expected_plan_digest: fixture.staged.plan_digest.clone(),
        roots: vec![AuthorizedRoot {
            target: NativeTarget::OpenCode,
            scope: PublicationScope::Project,
            path: &fixture.root,
        }],
    }
}

fn destination(fixture: &Fixture) -> PathBuf {
    let entry = &fixture.staged.plan.entries[0];
    let relative = mapping_for(entry.target, entry.scope)
        .unwrap()
        .map_artifact(&entry.artifact)
        .unwrap();
    fixture.root.join(relative.as_str())
}

fn source_graph(executable: bool) -> CompilationGraph {
    let package = Package::rule(
        SemanticIdentity::parse("rule:source-check-guidance").unwrap(),
        SourceProvenance::new("opencode", "rules/source-check-guidance.md").unwrap(),
        Resource::primary_instruction(
            ResourcePath::parse("RULE.md").unwrap(),
            ResourceContent::Text("# Source check guidance\n".to_owned()),
            executable,
        ),
    )
    .unwrap();
    CompilationGraph::new([package]).unwrap()
}

fn source_request<'a>(
    graph: &'a CompilationGraph,
    lowering: &'a rulette::LoweringPlan,
    root: &'a PathBuf,
    accepted_losses: Vec<ScopedAcceptedLoss<'a>>,
) -> SourceCheckRequest<'a> {
    SourceCheckRequest {
        graph,
        lowerings: vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering,
        }],
        roots: vec![AuthorizedRoot {
            target: NativeTarget::OpenCode,
            scope: PublicationScope::Project,
            path: root,
        }],
        accepted_losses,
    }
}

fn source_destination(root: &Path, lowering: &rulette::LoweringPlan) -> PathBuf {
    let artifact = &lowering.artifacts[0];
    root.join(
        mapping_for(artifact.target, PublicationScope::Project)
            .unwrap()
            .map_artifact(&rulette::publication::ArtifactDescriptor {
                class: artifact.class,
                native_path: artifact.path.clone(),
            })
            .unwrap()
            .as_str(),
    )
}

#[test]
fn source_check_classifies_absent_without_creating_a_stage_or_destination() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    let requested_stage = temporary.path().join("would-be-stage");
    fs::create_dir(&root).unwrap();
    let graph = source_graph(false);
    let lowering = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();

    let report = check_sources(source_request(&graph, &lowering, &root, vec![])).unwrap();

    assert_eq!(report.entries[0].state, DestinationState::Absent);
    assert!(!requested_stage.exists());
    assert!(!source_destination(&root, &lowering).exists());
    assert!(fs::read_dir(&root).unwrap().next().is_none());
}

#[test]
fn source_check_classifies_unchanged_and_conflicting_destinations_without_mutation() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    fs::create_dir(&root).unwrap();
    let graph = source_graph(false);
    let lowering = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();
    let destination = source_destination(&root, &lowering);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, &lowering.artifacts[0].bytes).unwrap();

    let unchanged = check_sources(source_request(&graph, &lowering, &root, vec![])).unwrap();

    assert_eq!(unchanged.entries[0].state, DestinationState::Unchanged);
    fs::write(&destination, b"source destination drift").unwrap();

    let conflicting = check_sources(source_request(&graph, &lowering, &root, vec![])).unwrap();

    assert_eq!(conflicting.entries[0].state, DestinationState::Conflict);
    assert_eq!(fs::read(&destination).unwrap(), b"source destination drift");
}

#[test]
fn source_check_uses_the_same_stable_entry_identifier_as_staging() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    let stage_dir = temporary.path().join("stage");
    fs::create_dir(&root).unwrap();
    let graph = source_graph(false);
    let lowering = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();

    let source = check_sources(source_request(&graph, &lowering, &root, vec![])).unwrap();
    let staged = stage(StageRequest {
        graph: &graph,
        lowerings: vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        roots: vec![StageRoot {
            target: NativeTarget::OpenCode,
            scope: PublicationScope::Project,
            path: &root,
        }],
        accepted_losses: vec![],
        stage_dir: &stage_dir,
    })
    .unwrap();

    assert_eq!(source.entries[0].entry_id, staged.plan.entries[0].entry_id);
}

#[test]
fn source_check_rejects_missing_or_surplus_authority_before_destination_reads() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    let surplus = temporary.path().join("surplus");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&surplus).unwrap();
    let graph = source_graph(false);
    let lowering = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();
    let missing = SourceCheckRequest {
        graph: &graph,
        lowerings: vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        roots: vec![],
        accepted_losses: vec![],
    };

    assert!(check_sources(missing)
        .unwrap_err()
        .to_string()
        .contains("missing"));
    let surplus_request = SourceCheckRequest {
        graph: &graph,
        lowerings: vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        roots: vec![
            AuthorizedRoot {
                target: NativeTarget::OpenCode,
                scope: PublicationScope::Project,
                path: &root,
            },
            AuthorizedRoot {
                target: NativeTarget::Claude,
                scope: PublicationScope::Project,
                path: &surplus,
            },
        ],
        accepted_losses: vec![],
    };

    assert!(check_sources(surplus_request)
        .unwrap_err()
        .to_string()
        .contains("surplus"));
    assert!(fs::read_dir(&root).unwrap().next().is_none());
}

#[test]
fn source_check_rejects_unaccepted_lowering_loss_without_writing() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    fs::create_dir(&root).unwrap();
    let graph = source_graph(true);
    let lowering = lower(
        &graph,
        NativeTarget::OpenCode,
        LoweringOptions::allow_lossy(),
    )
    .unwrap();

    let error = check_sources(source_request(&graph, &lowering, &root, vec![])).unwrap_err();

    assert!(error.to_string().contains("unaccepted"));
    assert!(fs::read_dir(&root).unwrap().next().is_none());
}

#[test]
fn plan_mode_check_classifies_an_absent_destination_without_mutation() {
    let fixture = fixture();
    let destination = destination(&fixture);

    let report = check_plan(request(&fixture)).unwrap();

    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].state, DestinationState::Absent);
    assert!(!destination.exists());
    assert!(!fixture.root.join(".opencode").exists());
}

#[test]
fn plan_mode_check_classifies_unchanged_and_conflicting_destinations_without_mutation() {
    let fixture = fixture();
    let destination = destination(&fixture);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    let artifact = fs::read(
        fixture
            .stage_dir
            .join(fixture.staged.plan.entries[0].stage_artifact_path.as_str()),
    )
    .unwrap();
    fs::write(&destination, &artifact).unwrap();

    let unchanged = check_plan(request(&fixture)).unwrap();

    assert_eq!(unchanged.entries[0].state, DestinationState::Unchanged);
    assert!(unchanged.is_clean());
    fs::write(&destination, b"destination drift").unwrap();

    let conflicting = check_plan(request(&fixture)).unwrap();

    assert_eq!(conflicting.entries[0].state, DestinationState::Conflict);
    assert!(!conflicting.is_clean());
    assert_eq!(fs::read(&destination).unwrap(), b"destination drift");
}

#[test]
fn raw_plan_digest_is_checked_before_untrusted_plan_json_is_parsed() {
    let fixture = fixture();
    fs::write(fixture.stage_dir.join("rulette.plan.json"), b"not JSON").unwrap();

    let error = check_plan(request(&fixture)).unwrap_err();

    assert!(error.to_string().contains("expected plan digest"));
    assert!(!destination(&fixture).exists());
}

#[test]
fn staged_artifact_tampering_fails_before_destination_mutation() {
    let fixture = fixture();
    let artifact_path = fixture
        .stage_dir
        .join(fixture.staged.plan.entries[0].stage_artifact_path.as_str());
    fs::write(artifact_path, b"tampered").unwrap();

    let error = apply_plan(request(&fixture), ApplyOptions { replace: false }).unwrap_err();

    assert!(error.to_string().contains("does not match"));
    assert!(!destination(&fixture).exists());
}

#[cfg(unix)]
#[test]
fn every_staged_artifact_is_verified_before_any_destination_is_inspected() {
    use std::os::unix::fs::symlink;

    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    let stage_dir = temporary.path().join("stage");
    let outside = temporary.path().join("outside");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&outside).unwrap();
    let packages = ["alpha", "beta"].map(|name| {
        Package::rule(
            SemanticIdentity::parse(format!("rule:{name}")).unwrap(),
            SourceProvenance::new("opencode", format!("rules/{name}.md")).unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("RULE.md").unwrap(),
                ResourceContent::Text(format!("# {name}\n")),
                false,
            ),
        )
        .unwrap()
    });
    let graph = CompilationGraph::new(packages).unwrap();
    let lowering = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();
    let staged = stage(StageRequest {
        graph: &graph,
        lowerings: vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        roots: vec![StageRoot {
            target: NativeTarget::OpenCode,
            scope: PublicationScope::Project,
            path: &root,
        }],
        accepted_losses: vec![],
        stage_dir: &stage_dir,
    })
    .unwrap();
    let plan = parse_plan_with_expected_digest(
        &fs::read(stage_dir.join("rulette.plan.json")).unwrap(),
        &staged.plan_digest,
    )
    .unwrap();
    for entry in &plan.entries {
        let destination = root.join(
            mapping_for(entry.target, entry.scope)
                .unwrap()
                .map_artifact(&entry.artifact)
                .unwrap()
                .as_str(),
        );
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        symlink(outside.join("destination"), destination).unwrap();
    }
    fs::write(
        stage_dir.join(plan.entries.last().unwrap().stage_artifact_path.as_str()),
        b"tampered staged artifact",
    )
    .unwrap();

    let error = check_plan(PlanOperationRequest {
        stage_dir: &stage_dir,
        expected_plan_digest: staged.plan_digest.clone(),
        roots: vec![AuthorizedRoot {
            target: NativeTarget::OpenCode,
            scope: PublicationScope::Project,
            path: &root,
        }],
    })
    .unwrap_err();

    assert!(error.to_string().contains("does not match"));
}

#[test]
fn apply_requires_replace_for_a_conflicting_destination_without_writing_it() {
    let fixture = fixture();
    let destination = destination(&fixture);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, b"existing conflict").unwrap();

    let error = apply_plan(request(&fixture), ApplyOptions { replace: false }).unwrap_err();

    assert!(error.to_string().contains("conflict"));
    assert_eq!(fs::read(&destination).unwrap(), b"existing conflict");
}

#[test]
fn apply_replaces_a_conflicting_destination_only_with_explicit_permission() {
    let fixture = fixture();
    let destination = destination(&fixture);
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(&destination, b"existing conflict").unwrap();

    let report = apply_plan(request(&fixture), ApplyOptions { replace: true }).unwrap();

    assert_eq!(
        report.replaced,
        vec![fixture.staged.plan.entries[0].entry_id.clone()]
    );
    assert_eq!(
        fs::read(&destination).unwrap(),
        fs::read(
            fixture
                .stage_dir
                .join(fixture.staged.plan.entries[0].stage_artifact_path.as_str())
        )
        .unwrap()
    );
}

#[test]
fn surplus_authority_root_is_rejected_before_checking_destinations() {
    let fixture = fixture();
    let unused_root = fixture._temporary.path().join("unused");
    fs::create_dir(&unused_root).unwrap();
    let mut operation = request(&fixture);
    operation.roots.push(AuthorizedRoot {
        target: NativeTarget::Claude,
        scope: PublicationScope::Project,
        path: &unused_root,
    });

    let error = check_plan(operation).unwrap_err();

    assert!(error.to_string().contains("surplus"));
    assert!(!destination(&fixture).exists());
}

#[test]
fn missing_authority_root_is_rejected_before_checking_destinations() {
    let fixture = fixture();
    let operation = PlanOperationRequest {
        stage_dir: &fixture.stage_dir,
        expected_plan_digest: fixture.staged.plan_digest.clone(),
        roots: vec![],
    };

    let error = check_plan(operation).unwrap_err();

    assert!(error.to_string().contains("missing"));
    assert!(!destination(&fixture).exists());
}

#[test]
fn root_identity_mismatch_is_rejected_before_destination_inspection() {
    let fixture = fixture();
    let different_root = fixture._temporary.path().join("different-project");
    fs::create_dir(&different_root).unwrap();
    let mut operation = request(&fixture);
    operation.roots[0].path = &different_root;

    let error = check_plan(operation).unwrap_err();

    assert!(error.to_string().contains("root identity"));
    assert!(!destination(&fixture).exists());
}

#[test]
fn same_root_ancestor_destinations_are_rejected_before_destination_mutation() {
    let fixture = fixture();
    let mut plan = fixture.staged.plan.clone();
    let original = plan.entries[0].clone();
    let mut descendant = original.clone();
    descendant.entry_id = "entry_descendant".to_owned();
    descendant.stage_artifact_path =
        ResourcePath::parse("artifacts/entry_descendant/artifact").unwrap();
    descendant.artifact.native_path =
        ResourcePath::parse("rules/repository-guidance.md/child.md").unwrap();
    plan.entries.push(descendant.clone());
    let plan_bytes = rulette::publication::canonical_plan_json(&plan).unwrap();
    let digest = rulette::publication::PlanDigest::from_bytes(&plan_bytes);
    fs::create_dir_all(fixture.stage_dir.join("artifacts/entry_descendant")).unwrap();
    fs::write(
        fixture
            .stage_dir
            .join(descendant.stage_artifact_path.as_str()),
        fs::read(
            fixture
                .stage_dir
                .join(original.stage_artifact_path.as_str()),
        )
        .unwrap(),
    )
    .unwrap();
    fs::write(fixture.stage_dir.join("rulette.plan.json"), plan_bytes).unwrap();
    let mut operation = request(&fixture);
    operation.expected_plan_digest = digest;

    let error = check_plan(operation).unwrap_err();

    assert!(error.to_string().contains("ancestor"));
    assert!(!destination(&fixture).exists());
}

#[cfg(unix)]
#[test]
fn staged_artifact_mode_tampering_fails_before_destination_mutation() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = fixture();
    let artifact_path = fixture
        .stage_dir
        .join(fixture.staged.plan.entries[0].stage_artifact_path.as_str());
    fs::set_permissions(&artifact_path, fs::Permissions::from_mode(0o700)).unwrap();

    let error = check_plan(request(&fixture)).unwrap_err();

    assert!(error.to_string().contains("executable"));
    assert!(!destination(&fixture).exists());
}
