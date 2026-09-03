#![cfg(any(target_os = "linux", target_os = "android"))]

use rulette::publication::{
    canonical_plan_json, parse_plan_with_expected_digest, stage, PlanDigest, PublicationScope,
    ScopedAcceptedLoss, ScopedLowering, StageRequest, StageRoot,
};
use rulette::{
    lower, CapabilitySeverity, CompilationGraph, LoweringOptions, NativeTarget, Package, Resource,
    ResourceContent, ResourcePath, SemanticIdentity, SourceProvenance,
};
use std::fs;

fn graph_with_rule(executable: bool) -> CompilationGraph {
    let package = Package::rule(
        SemanticIdentity::parse("rule:repository-guidance").unwrap(),
        SourceProvenance::new("codex", "AGENTS.md").unwrap(),
        Resource::primary_instruction(
            ResourcePath::parse("AGENTS.md").unwrap(),
            ResourceContent::Text("# Repository guidance\n".to_owned()),
            executable,
        ),
    )
    .unwrap();
    CompilationGraph::new([package]).unwrap()
}

fn request<'a>(
    graph: &'a CompilationGraph,
    lowerings: Vec<ScopedLowering<'a>>,
    roots: Vec<StageRoot<'a>>,
    accepted_losses: Vec<ScopedAcceptedLoss<'a>>,
    stage_dir: &'a std::path::Path,
) -> StageRequest<'a> {
    StageRequest {
        graph,
        lowerings,
        roots,
        accepted_losses,
        stage_dir,
    }
}

#[test]
fn stages_only_artifacts_and_a_digest_bearing_canonical_plan_without_touching_live_roots() {
    let temporary = tempfile::tempdir().unwrap();
    let live_root = temporary.path().join("live");
    let stage_dir = temporary.path().join("stage");
    fs::create_dir(&live_root).unwrap();
    fs::write(live_root.join("sentinel.txt"), "leave me alone").unwrap();
    let graph = graph_with_rule(false);
    let lowering = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();

    let staged = stage(request(
        &graph,
        vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        vec![StageRoot {
            target: NativeTarget::OpenCode,
            scope: PublicationScope::Project,
            path: &live_root,
        }],
        vec![],
        &stage_dir,
    ))
    .unwrap();

    let mut layout = fs::read_dir(&stage_dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<Vec<_>>();
    layout.sort();
    assert_eq!(layout, ["artifacts", "rulette.plan.json"]);
    assert_eq!(staged.plan.entries.len(), 1);
    assert!(!staged.plan.allow_lossy);
    assert_eq!(
        fs::read(stage_dir.join(staged.plan.entries[0].stage_artifact_path.as_str())).unwrap(),
        lowering.artifacts[0].bytes
    );
    let plan_bytes = fs::read(stage_dir.join("rulette.plan.json")).unwrap();
    assert_eq!(plan_bytes, canonical_plan_json(&staged.plan).unwrap());
    assert_eq!(
        staged.plan_digest.as_str(),
        PlanDigest::from_bytes(&plan_bytes).as_str()
    );
    assert_eq!(
        parse_plan_with_expected_digest(&plan_bytes, &staged.plan_digest)
            .unwrap()
            .entries,
        staged.plan.entries
    );
    assert_eq!(
        fs::read_to_string(live_root.join("sentinel.txt")).unwrap(),
        "leave me alone"
    );
    assert!(!live_root.join(".opencode").exists());
}

#[test]
fn independent_stages_have_identical_canonical_plan_and_artifact_bytes() {
    let temporary = tempfile::tempdir().unwrap();
    let live_root = temporary.path().join("live");
    let first_stage_dir = temporary.path().join("stage-one");
    let second_stage_dir = temporary.path().join("stage-two");
    fs::create_dir(&live_root).unwrap();
    let graph = graph_with_rule(false);
    let lowering = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();

    let first = stage(request(
        &graph,
        vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        vec![StageRoot {
            target: NativeTarget::OpenCode,
            scope: PublicationScope::Project,
            path: &live_root,
        }],
        vec![],
        &first_stage_dir,
    ))
    .unwrap();
    let second = stage(request(
        &graph,
        vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        vec![StageRoot {
            target: NativeTarget::OpenCode,
            scope: PublicationScope::Project,
            path: &live_root,
        }],
        vec![],
        &second_stage_dir,
    ))
    .unwrap();

    let first_plan = fs::read(first_stage_dir.join("rulette.plan.json")).unwrap();
    let second_plan = fs::read(second_stage_dir.join("rulette.plan.json")).unwrap();
    assert_eq!(first_plan, second_plan);
    assert_eq!(first.plan_digest, second.plan_digest);
    assert_eq!(first.plan.entries, second.plan.entries);
    assert_eq!(
        fs::read(first_stage_dir.join(first.plan.entries[0].stage_artifact_path.as_str())).unwrap(),
        fs::read(second_stage_dir.join(second.plan.entries[0].stage_artifact_path.as_str()))
            .unwrap()
    );
}

#[test]
fn rejects_an_existing_stage_directory_without_changing_it() {
    let temporary = tempfile::tempdir().unwrap();
    let live_root = temporary.path().join("live");
    let stage_dir = temporary.path().join("stage");
    fs::create_dir(&live_root).unwrap();
    fs::create_dir(&stage_dir).unwrap();
    fs::write(stage_dir.join("sentinel.txt"), "keep").unwrap();
    let graph = graph_with_rule(false);
    let lowering = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();

    let error = stage(request(
        &graph,
        vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        vec![StageRoot {
            target: NativeTarget::OpenCode,
            scope: PublicationScope::Project,
            path: &live_root,
        }],
        vec![],
        &stage_dir,
    ))
    .unwrap_err();

    assert!(error.to_string().contains("already exists"));
    assert_eq!(
        fs::read_to_string(stage_dir.join("sentinel.txt")).unwrap(),
        "keep"
    );
}

#[test]
fn rejects_a_stage_directory_inside_a_live_root() {
    let temporary = tempfile::tempdir().unwrap();
    let live_root = temporary.path().join("live");
    let stage_dir = live_root.join("stage");
    fs::create_dir(&live_root).unwrap();
    fs::write(live_root.join("sentinel.txt"), "leave me alone").unwrap();
    let graph = graph_with_rule(false);
    let lowering = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();

    let error = stage(request(
        &graph,
        vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        vec![StageRoot {
            target: NativeTarget::OpenCode,
            scope: PublicationScope::Project,
            path: &live_root,
        }],
        vec![],
        &stage_dir,
    ))
    .unwrap_err();

    assert!(error.to_string().contains("must not be contained"));
    assert!(!stage_dir.exists());
    assert_eq!(
        fs::read_to_string(live_root.join("sentinel.txt")).unwrap(),
        "leave me alone"
    );
}

#[test]
fn multi_target_stage_preflight_failure_creates_no_partial_stage() {
    let temporary = tempfile::tempdir().unwrap();
    let live_root = temporary.path().join("live");
    let stage_dir = temporary.path().join("stage");
    fs::create_dir(&live_root).unwrap();
    fs::write(live_root.join("sentinel.txt"), "leave me alone").unwrap();
    let graph = graph_with_rule(false);
    let codex = lower(&graph, NativeTarget::Codex, LoweringOptions::strict()).unwrap();
    let opencode = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();

    let error = stage(request(
        &graph,
        vec![
            ScopedLowering {
                scope: PublicationScope::Project,
                lowering: &codex,
            },
            ScopedLowering {
                scope: PublicationScope::Project,
                lowering: &opencode,
            },
        ],
        vec![StageRoot {
            target: NativeTarget::Codex,
            scope: PublicationScope::Project,
            path: &live_root,
        }],
        vec![],
        &stage_dir,
    ))
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("every scoped lowering requires exactly one live root binding"));
    assert!(!stage_dir.exists());
    assert_eq!(
        fs::read_to_string(live_root.join("sentinel.txt")).unwrap(),
        "leave me alone"
    );
}

#[test]
fn stage_collision_fails_before_creating_stage_or_touching_live_output() {
    let temporary = tempfile::tempdir().unwrap();
    let live_root = temporary.path().join("live");
    let stage_dir = temporary.path().join("stage");
    fs::create_dir(&live_root).unwrap();
    fs::write(live_root.join("sentinel.txt"), "keep me").unwrap();
    let graph = graph_with_rule(false);
    let lowering = lower(&graph, NativeTarget::Codex, LoweringOptions::strict()).unwrap();

    let error = stage(request(
        &graph,
        vec![
            ScopedLowering {
                scope: PublicationScope::Project,
                lowering: &lowering,
            },
            ScopedLowering {
                scope: PublicationScope::Project,
                lowering: &lowering,
            },
        ],
        vec![StageRoot {
            target: NativeTarget::Codex,
            scope: PublicationScope::Project,
            path: &live_root,
        }],
        vec![],
        &stage_dir,
    ))
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("duplicate target and scope lowerings"));
    assert!(!stage_dir.exists());
    assert_eq!(
        fs::read_to_string(live_root.join("sentinel.txt")).unwrap(),
        "keep me"
    );
}

#[test]
fn multi_target_stage_keeps_independent_artifacts_and_package_ids() {
    let temporary = tempfile::tempdir().unwrap();
    let live_root = temporary.path().join("live");
    let stage_dir = temporary.path().join("stage");
    fs::create_dir(&live_root).unwrap();
    let graph = graph_with_rule(false);
    let codex = lower(&graph, NativeTarget::Codex, LoweringOptions::strict()).unwrap();
    let opencode = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();

    let staged = stage(request(
        &graph,
        vec![
            ScopedLowering {
                scope: PublicationScope::Project,
                lowering: &codex,
            },
            ScopedLowering {
                scope: PublicationScope::Project,
                lowering: &opencode,
            },
        ],
        vec![
            StageRoot {
                target: NativeTarget::Codex,
                scope: PublicationScope::Project,
                path: &live_root,
            },
            StageRoot {
                target: NativeTarget::OpenCode,
                scope: PublicationScope::Project,
                path: &live_root,
            },
        ],
        vec![],
        &stage_dir,
    ))
    .unwrap();

    assert_eq!(staged.plan.entries.len(), 2);
    for entry in &staged.plan.entries {
        let lowering = match entry.target {
            NativeTarget::Codex => &codex,
            NativeTarget::OpenCode => &opencode,
            target => panic!("unexpected target {target:?}"),
        };
        let artifact = lowering
            .artifacts
            .iter()
            .find(|artifact| artifact.path == entry.artifact.native_path)
            .unwrap();
        assert_eq!(entry.source_package, artifact.source_package);
        assert_eq!(
            fs::read(stage_dir.join(entry.stage_artifact_path.as_str())).unwrap(),
            artifact.bytes
        );
    }
}

#[test]
fn records_accepted_loss_with_a_scoped_entry_identifier() {
    let temporary = tempfile::tempdir().unwrap();
    let project_root = temporary.path().join("project");
    let user_root = temporary.path().join("user");
    let stage_dir = temporary.path().join("stage");
    fs::create_dir(&project_root).unwrap();
    fs::create_dir(&user_root).unwrap();
    let graph = graph_with_rule(true);
    let lowering = lower(
        &graph,
        NativeTarget::OpenCode,
        LoweringOptions::allow_lossy(),
    )
    .unwrap();
    let accepted_losses = lowering
        .findings
        .iter()
        .filter(|finding| finding.severity != CapabilitySeverity::Supported)
        .flat_map(|finding| {
            [PublicationScope::Project, PublicationScope::User]
                .map(move |scope| ScopedAcceptedLoss { scope, finding })
        })
        .collect();

    let staged = stage(request(
        &graph,
        vec![
            ScopedLowering {
                scope: PublicationScope::Project,
                lowering: &lowering,
            },
            ScopedLowering {
                scope: PublicationScope::User,
                lowering: &lowering,
            },
        ],
        vec![
            StageRoot {
                target: NativeTarget::OpenCode,
                scope: PublicationScope::Project,
                path: &project_root,
            },
            StageRoot {
                target: NativeTarget::OpenCode,
                scope: PublicationScope::User,
                path: &user_root,
            },
        ],
        accepted_losses,
        &stage_dir,
    ))
    .unwrap();

    assert!(staged.plan.allow_lossy);
    assert_eq!(staged.plan.losses.len(), 4);
    let loss_ids = staged
        .plan
        .losses
        .iter()
        .map(|loss| &loss.id)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(loss_ids.len(), staged.plan.losses.len());
    for loss in &staged.plan.losses {
        let entry_id = loss
            .entry_id
            .as_ref()
            .expect("artifact loss has an entry ID");
        let entry = staged
            .plan
            .entries
            .iter()
            .find(|entry| &entry.entry_id == entry_id)
            .expect("loss entry ID references a staged entry");
        assert_eq!(loss.artifact.as_ref(), Some(&entry.artifact));
    }
}

#[test]
fn rejects_unaccepted_lowering_loss_without_creating_a_stage() {
    let temporary = tempfile::tempdir().unwrap();
    let live_root = temporary.path().join("live");
    let stage_dir = temporary.path().join("stage");
    fs::create_dir(&live_root).unwrap();
    let graph = graph_with_rule(true);
    let lowering = lower(
        &graph,
        NativeTarget::OpenCode,
        LoweringOptions::allow_lossy(),
    )
    .unwrap();

    let error = stage(request(
        &graph,
        vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        vec![StageRoot {
            target: NativeTarget::OpenCode,
            scope: PublicationScope::Project,
            path: &live_root,
        }],
        vec![],
        &stage_dir,
    ))
    .unwrap_err();

    assert!(error.to_string().contains("unaccepted capability loss"));
    assert!(!stage_dir.exists());
}
