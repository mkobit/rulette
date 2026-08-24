use assert_cmd::Command;
use rulette::cli::formats::InputFormat;
use rulette::inputs::observe_path;
use rulette::publication::{
    apply_plan, canonical_plan_json, check_sources, mapping_for, parse_plan_with_expected_digest,
    stage, ApplyOptions, ArtifactDescriptor, AuthorizedRoot, DestinationState, PlanDigest,
    PlanOperationRequest, PublicationScope, ScopedAcceptedLoss, ScopedLowering, SourceCheckRequest,
    StageRequest, StageRoot,
};
use rulette::{
    compile_graph, lower, CapabilityReasonCode, CapabilitySeverity, LoweringOptions, NativeTarget,
    PackageKind,
};
use std::fs;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/v0_1")
        .join(name)
}

fn make_accepted_losses<'a>(lowering: &'a rulette::LoweringPlan) -> Vec<ScopedAcceptedLoss<'a>> {
    lowering
        .findings
        .iter()
        .filter(|f| f.severity != CapabilitySeverity::Supported)
        .map(|f| ScopedAcceptedLoss {
            scope: PublicationScope::Project,
            finding: f,
        })
        .collect()
}

const CORE_FRONTENDS: [&str; 5] = ["codex", "claude", "cursor", "opencode", "antigravity"];
const CORE_TARGETS: [NativeTarget; 5] = [
    NativeTarget::Codex,
    NativeTarget::Claude,
    NativeTarget::Cursor,
    NativeTarget::OpenCode,
    NativeTarget::Antigravity,
];

#[test]
fn cross_harness_migration_matrix_compiles_and_lowers_all_pairs() {
    for frontend in CORE_FRONTENDS {
        let fixture_path = fixture(frontend);
        let observations = observe_path(&fixture_path).expect("fixture tree is safe to observe");
        let graph = compile_graph(&observations, InputFormat::Auto)
            .unwrap_or_else(|_| panic!("frontend {frontend} auto-detects and compiles"));

        for target in CORE_TARGETS {
            let has_unsupported = graph
                .packages
                .values()
                .any(|p| p.kind == PackageKind::Unsupported);
            let has_skills = graph
                .packages
                .values()
                .any(|p| p.kind == PackageKind::Skill);
            let has_opaque_resources = graph.packages.values().any(|p| {
                p.kind == PackageKind::Skill
                    && p.resources
                        .values()
                        .any(|r| r.role == rulette::ir::graph::ResourceRole::Opaque)
            });
            let is_same_domain = frontend == target.as_str();

            // Strict lowering: fails if source has unsupported semantics, target is Cursor with skills,
            // or cross-domain with opaque skill resources.
            let strict_result = lower(&graph, target, LoweringOptions::strict());
            let expect_strict_failure = has_unsupported
                || (target == NativeTarget::Cursor && has_skills)
                || (!is_same_domain && has_opaque_resources);

            if expect_strict_failure {
                assert!(
                    strict_result.is_err(),
                    "strict lowering from {frontend} to {} must reject representational loss",
                    target.as_str()
                );
            } else {
                let plan = strict_result.unwrap_or_else(|e| {
                    panic!(
                        "strict lowering from {frontend} to {} should succeed: {e}",
                        target.as_str()
                    )
                });
                assert!(
                    !plan.artifacts.is_empty(),
                    "strict lowering produced non-empty artifacts"
                );
            }

            // Allow-lossy lowering: must always succeed across all 5x5 combinations
            let lossy_plan =
                lower(&graph, target, LoweringOptions::allow_lossy()).unwrap_or_else(|e| {
                    panic!(
                        "lossy lowering from {frontend} to {} should always succeed: {e}",
                        target.as_str()
                    )
                });

            assert!(
                !lossy_plan.artifacts.is_empty(),
                "lossy lowering from {frontend} to {} produced artifacts",
                target.as_str()
            );

            // Verify structured capability findings
            if has_unsupported {
                assert!(
                    lossy_plan
                        .findings
                        .iter()
                        .any(|f| f.severity == CapabilitySeverity::Dropped),
                    "dropped unsupported packages recorded in findings"
                );
            }
            if target == NativeTarget::Cursor && has_skills {
                assert!(
                    lossy_plan.findings.iter().any(|f| {
                        f.reason_code == CapabilityReasonCode::SkillLoweredAsRule
                            || f.reason_code == CapabilityReasonCode::OpaqueResourceUnrepresentable
                    }),
                    "Cursor target records skill loss findings"
                );
            }
            if target != NativeTarget::Cursor && !is_same_domain && has_opaque_resources {
                assert!(
                    lossy_plan.findings.iter().any(|f| {
                        f.severity == CapabilitySeverity::Dropped
                            && f.reason_code == CapabilityReasonCode::OpaqueCrossDomain
                    }),
                    "Cross-domain opaque skill resources recorded as dropped"
                );
            }
        }
    }
}

#[test]
fn deterministic_lowering_and_staging_across_all_core_targets() {
    for frontend in CORE_FRONTENDS {
        let fixture_path = fixture(frontend);
        let observations = observe_path(&fixture_path).expect("fixture tree is safe to observe");
        let graph1 = compile_graph(&observations, InputFormat::Auto).unwrap();
        let graph2 = compile_graph(&observations, InputFormat::Auto).unwrap();

        // Compilation determinism
        let json1 = serde_json::to_string(&graph1).unwrap();
        let json2 = serde_json::to_string(&graph2).unwrap();
        assert_eq!(
            json1, json2,
            "compilation graph must be byte-for-byte deterministic"
        );

        for target in CORE_TARGETS {
            let lowering1 = lower(&graph1, target, LoweringOptions::allow_lossy()).unwrap();
            let lowering2 = lower(&graph2, target, LoweringOptions::allow_lossy()).unwrap();

            assert_eq!(
                lowering1.artifacts, lowering2.artifacts,
                "lowering artifacts must be deterministic for {target:?}"
            );
            assert_eq!(
                lowering1.findings, lowering2.findings,
                "lowering findings must be deterministic for {target:?}"
            );

            let temp_dir1 = tempfile::tempdir().unwrap();
            let temp_dir2 = tempfile::tempdir().unwrap();
            let stage_dir1 = temp_dir1.path().join("stage");
            let stage_dir2 = temp_dir2.path().join("stage");
            let live_root = temp_dir1.path().join("live");
            fs::create_dir_all(&live_root).unwrap();

            let staged1 = stage(StageRequest {
                graph: &graph1,
                lowerings: vec![ScopedLowering {
                    scope: PublicationScope::Project,
                    lowering: &lowering1,
                }],
                roots: vec![StageRoot {
                    target,
                    scope: PublicationScope::Project,
                    path: &live_root,
                }],
                accepted_losses: make_accepted_losses(&lowering1),
                stage_dir: &stage_dir1,
            })
            .unwrap();

            let staged2 = stage(StageRequest {
                graph: &graph2,
                lowerings: vec![ScopedLowering {
                    scope: PublicationScope::Project,
                    lowering: &lowering2,
                }],
                roots: vec![StageRoot {
                    target,
                    scope: PublicationScope::Project,
                    path: &live_root,
                }],
                accepted_losses: make_accepted_losses(&lowering2),
                stage_dir: &stage_dir2,
            })
            .unwrap();

            // Plan determinism
            assert_eq!(
                staged1.plan_digest, staged2.plan_digest,
                "staged plan digest must be deterministic for {target:?}"
            );
            assert_eq!(
                staged1.plan.entries, staged2.plan.entries,
                "staged plan entries must be deterministic for {target:?}"
            );

            let plan_bytes = fs::read(stage_dir1.join("rulette.plan.json")).unwrap();
            assert_eq!(
                canonical_plan_json(&staged1.plan).unwrap(),
                plan_bytes,
                "rulette.plan.json must be canonical JSON"
            );
            assert_eq!(
                staged1.plan_digest.as_str(),
                PlanDigest::from_bytes(&plan_bytes).as_str()
            );

            let parsed =
                parse_plan_with_expected_digest(&plan_bytes, &staged1.plan_digest).unwrap();
            let mut expected_entries = staged1.plan.entries.clone();
            expected_entries.sort_by_key(|e| e.entry_id.clone());
            assert_eq!(parsed.entries, expected_entries);

            // Staged artifact bytes match lowered artifacts
            for entry in &staged1.plan.entries {
                let staged_file = stage_dir1.join(entry.stage_artifact_path.as_str());
                assert!(staged_file.exists(), "staged artifact file exists");
                let content = fs::read(&staged_file).unwrap();
                let matching = lowering1
                    .artifacts
                    .iter()
                    .find(|a| a.path.as_str() == entry.artifact.native_path.as_str())
                    .expect("matching artifact in lowering plan");
                assert_eq!(content, matching.bytes, "staged artifact content matches");
            }
        }
    }
}

#[test]
fn publication_lifecycle_proves_repository_migration_journey() {
    let source_fixture = fixture("codex");
    let observations = observe_path(&source_fixture).unwrap();
    let graph = compile_graph(&observations, InputFormat::Auto).unwrap();

    for target in CORE_TARGETS {
        let lowering = lower(&graph, target, LoweringOptions::allow_lossy()).unwrap();

        let temp_dir = tempfile::tempdir().unwrap();
        let stage_dir = temp_dir.path().join("stage");
        let target_project_root = temp_dir.path().join("project");
        fs::create_dir_all(&target_project_root).unwrap();

        // 1. Stage artifacts
        let staged = stage(StageRequest {
            graph: &graph,
            lowerings: vec![ScopedLowering {
                scope: PublicationScope::Project,
                lowering: &lowering,
            }],
            roots: vec![StageRoot {
                target,
                scope: PublicationScope::Project,
                path: &target_project_root,
            }],
            accepted_losses: make_accepted_losses(&lowering),
            stage_dir: &stage_dir,
        })
        .unwrap();

        // 2. Pre-apply source check: all destination files should be Absent
        let check_report = check_sources(SourceCheckRequest {
            graph: &graph,
            lowerings: vec![ScopedLowering {
                scope: PublicationScope::Project,
                lowering: &lowering,
            }],
            roots: vec![AuthorizedRoot {
                target,
                scope: PublicationScope::Project,
                path: &target_project_root,
            }],
            accepted_losses: make_accepted_losses(&lowering),
        })
        .unwrap();

        assert_eq!(
            check_report.entries.len(),
            staged.plan.entries.len(),
            "destination count matches plan entries"
        );
        for dest in &check_report.entries {
            assert_eq!(
                dest.state,
                DestinationState::Absent,
                "all destinations must initially be absent"
            );
        }

        // 3. Apply staged plan
        let apply_report = apply_plan(
            PlanOperationRequest {
                stage_dir: &stage_dir,
                expected_plan_digest: staged.plan_digest.clone(),
                roots: vec![AuthorizedRoot {
                    target,
                    scope: PublicationScope::Project,
                    path: &target_project_root,
                }],
            },
            ApplyOptions { replace: false },
        )
        .unwrap();

        assert_eq!(
            apply_report.created.len(),
            staged.plan.entries.len(),
            "every planned entry is created during migration apply"
        );
        assert!(apply_report.replaced.is_empty());
        assert!(apply_report.unchanged.is_empty());

        // 4. Post-apply check: all destination files are now Unchanged
        let post_check = check_sources(SourceCheckRequest {
            graph: &graph,
            lowerings: vec![ScopedLowering {
                scope: PublicationScope::Project,
                lowering: &lowering,
            }],
            roots: vec![AuthorizedRoot {
                target,
                scope: PublicationScope::Project,
                path: &target_project_root,
            }],
            accepted_losses: make_accepted_losses(&lowering),
        })
        .unwrap();

        assert!(post_check.is_clean());
        for dest in &post_check.entries {
            assert_eq!(
                dest.state,
                DestinationState::Unchanged,
                "all applied files are unchanged on second check"
            );
        }

        // 5. Conflict handling: modifying a destination triggers conflict check
        let first_entry = &staged.plan.entries[0];
        let relative = mapping_for(first_entry.target, first_entry.scope)
            .unwrap()
            .map_artifact(&first_entry.artifact)
            .unwrap();
        let file_path = target_project_root.join(relative.as_str());
        fs::write(&file_path, "conflicting local modification").unwrap();

        let conflict_apply = apply_plan(
            PlanOperationRequest {
                stage_dir: &stage_dir,
                expected_plan_digest: staged.plan_digest.clone(),
                roots: vec![AuthorizedRoot {
                    target,
                    scope: PublicationScope::Project,
                    path: &target_project_root,
                }],
            },
            ApplyOptions { replace: false },
        );
        assert!(
            conflict_apply.is_err(),
            "apply without replace fails on conflicting file"
        );

        let replace_apply = apply_plan(
            PlanOperationRequest {
                stage_dir: &stage_dir,
                expected_plan_digest: staged.plan_digest.clone(),
                roots: vec![AuthorizedRoot {
                    target,
                    scope: PublicationScope::Project,
                    path: &target_project_root,
                }],
            },
            ApplyOptions { replace: true },
        )
        .unwrap();

        assert_eq!(replace_apply.replaced.len(), 1);
    }
}

#[test]
fn target_specific_normalized_snapshots() {
    let source = fixture("codex");
    let observations = observe_path(&source).unwrap();
    let graph = compile_graph(&observations, InputFormat::Auto).unwrap();

    let mapped_paths = |target: NativeTarget, lowering: &rulette::LoweringPlan| -> Vec<String> {
        let mapping = mapping_for(target, PublicationScope::Project).unwrap();
        lowering
            .artifacts
            .iter()
            .map(|a| {
                mapping
                    .map_artifact(&ArtifactDescriptor {
                        class: a.class,
                        native_path: a.path.clone(),
                    })
                    .unwrap()
                    .as_str()
                    .to_owned()
            })
            .collect()
    };

    // Codex target snapshot
    let codex_lowering = lower(&graph, NativeTarget::Codex, LoweringOptions::strict()).unwrap();
    let codex_paths = mapped_paths(NativeTarget::Codex, &codex_lowering);
    assert_eq!(
        codex_paths,
        vec![
            ".codex/skills/release/SKILL.md",
            ".codex/skills/release/references/checklist.md",
            "AGENTS.md",
        ]
    );

    // Claude target snapshot (allow-lossy because cross-domain drops opaque resources)
    let claude_lowering =
        lower(&graph, NativeTarget::Claude, LoweringOptions::allow_lossy()).unwrap();
    let claude_paths = mapped_paths(NativeTarget::Claude, &claude_lowering);
    assert_eq!(
        claude_paths,
        vec![".claude/skills/release/SKILL.md", "CLAUDE.md"]
    );

    // Cursor target snapshot (allow-lossy lowers skills as rules)
    let cursor_lowering =
        lower(&graph, NativeTarget::Cursor, LoweringOptions::allow_lossy()).unwrap();
    let cursor_paths = mapped_paths(NativeTarget::Cursor, &cursor_lowering);
    assert_eq!(
        cursor_paths,
        vec![".cursor/rules/release.mdc", ".cursor/rules/AGENTS.md.mdc"]
    );

    // OpenCode target snapshot (allow-lossy drops cross-domain opaque resources)
    let opencode_lowering = lower(
        &graph,
        NativeTarget::OpenCode,
        LoweringOptions::allow_lossy(),
    )
    .unwrap();
    let opencode_paths = mapped_paths(NativeTarget::OpenCode, &opencode_lowering);
    assert_eq!(
        opencode_paths,
        vec![
            ".opencode/skills/release/SKILL.md",
            ".opencode/rules/AGENTS.md.md"
        ]
    );

    // Antigravity target snapshot (allow-lossy drops cross-domain opaque resources)
    let antigravity_lowering = lower(
        &graph,
        NativeTarget::Antigravity,
        LoweringOptions::allow_lossy(),
    )
    .unwrap();
    let antigravity_paths = mapped_paths(NativeTarget::Antigravity, &antigravity_lowering);
    assert_eq!(
        antigravity_paths,
        vec![
            ".agents/skills/release/SKILL.md",
            ".agents/rules/AGENTS.md.md"
        ]
    );
}

#[test]
fn target_specific_normalized_snapshots_from_claude() {
    let source = fixture("claude");
    let observations = observe_path(&source).unwrap();
    let graph = compile_graph(&observations, InputFormat::Auto).unwrap();

    let mapped_paths = |target: NativeTarget, lowering: &rulette::LoweringPlan| -> Vec<String> {
        let mapping = mapping_for(target, PublicationScope::Project).unwrap();
        lowering
            .artifacts
            .iter()
            .map(|a| {
                mapping
                    .map_artifact(&ArtifactDescriptor {
                        class: a.class,
                        native_path: a.path.clone(),
                    })
                    .unwrap()
                    .as_str()
                    .to_owned()
            })
            .collect()
    };

    // Claude (same domain) retains skill and references
    let claude_lowering =
        lower(&graph, NativeTarget::Claude, LoweringOptions::allow_lossy()).unwrap();
    let claude_paths = mapped_paths(NativeTarget::Claude, &claude_lowering);
    assert_eq!(
        claude_paths,
        vec![
            ".claude/skills/review/SKILL.md",
            ".claude/skills/review/references/checklist.md",
            "CLAUDE.md"
        ]
    );

    // OpenCode target
    let opencode_lowering = lower(
        &graph,
        NativeTarget::OpenCode,
        LoweringOptions::allow_lossy(),
    )
    .unwrap();
    let opencode_paths = mapped_paths(NativeTarget::OpenCode, &opencode_lowering);
    assert_eq!(
        opencode_paths,
        vec![
            ".opencode/skills/review/SKILL.md",
            ".opencode/rules/CLAUDE.md.md"
        ]
    );
}

#[test]
fn target_specific_normalized_snapshots_from_cursor() {
    let source = fixture("cursor");
    let observations = observe_path(&source).unwrap();
    let graph = compile_graph(&observations, InputFormat::Auto).unwrap();

    let mapped_paths = |target: NativeTarget, lowering: &rulette::LoweringPlan| -> Vec<String> {
        let mapping = mapping_for(target, PublicationScope::Project).unwrap();
        lowering
            .artifacts
            .iter()
            .map(|a| {
                mapping
                    .map_artifact(&ArtifactDescriptor {
                        class: a.class,
                        native_path: a.path.clone(),
                    })
                    .unwrap()
                    .as_str()
                    .to_owned()
            })
            .collect()
    };

    // Cursor (same domain)
    let cursor_lowering =
        lower(&graph, NativeTarget::Cursor, LoweringOptions::allow_lossy()).unwrap();
    let cursor_paths = mapped_paths(NativeTarget::Cursor, &cursor_lowering);
    assert_eq!(
        cursor_paths,
        vec![".cursor/rules/review.mdc", ".cursor/rules/rust.mdc"]
    );

    // Codex target
    let codex_lowering =
        lower(&graph, NativeTarget::Codex, LoweringOptions::allow_lossy()).unwrap();
    let codex_paths = mapped_paths(NativeTarget::Codex, &codex_lowering);
    assert_eq!(
        codex_paths,
        vec![".codex/skills/review/SKILL.md", "AGENTS.md"]
    );
}

#[test]
fn target_specific_normalized_snapshots_from_opencode() {
    let source = fixture("opencode");
    let observations = observe_path(&source).unwrap();
    let graph = compile_graph(&observations, InputFormat::Auto).unwrap();

    let mapped_paths = |target: NativeTarget, lowering: &rulette::LoweringPlan| -> Vec<String> {
        let mapping = mapping_for(target, PublicationScope::Project).unwrap();
        lowering
            .artifacts
            .iter()
            .map(|a| {
                mapping
                    .map_artifact(&ArtifactDescriptor {
                        class: a.class,
                        native_path: a.path.clone(),
                    })
                    .unwrap()
                    .as_str()
                    .to_owned()
            })
            .collect()
    };

    // OpenCode (same domain) retains skill and references
    let opencode_lowering = lower(
        &graph,
        NativeTarget::OpenCode,
        LoweringOptions::allow_lossy(),
    )
    .unwrap();
    let opencode_paths = mapped_paths(NativeTarget::OpenCode, &opencode_lowering);
    assert_eq!(
        opencode_paths,
        vec![
            ".opencode/rules/review.md",
            ".opencode/skills/release/SKILL.md",
            ".opencode/skills/release/references/checklist.md",
        ]
    );

    // Antigravity target
    let antigravity_lowering = lower(
        &graph,
        NativeTarget::Antigravity,
        LoweringOptions::allow_lossy(),
    )
    .unwrap();
    let antigravity_paths = mapped_paths(NativeTarget::Antigravity, &antigravity_lowering);
    assert_eq!(
        antigravity_paths,
        vec![".agents/rules/review.md", ".agents/skills/release/SKILL.md",]
    );
}

#[test]
fn target_specific_normalized_snapshots_from_antigravity() {
    let source = fixture("antigravity");
    let observations = observe_path(&source).unwrap();
    let graph = compile_graph(&observations, InputFormat::Auto).unwrap();

    let mapped_paths = |target: NativeTarget, lowering: &rulette::LoweringPlan| -> Vec<String> {
        let mapping = mapping_for(target, PublicationScope::Project).unwrap();
        lowering
            .artifacts
            .iter()
            .map(|a| {
                mapping
                    .map_artifact(&ArtifactDescriptor {
                        class: a.class,
                        native_path: a.path.clone(),
                    })
                    .unwrap()
                    .as_str()
                    .to_owned()
            })
            .collect()
    };

    // Antigravity (same domain)
    let antigravity_lowering = lower(
        &graph,
        NativeTarget::Antigravity,
        LoweringOptions::allow_lossy(),
    )
    .unwrap();
    let antigravity_paths = mapped_paths(NativeTarget::Antigravity, &antigravity_lowering);
    assert_eq!(
        antigravity_paths,
        vec![
            ".agents/rules/security.md",
            ".agents/skills/review/SKILL.md",
        ]
    );

    // Codex target
    let codex_lowering =
        lower(&graph, NativeTarget::Codex, LoweringOptions::allow_lossy()).unwrap();
    let codex_paths = mapped_paths(NativeTarget::Codex, &codex_lowering);
    assert_eq!(
        codex_paths,
        vec!["AGENTS.md", ".codex/skills/review/SKILL.md"]
    );
}

#[test]
fn cli_migration_and_ci_workflow() {
    for frontend in CORE_FRONTENDS {
        let fixture_path = fixture(frontend);

        // 1. inspect --coverage --json contains core-five targets only
        let mut coverage_cmd = Command::cargo_bin("rulette").unwrap();
        let coverage_output = coverage_cmd
            .arg("-q")
            .arg("inspect")
            .arg(&fixture_path)
            .arg("--coverage")
            .arg("--json")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let coverage_entries: Vec<serde_json::Value> =
            serde_json::from_slice(&coverage_output).unwrap();
        assert!(
            coverage_entries.iter().all(|e| {
                let t = e["target"].as_str().unwrap();
                CORE_FRONTENDS.contains(&t)
            }),
            "coverage only contains core five harnesses"
        );

        // 2. inspect --to reports structured capability findings
        let mut inspect_cmd = Command::cargo_bin("rulette").unwrap();
        inspect_cmd
            .arg("inspect")
            .arg(&fixture_path)
            .arg("--to")
            .arg("cursor")
            .assert()
            .success();

        // 3. transform --target <target> --allow-lossy outputs valid compilation graph
        let mut transform_cmd = Command::cargo_bin("rulette").unwrap();
        let transform_output = transform_cmd
            .arg("transform")
            .arg(&fixture_path)
            .arg("--target")
            .arg("cursor")
            .arg("--allow-lossy")
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();

        let graph: serde_json::Value = serde_json::from_slice(&transform_output).unwrap();
        assert!(graph.get("graph_version").is_some());
        assert!(graph.get("packages").is_some());
    }
}
