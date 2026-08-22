use crate::emitters::lowering::{
    CapabilityReasonCode, CapabilitySeverity, NativeArtifactClass, NativeTarget,
};
use crate::ir::graph::{PackageId, ResourcePath};
use crate::publication::mapping_for;
use crate::publication::model::{
    ArtifactDescriptor, MappingVersion, PlanDigest, PlanEntry, PlanLossFinding, PublicationPlan,
    PublicationScope, RootBinding, RootIdentity,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const PLAN_VERSION: &str = "0.1";

/// Serializes a validated publication plan to the exact canonical bytes used
/// for its SHA-256 digest.
pub fn canonical_plan_json(plan: &PublicationPlan) -> Result<Vec<u8>> {
    validate_plan(plan)?;
    let wire = plan_to_wire(plan)?;
    let mut bytes = serde_json::to_vec(&wire)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Verifies the digest of the raw plan bytes before JSON parsing or any other
/// interpretation of their contents.
pub fn parse_plan_with_expected_digest(
    bytes: &[u8],
    expected_digest: &PlanDigest,
) -> Result<PublicationPlan> {
    let actual_digest = PlanDigest::from_bytes(bytes);
    if &actual_digest != expected_digest {
        bail!(
            "expected plan digest `{}`, got `{}`",
            expected_digest.as_str(),
            actual_digest.as_str(),
        );
    }

    let wire: PlanWire = serde_json::from_slice(bytes).context("invalid publication plan JSON")?;
    let plan = plan_from_wire(wire)?;
    let canonical = canonical_plan_json(&plan)?;
    if bytes != canonical.as_slice() {
        bail!("publication plan JSON is not canonical");
    }
    Ok(plan)
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PlanWire {
    plan_version: String,
    compiler_version: String,
    graph_version: String,
    graph_digest: String,
    mappings: Vec<MappingWire>,
    roots: Vec<RootBindingWire>,
    allow_lossy: bool,
    losses: Vec<PlanLossFindingWire>,
    entries: Vec<PlanEntryWire>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MappingWire {
    target: String,
    scope: PublicationScope,
    version: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RootBindingWire {
    target: String,
    scope: PublicationScope,
    identity: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ArtifactDescriptorWire {
    class: String,
    native_path: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PlanEntryWire {
    entry_id: String,
    target: String,
    mapping_version: String,
    scope: PublicationScope,
    stage_artifact_path: String,
    artifact: ArtifactDescriptorWire,
    content_digest: String,
    byte_length: u64,
    executable: bool,
    source_package: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PlanLossFindingWire {
    id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entry_id: Option<String>,
    package_id: String,
    target: String,
    artifact: Option<ArtifactDescriptorWire>,
    severity: String,
    reason_code: String,
    reason: String,
}

fn plan_to_wire(plan: &PublicationPlan) -> Result<PlanWire> {
    let mut roots = plan.roots.clone();
    roots.sort_by_key(|root| (root.target, root.scope, root.identity.clone()));

    let mut losses = plan.losses.clone();
    losses.sort_by_key(|finding| finding.id.clone());

    let mut entries = plan.entries.clone();
    entries.sort_by_key(|entry| entry.entry_id.clone());

    Ok(PlanWire {
        plan_version: plan.plan_version.to_owned(),
        compiler_version: plan.compiler_version.clone(),
        graph_version: plan.graph_version.clone(),
        graph_digest: plan.graph_digest.as_str().to_owned(),
        mappings: plan
            .mappings
            .iter()
            .map(|((target, scope), version)| MappingWire {
                target: target_name(*target).to_owned(),
                scope: *scope,
                version: version.as_str().to_owned(),
            })
            .collect(),
        roots: roots
            .into_iter()
            .map(|root| RootBindingWire {
                target: target_name(root.target).to_owned(),
                scope: root.scope,
                identity: root.identity.as_str().to_owned(),
            })
            .collect(),
        allow_lossy: plan.allow_lossy,
        losses: losses
            .into_iter()
            .map(loss_to_wire)
            .collect::<Result<_>>()?,
        entries: entries
            .into_iter()
            .map(entry_to_wire)
            .collect::<Result<_>>()?,
    })
}

fn plan_from_wire(wire: PlanWire) -> Result<PublicationPlan> {
    if wire.plan_version != PLAN_VERSION {
        bail!(
            "unsupported publication plan version `{}`",
            wire.plan_version
        );
    }

    let mut mappings = std::collections::BTreeMap::new();
    for mapping in wire.mappings {
        let target = parse_target(&mapping.target)?;
        let version = MappingVersion::parse(&mapping.version)?;
        let expected = mapping_for(target, mapping.scope)?;
        if expected.version() != version {
            bail!(
                "unsupported mapping version `{}` for {}@{}",
                mapping.version,
                mapping.target,
                scope_name(mapping.scope),
            );
        }
        if mappings.insert((target, mapping.scope), version).is_some() {
            bail!("publication plan contains duplicate target mapping");
        }
    }

    let mut roots = Vec::with_capacity(wire.roots.len());
    let mut root_keys = BTreeSet::new();
    for root in wire.roots {
        let target = parse_target(&root.target)?;
        if !root_keys.insert((target, root.scope)) {
            bail!("publication plan contains duplicate root binding");
        }
        roots.push(RootBinding {
            target,
            scope: root.scope,
            identity: RootIdentity::parse(root.identity)?,
        });
    }

    let losses = wire
        .losses
        .into_iter()
        .map(loss_from_wire)
        .collect::<Result<Vec<_>>>()?;
    let entries = wire
        .entries
        .into_iter()
        .map(entry_from_wire)
        .collect::<Result<Vec<_>>>()?;

    let plan = PublicationPlan {
        plan_version: PLAN_VERSION,
        compiler_version: wire.compiler_version,
        graph_version: wire.graph_version,
        graph_digest: PlanDigest::parse(wire.graph_digest)?,
        mappings,
        roots,
        allow_lossy: wire.allow_lossy,
        losses,
        entries,
    };
    validate_plan(&plan)?;
    Ok(plan)
}

fn entry_to_wire(entry: PlanEntry) -> Result<PlanEntryWire> {
    Ok(PlanEntryWire {
        entry_id: entry.entry_id,
        target: target_name(entry.target).to_owned(),
        mapping_version: entry.mapping_version.as_str().to_owned(),
        scope: entry.scope,
        stage_artifact_path: entry.stage_artifact_path.as_str().to_owned(),
        artifact: artifact_to_wire(entry.artifact),
        content_digest: entry.content_digest.as_str().to_owned(),
        byte_length: entry.byte_length,
        executable: entry.executable,
        source_package: entry.source_package.as_str().to_owned(),
    })
}

fn entry_from_wire(entry: PlanEntryWire) -> Result<PlanEntry> {
    let stage_artifact_path = ResourcePath::parse(entry.stage_artifact_path)?;
    if !stage_artifact_path.as_str().starts_with("artifacts/") {
        bail!("stage artifact path must be contained below `artifacts/`");
    }

    Ok(PlanEntry {
        entry_id: parse_identifier(&entry.entry_id, "entry identifier")?,
        target: parse_target(&entry.target)?,
        mapping_version: MappingVersion::parse(&entry.mapping_version)?,
        scope: entry.scope,
        stage_artifact_path,
        artifact: artifact_from_wire(entry.artifact)?,
        content_digest: PlanDigest::parse(entry.content_digest)?,
        byte_length: entry.byte_length,
        executable: entry.executable,
        source_package: package_id_from_wire(entry.source_package)?,
    })
}

fn loss_to_wire(finding: PlanLossFinding) -> Result<PlanLossFindingWire> {
    Ok(PlanLossFindingWire {
        id: finding.id,
        entry_id: finding.entry_id,
        package_id: finding.package_id.as_str().to_owned(),
        target: target_name(finding.target).to_owned(),
        artifact: finding.artifact.map(artifact_to_wire),
        severity: severity_name(finding.severity).to_owned(),
        reason_code: reason_code_name(finding.reason_code).to_owned(),
        reason: finding.reason,
    })
}

fn loss_from_wire(finding: PlanLossFindingWire) -> Result<PlanLossFinding> {
    Ok(PlanLossFinding {
        id: parse_identifier(&finding.id, "loss finding identifier")?,
        entry_id: finding
            .entry_id
            .map(|entry_id| parse_identifier(&entry_id, "loss finding entry identifier"))
            .transpose()?,
        package_id: package_id_from_wire(finding.package_id)?,
        target: parse_target(&finding.target)?,
        artifact: finding.artifact.map(artifact_from_wire).transpose()?,
        severity: parse_severity(&finding.severity)?,
        reason_code: parse_reason_code(&finding.reason_code)?,
        reason: parse_nonempty_string(&finding.reason, "loss finding reason")?,
    })
}

fn artifact_to_wire(artifact: ArtifactDescriptor) -> ArtifactDescriptorWire {
    ArtifactDescriptorWire {
        class: artifact_class_name(artifact.class).to_owned(),
        native_path: artifact.native_path.as_str().to_owned(),
    }
}

fn artifact_from_wire(artifact: ArtifactDescriptorWire) -> Result<ArtifactDescriptor> {
    Ok(ArtifactDescriptor {
        class: parse_artifact_class(&artifact.class)?,
        native_path: ResourcePath::parse(artifact.native_path)?,
    })
}

fn validate_plan(plan: &PublicationPlan) -> Result<()> {
    if plan.plan_version != PLAN_VERSION {
        bail!(
            "unsupported publication plan version `{}`",
            plan.plan_version
        );
    }
    parse_nonempty_string(&plan.compiler_version, "compiler version")?;
    parse_nonempty_string(&plan.graph_version, "graph version")?;
    PlanDigest::parse(plan.graph_digest.as_str())?;
    if plan.allow_lossy == plan.losses.is_empty() {
        bail!("allow_lossy must be true exactly when accepted losses are recorded");
    }

    for ((target, scope), version) in &plan.mappings {
        let mapping = mapping_for(*target, *scope)?;
        if mapping.version() != *version {
            bail!(
                "unsupported mapping version `{}` for {}@{}",
                version.as_str(),
                target_name(*target),
                scope_name(*scope),
            );
        }
    }

    let mut root_keys = BTreeSet::new();
    for root in &plan.roots {
        RootIdentity::parse(root.identity.as_str())?;
        if !root_keys.insert((root.target, root.scope)) {
            bail!("publication plan contains duplicate root binding");
        }
        if !plan.mappings.contains_key(&(root.target, root.scope)) {
            bail!("root binding has no matching target mapping");
        }
    }

    let mut entry_ids = BTreeSet::new();
    let mut stage_paths = BTreeSet::new();
    let mut artifact_paths = BTreeSet::new();
    for entry in &plan.entries {
        parse_identifier(&entry.entry_id, "entry identifier")?;
        if !entry_ids.insert(entry.entry_id.as_str()) {
            bail!("publication plan contains duplicate entry identifier");
        }
        if !entry.stage_artifact_path.as_str().starts_with("artifacts/") {
            bail!("stage artifact path must be contained below `artifacts/`");
        }
        if !stage_paths.insert(entry.stage_artifact_path.as_str()) {
            bail!("publication plan contains duplicate stage artifact path");
        }
        let mapping = mapping_for(entry.target, entry.scope)?;
        if mapping.version() != entry.mapping_version {
            bail!("entry has unsupported target mapping version");
        }
        if plan.mappings.get(&(entry.target, entry.scope)) != Some(&entry.mapping_version) {
            bail!("entry has no matching target mapping");
        }
        let mapped_path = mapping.map_artifact(&entry.artifact)?;
        if !artifact_paths.insert((entry.target, entry.scope, mapped_path)) {
            bail!("publication plan contains duplicate mapped artifact path");
        }
        PlanDigest::parse(entry.content_digest.as_str())?;
        validate_package_id(&entry.source_package)?;
    }

    let mut loss_ids = BTreeSet::new();
    for loss in &plan.losses {
        parse_identifier(&loss.id, "loss finding identifier")?;
        if !loss_ids.insert(loss.id.as_str()) {
            bail!("publication plan contains duplicate loss finding identifier");
        }
        validate_package_id(&loss.package_id)?;
        parse_nonempty_string(&loss.reason, "loss finding reason")?;
        match (&loss.entry_id, &loss.artifact) {
            (Some(entry_id), Some(artifact)) => {
                parse_identifier(entry_id, "loss finding entry identifier")?;
                if !entry_ids.contains(entry_id.as_str()) {
                    bail!("loss finding references an unknown staged artifact entry");
                }
                let entry = plan
                    .entries
                    .iter()
                    .find(|entry| entry.entry_id == *entry_id)
                    .expect("validated entry identifier exists");
                if entry.target != loss.target
                    || entry.source_package != loss.package_id
                    || entry.artifact != *artifact
                {
                    bail!("loss finding entry identifier does not match its artifact context");
                }
                ResourcePath::parse(artifact.native_path.as_str())?;
            }
            (None, None) => {}
            _ => bail!(
                "loss finding artifact and entry identifier must either both be present or absent"
            ),
        }
    }
    Ok(())
}

fn parse_target(value: &str) -> Result<NativeTarget> {
    match value {
        "codex" => Ok(NativeTarget::Codex),
        "opencode" => Ok(NativeTarget::OpenCode),
        "claude" => Ok(NativeTarget::Claude),
        "cursor" => Ok(NativeTarget::Cursor),
        "antigravity" => Ok(NativeTarget::Antigravity),
        _ => bail!("unsupported native target `{value}`"),
    }
}

fn target_name(target: NativeTarget) -> &'static str {
    target.as_str()
}

fn parse_artifact_class(value: &str) -> Result<NativeArtifactClass> {
    match value {
        "instruction" => Ok(NativeArtifactClass::Instruction),
        "rule" => Ok(NativeArtifactClass::Rule),
        "skill-instruction" => Ok(NativeArtifactClass::SkillInstruction),
        "skill-resource" => Ok(NativeArtifactClass::SkillResource),
        _ => bail!("unsupported native artifact class `{value}`"),
    }
}

fn artifact_class_name(class: NativeArtifactClass) -> &'static str {
    match class {
        NativeArtifactClass::Instruction => "instruction",
        NativeArtifactClass::Rule => "rule",
        NativeArtifactClass::SkillInstruction => "skill-instruction",
        NativeArtifactClass::SkillResource => "skill-resource",
    }
}

fn parse_severity(value: &str) -> Result<CapabilitySeverity> {
    match value {
        "supported" => Ok(CapabilitySeverity::Supported),
        "lossy" => Ok(CapabilitySeverity::Lossy),
        "dropped" => Ok(CapabilitySeverity::Dropped),
        _ => bail!("unsupported capability severity `{value}`"),
    }
}

fn severity_name(severity: CapabilitySeverity) -> &'static str {
    match severity {
        CapabilitySeverity::Supported => "supported",
        CapabilitySeverity::Lossy => "lossy",
        CapabilitySeverity::Dropped => "dropped",
    }
}

fn parse_reason_code(value: &str) -> Result<CapabilityReasonCode> {
    match value {
        "representable" => Ok(CapabilityReasonCode::Representable),
        "unsupported-semantic" => Ok(CapabilityReasonCode::UnsupportedSemantic),
        "opaque-cross-domain" => Ok(CapabilityReasonCode::OpaqueCrossDomain),
        "opaque-resource-unrepresentable" => {
            Ok(CapabilityReasonCode::OpaqueResourceUnrepresentable)
        }
        "executable-bit-unrepresentable" => Ok(CapabilityReasonCode::ExecutableBitUnrepresentable),
        "skill-lowered-as-rule" => Ok(CapabilityReasonCode::SkillLoweredAsRule),
        _ => bail!("unsupported capability reason code `{value}`"),
    }
}

fn reason_code_name(reason_code: CapabilityReasonCode) -> &'static str {
    reason_code.as_str()
}

fn package_id_from_wire(value: String) -> Result<PackageId> {
    let package_id = serde_json::from_value(serde_json::Value::String(value))
        .context("invalid source package identifier")?;
    validate_package_id(&package_id)?;
    Ok(package_id)
}

fn validate_package_id(package_id: &PackageId) -> Result<()> {
    let value = package_id.as_str();
    if value.len() != 68
        || !value.starts_with("pkg_")
        || !value[4..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("source package identifier must be `pkg_` followed by a SHA-256 hex digest");
    }
    Ok(())
}

fn parse_identifier(value: &str, label: &str) -> Result<String> {
    if value.is_empty()
        || value.chars().any(char::is_control)
        || value.contains('/')
        || value.contains('\\')
    {
        bail!("{label} must be non-empty and path-component safe");
    }
    Ok(value.to_owned())
}

fn parse_nonempty_string(value: &str, label: &str) -> Result<String> {
    if value.is_empty() || value.chars().any(char::is_control) {
        bail!("{label} must be non-empty and control-character free");
    }
    Ok(value.to_owned())
}

fn scope_name(scope: PublicationScope) -> &'static str {
    match scope {
        PublicationScope::Project => "project",
        PublicationScope::User => "user",
    }
}
