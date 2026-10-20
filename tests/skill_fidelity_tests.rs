#![cfg(any(target_os = "linux", target_os = "android"))]

use rulette::emitters::lowering::CapabilityReasonCode;
use rulette::publication::{
    stage, PublicationScope, ScopedAcceptedLoss, ScopedLowering, StageRequest, StageRoot,
};
use rulette::{
    lower, CapabilitySeverity, CompilationGraph, FrontendPayload, LoweringOptions, NativeTarget,
    Package, PackageKind, PackageRoot, Resource, ResourceContent, ResourcePath, SemanticIdentity,
    SemanticItem, SourceProvenance,
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
