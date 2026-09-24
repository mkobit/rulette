use crate::ir::graph::{
    CompilationGraph, Package, PackageId, PortableActivation, ResourceContent, ResourcePath,
    SemanticItem, SourceProvenance,
};
use crate::parsers::agent_plugin::{McpConfigWireV1, McpServerWireV1, PluginManifestWireV1};
use crate::ActivationMode;
use anyhow::{bail, Result};
use std::collections::BTreeMap;

/// The five native harnesses included in Rulette's v0.1 portability surface.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NativeTarget {
    Codex,
    OpenCode,
    Claude,
    Cursor,
    Antigravity,
    AgentPlugin,
}

impl NativeTarget {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::OpenCode => "opencode",
            Self::Claude => "claude",
            Self::Cursor => "cursor",
            Self::Antigravity => "antigravity",
            Self::AgentPlugin => "agent-plugin",
        }
    }
}

/// A native artifact category that the later publication layer maps below an
/// explicitly authorized root.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NativeArtifactClass {
    Instruction,
    Rule,
    SkillInstruction,
    SkillResource,
    PluginManifest,
    McpConfig,
    ClientExtension,
}

/// A target-relative artifact produced by a graph backend.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeArtifact {
    pub target: NativeTarget,
    pub class: NativeArtifactClass,
    pub path: ResourcePath,
    pub bytes: Vec<u8>,
    pub executable: bool,
    pub source_package: PackageId,
}

/// The severity of one representability decision.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CapabilitySeverity {
    Supported,
    Lossy,
    Dropped,
}

/// A stable machine-readable reason for a capability finding.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CapabilityReasonCode {
    Representable,
    UnsupportedSemantic,
    OpaqueCrossDomain,
    OpaqueResourceUnrepresentable,
    ExecutableBitUnrepresentable,
    SkillLoweredAsRule,
    RuleLoweredAsSkill,
    SynthesizedManifest,
}

impl CapabilityReasonCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Representable => "representable",
            Self::UnsupportedSemantic => "unsupported-semantic",
            Self::OpaqueCrossDomain => "opaque-cross-domain",
            Self::OpaqueResourceUnrepresentable => "opaque-resource-unrepresentable",
            Self::ExecutableBitUnrepresentable => "executable-bit-unrepresentable",
            Self::SkillLoweredAsRule => "skill-lowered-as-rule",
            Self::RuleLoweredAsSkill => "rule-lowered-as-skill",
            Self::SynthesizedManifest => "synthesized-manifest",
        }
    }
}

/// A deterministic capability decision for a package or one of its resources.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapabilityFinding {
    pub id: String,
    pub target: NativeTarget,
    pub package_id: PackageId,
    pub provenance: SourceProvenance,
    pub resource_path: Option<ResourcePath>,
    pub severity: CapabilitySeverity,
    pub reason_code: CapabilityReasonCode,
    pub reason: String,
    pub artifact_path: Option<ResourcePath>,
}

/// Ordered backend output that is safe to hand to a publication layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweringPlan {
    pub target: NativeTarget,
    pub artifacts: Vec<NativeArtifact>,
    pub findings: Vec<CapabilityFinding>,
}

/// Controls whether a caller explicitly accepts representational loss.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LoweringOptions {
    pub allow_lossy: bool,
}

impl LoweringOptions {
    pub const fn strict() -> Self {
        Self { allow_lossy: false }
    }

    pub const fn allow_lossy() -> Self {
        Self { allow_lossy: true }
    }
}

/// Lowers a validated graph without reading or writing caller paths.
pub fn lower(
    graph: &CompilationGraph,
    target: NativeTarget,
    options: LoweringOptions,
) -> Result<LoweringPlan> {
    graph.validate()?;

    if target == NativeTarget::AgentPlugin {
        return lower_agent_plugin(graph, options);
    }

    let mut artifacts = Vec::new();
    let mut findings: Vec<CapabilityFinding> = Vec::new();
    for package in graph.packages.values() {
        match &package.semantic_item {
            SemanticItem::Rule {
                primary_instruction,
                ..
            } => {
                let artifact = lower_rule(package, target)?;
                findings.push(supported_finding(
                    package,
                    target,
                    primary_instruction.clone(),
                    artifact.path.clone(),
                ));
                for resource in package.resources.values() {
                    if resource.path != *primary_instruction {
                        findings.push(dropped_finding(
                            package,
                            target,
                            CapabilityReasonCode::OpaqueResourceUnrepresentable,
                            Some(resource.path.clone()),
                            "rule packages have no native opaque-resource layout in the target",
                        ));
                    }
                }
                artifacts.push(artifact);
            }
            SemanticItem::Skill { .. } if target == NativeTarget::Cursor => {
                let artifact = lower_skill_as_cursor_rule(package)?;
                findings.push(loss_finding(
                    package,
                    target,
                    CapabilityReasonCode::SkillLoweredAsRule,
                    None,
                    Some(artifact.path.clone()),
                    "Cursor has no native skill-package target, so the skill primary instruction is lowered as a rule",
                ));
                let SemanticItem::Skill {
                    primary_instruction,
                    ..
                } = &package.semantic_item
                else {
                    unreachable!("cursor skill branch contains a skill package");
                };
                findings.push(supported_finding(
                    package,
                    target,
                    primary_instruction.clone(),
                    artifact.path.clone(),
                ));
                for resource in package.resources.values() {
                    if resource.path != *primary_instruction {
                        findings.push(dropped_finding(
                            package,
                            target,
                            CapabilityReasonCode::OpaqueResourceUnrepresentable,
                            Some(resource.path.clone()),
                            "Cursor has no native skill-package layout for opaque resources",
                        ));
                    }
                }
                artifacts.push(artifact);
            }
            SemanticItem::Skill { .. } => {
                let (skill_artifacts, skill_findings) = lower_skill_package(package, target)?;
                artifacts.extend(skill_artifacts);
                findings.extend(skill_findings);
            }
            SemanticItem::Unsupported { native_kind } => {
                findings.push(dropped_finding(
                    package,
                    target,
                    CapabilityReasonCode::UnsupportedSemantic,
                    None,
                    format!(
                        "{} is not a portable v0.1 semantic item for {}",
                        native_kind,
                        target.as_str()
                    ),
                ));
                for resource in package.resources.values() {
                    findings.push(dropped_finding(
                        package,
                        target,
                        CapabilityReasonCode::UnsupportedSemantic,
                        Some(resource.path.clone()),
                        "resource belongs to an unsupported native semantic package",
                    ));
                }
            }
        }
        for resource in package
            .resources
            .values()
            .filter(|resource| resource.executable)
        {
            let is_primary = matches!(
                &package.semantic_item,
                SemanticItem::Rule {
                    primary_instruction,
                    ..
                } | SemanticItem::Skill {
                    primary_instruction,
                    ..
                } if primary_instruction == &resource.path
            );
            let artifact_path = is_primary
                .then(|| {
                    artifacts
                        .iter()
                        .rev()
                        .find(|artifact| artifact.source_package == package.id)
                        .map(|artifact| artifact.path.clone())
                })
                .flatten();
            findings.push(loss_finding(
                package,
                target,
                CapabilityReasonCode::ExecutableBitUnrepresentable,
                Some(resource.path.clone()),
                artifact_path,
                "the target native artifact format does not preserve executable-bit metadata",
            ));
        }
    }

    append_package_outcomes(graph, target, &artifacts, &mut findings);
    findings.sort_by(|left, right| left.id.cmp(&right.id));

    validate_no_collisions(&artifacts)?;
    if !options.allow_lossy
        && findings
            .iter()
            .any(|finding| finding.severity != CapabilitySeverity::Supported)
    {
        bail!(
            "{} lowering has unaccepted capability loss",
            target.as_str()
        );
    }

    Ok(LoweringPlan {
        target,
        artifacts,
        findings,
    })
}

fn lower_skill_package(
    package: &Package,
    target: NativeTarget,
) -> Result<(Vec<NativeArtifact>, Vec<CapabilityFinding>)> {
    let SemanticItem::Skill {
        primary_instruction,
        description,
        ..
    } = &package.semantic_item
    else {
        unreachable!("lower_skill_package only accepts skill packages");
    };
    let primary = package
        .resources
        .get(primary_instruction)
        .expect("validated skill package contains its primary instruction");
    let ResourceContent::Text(text) = &primary.content else {
        unreachable!("validated primary instruction is UTF-8 text");
    };
    let name = logical_name(package);
    #[derive(serde::Serialize)]
    struct SkillFrontmatter<'a> {
        name: &'a str,
        description: &'a str,
    }
    let mut rendered = String::from("---\n");
    rendered.push_str(&serde_yaml::to_string(&SkillFrontmatter {
        name,
        description,
    })?);
    rendered.push_str("---\n");
    rendered.push_str(text);

    let mut artifacts = vec![NativeArtifact {
        target,
        class: NativeArtifactClass::SkillInstruction,
        path: ResourcePath::parse(format!("skills/{name}/SKILL.md"))?,
        bytes: rendered.into_bytes(),
        executable: false,
        source_package: package.id.clone(),
    }];
    let mut findings = Vec::new();
    findings.push(supported_finding(
        package,
        target,
        primary_instruction.clone(),
        artifacts[0].path.clone(),
    ));
    let supports_same_domain_resources = package.provenance.frontend == target.as_str();
    for resource in package.resources.values() {
        if resource.path == *primary_instruction {
            continue;
        }
        if supports_same_domain_resources {
            artifacts.push(NativeArtifact {
                target,
                class: NativeArtifactClass::SkillResource,
                path: ResourcePath::parse(format!("skills/{name}/{}", resource.path.as_str()))?,
                bytes: resource_content_bytes(&resource.content),
                executable: false,
                source_package: package.id.clone(),
            });
            let artifact_path = artifacts
                .last()
                .expect("opaque resource artifact was just appended")
                .path
                .clone();
            findings.push(supported_finding(
                package,
                target,
                resource.path.clone(),
                artifact_path,
            ));
        } else {
            findings.push(dropped_finding(
                package,
                target,
                CapabilityReasonCode::OpaqueCrossDomain,
                Some(resource.path.clone()),
                "opaque native resource cannot cross harness domains because the target has not declared the source package shape compatible",
            ));
        }
    }
    Ok((artifacts, findings))
}

fn append_package_outcomes(
    graph: &CompilationGraph,
    target: NativeTarget,
    artifacts: &[NativeArtifact],
    findings: &mut Vec<CapabilityFinding>,
) {
    for package in graph.packages.values() {
        if findings
            .iter()
            .any(|finding| finding.package_id == package.id && finding.resource_path.is_none())
        {
            continue;
        }
        let worst = findings
            .iter()
            .filter(|finding| finding.package_id == package.id)
            .max_by(|left, right| {
                left.severity
                    .cmp(&right.severity)
                    .then_with(|| left.reason_code.cmp(&right.reason_code))
            });
        let (severity, reason_code, reason) = worst.map_or(
            (
                CapabilitySeverity::Supported,
                CapabilityReasonCode::Representable,
                "all package resources are representable by the target".to_owned(),
            ),
            |finding| {
                (
                    finding.severity,
                    finding.reason_code,
                    format!("package outcome: {}", finding.reason),
                )
            },
        );
        let artifact_path = artifacts
            .iter()
            .find(|artifact| artifact.source_package == package.id)
            .map(|artifact| artifact.path.clone());
        findings.push(capability_finding(
            package,
            target,
            severity,
            reason_code,
            None,
            artifact_path,
            reason,
        ));
    }
}

fn resource_content_bytes(content: &ResourceContent) -> Vec<u8> {
    match content {
        ResourceContent::Text(text) => text.as_bytes().to_vec(),
        ResourceContent::Bytes(bytes) => bytes.clone(),
    }
}

fn supported_finding(
    package: &Package,
    target: NativeTarget,
    resource_path: ResourcePath,
    artifact_path: ResourcePath,
) -> CapabilityFinding {
    capability_finding(
        package,
        target,
        CapabilitySeverity::Supported,
        CapabilityReasonCode::Representable,
        Some(resource_path),
        Some(artifact_path),
        "resource is represented by a native artifact",
    )
}

fn lower_skill_as_cursor_rule(package: &Package) -> Result<NativeArtifact> {
    let SemanticItem::Skill {
        primary_instruction,
        description,
        ..
    } = &package.semantic_item
    else {
        unreachable!("lower_skill_as_cursor_rule only accepts skill packages");
    };
    let resource = package
        .resources
        .get(primary_instruction)
        .expect("validated skill package contains its primary instruction");
    let ResourceContent::Text(text) = &resource.content else {
        unreachable!("validated primary instruction is UTF-8 text");
    };
    #[derive(serde::Serialize)]
    struct CursorSkillRule<'a> {
        description: &'a str,
    }
    let mut rendered = String::from("---\n");
    rendered.push_str(&serde_yaml::to_string(&CursorSkillRule { description })?);
    rendered.push_str("---\n");
    rendered.push_str(text);
    Ok(NativeArtifact {
        target: NativeTarget::Cursor,
        class: NativeArtifactClass::Rule,
        path: ResourcePath::parse(format!("rules/{}.mdc", logical_name(package)))?,
        bytes: rendered.into_bytes(),
        executable: false,
        source_package: package.id.clone(),
    })
}

fn loss_finding(
    package: &Package,
    target: NativeTarget,
    reason_code: CapabilityReasonCode,
    resource_path: Option<ResourcePath>,
    artifact_path: Option<ResourcePath>,
    reason: impl Into<String>,
) -> CapabilityFinding {
    capability_finding(
        package,
        target,
        CapabilitySeverity::Lossy,
        reason_code,
        resource_path,
        artifact_path,
        reason,
    )
}

fn dropped_finding(
    package: &Package,
    target: NativeTarget,
    reason_code: CapabilityReasonCode,
    resource_path: Option<ResourcePath>,
    reason: impl Into<String>,
) -> CapabilityFinding {
    capability_finding(
        package,
        target,
        CapabilitySeverity::Dropped,
        reason_code,
        resource_path,
        None,
        reason,
    )
}

fn capability_finding(
    package: &Package,
    target: NativeTarget,
    severity: CapabilitySeverity,
    reason_code: CapabilityReasonCode,
    resource_path: Option<ResourcePath>,
    artifact_path: Option<ResourcePath>,
    reason: impl Into<String>,
) -> CapabilityFinding {
    let resource_identifier = resource_path
        .as_ref()
        .map_or("package", ResourcePath::as_str);
    CapabilityFinding {
        id: format!(
            "{}:{}:{}:{}",
            target.as_str(),
            package.id.as_str(),
            resource_identifier,
            reason_code.as_str(),
        ),
        target,
        package_id: package.id.clone(),
        provenance: package.provenance.clone(),
        resource_path,
        severity,
        reason_code,
        reason: reason.into(),
        artifact_path,
    }
}

fn lower_rule(package: &Package, target: NativeTarget) -> Result<NativeArtifact> {
    let SemanticItem::Rule {
        primary_instruction,
        ..
    } = &package.semantic_item
    else {
        unreachable!("lower_rule only accepts rule packages");
    };
    let resource = package
        .resources
        .get(primary_instruction)
        .expect("validated rule package contains its primary instruction");
    let ResourceContent::Text(text) = &resource.content else {
        unreachable!("validated primary instruction is UTF-8 text");
    };
    let name = logical_name(package);
    let (class, path) = match target {
        NativeTarget::Codex => (NativeArtifactClass::Instruction, "AGENTS.md".to_owned()),
        NativeTarget::OpenCode => (NativeArtifactClass::Rule, format!("rules/{name}.md")),
        NativeTarget::Claude => (NativeArtifactClass::Instruction, "CLAUDE.md".to_owned()),
        NativeTarget::Cursor => (NativeArtifactClass::Rule, format!("rules/{name}.mdc")),
        NativeTarget::Antigravity => (NativeArtifactClass::Rule, format!("rules/{name}.md")),
        NativeTarget::AgentPlugin => (NativeArtifactClass::Rule, format!("rules/{name}.md")),
    };

    Ok(NativeArtifact {
        target,
        class,
        path: ResourcePath::parse(path)?,
        bytes: render_rule(package, target, text)?,
        executable: false,
        source_package: package.id.clone(),
    })
}

fn render_rule(package: &Package, target: NativeTarget, text: &str) -> Result<Vec<u8>> {
    let (description, activation) = match &package.semantic_item {
        SemanticItem::Rule {
            description,
            activation,
            ..
        } => (description.as_deref(), activation.as_ref()),
        _ => unreachable!("render_rule only accepts rule packages"),
    };

    match target {
        NativeTarget::Cursor => render_cursor_rule(description, activation, text),
        NativeTarget::Antigravity => render_antigravity_rule(description, activation, text),
        NativeTarget::Codex
        | NativeTarget::OpenCode
        | NativeTarget::Claude
        | NativeTarget::AgentPlugin => Ok(text.as_bytes().to_vec()),
    }
}

fn render_cursor_rule(
    description: Option<&str>,
    activation: Option<&crate::TargetActivation>,
    text: &str,
) -> Result<Vec<u8>> {
    #[derive(serde::Serialize)]
    struct CursorFrontmatter<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<&'a str>,
        #[serde(rename = "alwaysApply", skip_serializing_if = "Option::is_none")]
        always_apply: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        globs: Option<Vec<String>>,
    }

    let activation = activation.map(|activation| activation.resolve("cursor-mdc"));
    let (always_apply, globs) = activation.map(cursor_fields).unwrap_or((None, None));
    let mut output = String::from("---\n");
    output.push_str(&serde_yaml::to_string(&CursorFrontmatter {
        description,
        always_apply,
        globs,
    })?);
    output.push_str("---\n");
    output.push_str(text);
    Ok(output.into_bytes())
}

fn render_antigravity_rule(
    description: Option<&str>,
    activation: Option<&crate::TargetActivation>,
    text: &str,
) -> Result<Vec<u8>> {
    #[derive(serde::Serialize)]
    struct AntigravityFrontmatter<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        trigger: Option<&'static str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        globs: Option<Vec<String>>,
    }

    let activation = activation.map(|activation| activation.resolve("antigravity"));
    let (trigger, globs) = activation.map(antigravity_fields).unwrap_or((None, None));
    let mut output = String::from("---\n");
    output.push_str(&serde_yaml::to_string(&AntigravityFrontmatter {
        description,
        trigger,
        globs,
    })?);
    output.push_str("---\n");
    output.push_str(text);
    Ok(output.into_bytes())
}

fn cursor_fields(activation: &PortableActivation) -> (Option<bool>, Option<Vec<String>>) {
    if activation.mode.contains(&ActivationMode::Always) {
        (Some(true), activation.globs.clone())
    } else if activation.mode.contains(&ActivationMode::Glob) {
        (Some(false), activation.globs.clone())
    } else {
        (Some(false), None)
    }
}

fn antigravity_fields(
    activation: &PortableActivation,
) -> (Option<&'static str>, Option<Vec<String>>) {
    if activation.mode.contains(&ActivationMode::Always) {
        (Some("always_on"), activation.globs.clone())
    } else if activation.mode.contains(&ActivationMode::Glob) {
        (Some("glob"), activation.globs.clone())
    } else if activation.mode.contains(&ActivationMode::Model) {
        (Some("model_decision"), None)
    } else if activation.mode.contains(&ActivationMode::Manual) {
        (Some("manual"), None)
    } else if activation.mode.contains(&ActivationMode::Pattern) {
        (Some("glob"), activation.globs.clone())
    } else {
        (None, None)
    }
}

fn logical_name(package: &Package) -> &str {
    package
        .semantic_identity
        .as_str()
        .split_once(':')
        .expect("validated semantic identity has a kind prefix")
        .1
}

fn validate_no_collisions(artifacts: &[NativeArtifact]) -> Result<()> {
    let mut previous: Option<(NativeArtifactClass, &str, &PackageId)> = None;
    let mut keys: Vec<_> = artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.class,
                artifact.path.as_str(),
                &artifact.source_package,
            )
        })
        .collect();
    keys.sort_unstable();
    for (class, path, package_id) in keys {
        if let Some((previous_class, previous_path, previous_package)) = previous {
            if (class, path) == (previous_class, previous_path) {
                bail!(
                    "artifact-collision: native artifact collision for {} at `{}` between `{}` and `{}`",
                    artifact_class_name(class),
                    path,
                    previous_package.as_str(),
                    package_id.as_str(),
                );
            }
        }
        previous = Some((class, path, package_id));
    }
    Ok(())
}

fn artifact_class_name(class: NativeArtifactClass) -> &'static str {
    match class {
        NativeArtifactClass::Instruction => "instruction",
        NativeArtifactClass::Rule => "rule",
        NativeArtifactClass::SkillInstruction => "skill-instruction",
        NativeArtifactClass::SkillResource => "skill-resource",
        NativeArtifactClass::PluginManifest => "plugin-manifest",
        NativeArtifactClass::McpConfig => "mcp-config",
        NativeArtifactClass::ClientExtension => "client-extension",
    }
}

fn is_manifest_package(package: &Package) -> bool {
    package
        .semantic_identity
        .as_str()
        .starts_with("unsupported:agent-plugin-manifest/")
        || matches!(
            &package.semantic_item,
            SemanticItem::Unsupported { native_kind } if native_kind == "manifest"
        )
}

fn is_mcp_package(package: &Package) -> bool {
    matches!(
        &package.semantic_item,
        SemanticItem::Unsupported { native_kind } if native_kind == "mcp-server"
    ) || package
        .semantic_identity
        .as_str()
        .starts_with("unsupported:agent-plugin-mcp-server/")
        || package.semantic_identity.as_str().contains("mcp-server")
}

fn is_extension_package(package: &Package) -> bool {
    package
        .semantic_identity
        .as_str()
        .starts_with("unsupported:agent-plugin-extension/")
        || matches!(
            &package.semantic_item,
            SemanticItem::Unsupported { native_kind } if native_kind == "extension"
        )
}

fn sanitize_skill_name(raw: &str) -> String {
    let mut result = String::new();
    let mut last_was_dash = false;
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            result.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            result.push('-');
            last_was_dash = true;
        }
    }
    let trimmed = result.trim_matches('-');
    let mut name = if trimmed.is_empty() {
        "rule".to_string()
    } else {
        trimmed.to_string()
    };
    if name.chars().count() > 64 {
        name = name.chars().take(64).collect();
        name = name.trim_end_matches('-').to_string();
        if name.is_empty() {
            name = "rule".to_string();
        }
    }
    name
}

fn read_mcp_server_from_resources(package: &Package) -> Result<McpServerWireV1> {
    for resource in package.resources.values() {
        let bytes = resource_content_bytes(&resource.content);
        if let Ok(server) = serde_json::from_slice::<McpServerWireV1>(&bytes) {
            return Ok(server);
        }
    }
    bail!("failed to read MCP server configuration from package resources");
}

fn lower_agent_plugin(graph: &CompilationGraph, options: LoweringOptions) -> Result<LoweringPlan> {
    let target = NativeTarget::AgentPlugin;
    let mut artifacts = Vec::new();
    let mut findings = Vec::new();

    // 1. Manifest
    let manifest_packages: Vec<&Package> = graph
        .packages
        .values()
        .filter(|p| is_manifest_package(p))
        .collect();

    if manifest_packages.len() > 1 {
        bail!("aggregation collision: multiple manifest packages found for agent-plugin");
    } else if let Some(manifest_pkg) = manifest_packages.first() {
        let manifest_bytes = if let Some(resource) = manifest_pkg
            .resources
            .values()
            .find(|r| r.path.as_str() == "plugin.json")
        {
            resource_content_bytes(&resource.content)
        } else if let Some(payload) = &manifest_pkg.frontend_payload {
            let map: serde_json::Map<String, serde_json::Value> =
                payload.fields.clone().into_iter().collect();
            let mut s = serde_json::to_string_pretty(&map)?;
            s.push('\n');
            s.into_bytes()
        } else {
            bail!("manifest package is missing manifest content");
        };

        artifacts.push(NativeArtifact {
            target,
            class: NativeArtifactClass::PluginManifest,
            path: ResourcePath::parse("plugin.json")?,
            bytes: manifest_bytes,
            executable: false,
            source_package: manifest_pkg.id.clone(),
        });

        if manifest_pkg
            .resources
            .contains_key(&ResourcePath::parse("plugin.json")?)
        {
            findings.push(supported_finding(
                manifest_pkg,
                target,
                ResourcePath::parse("plugin.json")?,
                ResourcePath::parse("plugin.json")?,
            ));
        } else {
            findings.push(capability_finding(
                manifest_pkg,
                target,
                CapabilitySeverity::Supported,
                CapabilityReasonCode::Representable,
                None,
                Some(ResourcePath::parse("plugin.json")?),
                "plugin manifest is preserved as plugin.json",
            ));
        }
        for resource in manifest_pkg.resources.values() {
            if resource.path.as_str() != "plugin.json" {
                findings.push(dropped_finding(
                    manifest_pkg,
                    target,
                    CapabilityReasonCode::UnsupportedSemantic,
                    Some(resource.path.clone()),
                    "manifest package contains extra resource not representable in agent-plugin",
                ));
            }
        }
    } else {
        // Zero manifest packages
        if let Some(first_pkg) = graph.packages.values().next() {
            findings.push(loss_finding(
                first_pkg,
                target,
                CapabilityReasonCode::SynthesizedManifest,
                None,
                Some(ResourcePath::parse("plugin.json")?),
                "agent-plugin target requires a manifest, so a minimal plugin.json was synthesized",
            ));

            let synthesized_name = {
                let raw = logical_name(first_pkg);
                let after_slash = raw.rsplit_once('/').map(|(_, after)| after).unwrap_or(raw);
                let sanitized = sanitize_skill_name(after_slash);
                if sanitized.is_empty() {
                    "agent-plugin".to_string()
                } else {
                    sanitized
                }
            };
            let wire = PluginManifestWireV1::new(synthesized_name);
            let mut manifest_str = serde_json::to_string_pretty(&wire)?;
            manifest_str.push('\n');
            artifacts.push(NativeArtifact {
                target,
                class: NativeArtifactClass::PluginManifest,
                path: ResourcePath::parse("plugin.json")?,
                bytes: manifest_str.into_bytes(),
                executable: false,
                source_package: first_pkg.id.clone(),
            });
        }
    }

    // 2. Skills and Rules
    for package in graph.packages.values() {
        match &package.semantic_item {
            SemanticItem::Skill { .. } => {
                let (skill_artifacts, skill_findings) = lower_skill_package(package, target)?;
                artifacts.extend(skill_artifacts);
                findings.extend(skill_findings);
            }
            SemanticItem::Rule {
                description,
                primary_instruction,
                ..
            } => {
                if !options.allow_lossy {
                    findings.push(dropped_finding(
                        package,
                        target,
                        CapabilityReasonCode::UnsupportedSemantic,
                        None,
                        "rules are not supported by the agent-plugin target",
                    ));
                    for resource in package.resources.values() {
                        findings.push(dropped_finding(
                            package,
                            target,
                            CapabilityReasonCode::UnsupportedSemantic,
                            Some(resource.path.clone()),
                            "rule resource is not supported by the agent-plugin target",
                        ));
                    }
                } else {
                    let sanitized_name = sanitize_skill_name(logical_name(package));
                    let fallback_desc = format!("Rule lowered as skill: {sanitized_name}");
                    let desc = match description
                        .as_deref()
                        .map(str::trim)
                        .filter(|d| !d.is_empty())
                    {
                        Some(d) => {
                            if d.chars().count() > 1024 {
                                d.chars().take(1024).collect::<String>()
                            } else {
                                d.to_string()
                            }
                        }
                        None => fallback_desc,
                    };

                    let primary = package
                        .resources
                        .get(primary_instruction)
                        .expect("validated rule package contains its primary instruction");
                    let ResourceContent::Text(text) = &primary.content else {
                        unreachable!("validated primary instruction is UTF-8 text");
                    };

                    #[derive(serde::Serialize)]
                    struct SkillFrontmatter<'a> {
                        name: &'a str,
                        description: &'a str,
                    }
                    let mut rendered = String::from("---\n");
                    rendered.push_str(&serde_yaml::to_string(&SkillFrontmatter {
                        name: &sanitized_name,
                        description: &desc,
                    })?);
                    rendered.push_str("---\n");
                    rendered.push_str(text);

                    let artifact_path =
                        ResourcePath::parse(format!("skills/{sanitized_name}/SKILL.md"))?;
                    artifacts.push(NativeArtifact {
                        target,
                        class: NativeArtifactClass::SkillInstruction,
                        path: artifact_path.clone(),
                        bytes: rendered.into_bytes(),
                        executable: false,
                        source_package: package.id.clone(),
                    });

                    findings.push(loss_finding(
                        package,
                        target,
                        CapabilityReasonCode::RuleLoweredAsSkill,
                        None,
                        Some(artifact_path.clone()),
                        "rules have no native activation model in agent-plugin, so the rule is lowered as a skill",
                    ));
                    findings.push(supported_finding(
                        package,
                        target,
                        primary_instruction.clone(),
                        artifact_path,
                    ));
                    for resource in package.resources.values() {
                        if resource.path != *primary_instruction {
                            findings.push(dropped_finding(
                                package,
                                target,
                                CapabilityReasonCode::OpaqueResourceUnrepresentable,
                                Some(resource.path.clone()),
                                "rule packages have no native opaque-resource layout in agent-plugin",
                            ));
                        }
                    }
                }
            }
            SemanticItem::Unsupported { native_kind } => {
                if !is_manifest_package(package)
                    && !is_mcp_package(package)
                    && !is_extension_package(package)
                {
                    findings.push(dropped_finding(
                        package,
                        target,
                        CapabilityReasonCode::UnsupportedSemantic,
                        None,
                        format!(
                            "{} is not a portable v0.1 semantic item for {}",
                            native_kind,
                            target.as_str()
                        ),
                    ));
                    for resource in package.resources.values() {
                        findings.push(dropped_finding(
                            package,
                            target,
                            CapabilityReasonCode::UnsupportedSemantic,
                            Some(resource.path.clone()),
                            "resource belongs to an unsupported native semantic package",
                        ));
                    }
                }
            }
        }
    }

    // 3. MCP Servers
    let mut mcp_servers = BTreeMap::new();
    let mut same_domain_mcp_packages = Vec::new();
    for package in graph.packages.values() {
        if !is_mcp_package(package) {
            continue;
        }
        if package.provenance.frontend != "agent-plugin" {
            findings.push(dropped_finding(
                package,
                target,
                CapabilityReasonCode::OpaqueCrossDomain,
                None,
                "foreign MCP server cannot cross harness domains into agent-plugin",
            ));
            for resource in package.resources.values() {
                findings.push(dropped_finding(
                    package,
                    target,
                    CapabilityReasonCode::OpaqueCrossDomain,
                    Some(resource.path.clone()),
                    "foreign MCP server resource cannot cross harness domains into agent-plugin",
                ));
            }
            continue;
        }

        let server_name = package
            .frontend_payload
            .as_ref()
            .and_then(|p| p.fields.get("name"))
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| {
                package
                    .semantic_identity
                    .as_str()
                    .strip_prefix("unsupported:agent-plugin-mcp-server/")
                    .map(String::from)
            })
            .unwrap_or_else(|| logical_name(package).to_string());

        let server_wire: McpServerWireV1 = if let Some(payload) = &package.frontend_payload {
            if let Some(val) = payload
                .fields
                .get("server")
                .or_else(|| payload.fields.get("config"))
            {
                serde_json::from_value(val.clone())?
            } else {
                read_mcp_server_from_resources(package)?
            }
        } else {
            read_mcp_server_from_resources(package)?
        };

        if mcp_servers.contains_key(&server_name) {
            bail!("duplicate MCP server `{server_name}`");
        }
        mcp_servers.insert(server_name, server_wire);
        same_domain_mcp_packages.push(package);
        for resource in package.resources.values() {
            findings.push(supported_finding(
                package,
                target,
                resource.path.clone(),
                ResourcePath::parse("mcp.json")?,
            ));
        }
        if package.resources.is_empty() {
            findings.push(capability_finding(
                package,
                target,
                CapabilitySeverity::Supported,
                CapabilityReasonCode::Representable,
                None,
                Some(ResourcePath::parse("mcp.json")?),
                "MCP server is re-aggregated into mcp.json",
            ));
        }
    }

    if !mcp_servers.is_empty() {
        let config_wire = McpConfigWireV1::new(mcp_servers);
        let mut config_str = serde_json::to_string_pretty(&config_wire)?;
        config_str.push('\n');
        artifacts.push(NativeArtifact {
            target,
            class: NativeArtifactClass::McpConfig,
            path: ResourcePath::parse("mcp.json")?,
            bytes: config_str.into_bytes(),
            executable: false,
            source_package: same_domain_mcp_packages[0].id.clone(),
        });
    }

    // 4. Client Extensions
    for package in graph.packages.values() {
        if !is_extension_package(package) {
            continue;
        }
        if package.provenance.frontend != "agent-plugin" {
            findings.push(dropped_finding(
                package,
                target,
                CapabilityReasonCode::OpaqueCrossDomain,
                None,
                "foreign client extension cannot cross harness domains into agent-plugin",
            ));
            for resource in package.resources.values() {
                findings.push(dropped_finding(
                    package,
                    target,
                    CapabilityReasonCode::OpaqueCrossDomain,
                    Some(resource.path.clone()),
                    "foreign client extension resource cannot cross harness domains into agent-plugin",
                ));
            }
            continue;
        }

        let extension_dir = package
            .semantic_identity
            .as_str()
            .strip_prefix("unsupported:agent-plugin-extension/")
            .unwrap_or_else(|| package.package_root.as_str());

        for resource in package.resources.values() {
            let path = ResourcePath::parse(format!("{extension_dir}/{}", resource.path.as_str()))?;
            artifacts.push(NativeArtifact {
                target,
                class: NativeArtifactClass::ClientExtension,
                path: path.clone(),
                bytes: resource_content_bytes(&resource.content),
                executable: false,
                source_package: package.id.clone(),
            });
            findings.push(supported_finding(
                package,
                target,
                resource.path.clone(),
                path,
            ));
        }
    }

    // 5. Executable bit metadata
    for package in graph.packages.values() {
        for resource in package
            .resources
            .values()
            .filter(|resource| resource.executable)
        {
            let is_primary = matches!(
                &package.semantic_item,
                SemanticItem::Rule {
                    primary_instruction,
                    ..
                } | SemanticItem::Skill {
                    primary_instruction,
                    ..
                } if primary_instruction == &resource.path
            );
            let artifact_path = is_primary
                .then(|| {
                    artifacts
                        .iter()
                        .rev()
                        .find(|artifact| artifact.source_package == package.id)
                        .map(|artifact| artifact.path.clone())
                })
                .flatten();
            findings.push(loss_finding(
                package,
                target,
                CapabilityReasonCode::ExecutableBitUnrepresentable,
                Some(resource.path.clone()),
                artifact_path,
                "the target native artifact format does not preserve executable-bit metadata",
            ));
        }
    }

    append_package_outcomes(graph, target, &artifacts, &mut findings);
    findings.sort_by(|left, right| left.id.cmp(&right.id));

    validate_no_collisions(&artifacts)?;
    if !options.allow_lossy
        && findings
            .iter()
            .any(|finding| finding.severity != CapabilitySeverity::Supported)
    {
        bail!(
            "{} lowering has unaccepted capability loss",
            target.as_str()
        );
    }

    Ok(LoweringPlan {
        target,
        artifacts,
        findings,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        lower, CapabilityReasonCode, CapabilitySeverity, LoweringOptions, NativeArtifactClass,
        NativeTarget,
    };
    use crate::{
        ActivationMode, CompilationGraph, FrontendPayload, Package, PackageKind, PackageRoot,
        PortableActivation, Resource, ResourceContent, ResourcePath, SemanticIdentity,
        SemanticItem, SourceProvenance, TargetActivation, TargetActivationOverrides,
    };
    use std::collections::BTreeMap;

    fn rule_graph() -> CompilationGraph {
        let rule = Package::rule(
            SemanticIdentity::parse("rule:rust-style").unwrap(),
            SourceProvenance::new("cursor", ".cursor/rules/rust-style.mdc").unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("rust-style.mdc").unwrap(),
                ResourceContent::Text("Use rustfmt.".to_owned()),
                false,
            ),
        )
        .unwrap();
        CompilationGraph::new([rule]).unwrap()
    }

    #[test]
    fn lowers_a_rule_to_each_core_target_with_target_relative_paths() {
        let graph = rule_graph();
        let expected = [
            (
                NativeTarget::Codex,
                NativeArtifactClass::Instruction,
                "AGENTS.md",
            ),
            (
                NativeTarget::OpenCode,
                NativeArtifactClass::Rule,
                "rules/rust-style.md",
            ),
            (
                NativeTarget::Claude,
                NativeArtifactClass::Instruction,
                "CLAUDE.md",
            ),
            (
                NativeTarget::Cursor,
                NativeArtifactClass::Rule,
                "rules/rust-style.mdc",
            ),
            (
                NativeTarget::Antigravity,
                NativeArtifactClass::Rule,
                "rules/rust-style.md",
            ),
        ];

        for (target, class, path) in expected {
            let plan = lower(&graph, target, LoweringOptions::strict()).unwrap();
            assert_eq!(plan.artifacts.len(), 1, "{target:?}");
            assert_eq!(plan.artifacts[0].class, class, "{target:?}");
            assert_eq!(plan.artifacts[0].path.as_str(), path, "{target:?}");
            if matches!(target, NativeTarget::Cursor | NativeTarget::Antigravity) {
                let content = std::str::from_utf8(&plan.artifacts[0].bytes).unwrap();
                assert!(content.starts_with("---\n"), "{target:?}");
                assert!(content.ends_with("Use rustfmt."), "{target:?}");
            } else {
                assert_eq!(plan.artifacts[0].bytes, b"Use rustfmt.", "{target:?}");
            }
            assert!(
                plan.findings
                    .iter()
                    .all(|finding| finding.severity == CapabilitySeverity::Supported),
                "{target:?}"
            );
        }
    }

    #[test]
    fn cursor_uses_the_family_activation_override_before_the_default() {
        let primary = Resource::primary_instruction(
            ResourcePath::parse("typescript.mdc").unwrap(),
            ResourceContent::Text("Prefer explicit return types.".to_owned()),
            false,
        );
        let primary_path = primary.path.clone();
        let mut resources = BTreeMap::new();
        resources.insert(primary_path.clone(), primary);
        let activation = TargetActivation::Wrapped(TargetActivationOverrides {
            default: PortableActivation {
                mode: vec![ActivationMode::Manual],
                globs: None,
                pattern: None,
                description: None,
            },
            overrides: BTreeMap::from([(
                "cursor".to_owned(),
                PortableActivation {
                    mode: vec![ActivationMode::Always],
                    globs: Some(vec!["**/*.ts".to_owned()]),
                    pattern: None,
                    description: None,
                },
            )]),
        });
        let package = Package::new(
            PackageKind::Rule,
            SemanticIdentity::parse("rule:typescript").unwrap(),
            SourceProvenance::new("cursor", ".cursor/rules/typescript.mdc").unwrap(),
            PackageRoot::root(),
            SemanticItem::Rule {
                primary_instruction: primary_path,
                description: Some("TypeScript conventions".to_owned()),
                activation: Some(activation),
                frontend_payload: None,
            },
            resources,
            None,
        )
        .unwrap();
        let graph = CompilationGraph::new([package]).unwrap();

        let plan = lower(&graph, NativeTarget::Cursor, LoweringOptions::strict()).unwrap();
        let content = std::str::from_utf8(&plan.artifacts[0].bytes).unwrap();
        assert!(content.starts_with("---\n"));
        assert!(content.contains("alwaysApply: true"));
        assert!(content.contains("**/*.ts"));
        assert!(content.ends_with("Prefer explicit return types."));
    }

    #[test]
    fn skill_to_rule_lowering_requires_explicit_loss_acceptance() {
        let primary = Resource::primary_instruction(
            ResourcePath::parse("SKILL.md").unwrap(),
            ResourceContent::Text("Review the diff for correctness.".to_owned()),
            false,
        );
        let primary_path = primary.path.clone();
        let mut resources = BTreeMap::new();
        resources.insert(primary_path.clone(), primary);
        let package = Package::new(
            PackageKind::Skill,
            SemanticIdentity::parse("skill:code-review").unwrap(),
            SourceProvenance::new("opencode", ".opencode/skills/code-review/SKILL.md").unwrap(),
            PackageRoot::root(),
            SemanticItem::Skill {
                primary_instruction: primary_path,
                description: "Review a change before it ships.".to_owned(),
                frontend_payload: None,
            },
            resources,
            None,
        )
        .unwrap();
        let graph = CompilationGraph::new([package]).unwrap();

        let error = lower(&graph, NativeTarget::Cursor, LoweringOptions::strict()).unwrap_err();
        assert!(error.to_string().contains("unaccepted capability loss"));

        let plan = lower(&graph, NativeTarget::Cursor, LoweringOptions::allow_lossy()).unwrap();
        assert_eq!(plan.artifacts.len(), 1);
        assert_eq!(plan.artifacts[0].class, NativeArtifactClass::Rule);
        assert_eq!(plan.artifacts[0].path.as_str(), "rules/code-review.mdc");
        assert!(plan.findings.iter().any(|finding| {
            finding.severity == CapabilitySeverity::Lossy
                && finding.reason_code == CapabilityReasonCode::SkillLoweredAsRule
        }));
    }

    #[test]
    fn executable_primary_instruction_is_a_loss_for_every_native_target() {
        let package = Package::rule(
            SemanticIdentity::parse("rule:release-check").unwrap(),
            SourceProvenance::new("claude", "CLAUDE.md").unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("CLAUDE.md").unwrap(),
                ResourceContent::Text("Run the release check.".to_owned()),
                true,
            ),
        )
        .unwrap();
        let graph = CompilationGraph::new([package]).unwrap();

        for target in [
            NativeTarget::Codex,
            NativeTarget::OpenCode,
            NativeTarget::Claude,
            NativeTarget::Cursor,
            NativeTarget::Antigravity,
        ] {
            assert!(lower(&graph, target, LoweringOptions::strict()).is_err());
            let plan = lower(&graph, target, LoweringOptions::allow_lossy()).unwrap();
            assert_eq!(plan.artifacts.len(), 1, "{target:?}");
            assert!(!plan.artifacts[0].executable, "{target:?}");
            assert!(plan.findings.iter().any(|finding| {
                finding.severity == CapabilitySeverity::Lossy
                    && finding.reason_code == CapabilityReasonCode::ExecutableBitUnrepresentable
                    && finding
                        .resource_path
                        .as_ref()
                        .is_some_and(|path| path.as_str() == "CLAUDE.md")
            }));
        }
    }

    #[test]
    fn opaque_skill_resources_are_emitted_only_for_a_supported_same_domain_package() {
        let primary = Resource::primary_instruction(
            ResourcePath::parse("SKILL.md").unwrap(),
            ResourceContent::Text("Review the diff.".to_owned()),
            false,
        );
        let primary_path = primary.path.clone();
        let opaque = Resource::opaque(
            ResourcePath::parse("references/checklist.md").unwrap(),
            ResourceContent::Text("Check error paths.".to_owned()),
            false,
        );
        let mut resources = BTreeMap::new();
        resources.insert(primary_path.clone(), primary);
        resources.insert(opaque.path.clone(), opaque);
        let package = Package::new(
            PackageKind::Skill,
            SemanticIdentity::parse("skill:code-review").unwrap(),
            SourceProvenance::new("opencode", ".opencode/skills/code-review/SKILL.md").unwrap(),
            PackageRoot::root(),
            SemanticItem::Skill {
                primary_instruction: primary_path,
                description: "Review a change before it ships.".to_owned(),
                frontend_payload: None,
            },
            resources,
            None,
        )
        .unwrap();
        let graph = CompilationGraph::new([package]).unwrap();

        let same_domain = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();
        assert_eq!(same_domain.artifacts.len(), 2);
        assert!(same_domain.artifacts.iter().any(|artifact| {
            artifact.class == NativeArtifactClass::SkillInstruction
                && artifact.path.as_str() == "skills/code-review/SKILL.md"
        }));
        assert!(same_domain.artifacts.iter().any(|artifact| {
            artifact.class == NativeArtifactClass::SkillResource
                && artifact.path.as_str() == "skills/code-review/references/checklist.md"
                && artifact.bytes == b"Check error paths."
        }));

        assert!(lower(&graph, NativeTarget::Claude, LoweringOptions::strict()).is_err());
        let cross_domain =
            lower(&graph, NativeTarget::Claude, LoweringOptions::allow_lossy()).unwrap();
        assert_eq!(cross_domain.artifacts.len(), 1);
        assert!(cross_domain.findings.iter().any(|finding| {
            finding.severity == CapabilitySeverity::Dropped
                && finding.reason_code == CapabilityReasonCode::OpaqueCrossDomain
                && finding
                    .resource_path
                    .as_ref()
                    .is_some_and(|path| path.as_str() == "references/checklist.md")
                && finding.provenance.frontend == "opencode"
        }));
    }

    #[test]
    fn duplicate_codex_instruction_artifacts_fail_before_loss_policy_gating() {
        let first = Package::rule(
            SemanticIdentity::parse("rule:first").unwrap(),
            SourceProvenance::new("codex", "AGENTS.md").unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("AGENTS.md").unwrap(),
                ResourceContent::Text("First instruction.".to_owned()),
                false,
            ),
        )
        .unwrap();
        let second = Package::rule(
            SemanticIdentity::parse("rule:second").unwrap(),
            SourceProvenance::new("codex", "nested/AGENTS.md").unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("AGENTS.md").unwrap(),
                ResourceContent::Text("Second instruction.".to_owned()),
                true,
            ),
        )
        .unwrap();
        let graph = CompilationGraph::new([first, second]).unwrap();

        for options in [LoweringOptions::strict(), LoweringOptions::allow_lossy()] {
            let error = lower(&graph, NativeTarget::Codex, options).unwrap_err();
            assert!(error.to_string().contains("artifact-collision"));
            assert!(!error.to_string().contains("unaccepted capability loss"));
        }
    }

    #[test]
    fn capability_findings_include_stable_supported_resource_and_package_outcomes() {
        let graph = rule_graph();
        let plan = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();
        assert_eq!(plan.findings.len(), 2);
        assert!(plan.findings.windows(2).all(|pair| pair[0].id < pair[1].id));
        assert!(plan.findings.iter().any(|finding| {
            finding.resource_path.is_none()
                && finding.severity == CapabilitySeverity::Supported
                && finding.reason_code == CapabilityReasonCode::Representable
                && finding
                    .artifact_path
                    .as_ref()
                    .is_some_and(|path| path.as_str() == "rules/rust-style.md")
        }));
        assert!(plan.findings.iter().any(|finding| {
            finding
                .resource_path
                .as_ref()
                .is_some_and(|path| path.as_str() == "rust-style.mdc")
                && finding.severity == CapabilitySeverity::Supported
                && finding.reason_code == CapabilityReasonCode::Representable
        }));
    }

    #[test]
    fn antigravity_encodes_a_model_activation_in_native_rule_frontmatter() {
        let primary = Resource::primary_instruction(
            ResourcePath::parse("security.md").unwrap(),
            ResourceContent::Text("Treat untrusted input as data.".to_owned()),
            false,
        );
        let primary_path = primary.path.clone();
        let mut resources = BTreeMap::new();
        resources.insert(primary_path.clone(), primary);
        let package = Package::new(
            PackageKind::Rule,
            SemanticIdentity::parse("rule:security").unwrap(),
            SourceProvenance::new("antigravity", ".agents/rules/security.md").unwrap(),
            PackageRoot::root(),
            SemanticItem::Rule {
                primary_instruction: primary_path,
                description: Some("Security guidance".to_owned()),
                activation: Some(TargetActivation::Bare(PortableActivation {
                    mode: vec![ActivationMode::Model],
                    globs: None,
                    pattern: None,
                    description: Some("Use for security-sensitive changes".to_owned()),
                })),
                frontend_payload: None,
            },
            resources,
            None,
        )
        .unwrap();
        let graph = CompilationGraph::new([package]).unwrap();

        let plan = lower(&graph, NativeTarget::Antigravity, LoweringOptions::strict()).unwrap();
        let content = std::str::from_utf8(&plan.artifacts[0].bytes).unwrap();
        assert!(content.starts_with("---\n"));
        assert!(content.contains("trigger: model_decision"));
        assert!(content.contains("description: Security guidance"));
        assert!(content.ends_with("Treat untrusted input as data."));
    }

    #[test]
    fn agent_plugin_strict_lowering_preserves_manifest_and_skills() {
        let manifest_bytes = serde_json::to_vec_pretty(
            &crate::parsers::agent_plugin::PluginManifestWireV1::new("my-plugin"),
        )
        .unwrap();
        let manifest_path = ResourcePath::parse("plugin.json").unwrap();
        let manifest_pkg = Package::new(
            PackageKind::Unsupported,
            SemanticIdentity::parse("unsupported:agent-plugin-manifest/plugin.json").unwrap(),
            SourceProvenance::new("agent-plugin", "plugin.json").unwrap(),
            PackageRoot::root(),
            SemanticItem::Unsupported {
                native_kind: "manifest".to_owned(),
            },
            BTreeMap::from([(
                manifest_path.clone(),
                Resource::opaque(
                    manifest_path,
                    ResourceContent::Bytes(manifest_bytes.clone()),
                    false,
                ),
            )]),
            None,
        )
        .unwrap();

        let skill_path = ResourcePath::parse("SKILL.md").unwrap();
        let skill_pkg = Package::new(
            PackageKind::Skill,
            SemanticIdentity::parse("skill:helper").unwrap(),
            SourceProvenance::new("agent-plugin", "skills/helper/SKILL.md").unwrap(),
            PackageRoot::parse("skills/helper").unwrap(),
            SemanticItem::Skill {
                primary_instruction: skill_path.clone(),
                description: "A helper skill".to_owned(),
                frontend_payload: None,
            },
            BTreeMap::from([(
                skill_path,
                Resource::primary_instruction(
                    ResourcePath::parse("SKILL.md").unwrap(),
                    ResourceContent::Text("Help the user.".to_owned()),
                    false,
                ),
            )]),
            None,
        )
        .unwrap();

        let graph = CompilationGraph::new([manifest_pkg, skill_pkg]).unwrap();
        let plan = lower(&graph, NativeTarget::AgentPlugin, LoweringOptions::strict()).unwrap();
        assert_eq!(plan.artifacts.len(), 2);
        let manifest_art = plan
            .artifacts
            .iter()
            .find(|a| a.path.as_str() == "plugin.json")
            .unwrap();
        assert_eq!(manifest_art.class, NativeArtifactClass::PluginManifest);
        assert_eq!(manifest_art.bytes, manifest_bytes);

        let skill_art = plan
            .artifacts
            .iter()
            .find(|a| a.path.as_str() == "skills/helper/SKILL.md")
            .unwrap();
        assert_eq!(skill_art.class, NativeArtifactClass::SkillInstruction);
        let skill_str = std::str::from_utf8(&skill_art.bytes).unwrap();
        assert!(skill_str.contains("name: helper"));
        assert!(skill_str.contains("description: A helper skill"));
        assert!(skill_str.ends_with("Help the user."));

        assert!(plan
            .findings
            .iter()
            .all(|f| f.severity == CapabilitySeverity::Supported));
    }

    #[test]
    fn agent_plugin_multiple_manifests_fail_with_aggregation_collision() {
        let manifest_bytes = serde_json::to_vec_pretty(
            &crate::parsers::agent_plugin::PluginManifestWireV1::new("plugin-1"),
        )
        .unwrap();
        let p1 = ResourcePath::parse("plugin.json").unwrap();
        let pkg1 = Package::new(
            PackageKind::Unsupported,
            SemanticIdentity::parse("unsupported:agent-plugin-manifest/plugin-1.json").unwrap(),
            SourceProvenance::new("agent-plugin", "plugin.json").unwrap(),
            PackageRoot::root(),
            SemanticItem::Unsupported {
                native_kind: "manifest".to_owned(),
            },
            BTreeMap::from([(
                p1.clone(),
                Resource::opaque(p1, ResourceContent::Bytes(manifest_bytes.clone()), false),
            )]),
            None,
        )
        .unwrap();

        let p2 = ResourcePath::parse("plugin.json").unwrap();
        let pkg2 = Package::new(
            PackageKind::Unsupported,
            SemanticIdentity::parse("unsupported:agent-plugin-manifest/plugin-2.json").unwrap(),
            SourceProvenance::new("agent-plugin", "plugin.json").unwrap(),
            PackageRoot::root(),
            SemanticItem::Unsupported {
                native_kind: "manifest".to_owned(),
            },
            BTreeMap::from([(
                p2.clone(),
                Resource::opaque(p2, ResourceContent::Bytes(manifest_bytes), false),
            )]),
            None,
        )
        .unwrap();

        let graph = CompilationGraph::new([pkg1, pkg2]).unwrap();
        let err = lower(&graph, NativeTarget::AgentPlugin, LoweringOptions::strict()).unwrap_err();
        assert!(err.to_string().contains("aggregation collision"));
    }

    #[test]
    fn agent_plugin_lowering_rules_in_strict_mode_fails_with_capability_loss() {
        let graph = rule_graph();
        let err = lower(&graph, NativeTarget::AgentPlugin, LoweringOptions::strict()).unwrap_err();
        assert!(err.to_string().contains("capability loss"));
    }

    #[test]
    fn agent_plugin_lowering_rules_with_allow_lossy_converts_to_skills_and_synthesizes_manifest() {
        let primary = Resource::primary_instruction(
            ResourcePath::parse("rust-style.mdc").unwrap(),
            ResourceContent::Text("Use rustfmt.".to_owned()),
            false,
        );
        let primary_path = primary.path.clone();
        let mut resources = BTreeMap::new();
        resources.insert(primary_path.clone(), primary);
        let rule = Package::new(
            PackageKind::Rule,
            SemanticIdentity::parse("rule:RustStyle_v1.0").unwrap(),
            SourceProvenance::new("cursor", ".cursor/rules/rust-style.mdc").unwrap(),
            PackageRoot::root(),
            SemanticItem::Rule {
                primary_instruction: primary_path,
                description: None,
                activation: None,
                frontend_payload: None,
            },
            resources,
            None,
        )
        .unwrap();
        let graph = CompilationGraph::new([rule]).unwrap();

        let plan = lower(
            &graph,
            NativeTarget::AgentPlugin,
            LoweringOptions::allow_lossy(),
        )
        .unwrap();
        let manifest_art = plan
            .artifacts
            .iter()
            .find(|a| a.path.as_str() == "plugin.json")
            .unwrap();
        assert_eq!(manifest_art.class, NativeArtifactClass::PluginManifest);

        let skill_art = plan
            .artifacts
            .iter()
            .find(|a| a.path.as_str() == "skills/ruststyle-v1-0/SKILL.md")
            .unwrap();
        assert_eq!(skill_art.class, NativeArtifactClass::SkillInstruction);
        let content = std::str::from_utf8(&skill_art.bytes).unwrap();
        assert!(content.contains("name: ruststyle-v1-0"));
        assert!(content.contains("Rule lowered as skill: ruststyle-v1-0"));
        assert!(content.ends_with("Use rustfmt."));

        assert!(plan
            .findings
            .iter()
            .any(|f| f.reason_code == CapabilityReasonCode::RuleLoweredAsSkill));
        assert!(plan
            .findings
            .iter()
            .any(|f| f.reason_code == CapabilityReasonCode::SynthesizedManifest));
    }

    #[test]
    fn agent_plugin_lowering_multiple_same_domain_mcp_servers_reaggregates_into_single_mcp_json() {
        let manifest_bytes = serde_json::to_vec_pretty(
            &crate::parsers::agent_plugin::PluginManifestWireV1::new("my-plugin"),
        )
        .unwrap();
        let manifest_path = ResourcePath::parse("plugin.json").unwrap();
        let manifest_pkg = Package::new(
            PackageKind::Unsupported,
            SemanticIdentity::parse("unsupported:agent-plugin-manifest/plugin.json").unwrap(),
            SourceProvenance::new("agent-plugin", "plugin.json").unwrap(),
            PackageRoot::root(),
            SemanticItem::Unsupported {
                native_kind: "manifest".to_owned(),
            },
            BTreeMap::from([(
                manifest_path.clone(),
                Resource::opaque(manifest_path, ResourceContent::Bytes(manifest_bytes), false),
            )]),
            None,
        )
        .unwrap();

        let server1_wire = crate::parsers::agent_plugin::McpServerWireV1::Stdio {
            command: "./bin/fetch".to_string(),
            args: vec!["--verbose".to_string()],
            env: BTreeMap::new(),
            cwd: None,
        };
        let s1_path = ResourcePath::parse("mcp-servers/fetch.json").unwrap();
        let mut s1_payload = BTreeMap::new();
        s1_payload.insert(
            "name".to_string(),
            serde_json::Value::String("fetch".to_string()),
        );
        s1_payload.insert(
            "server".to_string(),
            serde_json::to_value(&server1_wire).unwrap(),
        );
        let mcp1_pkg = Package::new(
            PackageKind::Unsupported,
            SemanticIdentity::parse("unsupported:agent-plugin-mcp-server/fetch").unwrap(),
            SourceProvenance::new("agent-plugin", "mcp.json").unwrap(),
            PackageRoot::root(),
            SemanticItem::Unsupported {
                native_kind: "mcp-server".to_owned(),
            },
            BTreeMap::from([(
                s1_path.clone(),
                Resource::opaque(
                    s1_path,
                    ResourceContent::Bytes(serde_json::to_vec(&server1_wire).unwrap()),
                    false,
                ),
            )]),
            Some(FrontendPayload {
                namespace: "agent-plugin.mcp-server".to_owned(),
                fields: s1_payload,
            }),
        )
        .unwrap();

        let server2_wire = crate::parsers::agent_plugin::McpServerWireV1::Sse {
            url: "https://example.com/sse".to_string(),
            headers: BTreeMap::new(),
        };
        let s2_path = ResourcePath::parse("mcp-servers/db.json").unwrap();
        let mut s2_payload = BTreeMap::new();
        s2_payload.insert(
            "name".to_string(),
            serde_json::Value::String("db".to_string()),
        );
        s2_payload.insert(
            "server".to_string(),
            serde_json::to_value(&server2_wire).unwrap(),
        );
        let mcp2_pkg = Package::new(
            PackageKind::Unsupported,
            SemanticIdentity::parse("unsupported:agent-plugin-mcp-server/db").unwrap(),
            SourceProvenance::new("agent-plugin", "mcp.json").unwrap(),
            PackageRoot::root(),
            SemanticItem::Unsupported {
                native_kind: "mcp-server".to_owned(),
            },
            BTreeMap::from([(
                s2_path.clone(),
                Resource::opaque(
                    s2_path,
                    ResourceContent::Bytes(serde_json::to_vec(&server2_wire).unwrap()),
                    false,
                ),
            )]),
            Some(FrontendPayload {
                namespace: "agent-plugin.mcp-server".to_owned(),
                fields: s2_payload,
            }),
        )
        .unwrap();

        let graph = CompilationGraph::new([manifest_pkg, mcp1_pkg, mcp2_pkg]).unwrap();
        let plan = lower(&graph, NativeTarget::AgentPlugin, LoweringOptions::strict()).unwrap();

        let mcp_artifacts: Vec<_> = plan
            .artifacts
            .iter()
            .filter(|a| a.path.as_str() == "mcp.json")
            .collect();
        assert_eq!(mcp_artifacts.len(), 1);
        assert_eq!(mcp_artifacts[0].class, NativeArtifactClass::McpConfig);

        let mcp_config: crate::parsers::agent_plugin::McpConfigWireV1 =
            serde_json::from_slice(&mcp_artifacts[0].bytes).unwrap();
        assert_eq!(mcp_config.mcp_servers.len(), 2);
        assert!(mcp_config.mcp_servers.contains_key("fetch"));
        assert!(mcp_config.mcp_servers.contains_key("db"));
    }

    #[test]
    fn agent_plugin_foreign_mcp_servers_dropped_with_opaque_cross_domain() {
        let manifest_bytes = serde_json::to_vec_pretty(
            &crate::parsers::agent_plugin::PluginManifestWireV1::new("my-plugin"),
        )
        .unwrap();
        let manifest_path = ResourcePath::parse("plugin.json").unwrap();
        let manifest_pkg = Package::new(
            PackageKind::Unsupported,
            SemanticIdentity::parse("unsupported:agent-plugin-manifest/plugin.json").unwrap(),
            SourceProvenance::new("agent-plugin", "plugin.json").unwrap(),
            PackageRoot::root(),
            SemanticItem::Unsupported {
                native_kind: "manifest".to_owned(),
            },
            BTreeMap::from([(
                manifest_path.clone(),
                Resource::opaque(manifest_path, ResourceContent::Bytes(manifest_bytes), false),
            )]),
            None,
        )
        .unwrap();

        let claude_mcp_path = ResourcePath::parse("settings.json").unwrap();
        let claude_mcp = Package::new(
            PackageKind::Unsupported,
            SemanticIdentity::parse("unsupported:claude-mcp-server/fetch").unwrap(),
            SourceProvenance::new("claude", ".claude/settings.json").unwrap(),
            PackageRoot::root(),
            SemanticItem::Unsupported {
                native_kind: "mcp-server".to_owned(),
            },
            BTreeMap::from([(
                claude_mcp_path.clone(),
                Resource::opaque(
                    claude_mcp_path,
                    ResourceContent::Text("{}".to_owned()),
                    false,
                ),
            )]),
            None,
        )
        .unwrap();

        let graph = CompilationGraph::new([manifest_pkg, claude_mcp]).unwrap();
        assert!(lower(&graph, NativeTarget::AgentPlugin, LoweringOptions::strict()).is_err());

        let plan = lower(
            &graph,
            NativeTarget::AgentPlugin,
            LoweringOptions::allow_lossy(),
        )
        .unwrap();
        assert!(plan.artifacts.iter().all(|a| a.path.as_str() != "mcp.json"));
        assert!(plan.findings.iter().any(|f| {
            f.reason_code == CapabilityReasonCode::OpaqueCrossDomain
                && f.severity == CapabilitySeverity::Dropped
        }));
    }

    #[test]
    fn agent_plugin_client_extensions_lowering() {
        let manifest_bytes = serde_json::to_vec_pretty(
            &crate::parsers::agent_plugin::PluginManifestWireV1::new("my-plugin"),
        )
        .unwrap();
        let manifest_path = ResourcePath::parse("plugin.json").unwrap();
        let manifest_pkg = Package::new(
            PackageKind::Unsupported,
            SemanticIdentity::parse("unsupported:agent-plugin-manifest/plugin.json").unwrap(),
            SourceProvenance::new("agent-plugin", "plugin.json").unwrap(),
            PackageRoot::root(),
            SemanticItem::Unsupported {
                native_kind: "manifest".to_owned(),
            },
            BTreeMap::from([(
                manifest_path.clone(),
                Resource::opaque(manifest_path, ResourceContent::Bytes(manifest_bytes), false),
            )]),
            None,
        )
        .unwrap();

        let ext_res_path = ResourcePath::parse("settings.json").unwrap();
        let ext_pkg = Package::new(
            PackageKind::Unsupported,
            SemanticIdentity::parse("unsupported:agent-plugin-extension/com.example.client")
                .unwrap(),
            SourceProvenance::new("agent-plugin", "com.example.client/settings.json").unwrap(),
            PackageRoot::parse("com.example.client").unwrap(),
            SemanticItem::Unsupported {
                native_kind: "extension".to_owned(),
            },
            BTreeMap::from([(
                ext_res_path.clone(),
                Resource::opaque(
                    ext_res_path,
                    ResourceContent::Text("{\"key\":\"val\"}".to_owned()),
                    false,
                ),
            )]),
            None,
        )
        .unwrap();

        let graph = CompilationGraph::new([manifest_pkg, ext_pkg]).unwrap();
        let plan = lower(&graph, NativeTarget::AgentPlugin, LoweringOptions::strict()).unwrap();
        let ext_art = plan
            .artifacts
            .iter()
            .find(|a| a.path.as_str() == "com.example.client/settings.json")
            .unwrap();
        assert_eq!(ext_art.class, NativeArtifactClass::ClientExtension);
        assert_eq!(ext_art.bytes, b"{\"key\":\"val\"}");
    }
}
