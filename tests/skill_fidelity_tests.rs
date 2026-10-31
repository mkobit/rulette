#![cfg(any(target_os = "linux", target_os = "android"))]

use rulette::emitters::lowering::CapabilityReasonCode;
use rulette::inputs::{ArtifactObservation, InputOrigin};
use rulette::publication::{
    stage, PublicationScope, ScopedAcceptedLoss, ScopedLowering, StageRequest, StageRoot,
};
use rulette::{
    compile_graph, lower, CapabilitySeverity, CompilationGraph, DecoderSelection, FrontendPayload,
    LoweringOptions, NativeFrontend, NativeTarget, Package, PackageKind, PackageRoot, Resource,
    ResourceContent, ResourcePath, SemanticIdentity, SemanticItem, SourceProvenance,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn request<'a>(
    graph: &'a CompilationGraph,
    lowerings: Vec<ScopedLowering<'a>>,
    roots: Vec<StageRoot<'a>>,
    accepted_losses: Vec<ScopedAcceptedLoss<'a>>,
    stage_dir: &'a Path,
) -> StageRequest<'a> {
    StageRequest {
        graph,
        lowerings,
        roots,
        accepted_losses,
        stage_dir,
    }
}

fn make_skill_package(
    frontend: &'static str,
    root: &str,
    name: &str,
    description: &str,
    companion_executable: bool,
    extra_frontmatter: Option<BTreeMap<String, serde_json::Value>>,
) -> Package {
    let primary_resource_path = ResourcePath::parse("SKILL.md").unwrap();
    let primary = Resource::primary_instruction(
        primary_resource_path.clone(),
        ResourceContent::Text(format!("# {name}\nDeploy steps.\n")),
        false,
    );
    let companion_resource_path = ResourcePath::parse("run.sh").unwrap();
    let companion = Resource::opaque(
        companion_resource_path.clone(),
        ResourceContent::Bytes(b"#!/bin/sh\necho deploy\n".to_vec()),
        companion_executable,
    );

    let mut resources = BTreeMap::new();
    resources.insert(primary_resource_path.clone(), primary);
    resources.insert(companion_resource_path, companion);

    let payload = extra_frontmatter.map(|fields| FrontendPayload {
        namespace: format!("{frontend}.skill-frontmatter"),
        fields,
    });

    Package::new(
        PackageKind::Skill,
        SemanticIdentity::parse(format!("skill:{name}")).unwrap(),
        SourceProvenance::new(frontend, format!("{root}/SKILL.md")).unwrap(),
        PackageRoot::parse(root).unwrap(),
        SemanticItem::Skill {
            primary_instruction: primary_resource_path,
            description: description.to_owned(),
            metadata: Default::default(),
            frontend_payload: payload,
        },
        resources,
        None,
    )
    .unwrap()
}

#[test]
fn a01_codex_skill_with_executable_companion_preserves_0755_on_codex_project() {
    let pkg = make_skill_package(
        "codex",
        ".codex/skills/deploy",
        "deploy",
        "Deploy the service",
        true,
        None,
    );
    let graph = CompilationGraph::new([pkg]).unwrap();

    let lowering = lower(&graph, NativeTarget::Codex, LoweringOptions::strict())
        .expect("A01 strict lowering must pass");
    assert_eq!(lowering.artifacts.len(), 2);

    let skill_md = lowering
        .artifacts
        .iter()
        .find(|a| a.path.as_str() == "skills/deploy/SKILL.md")
        .unwrap();
    assert!(!skill_md.executable);

    let run_sh = lowering
        .artifacts
        .iter()
        .find(|a| a.path.as_str() == "skills/deploy/run.sh")
        .unwrap();
    assert!(run_sh.executable);

    assert!(lowering
        .findings
        .iter()
        .all(|f| f.severity == CapabilitySeverity::Supported));

    let temp = tempfile::tempdir().unwrap();
    let live_root = temp.path().join("live");
    let stage_dir = temp.path().join("stage");
    fs::create_dir(&live_root).unwrap();

    let staged = stage(request(
        &graph,
        vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        vec![StageRoot {
            target: NativeTarget::Codex,
            scope: PublicationScope::Project,
            path: &live_root,
        }],
        vec![],
        &stage_dir,
    ))
    .expect("A01 staging must pass under strict mode");

    assert_eq!(staged.plan.entries.len(), 2);
    let staged_run_sh = staged
        .plan
        .entries
        .iter()
        .find(|e| e.artifact.native_path.as_str() == "skills/deploy/run.sh")
        .unwrap();
    assert!(staged_run_sh.executable);
    let mapping =
        rulette::publication::mapping_for(staged_run_sh.target, staged_run_sh.scope).unwrap();
    assert_eq!(
        mapping
            .map_artifact(&staged_run_sh.artifact)
            .unwrap()
            .as_str(),
        ".codex/skills/deploy/run.sh"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let staged_file = stage_dir.join(staged_run_sh.stage_artifact_path.as_str());
        let mode = fs::metadata(staged_file).unwrap().mode();
        assert_ne!(mode & 0o111, 0, "run.sh must have executable bit set");
    }
}

#[test]
fn a02_codex_skill_with_executable_companion_preserves_0755_on_claude_project() {
    let pkg = make_skill_package(
        "codex",
        ".codex/skills/deploy",
        "deploy",
        "Deploy the service",
        true,
        None,
    );
    let graph = CompilationGraph::new([pkg]).unwrap();

    let lowering = lower(&graph, NativeTarget::Claude, LoweringOptions::strict())
        .expect("A02 cross-domain strict lowering must pass");
    assert_eq!(lowering.artifacts.len(), 2);

    let run_sh = lowering
        .artifacts
        .iter()
        .find(|a| a.path.as_str() == "skills/deploy/run.sh")
        .unwrap();
    assert!(run_sh.executable);

    assert!(lowering
        .findings
        .iter()
        .all(|f| f.severity == CapabilitySeverity::Supported));

    let temp = tempfile::tempdir().unwrap();
    let live_root = temp.path().join("live");
    let stage_dir = temp.path().join("stage");
    fs::create_dir(&live_root).unwrap();

    let staged = stage(request(
        &graph,
        vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        vec![StageRoot {
            target: NativeTarget::Claude,
            scope: PublicationScope::Project,
            path: &live_root,
        }],
        vec![],
        &stage_dir,
    ))
    .expect("A02 staging must pass under strict mode");

    let staged_run_sh = staged
        .plan
        .entries
        .iter()
        .find(|e| e.artifact.native_path.as_str() == "skills/deploy/run.sh")
        .unwrap();
    assert!(staged_run_sh.executable);
    let mapping =
        rulette::publication::mapping_for(staged_run_sh.target, staged_run_sh.scope).unwrap();
    assert_eq!(
        mapping
            .map_artifact(&staged_run_sh.artifact)
            .unwrap()
            .as_str(),
        ".claude/skills/deploy/run.sh"
    );
}

#[test]
fn a03_claude_skill_with_executable_companion_preserves_0755_on_codex_user() {
    let pkg = make_skill_package(
        "claude",
        ".claude/skills/deploy",
        "deploy",
        "Deploy the service",
        true,
        None,
    );
    let graph = CompilationGraph::new([pkg]).unwrap();

    let lowering = lower(&graph, NativeTarget::Codex, LoweringOptions::strict())
        .expect("A03 strict lowering must pass");
    assert_eq!(lowering.artifacts.len(), 2);

    let run_sh = lowering
        .artifacts
        .iter()
        .find(|a| a.path.as_str() == "skills/deploy/run.sh")
        .unwrap();
    assert!(run_sh.executable);

    let temp = tempfile::tempdir().unwrap();
    let live_root = temp.path().join("user_live");
    let stage_dir = temp.path().join("stage");
    fs::create_dir(&live_root).unwrap();

    let staged = stage(request(
        &graph,
        vec![ScopedLowering {
            scope: PublicationScope::User,
            lowering: &lowering,
        }],
        vec![StageRoot {
            target: NativeTarget::Codex,
            scope: PublicationScope::User,
            path: &live_root,
        }],
        vec![],
        &stage_dir,
    ))
    .expect("A03 user scope staging must pass under strict mode");

    let staged_run_sh = staged
        .plan
        .entries
        .iter()
        .find(|e| e.artifact.native_path.as_str() == "skills/deploy/run.sh")
        .unwrap();
    assert!(staged_run_sh.executable);
}

#[test]
fn a04_codex_skill_lowered_to_cursor_drops_companion_and_reports_findings() {
    let pkg = make_skill_package(
        "codex",
        ".codex/skills/deploy",
        "deploy",
        "Deploy the service",
        true,
        None,
    );
    let graph = CompilationGraph::new([pkg]).unwrap();

    assert!(
        lower(&graph, NativeTarget::Cursor, LoweringOptions::strict()).is_err(),
        "A04 strict lowering must fail for Cursor"
    );

    let lowering = lower(&graph, NativeTarget::Cursor, LoweringOptions::allow_lossy())
        .expect("A04 allow_lossy lowering must succeed");

    assert_eq!(lowering.artifacts.len(), 1);
    assert_eq!(lowering.artifacts[0].path.as_str(), "rules/deploy.mdc");
    assert!(!lowering.artifacts[0].executable);

    assert!(lowering.findings.iter().any(|f| {
        f.severity == CapabilitySeverity::Lossy
            && f.reason_code == CapabilityReasonCode::SkillLoweredAsRule
    }));

    assert!(lowering.findings.iter().any(|f| {
        f.severity == CapabilitySeverity::Dropped
            && f.reason_code == CapabilityReasonCode::OpaqueResourceUnrepresentable
            && f.resource_path
                .as_ref()
                .is_some_and(|p| p.as_str() == "run.sh")
    }));

    assert!(lowering.findings.iter().any(|f| {
        f.severity == CapabilitySeverity::Lossy
            && f.reason_code == CapabilityReasonCode::ExecutableBitUnrepresentable
            && f.resource_path
                .as_ref()
                .is_some_and(|p| p.as_str() == "run.sh")
    }));
}

#[test]
fn a05_codex_skill_with_unmapped_frontmatter_reports_loss_cross_domain_and_preserves_same_domain() {
    let mut extra = BTreeMap::new();
    extra.insert(
        "custom_key".to_string(),
        serde_json::Value::String("custom_val".to_string()),
    );
    let pkg = make_skill_package(
        "codex",
        ".codex/skills/deploy",
        "deploy",
        "Deploy the service",
        false,
        Some(extra),
    );
    let graph = CompilationGraph::new([pkg]).unwrap();

    assert!(
        lower(&graph, NativeTarget::Claude, LoweringOptions::strict()).is_err(),
        "A05 strict cross-domain lowering must fail due to unmapped frontmatter"
    );

    let claude_lowering = lower(&graph, NativeTarget::Claude, LoweringOptions::allow_lossy())
        .expect("A05 allow_lossy lowering must succeed");
    assert!(claude_lowering.findings.iter().any(|f| {
        f.severity == CapabilitySeverity::Lossy
            && f.reason_code == CapabilityReasonCode::SkillFrontmatterUnrepresentable
    }));
    let claude_skill_md = std::str::from_utf8(&claude_lowering.artifacts[0].bytes).unwrap();
    assert!(!claude_skill_md.contains("custom_key"));

    let codex_lowering = lower(&graph, NativeTarget::Codex, LoweringOptions::strict())
        .expect("A05 same-domain lowering must preserve frontmatter under strict mode");
    assert!(codex_lowering
        .findings
        .iter()
        .all(|f| f.severity == CapabilitySeverity::Supported));
    let codex_skill_md = std::str::from_utf8(&codex_lowering.artifacts[0].bytes).unwrap();
    assert!(codex_skill_md.contains("custom_key"));
    assert!(codex_skill_md.contains("custom_val"));
}

#[test]
fn a06_instruction_markdown_with_executable_bit_reports_lossy_and_drops_bit() {
    let package = Package::rule(
        SemanticIdentity::parse("rule:repository-guidance").unwrap(),
        SourceProvenance::new("codex", "AGENTS.md").unwrap(),
        Resource::primary_instruction(
            ResourcePath::parse("AGENTS.md").unwrap(),
            ResourceContent::Text("# Guidance\n".to_owned()),
            true,
        ),
    )
    .unwrap();
    let graph = CompilationGraph::new([package]).unwrap();

    assert!(
        lower(&graph, NativeTarget::Claude, LoweringOptions::strict()).is_err(),
        "A06 strict lowering must fail when instruction markdown has executable bit"
    );

    let lowering = lower(&graph, NativeTarget::Claude, LoweringOptions::allow_lossy())
        .expect("A06 allow_lossy lowering must succeed");
    assert_eq!(lowering.artifacts.len(), 1);
    assert_eq!(lowering.artifacts[0].path.as_str(), "CLAUDE.md");
    assert!(!lowering.artifacts[0].executable);

    assert!(lowering.findings.iter().any(|f| {
        f.severity == CapabilitySeverity::Lossy
            && f.reason_code == CapabilityReasonCode::ExecutableBitUnrepresentable
            && f.resource_path
                .as_ref()
                .is_some_and(|p| p.as_str() == "AGENTS.md")
            && f.artifact_path
                .as_ref()
                .is_some_and(|p| p.as_str() == "CLAUDE.md")
    }));
}

#[test]
fn tar_archive_skill_input_preserves_executable_permission_across_targets() {
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("source.tar");
    let mut tar = tar::Builder::new(Vec::new());

    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Regular);
    header.set_mode(0o644);
    let skill_md_content = b"---\nname: deploy\ndescription: Deploy app\n---\n# Deploy\n";
    header.set_size(skill_md_content.len() as u64);
    header.set_cksum();
    tar.append_data(
        &mut header,
        ".codex/skills/deploy/SKILL.md",
        &skill_md_content[..],
    )
    .unwrap();

    let mut header2 = tar::Header::new_gnu();
    header2.set_entry_type(tar::EntryType::Regular);
    header2.set_mode(0o755);
    let run_sh_content = b"#!/bin/sh\nexit 0\n";
    header2.set_size(run_sh_content.len() as u64);
    header2.set_cksum();
    tar.append_data(
        &mut header2,
        ".codex/skills/deploy/run.sh",
        &run_sh_content[..],
    )
    .unwrap();

    fs::write(&archive_path, tar.into_inner().unwrap()).unwrap();

    let observations = rulette::inputs::observe_path(&archive_path).unwrap();
    let graph = rulette::compile_graph(
        &observations,
        rulette::DecoderSelection::Native(rulette::NativeFrontend::Codex),
    )
    .unwrap();

    let lowering = lower(&graph, NativeTarget::Claude, LoweringOptions::strict())
        .expect("tar input skill lowering must succeed strictly");
    assert_eq!(lowering.artifacts.len(), 2);

    let run_sh = lowering
        .artifacts
        .iter()
        .find(|a| a.path.as_str() == "skills/deploy/run.sh")
        .unwrap();
    assert!(
        run_sh.executable,
        "tar entry 0755 must be preserved on artifact"
    );
}

fn make_obs(path: &str, bytes: &[u8], executable: bool) -> ArtifactObservation {
    ArtifactObservation::new(
        bytes.to_vec(),
        path,
        executable,
        InputOrigin::Filesystem,
        "test-input",
        None,
    )
    .unwrap()
}

#[test]
fn a07_mixed_agents_md_and_agents_skills_auto_detects_codex_and_emits_claude_project() {
    let obs = [
        make_obs("AGENTS.md", b"# Rule\nFollow guidelines.\n", false),
        make_obs(
            ".agents/skills/deploy/SKILL.md",
            b"---\nname: deploy\ndescription: Deploy app\n---\n# Deploy\n",
            false,
        ),
        make_obs(
            ".agents/skills/deploy/run.sh",
            b"#!/bin/sh\necho deploy\n",
            true,
        ),
    ];

    let graph = compile_graph(&obs, DecoderSelection::Auto)
        .expect("A07 auto-detection must resolve unambiguously to codex");
    assert_eq!(graph.packages.len(), 2);

    let lowering = lower(&graph, NativeTarget::Claude, LoweringOptions::strict())
        .expect("A07 strict lowering to claude must pass");
    assert_eq!(lowering.artifacts.len(), 3);

    let temp = tempfile::tempdir().unwrap();
    let live_root = temp.path().join("live");
    let stage_dir = temp.path().join("stage");
    fs::create_dir(&live_root).unwrap();

    let staged = stage(request(
        &graph,
        vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        vec![StageRoot {
            target: NativeTarget::Claude,
            scope: PublicationScope::Project,
            path: &live_root,
        }],
        vec![],
        &stage_dir,
    ))
    .expect("A07 stage must succeed");
    assert_eq!(staged.plan.entries.len(), 3);

    let run_sh = staged
        .plan
        .entries
        .iter()
        .find(|e| e.artifact.native_path.as_str() == "skills/deploy/run.sh")
        .unwrap();
    assert!(run_sh.executable);
    let mapping = rulette::publication::mapping_for(run_sh.target, run_sh.scope).unwrap();
    assert_eq!(
        mapping.map_artifact(&run_sh.artifact).unwrap().as_str(),
        ".claude/skills/deploy/run.sh"
    );

    let claude_md = staged
        .plan
        .entries
        .iter()
        .find(|e| e.artifact.native_path.as_str() == "CLAUDE.md")
        .unwrap();
    assert_eq!(
        mapping.map_artifact(&claude_md.artifact).unwrap().as_str(),
        "CLAUDE.md"
    );
}

#[test]
fn a08_mixed_cursor_rules_and_agents_skills_auto_detects_cursor_and_emits_antigravity() {
    let obs = [
        make_obs(
            ".cursor/rules/deploy.mdc",
            b"---\ndescription: Deployment guidance\nglobs: [\"deploy/**\"]\n---\nGuidance content.\n",
            false,
        ),
        make_obs(
            ".agents/skills/deploy/SKILL.md",
            b"---\nname: deploy\ndescription: Deploy app\n---\n# Deploy\n",
            false,
        ),
        make_obs(
            ".agents/skills/deploy/run.sh",
            b"#!/bin/sh\necho deploy\n",
            true,
        ),
    ];

    let graph = compile_graph(&obs, DecoderSelection::Auto)
        .expect("A08 auto-detection must resolve unambiguously to cursor-mdc");
    assert_eq!(graph.packages.len(), 2);

    let lowering = lower(&graph, NativeTarget::Antigravity, LoweringOptions::strict())
        .expect("A08 strict lowering to antigravity must pass");
    assert_eq!(lowering.artifacts.len(), 3);

    let temp = tempfile::tempdir().unwrap();
    let live_root = temp.path().join("live");
    let stage_dir = temp.path().join("stage");
    fs::create_dir(&live_root).unwrap();

    let staged = stage(request(
        &graph,
        vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        vec![StageRoot {
            target: NativeTarget::Antigravity,
            scope: PublicationScope::Project,
            path: &live_root,
        }],
        vec![],
        &stage_dir,
    ))
    .expect("A08 stage must succeed");
    assert_eq!(staged.plan.entries.len(), 3);

    let run_sh = staged
        .plan
        .entries
        .iter()
        .find(|e| e.artifact.native_path.as_str() == "skills/deploy/run.sh")
        .unwrap();
    let mapping = rulette::publication::mapping_for(run_sh.target, run_sh.scope).unwrap();
    assert_eq!(
        mapping.map_artifact(&run_sh.artifact).unwrap().as_str(),
        ".agents/skills/deploy/run.sh"
    );
}

#[test]
fn a09_pure_agents_skills_snapshot_compiles_with_explicit_codex_and_auto() {
    let obs = [
        make_obs(
            ".agents/skills/deploy/SKILL.md",
            b"---\nname: deploy\ndescription: Deploy app\n---\n# Deploy\n",
            false,
        ),
        make_obs(
            ".agents/skills/deploy/run.sh",
            b"#!/bin/sh\necho deploy\n",
            true,
        ),
    ];

    // Explicit --from codex
    let graph_explicit = compile_graph(&obs, DecoderSelection::Native(NativeFrontend::Codex))
        .expect("A09 explicit codex must pass");
    assert_eq!(graph_explicit.packages.len(), 1);

    // Auto-detection resolves to default skill frontend (Codex)
    let graph_auto = compile_graph(&obs, DecoderSelection::Auto)
        .expect("A09 auto-detection on standalone skills must resolve to codex");
    assert_eq!(graph_auto.packages.len(), 1);

    let lowering = lower(
        &graph_explicit,
        NativeTarget::Codex,
        LoweringOptions::strict(),
    )
    .expect("A09 strict lowering to codex must pass");
    assert_eq!(lowering.artifacts.len(), 2);

    let temp = tempfile::tempdir().unwrap();
    let live_root = temp.path().join("live");
    let stage_dir = temp.path().join("stage");
    fs::create_dir(&live_root).unwrap();

    let staged = stage(request(
        &graph_explicit,
        vec![ScopedLowering {
            scope: PublicationScope::Project,
            lowering: &lowering,
        }],
        vec![StageRoot {
            target: NativeTarget::Codex,
            scope: PublicationScope::Project,
            path: &live_root,
        }],
        vec![],
        &stage_dir,
    ))
    .expect("A09 stage must succeed");
    assert_eq!(staged.plan.entries.len(), 2);

    let run_sh = staged
        .plan
        .entries
        .iter()
        .find(|e| e.artifact.native_path.as_str() == "skills/deploy/run.sh")
        .unwrap();
    let mapping = rulette::publication::mapping_for(run_sh.target, run_sh.scope).unwrap();
    assert_eq!(
        mapping.map_artifact(&run_sh.artifact).unwrap().as_str(),
        ".codex/skills/deploy/run.sh"
    );
}

#[test]
fn a10_codex_skills_and_agents_skills_duplicate_skill_collides_before_lowering() {
    let obs = [
        make_obs(
            ".codex/skills/deploy/SKILL.md",
            b"---\nname: deploy\ndescription: Deploy app (codex)\n---\n# Deploy\n",
            false,
        ),
        make_obs(
            ".agents/skills/deploy/SKILL.md",
            b"---\nname: deploy\ndescription: Deploy app (agents)\n---\n# Deploy\n",
            false,
        ),
    ];

    let error = compile_graph(&obs, DecoderSelection::Native(NativeFrontend::Codex))
        .expect_err("A10 duplicate semantic identity must fail before lowering");
    assert!(
        error.to_string().contains("skill:deploy"),
        "error must mention colliding skill identity: {error}"
    );
}

#[test]
fn a11_skill_with_symlink_companion_is_rejected_at_observation() {
    let temp = tempfile::tempdir().unwrap();
    let skill_dir = temp.path().join("deploy");
    fs::create_dir(&skill_dir).unwrap();
    fs::write(
        skill_dir.join("SKILL.md"),
        b"---\nname: deploy\ndescription: Deploy app\n---\n# Deploy\n",
    )
    .unwrap();

    let target = temp.path().join("target.sh");
    fs::write(&target, b"#!/bin/sh\nexit 0\n").unwrap();
    std::os::unix::fs::symlink(&target, skill_dir.join("run.sh")).unwrap();

    let error = rulette::inputs::observe_path(&skill_dir)
        .expect_err("A11 symlinked companion must be rejected during observation");
    assert!(
        error.to_string().contains("symlink"),
        "error must report symlink: {error}"
    );
}

#[test]
fn a12_valid_skill_package_rejected_at_cursor_user_target() {
    let pkg = make_skill_package(
        "codex",
        ".codex/skills/deploy",
        "deploy",
        "Deploy the service",
        true,
        None,
    );
    let graph = CompilationGraph::new([pkg]).unwrap();
    let lowering = lower(&graph, NativeTarget::Cursor, LoweringOptions::allow_lossy()).unwrap();

    let temp = tempfile::tempdir().unwrap();
    let live_root = temp.path().join("live");
    let stage_dir = temp.path().join("stage");
    fs::create_dir(&live_root).unwrap();

    let error = stage(request(
        &graph,
        vec![ScopedLowering {
            scope: PublicationScope::User,
            lowering: &lowering,
        }],
        vec![StageRoot {
            target: NativeTarget::Cursor,
            scope: PublicationScope::User,
            path: &live_root,
        }],
        vec![],
        &stage_dir,
    ))
    .expect_err("A12 cursor@user must be rejected as unsupported scope");
    assert!(
        error
            .to_string()
            .contains("user mapping is unavailable for target `cursor`"),
        "error must report unavailable user mapping for cursor: {error}"
    );
}

#[test]
fn direct_directory_skill_inputs_supported_across_single_and_multi_skill_layouts() {
    // Single skill direct input (SKILL.md + run.sh at root ".")
    let single_obs = [
        make_obs(
            "SKILL.md",
            b"---\nname: single\ndescription: Single direct skill\n---\n# Single\n",
            false,
        ),
        make_obs("run.sh", b"#!/bin/sh\necho single\n", true),
    ];
    let graph_single = compile_graph(&single_obs, DecoderSelection::Auto).unwrap();
    assert_eq!(graph_single.packages.len(), 1);
    let lowering_single = lower(
        &graph_single,
        NativeTarget::Claude,
        LoweringOptions::strict(),
    )
    .unwrap();
    assert_eq!(lowering_single.artifacts.len(), 2);

    // Multi-skill direct input (<name>/SKILL.md + <name>/run.sh)
    let multi_obs = [
        make_obs(
            "alpha/SKILL.md",
            b"---\nname: alpha\ndescription: Alpha direct skill\n---\n# Alpha\n",
            false,
        ),
        make_obs("alpha/run.sh", b"#!/bin/sh\necho alpha\n", true),
        make_obs(
            "beta/SKILL.md",
            b"---\nname: beta\ndescription: Beta direct skill\n---\n# Beta\n",
            false,
        ),
    ];
    let graph_multi = compile_graph(&multi_obs, DecoderSelection::Auto).unwrap();
    assert_eq!(graph_multi.packages.len(), 2);
    let lowering_multi =
        lower(&graph_multi, NativeTarget::Codex, LoweringOptions::strict()).unwrap();
    assert_eq!(lowering_multi.artifacts.len(), 3);
}

#[test]
fn codex_auto_detected_skill_ingests_openai_yaml_sidecar_without_unrecognized_diagnostics() {
    let obs = [
        make_obs(
            ".agents/skills/reviewer/SKILL.md",
            b"---\nname: reviewer\ndescription: Review pull requests\n---\n# Reviewer\n",
            false,
        ),
        make_obs(
            ".agents/skills/reviewer/agents/openai.yaml",
            br##"interface:
  display_name: "PR Reviewer"
  icon_small: "./assets/icon.svg"
  brand_color: "#10A37F"
  default_prompt: "Review staged diff."
dependencies:
  tools:
    - type: "mcp"
      value: "github"
policy:
  allow_implicit_invocation: false
"##,
            false,
        ),
    ];

    let graph = compile_graph(&obs, DecoderSelection::Auto)
        .expect("auto-detection compiles .agents/skills with agents/openai.yaml cleanly");
    assert!(
        graph.diagnostics.is_empty(),
        "agents/openai.yaml must be consumed as PackageContent with zero diagnostics"
    );
    assert_eq!(graph.packages.len(), 1);
    let pkg = graph.packages.values().next().unwrap();
    assert!(
        !pkg.resources
            .contains_key(&ResourcePath::parse("agents/openai.yaml").unwrap()),
        "sidecar should populate SkillMetadata rather than remain an opaque resource"
    );
    let SemanticItem::Skill { metadata, .. } = &pkg.semantic_item else {
        panic!("expected skill package");
    };
    assert_eq!(metadata.display_name.as_deref(), Some("PR Reviewer"));
    assert_eq!(metadata.icon.as_deref(), Some("./assets/icon.svg"));
    assert_eq!(metadata.color.as_deref(), Some("#10A37F"));
    assert_eq!(
        metadata.default_prompt.as_deref(),
        Some("Review staged diff.")
    );
    assert_eq!(metadata.disable_model_invocation, Some(true));
    assert_eq!(metadata.allowed_tools, Some(vec!["mcp:github".to_owned()]));
}
