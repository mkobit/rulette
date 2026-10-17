#![cfg(any(target_os = "linux", target_os = "android"))]

use rulette::publication::{
    canonical_plan_json, parse_plan_with_expected_digest, stage, PlanDigest, PublicationScope,
    ScopedAcceptedLoss, ScopedLowering, StageRequest, StageRoot,
};
use rulette::{
    lower, ActivationMode, CapabilitySeverity, CompilationGraph, LoweringOptions, NativeTarget,
    Package, PortableActivation, Resource, ResourceContent, ResourcePath, SemanticIdentity,
    SemanticItem, SourceProvenance, TargetActivation, TargetActivationOverrides,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

const DESCRIPTION: &str = "rule-description-unrepresentable";
const ACTIVATION: &str = "rule-activation-unrepresentable";
const TARGETS: [NativeTarget; 5] = [
    NativeTarget::Codex,
    NativeTarget::Claude,
    NativeTarget::OpenCode,
    NativeTarget::Cursor,
    NativeTarget::Antigravity,
];

fn activation(mode: Vec<ActivationMode>) -> PortableActivation {
    PortableActivation {
        mode,
        globs: None,
        pattern: None,
        description: None,
    }
}

fn graph(description: Option<&str>, activation: Option<TargetActivation>) -> CompilationGraph {
    let mut package = Package::rule(
        SemanticIdentity::parse("rule:guidance").unwrap(),
        SourceProvenance::new("cursor", ".cursor/rules/guidance.mdc").unwrap(),
        Resource::primary_instruction(
            ResourcePath::parse("guidance.md").unwrap(),
            ResourceContent::Text("Keep the body.\n".into()),
            false,
        ),
    )
    .unwrap();
    let SemanticItem::Rule {
        description: rule_description,
        activation: rule_activation,
        ..
    } = &mut package.semantic_item
    else {
        unreachable!()
    };
    *rule_description = description.map(str::to_owned);
    *rule_activation = activation;
    CompilationGraph::new([package]).unwrap()
}

fn assert_activation_loss(
    target: NativeTarget,
    description: Option<&str>,
    activation: PortableActivation,
    fields: &[&str],
) {
    let graph = graph(description, Some(TargetActivation::Bare(activation)));
    let plan = lower(&graph, target, LoweringOptions::allow_lossy()).unwrap();
    let findings: Vec<_> = plan
        .findings
        .iter()
        .filter(|finding| finding.reason_code.as_str() == ACTIVATION)
        .collect();
    if fields.is_empty() {
        assert!(findings.is_empty(), "{target:?}: {findings:?}");
        lower(&graph, target, LoweringOptions::strict()).unwrap();
    } else {
        assert_eq!(findings.len(), 1, "{target:?}: {:?}", plan.findings);
        let reason = &findings[0].reason;
        let mut previous_end = 0;
        for field in fields {
            let position = previous_end
                + reason[previous_end..]
                    .find(field)
                    .unwrap_or_else(|| panic!("{reason}: {field}"));
            assert!(position >= previous_end, "unstable field order: {reason}");
            previous_end = position + field.len();
        }
        let error = lower(&graph, target, LoweringOptions::strict()).unwrap_err();
        assert!(error.to_string().contains(ACTIVATION), "{error:#}");
        for field in fields {
            assert!(error.to_string().contains(field), "{error:#}");
        }
    }
}

#[test]
fn plain_rule_losses_reject_strictly_and_aggregate_with_provenance() {
    let mut metadata = activation(vec![ActivationMode::Glob, ActivationMode::Manual]);
    metadata.globs = Some(vec!["*.rs".into()]);
    metadata.pattern = Some("rust".into());
    metadata.description = Some("When editing Rust".into());
    let graph = graph(
        Some("Rust guidance"),
        Some(TargetActivation::Bare(metadata)),
    );
    for target in &TARGETS[..3] {
        let error = lower(&graph, *target, LoweringOptions::strict()).unwrap_err();
        for detail in [
            DESCRIPTION,
            ACTIVATION,
            "mode",
            "globs",
            "pattern",
            "description",
        ] {
            assert!(error.to_string().contains(detail), "{error:#}");
        }
        let plan = lower(&graph, *target, LoweringOptions::allow_lossy()).unwrap();
        assert_eq!(plan.artifacts[0].bytes, b"Keep the body.\n");
        let losses: Vec<_> = plan
            .findings
            .iter()
            .filter(|finding| finding.severity == CapabilitySeverity::Lossy)
            .collect();
        assert_eq!(losses.len(), 2);
        assert_eq!(
            losses
                .iter()
                .map(|finding| &finding.id)
                .collect::<BTreeSet<_>>()
                .len(),
            2
        );
        for finding in losses {
            let package = graph.packages.values().next().unwrap();
            assert_eq!(finding.package_id, package.id);
            assert_eq!(finding.provenance, package.provenance);
            assert_eq!(
                finding.artifact_path.as_ref(),
                Some(&plan.artifacts[0].path)
            );
        }
        assert_eq!(
            plan,
            lower(&graph, *target, LoweringOptions::allow_lossy()).unwrap()
        );
    }
}

#[test]
fn absent_unconditional_and_unused_overrides_are_supported() {
    for target in TARGETS {
        for metadata in [
            None,
            Some(TargetActivation::Bare(activation(vec![
                ActivationMode::Always,
            ]))),
        ] {
            let graph = graph(None, metadata);
            let plan = lower(&graph, target, LoweringOptions::strict()).unwrap();
            assert!(plan
                .findings
                .iter()
                .all(|finding| finding.severity == CapabilitySeverity::Supported));
        }
        let metadata = TargetActivation::Wrapped(TargetActivationOverrides {
            default: activation(vec![ActivationMode::Always]),
            overrides: BTreeMap::from([(
                "unused".into(),
                activation(vec![ActivationMode::Pattern]),
            )]),
        });
        lower(
            &graph(None, Some(metadata)),
            target,
            LoweringOptions::strict(),
        )
        .unwrap();
    }
    for target in &TARGETS[..3] {
        lower(
            &graph(Some("   "), None),
            *target,
            LoweringOptions::strict(),
        )
        .unwrap();
    }
}

#[test]
fn native_modes_follow_selected_rendering_branches() {
    for target in [NativeTarget::Cursor, NativeTarget::Antigravity] {
        for mode in [
            ActivationMode::Always,
            ActivationMode::Glob,
            ActivationMode::Manual,
        ] {
            let mut metadata = activation(vec![mode]);
            if metadata.mode.contains(&ActivationMode::Glob) {
                metadata.globs = Some(vec!["*.rs".into()]);
            }
            assert_activation_loss(target, Some("Guidance"), metadata, &[]);
        }
        let mut metadata = activation(vec![ActivationMode::Always, ActivationMode::Glob]);
        metadata.globs = Some(vec!["*.rs".into()]);
        assert_activation_loss(target, None, metadata, &["mode"]);
        let mut metadata = activation(vec![ActivationMode::Manual]);
        metadata.globs = Some(vec!["*.rs".into()]);
        assert_activation_loss(target, None, metadata, &["globs"]);
        assert_activation_loss(
            target,
            None,
            activation(vec![ActivationMode::Always, ActivationMode::Always]),
            &[],
        );
    }
    assert_activation_loss(
        NativeTarget::Cursor,
        None,
        activation(vec![ActivationMode::Model]),
        &["mode"],
    );
    assert_activation_loss(
        NativeTarget::Antigravity,
        None,
        activation(vec![ActivationMode::Model, ActivationMode::Manual]),
        &["mode"],
    );
    let mut metadata = activation(vec![ActivationMode::Model]);
    metadata.globs = Some(vec!["*.rs".into()]);
    assert_activation_loss(NativeTarget::Antigravity, None, metadata, &["globs"]);
    let mut metadata = activation(vec![ActivationMode::Pattern]);
    metadata.globs = Some(vec!["*.rs".into()]);
    metadata.pattern = Some("Rust files".into());
    assert_activation_loss(
        NativeTarget::Antigravity,
        None,
        metadata.clone(),
        &["pattern"],
    );
    assert_activation_loss(
        NativeTarget::Cursor,
        None,
        metadata,
        &["mode", "globs", "pattern"],
    );
}

#[test]
fn cursor_glob_without_globs_loses_glob_mode() {
    assert_activation_loss(
        NativeTarget::Cursor,
        None,
        activation(vec![ActivationMode::Glob]),
        &["mode (glob)"],
    );
}

#[test]
fn cursor_glob_with_empty_globs_loses_glob_mode() {
    let mut metadata = activation(vec![ActivationMode::Glob]);
    metadata.globs = Some(vec![]);
    assert_activation_loss(NativeTarget::Cursor, None, metadata, &["mode (glob)"]);
}

#[test]
fn cursor_glob_manual_precedence_follows_emitted_globs() {
    for modes in [
        vec![ActivationMode::Glob, ActivationMode::Manual],
        vec![ActivationMode::Manual, ActivationMode::Glob],
    ] {
        for globs in [None, Some(vec![]), Some(vec!["*.rs".into()])] {
            let mut metadata = activation(modes.clone());
            metadata.globs = globs.clone();
            let graph = graph(None, Some(TargetActivation::Bare(metadata)));
            let plan = lower(&graph, NativeTarget::Cursor, LoweringOptions::allow_lossy()).unwrap();
            let output = std::str::from_utf8(&plan.artifacts[0].bytes).unwrap();
            let frontmatter: serde_yaml::Value =
                serde_yaml::from_str(output.split("---\n").nth(1).unwrap()).unwrap();
            assert_eq!(frontmatter["alwaysApply"], false);
            let emitted_globs: Option<Vec<String>> =
                serde_yaml::from_value(frontmatter["globs"].clone()).unwrap();
            assert_eq!(emitted_globs, globs);
            let discarded = if emitted_globs
                .as_ref()
                .is_some_and(|globs| !globs.is_empty())
            {
                "manual"
            } else {
                "glob"
            };
            let losses: Vec<_> = plan
                .findings
                .iter()
                .filter(|finding| finding.reason_code.as_str() == ACTIVATION)
                .collect();
            assert_eq!(losses.len(), 1);
            assert_eq!(
                losses[0].reason,
                format!("lost activation fields: mode ({discarded})")
            );
            let error = lower(&graph, NativeTarget::Cursor, LoweringOptions::strict()).unwrap_err();
            assert!(error.to_string().contains(ACTIVATION), "{error:#}");
        }
    }
}

#[test]
fn antigravity_model_description_only_preserved_when_emitted_and_matching() {
    let mut metadata = activation(vec![ActivationMode::Model]);
    metadata.description = Some(" Rust  guidance\n".into());
    assert_activation_loss(
        NativeTarget::Antigravity,
        Some("Rust guidance"),
        metadata.clone(),
        &[],
    );
    assert_activation_loss(
        NativeTarget::Antigravity,
        Some("Other guidance"),
        metadata.clone(),
        &["description"],
    );
    assert_activation_loss(
        NativeTarget::Antigravity,
        None,
        metadata.clone(),
        &["description"],
    );
    metadata.mode = vec![ActivationMode::Always, ActivationMode::Model];
    assert_activation_loss(
        NativeTarget::Antigravity,
        Some("Rust guidance"),
        metadata,
        &["mode", "description"],
    );
}

#[test]
fn native_frontmatter_and_aggregated_findings_match_actual_branches() {
    for (target, modes, native_field, emitted_globs, discarded) in [
        (
            NativeTarget::Cursor,
            vec![
                ActivationMode::Always,
                ActivationMode::Always,
                ActivationMode::Glob,
            ],
            "alwaysApply: true",
            true,
            "glob",
        ),
        (
            NativeTarget::Cursor,
            vec![
                ActivationMode::Glob,
                ActivationMode::Model,
                ActivationMode::Manual,
            ],
            "alwaysApply: false",
            true,
            "manual, model",
        ),
        (
            NativeTarget::Cursor,
            vec![ActivationMode::Manual, ActivationMode::Model],
            "alwaysApply: false",
            false,
            "model",
        ),
        (
            NativeTarget::Antigravity,
            vec![
                ActivationMode::Glob,
                ActivationMode::Model,
                ActivationMode::Manual,
                ActivationMode::Pattern,
            ],
            "trigger: glob",
            true,
            "manual, model, pattern",
        ),
        (
            NativeTarget::Antigravity,
            vec![
                ActivationMode::Model,
                ActivationMode::Manual,
                ActivationMode::Pattern,
            ],
            "trigger: model_decision",
            false,
            "manual, pattern",
        ),
        (
            NativeTarget::Antigravity,
            vec![ActivationMode::Manual, ActivationMode::Pattern],
            "trigger: manual",
            false,
            "pattern",
        ),
        (
            NativeTarget::Antigravity,
            vec![ActivationMode::Pattern, ActivationMode::Pattern],
            "trigger: glob",
            true,
            "",
        ),
    ] {
        let mut metadata = activation(modes);
        metadata.globs = Some(vec!["*.rs".into()]);
        metadata.pattern = Some("Rust files".into());
        metadata.description = Some("Independent description".into());
        let graph = graph(Some("Guidance"), Some(TargetActivation::Bare(metadata)));
        let plan = lower(&graph, target, LoweringOptions::allow_lossy()).unwrap();
        let rendered = std::str::from_utf8(&plan.artifacts[0].bytes).unwrap();
        assert!(rendered.contains(native_field), "{rendered}");
        assert_eq!(rendered.contains("globs:"), emitted_globs, "{rendered}");
        assert!(rendered.contains("description: Guidance"));
        assert!(rendered.ends_with("Keep the body.\n"));
        let losses: Vec<_> = plan
            .findings
            .iter()
            .filter(|finding| finding.severity == CapabilitySeverity::Lossy)
            .collect();
        assert_eq!(losses.len(), 1);
        let mut fields = Vec::new();
        if !discarded.is_empty() {
            fields.push(format!("mode ({discarded})"));
        }
        if !emitted_globs {
            fields.push("globs".into());
        }
        fields.extend(["pattern".into(), "description".into()]);
        assert_eq!(
            losses[0].reason,
            format!("lost activation fields: {}", fields.join(", "))
        );
        assert_eq!(
            plan,
            lower(&graph, target, LoweringOptions::allow_lossy()).unwrap()
        );
    }
}

#[test]
fn rule_metadata_loss_is_visible_in_capability_inspection() {
    let temporary = tempfile::tempdir().unwrap();
    let input = temporary.path().join("graph.json");
    let graph = graph(
        Some("Guidance"),
        Some(TargetActivation::Bare(activation(vec![
            ActivationMode::Manual,
        ]))),
    );
    fs::write(&input, serde_json::to_vec(&graph).unwrap()).unwrap();
    let output = assert_cmd::Command::cargo_bin("rulette")
        .unwrap()
        .arg("inspect")
        .arg(&input)
        .args(["--from", "graph-json", "--to", "codex"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let findings = String::from_utf8_lossy(&output.stdout);
    assert!(findings.contains(DESCRIPTION), "{findings}");
    assert!(findings.contains(ACTIVATION), "{findings}");
    assert!(
        findings.contains(".cursor/rules/guidance.mdc"),
        "{findings}"
    );
    assert_cmd::Command::cargo_bin("rulette")
        .unwrap()
        .arg("inspect")
        .arg(&input)
        .args(["--from", "graph-json", "--coverage", "--strict"])
        .assert()
        .failure();
}

#[test]
fn resolved_overrides_work_both_directions_and_keep_cursor_alias_precedence() {
    for target in TARGETS {
        let key = if target == NativeTarget::Cursor {
            "cursor-mdc"
        } else {
            target.as_str()
        };
        for (default_mode, override_mode, lossy) in [
            (ActivationMode::Pattern, ActivationMode::Always, false),
            (ActivationMode::Always, ActivationMode::Pattern, true),
        ] {
            let mut default = activation(vec![default_mode]);
            default.pattern = Some("default pattern".into());
            let mut selected = activation(vec![override_mode]);
            if lossy {
                selected.pattern = Some("selected pattern".into());
            }
            let metadata = TargetActivation::Wrapped(TargetActivationOverrides {
                default,
                overrides: BTreeMap::from([(key.into(), selected)]),
            });
            assert_eq!(
                lower(
                    &graph(None, Some(metadata)),
                    target,
                    LoweringOptions::strict()
                )
                .is_err(),
                lossy,
                "{target:?}"
            );
        }
    }
    for (overrides, lossy) in [
        (
            BTreeMap::from([("cursor".into(), activation(vec![ActivationMode::Manual]))]),
            false,
        ),
        (
            BTreeMap::from([
                ("cursor".into(), activation(vec![ActivationMode::Model])),
                (
                    "cursor-mdc".into(),
                    activation(vec![ActivationMode::Manual]),
                ),
            ]),
            false,
        ),
        (
            BTreeMap::from([
                ("cursor".into(), activation(vec![ActivationMode::Manual])),
                ("cursor-mdc".into(), activation(vec![ActivationMode::Model])),
            ]),
            true,
        ),
    ] {
        let metadata = TargetActivation::Wrapped(TargetActivationOverrides {
            default: activation(vec![ActivationMode::Always]),
            overrides,
        });
        assert_eq!(
            lower(
                &graph(None, Some(metadata)),
                NativeTarget::Cursor,
                LoweringOptions::strict()
            )
            .is_err(),
            lossy
        );
    }
}

#[test]
fn permitted_rule_losses_roundtrip_through_staged_plan() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    let stage_dir = temporary.path().join("stage");
    fs::create_dir(&root).unwrap();
    let graph = graph(
        Some("Guidance"),
        Some(TargetActivation::Bare(activation(vec![
            ActivationMode::Manual,
        ]))),
    );
    let lowering = lower(
        &graph,
        NativeTarget::OpenCode,
        LoweringOptions::allow_lossy(),
    )
    .unwrap();
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
        accepted_losses: lowering
            .findings
            .iter()
            .filter(|finding| finding.severity == CapabilitySeverity::Lossy)
            .map(|finding| ScopedAcceptedLoss {
                scope: PublicationScope::Project,
                finding,
            })
            .collect(),
        stage_dir: &stage_dir,
    })
    .unwrap();
    assert!(staged.plan.allow_lossy);
    assert_eq!(staged.plan.losses.len(), 2);
    assert_eq!(
        staged
            .plan
            .losses
            .iter()
            .map(|loss| loss.reason_code.as_str())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([DESCRIPTION, ACTIVATION])
    );
    let bytes = canonical_plan_json(&staged.plan).unwrap();
    let parsed = parse_plan_with_expected_digest(&bytes, &PlanDigest::from_bytes(&bytes)).unwrap();
    assert_eq!(parsed, staged.plan);
    assert!(staged
        .plan
        .losses
        .iter()
        .all(|loss| loss.entry_id.is_some()));
    assert!(fs::read_dir(&root).unwrap().next().is_none());
}

#[test]
fn multi_target_strict_rule_loss_creates_no_partial_stage() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    fs::create_dir(&root).unwrap();
    let input = temporary.path().join("graph.json");
    let stage_dir = temporary.path().join("stage");
    let graph = graph(
        Some("Guidance"),
        Some(TargetActivation::Bare(activation(vec![
            ActivationMode::Manual,
        ]))),
    );
    lower(&graph, NativeTarget::Cursor, LoweringOptions::strict()).unwrap();
    fs::write(&input, serde_json::to_vec(&graph).unwrap()).unwrap();
    let output = assert_cmd::Command::cargo_bin("rulette")
        .unwrap()
        .arg("transform")
        .arg(&input)
        .args([
            "--from",
            "graph-json",
            "--target",
            "cursor@project",
            "--target",
            "codex@project",
            "--stage",
        ])
        .arg(&stage_dir)
        .arg("--project-root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains(DESCRIPTION), "{error}");
    assert!(error.contains(ACTIVATION), "{error}");
    assert!(!stage_dir.exists());
    assert!(fs::read_dir(&root).unwrap().next().is_none());
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 2);
    let output = assert_cmd::Command::cargo_bin("rulette")
        .unwrap()
        .arg("transform")
        .arg(&input)
        .args([
            "--from",
            "graph-json",
            "--target",
            "cursor@project",
            "--target",
            "codex@project",
            "--allow-lossy",
            "--stage",
        ])
        .arg(&stage_dir)
        .arg("--project-root")
        .arg(&root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = fs::read(stage_dir.join("rulette.plan.json")).unwrap();
    let plan = parse_plan_with_expected_digest(&bytes, &PlanDigest::from_bytes(&bytes)).unwrap();
    assert!(plan.allow_lossy);
    assert_eq!(plan.losses.len(), 2);
    assert!(plan
        .losses
        .iter()
        .all(|loss| loss.target == NativeTarget::Codex));
    assert_eq!(plan.entries.len(), 2);
    assert!(fs::read_dir(&root).unwrap().next().is_none());
}
