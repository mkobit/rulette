use rulette::{
    publication::{
        canonical_plan_json, mapping_for, parse_plan_with_expected_digest, ArtifactDescriptor,
        MappingVersion, PlanDigest, PlanEntry, PlanLossFinding, PublicationPlan, PublicationScope,
        RootBinding, RootIdentity,
    },
    CapabilityReasonCode, CapabilitySeverity, NativeArtifactClass, NativeTarget, PackageId,
    ResourcePath,
};
use std::collections::BTreeMap;

fn descriptor(class: NativeArtifactClass, path: &str) -> ArtifactDescriptor {
    ArtifactDescriptor {
        class,
        native_path: ResourcePath::parse(path).expect("test artifact path is valid"),
    }
}

#[test]
fn every_core_target_has_a_project_mapping() {
    for target in [
        NativeTarget::Codex,
        NativeTarget::OpenCode,
        NativeTarget::Claude,
        NativeTarget::Cursor,
        NativeTarget::Antigravity,
    ] {
        let mapping = mapping_for(target, PublicationScope::Project)
            .expect("every core target has a project mapping");
        assert_eq!(mapping.version(), MappingVersion::V0_1);
    }
}

#[test]
fn project_mappings_apply_only_their_documented_class_prefixes() {
    let cases = [
        (
            NativeTarget::Codex,
            descriptor(NativeArtifactClass::Instruction, "AGENTS.md"),
            "AGENTS.md",
        ),
        (
            NativeTarget::Codex,
            descriptor(
                NativeArtifactClass::SkillInstruction,
                "skills/review/SKILL.md",
            ),
            ".codex/skills/review/SKILL.md",
        ),
        (
            NativeTarget::OpenCode,
            descriptor(NativeArtifactClass::Rule, "rules/review.md"),
            ".opencode/rules/review.md",
        ),
        (
            NativeTarget::Claude,
            descriptor(NativeArtifactClass::Instruction, "CLAUDE.md"),
            "CLAUDE.md",
        ),
        (
            NativeTarget::Claude,
            descriptor(
                NativeArtifactClass::SkillResource,
                "skills/review/scripts/check.sh",
            ),
            ".claude/skills/review/scripts/check.sh",
        ),
        (
            NativeTarget::Cursor,
            descriptor(NativeArtifactClass::Rule, "rules/review.mdc"),
            ".cursor/rules/review.mdc",
        ),
        (
            NativeTarget::Antigravity,
            descriptor(NativeArtifactClass::Rule, "rules/review.md"),
            ".agents/rules/review.md",
        ),
        (
            NativeTarget::Antigravity,
            descriptor(
                NativeArtifactClass::SkillInstruction,
                "skills/review/SKILL.md",
            ),
            ".agents/skills/review/SKILL.md",
        ),
    ];

    for (target, artifact, expected_path) in cases {
        let mapped = mapping_for(target, PublicationScope::Project)
            .expect("project mapping exists")
            .map_artifact(&artifact)
            .expect("documented artifact is accepted");
        assert_eq!(mapped.as_str(), expected_path);
    }
}

#[test]
fn user_mappings_are_allowlisted_and_cursor_is_unavailable() {
    for target in [
        NativeTarget::Codex,
        NativeTarget::OpenCode,
        NativeTarget::Claude,
        NativeTarget::Antigravity,
    ] {
        assert!(mapping_for(target, PublicationScope::User).is_ok());
    }

    assert!(mapping_for(NativeTarget::Cursor, PublicationScope::User).is_err());
}

#[test]
fn scope_deserialization_rejects_non_v0_1_tiers() {
    for invalid_scope in ["local", "enterprise", "managed", "system"] {
        assert!(
            serde_json::from_value::<PublicationScope>(serde_json::json!(invalid_scope)).is_err()
        );
    }
}

#[test]
fn user_mapping_is_relative_to_the_explicit_harness_root() {
    let mapping = mapping_for(NativeTarget::Claude, PublicationScope::User)
        .expect("Claude user mapping is allowlisted");
    let path = mapping
        .map_artifact(&descriptor(
            NativeArtifactClass::SkillInstruction,
            "skills/review/SKILL.md",
        ))
        .expect("documented user skill artifact is accepted");

    assert_eq!(path.as_str(), "skills/review/SKILL.md");
}

#[test]
fn registry_rejects_an_artifact_class_or_path_outside_the_mapping_grammar() {
    let cursor = mapping_for(NativeTarget::Cursor, PublicationScope::Project)
        .expect("Cursor project mapping exists");

    assert!(cursor
        .map_artifact(&descriptor(
            NativeArtifactClass::SkillInstruction,
            "skills/review/SKILL.md"
        ))
        .is_err());
    assert!(cursor
        .map_artifact(&descriptor(NativeArtifactClass::Rule, "rules/review.md"))
        .is_err());
}

#[test]
fn root_identity_is_stable_and_exposes_only_its_digest() {
    let first =
        RootIdentity::from_platform_components("/workspace/project", b"volume-1", b"file-1");
    let same = RootIdentity::from_platform_components("/workspace/project", b"volume-1", b"file-1");
    let different =
        RootIdentity::from_platform_components("/workspace/project", b"volume-1", b"file-2");

    assert_eq!(first, same);
    assert_ne!(first, different);
    assert!(first.as_str().starts_with("root_"));
    assert!(!first.as_str().contains("workspace"));
}

fn package_id() -> PackageId {
    serde_json::from_value(serde_json::json!(
        "pkg_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    ))
    .expect("fixture package ID is structurally valid")
}

fn sample_plan() -> PublicationPlan {
    let artifact = ArtifactDescriptor {
        class: NativeArtifactClass::Rule,
        native_path: ResourcePath::parse("rules/review.md").expect("valid native path"),
    };
    PublicationPlan {
        plan_version: "0.1",
        compiler_version: "0.1.0-test".to_owned(),
        graph_version: "0.1".to_owned(),
        graph_digest: PlanDigest::from_bytes(b"graph"),
        mappings: BTreeMap::from([(
            (NativeTarget::OpenCode, PublicationScope::Project),
            MappingVersion::V0_1,
        )]),
        roots: vec![RootBinding {
            target: NativeTarget::OpenCode,
            scope: PublicationScope::Project,
            identity: RootIdentity::from_platform_components("/repo", b"volume", b"file"),
        }],
        allow_lossy: false,
        losses: vec![],
        entries: vec![PlanEntry {
            entry_id: "entry_review".to_owned(),
            target: NativeTarget::OpenCode,
            mapping_version: MappingVersion::V0_1,
            scope: PublicationScope::Project,
            stage_artifact_path: ResourcePath::parse("artifacts/entry_review/rules/review.md")
                .expect("valid stage artifact path"),
            artifact,
            content_digest: PlanDigest::from_bytes(b"artifact"),
            byte_length: 8,
            executable: false,
            source_package: package_id(),
        }],
    }
}

fn tampered_plan(mutator: impl FnOnce(&mut serde_json::Value)) -> (Vec<u8>, PlanDigest) {
    let mut value: serde_json::Value =
        serde_json::from_slice(&canonical_plan_json(&sample_plan()).expect("serializes plan"))
            .expect("canonical plan is JSON");
    mutator(&mut value);
    let bytes = serde_json::to_vec(&value).expect("serializes tampered JSON");
    let digest = PlanDigest::from_bytes(&bytes);
    (bytes, digest)
}

#[test]
fn plan_serialization_is_canonical_and_digest_bearing() {
    let plan = sample_plan();
    let first = canonical_plan_json(&plan).expect("serializes plan");
    let second = canonical_plan_json(&plan).expect("serializes plan again");

    assert_eq!(first, second);
    assert!(first.ends_with(b"\n"));
    assert_eq!(PlanDigest::from_bytes(&first).as_str().len(), 71);
}

#[test]
fn artifact_loss_requires_its_entry_and_allow_lossy_tracks_recorded_losses() {
    let mut plan = sample_plan();
    plan.losses.push(PlanLossFinding {
        id: "loss_review".to_owned(),
        entry_id: Some(plan.entries[0].entry_id.clone()),
        package_id: plan.entries[0].source_package.clone(),
        target: NativeTarget::OpenCode,
        artifact: Some(plan.entries[0].artifact.clone()),
        severity: CapabilitySeverity::Lossy,
        reason_code: CapabilityReasonCode::ExecutableBitUnrepresentable,
        reason: "target loses executable metadata".to_owned(),
    });

    assert!(canonical_plan_json(&plan).is_err());

    plan.allow_lossy = true;
    let bytes = canonical_plan_json(&plan).expect("serializes accepted artifact loss");
    let digest = PlanDigest::from_bytes(&bytes);
    let parsed = parse_plan_with_expected_digest(&bytes, &digest).expect("parses loss entry ID");
    assert_eq!(parsed.losses[0].entry_id.as_deref(), Some("entry_review"));

    plan.losses[0].entry_id = None;
    assert!(canonical_plan_json(&plan).is_err());
}

#[test]
fn expected_digest_is_checked_before_plan_json_is_deserialized() {
    let bytes = br#"this is not JSON"#;
    let expected = PlanDigest::from_bytes(b"a different plan");

    let error =
        parse_plan_with_expected_digest(bytes, &expected).expect_err("digest mismatch fails");

    assert!(error.to_string().contains("expected plan digest"));
}

#[test]
fn plan_parser_rejects_unknown_authority_or_root_fields() {
    let (bytes, digest) = tampered_plan(|plan| {
        plan.as_object_mut().expect("plan is an object").insert(
            "authorized_root".to_owned(),
            serde_json::json!("/tmp/unsafe"),
        );
    });

    assert!(parse_plan_with_expected_digest(&bytes, &digest).is_err());
}

#[test]
fn plan_parser_rejects_duplicate_entry_identifiers_and_artifact_paths() {
    let (bytes, digest) = tampered_plan(|plan| {
        let entries = plan["entries"].as_array_mut().expect("entries array");
        entries.push(entries[0].clone());
    });
    assert!(parse_plan_with_expected_digest(&bytes, &digest).is_err());

    let (bytes, digest) = tampered_plan(|plan| {
        let duplicate = plan["entries"][0].clone();
        let mut duplicate = duplicate.as_object().expect("entry object").clone();
        duplicate.insert("entry_id".to_owned(), serde_json::json!("entry_other"));
        plan["entries"]
            .as_array_mut()
            .expect("entries array")
            .push(serde_json::Value::Object(duplicate));
    });
    assert!(parse_plan_with_expected_digest(&bytes, &digest).is_err());
}

#[test]
fn plan_parser_rejects_unsafe_artifact_paths_and_unknown_mapping_versions() {
    let (bytes, digest) = tampered_plan(|plan| {
        plan["entries"][0]["stage_artifact_path"] = serde_json::json!("../escape");
    });
    assert!(parse_plan_with_expected_digest(&bytes, &digest).is_err());

    let (bytes, digest) = tampered_plan(|plan| {
        plan["mappings"][0]["version"] = serde_json::json!("99.0");
    });
    assert!(parse_plan_with_expected_digest(&bytes, &digest).is_err());
}
