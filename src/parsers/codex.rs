use crate::inputs::ArtifactObservation;
use crate::ir::SkillMetadata;
use crate::parsers::frontend::{NativeCompilation, NativeFrontend, NativeObservationDisposition};
use crate::parsers::skills::DirectSkillRoots;
use crate::{
    FrontendPayload, Package, PackageKind, PackageRoot, Resource, ResourceContent, ResourcePath,
    SemanticIdentity, SemanticItem, SourceProvenance,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Compiles documented Codex project files into the package-aware graph.
///
/// The caller supplies observations in their source order.
/// This function sorts them again so package identity and resources remain
/// deterministic when it is used independently from the shared coordinator.
#[cfg(test)]
fn parse_graph(observations: &[ArtifactObservation]) -> Result<crate::CompilationGraph> {
    compile_native(observations)?.into_graph()
}

/// Compiles Codex observations while recording the disposition of each input.
pub(crate) fn compile_native(observations: &[ArtifactObservation]) -> Result<NativeCompilation> {
    let direct_roots = DirectSkillRoots::from_observations(observations);
    let mut ordered_observations: Vec<_> = observations.iter().enumerate().collect();
    ordered_observations
        .sort_by(|left, right| observation_key(left.1).cmp(&observation_key(right.1)));

    let mut packages = Vec::new();
    let mut dispositions = vec![None; observations.len()];
    let mut skill_members: BTreeMap<SkillGroupKey, Vec<(usize, &ArtifactObservation, &str)>> =
        BTreeMap::new();

    for (index, observation) in ordered_observations {
        let path = observation.source_path.as_str();
        if let Some((root, member)) = direct_roots.match_skill(Some(".codex/skills/"), observation)
        {
            skill_members
                .entry(SkillGroupKey::from_observation(root, observation))
                .or_default()
                .push((index, observation, member));
        } else if path == "AGENTS.md" || path.ends_with("/AGENTS.md") {
            packages.push(rule_package(observation)?);
            dispositions[index] = Some(NativeObservationDisposition::PackageContent);
        } else if path == ".codex/config.toml" {
            packages.push(unsupported_package(
                observation,
                "codex-config",
                None,
                Some(config_payload(observation)?),
            )?);
            dispositions[index] = Some(NativeObservationDisposition::RetainedUnsupportedContent);
        } else {
            dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
        }
    }

    for (key, members) in skill_members {
        if let Some((_, primary, _)) = members
            .iter()
            .find(|(_, _, member)| *member == "SKILL.md")
            .copied()
        {
            let member_pairs = members
                .iter()
                .map(|(_, observation, member)| (*observation, *member))
                .collect::<Vec<_>>();
            packages.push(skill_package(&key.root, primary, &member_pairs)?);
            for (index, _, _) in members {
                dispositions[index] = Some(NativeObservationDisposition::PackageContent);
            }
        } else {
            for (index, _, _) in members {
                dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
            }
        }
    }

    NativeCompilation::new(
        NativeFrontend::Codex,
        observations,
        packages,
        dispositions
            .into_iter()
            .collect::<Option<Vec<NativeObservationDisposition>>>()
            .expect("Codex parser classifies every observation"),
    )
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SkillGroupKey {
    root: String,
    input_label: String,
}

impl SkillGroupKey {
    fn from_observation(root: &str, observation: &ArtifactObservation) -> Self {
        Self {
            root: root.to_owned(),
            input_label: observation.provenance.input_label.clone(),
        }
    }
}

fn observation_key(
    observation: &ArtifactObservation,
) -> (&str, Option<&ResourcePath>, &ResourcePath) {
    (
        &observation.provenance.input_label,
        observation.provenance.archive_member.as_ref(),
        &observation.source_path,
    )
}

fn rule_package(observation: &ArtifactObservation) -> Result<Package> {
    let path = observation.source_path.as_str();
    let (root, name) = split_parent(path)?;
    let content = std::str::from_utf8(&observation.bytes)
        .context("Codex AGENTS.md must be valid UTF-8")?
        .to_owned();
    let primary_path = ResourcePath::parse(name)?;
    let resource = Resource::primary_instruction(
        primary_path.clone(),
        ResourceContent::Text(content),
        observation.executable,
    );
    let mut resources = BTreeMap::new();
    resources.insert(primary_path.clone(), resource);
    Package::new(
        PackageKind::Rule,
        SemanticIdentity::parse(format!("rule:{path}"))?,
        provenance(observation)?,
        PackageRoot::parse(root)?,
        SemanticItem::Rule {
            primary_instruction: primary_path,
            description: None,
            activation: None,
            frontend_payload: None,
        },
        resources,
        None,
    )
}

fn skill_package(
    root: &str,
    primary: &ArtifactObservation,
    members: &[(&ArtifactObservation, &str)],
) -> Result<Package> {
    let primary_text =
        std::str::from_utf8(&primary.bytes).context("Codex SKILL.md must be valid UTF-8")?;
    let document = parse_skill_document(primary_text)?;
    let primary_path = ResourcePath::parse("SKILL.md")?;
    let mut resources = BTreeMap::new();
    let mut sidecar_metadata: Option<SkillMetadata> = None;

    for &(member, member_path) in members {
        if member_path == "agents/openai.yaml" {
            if sidecar_metadata.is_some() {
                bail!("duplicate Codex skill sidecar `agents/openai.yaml`");
            }
            let sidecar_text = std::str::from_utf8(&member.bytes)
                .context("Codex agents/openai.yaml must be valid UTF-8")?;
            sidecar_metadata = Some(parse_openai_sidecar(sidecar_text)?);
            continue;
        }
        let path = ResourcePath::parse(member_path)?;
        let resource = if member.source_path == primary.source_path {
            Resource::primary_instruction(
                path.clone(),
                ResourceContent::Text(document.body.clone()),
                member.executable,
            )
        } else {
            Resource::opaque(
                path.clone(),
                ResourceContent::Bytes(member.bytes.clone()),
                member.executable,
            )
        };
        if resources.insert(path.clone(), resource).is_some() {
            bail!("duplicate Codex skill resource `{}`", path.as_str());
        }
    }

    let payload = (!document.frontmatter.is_empty()).then(|| FrontendPayload {
        namespace: "codex.skill-frontmatter".to_owned(),
        fields: document.frontmatter,
    });
    Package::new(
        PackageKind::Skill,
        SemanticIdentity::parse(format!("skill:{}", document.name))?,
        provenance(primary)?,
        PackageRoot::parse(root)?,
        SemanticItem::Skill {
            primary_instruction: primary_path,
            description: document.description,
            metadata: sidecar_metadata.unwrap_or_default(),
            frontend_payload: payload,
        },
        resources,
        None,
    )
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CodexOpenAiSidecar {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface: Option<CodexSidecarInterface>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<CodexSidecarDependencies>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<CodexSidecarPolicy>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CodexSidecarInterface {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_small: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_large: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub brand_color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_prompt: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CodexSidecarDependencies {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<CodexSidecarToolEntry>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CodexSidecarToolDependency {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub(crate) enum CodexSidecarToolEntry {
    Name(String),
    Structured(CodexSidecarToolDependency),
}

impl<'de> serde::Deserialize<'de> for CodexSidecarToolEntry {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ToolEntryVisitor;

        impl<'de> serde::de::Visitor<'de> for ToolEntryVisitor {
            type Value = CodexSidecarToolEntry;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a tool identifier string or tool dependency mapping")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(CodexSidecarToolEntry::Name(value.to_owned()))
            }

            fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(CodexSidecarToolEntry::Name(value))
            }

            fn visit_map<A>(self, map: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let dep = CodexSidecarToolDependency::deserialize(
                    serde::de::value::MapAccessDeserializer::new(map),
                )?;
                Ok(CodexSidecarToolEntry::Structured(dep))
            }
        }

        deserializer.deserialize_any(ToolEntryVisitor)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CodexSidecarPolicy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_implicit_invocation: Option<bool>,
}

fn parse_openai_sidecar(input: &str) -> Result<SkillMetadata> {
    let sidecar: CodexOpenAiSidecar = if input.trim().is_empty() {
        CodexOpenAiSidecar::default()
    } else {
        serde_yaml::from_str::<Option<CodexOpenAiSidecar>>(input)
            .context("Codex agents/openai.yaml must match the sidecar schema")?
            .unwrap_or_default()
    };

    let (display_name, default_prompt, color, icon) = match sidecar.interface {
        Some(interface) => {
            validate_optional_sidecar_line(&interface.display_name, "interface.display_name")?;
            validate_optional_sidecar_text(
                &interface.short_description,
                "interface.short_description",
            )?;
            validate_optional_sidecar_line(&interface.icon_small, "interface.icon_small")?;
            validate_optional_sidecar_line(&interface.icon_large, "interface.icon_large")?;
            validate_optional_sidecar_line(&interface.brand_color, "interface.brand_color")?;
            validate_optional_sidecar_text(&interface.default_prompt, "interface.default_prompt")?;
            (
                interface.display_name,
                interface.default_prompt,
                interface.brand_color,
                interface.icon_small,
            )
        }
        None => (None, None, None, None),
    };

    let disable_model_invocation = sidecar
        .policy
        .and_then(|policy| policy.allow_implicit_invocation)
        .map(|allow_implicit| !allow_implicit);

    let allowed_tools = match sidecar.dependencies.and_then(|deps| deps.tools) {
        Some(entries) if !entries.is_empty() => {
            let mut extracted = Vec::with_capacity(entries.len());
            for entry in entries {
                extracted.push(extract_sidecar_tool_string(entry)?);
            }
            Some(extracted)
        }
        _ => None,
    };

    let metadata = SkillMetadata {
        display_name,
        allowed_tools,
        paths: None,
        disable_model_invocation,
        user_invocable: None,
        context: None,
        agent: None,
        icon,
        color,
        default_prompt,
    };
    metadata.validate()?;
    Ok(metadata)
}

fn validate_optional_sidecar_line(value: &Option<String>, field: &str) -> Result<()> {
    if let Some(raw) = value {
        if raw.trim().is_empty() || raw.chars().any(char::is_control) {
            bail!("Codex sidecar `{field}` must be non-empty and control-character free");
        }
    }
    Ok(())
}

fn validate_optional_sidecar_text(value: &Option<String>, field: &str) -> Result<()> {
    if let Some(raw) = value {
        if raw.trim().is_empty()
            || raw
                .chars()
                .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            bail!(
                "Codex sidecar `{field}` must be non-empty and free of unsafe control characters"
            );
        }
    }
    Ok(())
}

fn extract_sidecar_tool_string(entry: CodexSidecarToolEntry) -> Result<String> {
    match entry {
        CodexSidecarToolEntry::Name(raw) => {
            let trimmed = raw.trim();
            if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
                bail!("Codex skill tool dependency must be non-empty and control-character free");
            }
            Ok(trimmed.to_owned())
        }
        CodexSidecarToolEntry::Structured(dep) => {
            validate_optional_sidecar_text(&dep.description, "dependencies.tools[].description")?;
            validate_optional_sidecar_line(&dep.transport, "dependencies.tools[].transport")?;
            validate_optional_sidecar_line(&dep.url, "dependencies.tools[].url")?;
            let value = dep.value.trim();
            if value.is_empty() || value.chars().any(char::is_control) {
                bail!(
                    "Codex skill tool dependency value must be non-empty and control-character free"
                );
            }
            match dep.tool_type {
                None => Ok(value.to_owned()),
                Some(raw_type) => {
                    let kind = raw_type.trim();
                    if kind.is_empty() || kind.chars().any(char::is_control) {
                        bail!(
                            "Codex skill tool dependency type must be non-empty and control-character free"
                        );
                    }
                    if kind == "tool" || value.starts_with(&format!("{kind}:")) {
                        Ok(value.to_owned())
                    } else {
                        Ok(format!("{kind}:{value}"))
                    }
                }
            }
        }
    }
}

fn unsupported_package(
    observation: &ArtifactObservation,
    native_kind: &str,
    identity_suffix: Option<&str>,
    frontend_payload: Option<FrontendPayload>,
) -> Result<Package> {
    let source_path = observation.source_path.as_str();
    let (root, name) = split_parent(source_path)?;
    let resource_path = ResourcePath::parse(name)?;
    let mut resources = BTreeMap::new();
    resources.insert(
        resource_path.clone(),
        Resource::opaque(
            resource_path,
            ResourceContent::Bytes(observation.bytes.clone()),
            observation.executable,
        ),
    );
    let identity = identity_suffix.map_or_else(
        || format!("unsupported:{native_kind}/{source_path}"),
        |suffix| format!("unsupported:{native_kind}/{source_path}/{suffix}"),
    );
    Package::new(
        PackageKind::Unsupported,
        SemanticIdentity::parse(identity)?,
        provenance(observation)?,
        PackageRoot::parse(root)?,
        SemanticItem::Unsupported {
            native_kind: native_kind.to_owned(),
        },
        resources,
        frontend_payload,
    )
}

fn config_payload(observation: &ArtifactObservation) -> Result<FrontendPayload> {
    let input =
        std::str::from_utf8(&observation.bytes).context("Codex config.toml must be valid UTF-8")?;
    let config: toml::Value =
        toml::from_str(input).context("Codex config.toml must be valid TOML")?;
    let fields = serde_json::to_value(config)
        .context("Codex config.toml cannot be represented as graph payload JSON")?
        .as_object()
        .cloned()
        .context("Codex config.toml must contain a TOML table")?
        .into_iter()
        .collect();
    Ok(FrontendPayload {
        namespace: "codex.config".to_owned(),
        fields,
    })
}

fn provenance(observation: &ArtifactObservation) -> Result<SourceProvenance> {
    let mut provenance = SourceProvenance::new("codex", &observation.provenance.input_label)?;
    provenance.archive_member = observation.provenance.archive_member.clone();
    Ok(provenance)
}

fn split_parent(path: &str) -> Result<(&str, &str)> {
    path.rsplit_once('/').map_or_else(
        || Ok((".", path)),
        |(parent, name)| {
            if parent.is_empty() || name.is_empty() {
                bail!("native source path must have a non-empty file name")
            }
            Ok((parent, name))
        },
    )
}

struct SkillDocument {
    name: String,
    description: String,
    frontmatter: BTreeMap<String, serde_json::Value>,
    body: String,
}

fn parse_skill_document(input: &str) -> Result<SkillDocument> {
    let (frontmatter, body) = split_frontmatter(input)?;
    let mut fields: BTreeMap<String, serde_json::Value> = serde_yaml::from_str(frontmatter)
        .context("Codex SKILL.md frontmatter must be valid YAML")?;
    let name = required_string(&mut fields, "name")?;
    let description = required_string(&mut fields, "description")?;
    Ok(SkillDocument {
        name,
        description,
        frontmatter: fields,
        body: body.to_owned(),
    })
}

fn split_frontmatter(input: &str) -> Result<(&str, &str)> {
    let body_start = if input.starts_with("---\r\n") {
        5
    } else if input.starts_with("---\n") {
        4
    } else {
        bail!("SKILL.md must start with YAML frontmatter")
    };
    let rest = &input[body_start..];
    let mut frontmatter_len = 0;
    for line in rest.split_inclusive('\n') {
        let line_without_ending = line.trim_end_matches(['\r', '\n']);
        if line_without_ending == "---" {
            return Ok((
                &input[body_start..body_start + frontmatter_len],
                &rest[frontmatter_len + line.len()..],
            ));
        }
        frontmatter_len += line.len();
    }
    bail!("SKILL.md frontmatter must end with a YAML delimiter")
}

fn required_string(fields: &mut BTreeMap<String, serde_json::Value>, name: &str) -> Result<String> {
    fields
        .remove(name)
        .and_then(|value| value.as_str().map(ToOwned::to_owned))
        .filter(|value| !value.is_empty())
        .with_context(|| format!("SKILL.md frontmatter requires a non-empty `{name}` string"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::{ArtifactObservation, InputOrigin};
    use crate::{PackageKind, ResourceContent, ResourceRole};

    fn observation(path: &str, bytes: impl Into<Vec<u8>>, executable: bool) -> ArtifactObservation {
        observation_with_label(path, bytes, executable, "fixtures/codex-project")
    }

    fn observation_with_label(
        path: &str,
        bytes: impl Into<Vec<u8>>,
        executable: bool,
        input_label: &str,
    ) -> ArtifactObservation {
        ArtifactObservation::new(
            bytes.into(),
            path,
            executable,
            InputOrigin::Filesystem,
            input_label,
            None,
        )
        .expect("test observation is valid")
    }

    fn archive_observation(
        path: &str,
        bytes: impl Into<Vec<u8>>,
        executable: bool,
    ) -> ArtifactObservation {
        ArtifactObservation::new(
            bytes.into(),
            path,
            executable,
            InputOrigin::Tar,
            "fixtures/codex-project.tar",
            Some(crate::ResourcePath::parse(path).unwrap()),
        )
        .expect("test archive observation is valid")
    }

    #[test]
    fn parses_nested_agents_file_as_a_scoped_rule_package() {
        let graph = parse_graph(&[observation(
            "services/api/AGENTS.md",
            "Keep API changes backwards compatible.\n",
            false,
        )])
        .expect("Codex AGENTS.md parses into a graph");

        let package = graph
            .packages
            .values()
            .next()
            .expect("one rule package is present");
        assert_eq!(package.kind, PackageKind::Rule);
        assert_eq!(
            package.semantic_identity.as_str(),
            "rule:services/api/AGENTS.md"
        );
        assert_eq!(package.package_root().as_str(), "services/api");
        assert_eq!(package.provenance.frontend, "codex");
        assert_eq!(package.provenance.input_label, "fixtures/codex-project");
        let resource = package
            .resources
            .values()
            .next()
            .expect("rule has one primary instruction");
        assert_eq!(resource.path.as_str(), "AGENTS.md");
        assert_eq!(resource.role, ResourceRole::PrimaryInstruction);
        assert_eq!(
            resource.content,
            ResourceContent::Text("Keep API changes backwards compatible.\n".to_owned())
        );
    }

    #[test]
    fn parses_codex_skill_with_opaque_resources_and_frontmatter_payload() {
        let graph = parse_graph(&[
            observation(
                ".codex/skills/release-workflow/SKILL.md",
                "---\nname: release-workflow\ndescription: Prepare a release.\ncustom-field: preserve-me\n---\n# Release\n\nShip it.\n",
                false,
            ),
            observation(
                ".codex/skills/release-workflow/scripts/check.sh",
                vec![0, 1, 2, 3],
                true,
            ),
        ])
        .expect("Codex skill parses into a graph");

        let package = graph
            .packages
            .values()
            .next()
            .expect("one skill package is present");
        assert_eq!(package.kind, PackageKind::Skill);
        assert_eq!(package.semantic_identity.as_str(), "skill:release-workflow");
        assert_eq!(
            package.package_root().as_str(),
            ".codex/skills/release-workflow"
        );

        let primary = package
            .resources
            .get(&crate::ResourcePath::parse("SKILL.md").unwrap())
            .expect("skill retains its entrypoint");
        assert_eq!(primary.role, ResourceRole::PrimaryInstruction);
        assert_eq!(
            primary.content,
            ResourceContent::Text("# Release\n\nShip it.\n".to_owned())
        );

        let companion = package
            .resources
            .get(&crate::ResourcePath::parse("scripts/check.sh").unwrap())
            .expect("skill retains opaque companion bytes");
        assert_eq!(companion.role, ResourceRole::Opaque);
        assert_eq!(companion.content, ResourceContent::Bytes(vec![0, 1, 2, 3]));
        assert!(companion.executable);

        let crate::SemanticItem::Skill {
            frontend_payload: Some(payload),
            ..
        } = &package.semantic_item
        else {
            panic!("skill retains its native frontmatter payload");
        };
        assert_eq!(payload.namespace, "codex.skill-frontmatter");
        assert_eq!(
            payload.fields.get("custom-field"),
            Some(&serde_json::Value::String("preserve-me".to_owned()))
        );
    }

    #[test]
    fn retains_every_nonprimary_file_below_a_codex_skill_root() {
        let graph = parse_graph(&[
            observation(
                ".codex/skills/release-workflow/SKILL.md",
                "---\nname: release-workflow\ndescription: Prepare a release.\n---\n# Release\n",
                false,
            ),
            observation(
                ".codex/skills/release-workflow/AGENTS.md",
                "Do not publish from a dirty checkout.\n",
                false,
            ),
        ])
        .expect("all skill members stay in one package");

        assert_eq!(graph.packages.len(), 1);
        let package = graph.packages.values().next().unwrap();
        assert_eq!(package.kind, PackageKind::Skill);
        let agents = package
            .resources
            .get(&crate::ResourcePath::parse("AGENTS.md").unwrap())
            .expect("AGENTS.md is retained as a skill resource");
        assert_eq!(agents.role, ResourceRole::Opaque);
        assert_eq!(
            agents.content,
            ResourceContent::Bytes(b"Do not publish from a dirty checkout.\n".to_vec())
        );
    }

    #[test]
    fn rejects_same_codex_skill_identity_from_distinct_inputs() {
        let error = parse_graph(&[
            observation_with_label(
                ".codex/skills/release-workflow/SKILL.md",
                "---\nname: release-workflow\ndescription: Prepare a release.\n---\n# First\n",
                false,
                "fixtures/first",
            ),
            observation_with_label(
                ".codex/skills/release-workflow/SKILL.md",
                "---\nname: release-workflow\ndescription: Prepare a release.\n---\n# Second\n",
                false,
                "fixtures/second",
            ),
        ])
        .expect_err("separate source packages with the same skill identity must collide");

        assert!(error
            .to_string()
            .contains("semantic identity `skill:release-workflow`"));
    }

    #[test]
    fn groups_codex_skill_members_from_one_tar_input() {
        let graph = parse_graph(&[
            archive_observation(
                ".codex/skills/release-workflow/SKILL.md",
                "---\nname: release-workflow\ndescription: Prepare a release.\n---\n# Release\n",
                false,
            ),
            archive_observation(
                ".codex/skills/release-workflow/scripts/check.sh",
                vec![0, 1, 2, 3],
                true,
            ),
        ])
        .expect("one archive skill stays one package");

        let package = graph.packages.values().next().unwrap();
        assert_eq!(graph.packages.len(), 1);
        assert!(package
            .resources
            .contains_key(&crate::ResourcePath::parse("scripts/check.sh").unwrap()));
        assert_eq!(
            package
                .provenance
                .archive_member
                .as_ref()
                .map(crate::ResourcePath::as_str),
            Some(".codex/skills/release-workflow/SKILL.md")
        );
    }

    #[test]
    fn retains_codex_config_as_an_unsupported_opaque_package() {
        let graph = parse_graph(&[observation(
            ".codex/config.toml",
            "model = \"gpt-5\"\ncustom-field = \"preserve-me\"\n",
            false,
        )])
        .expect("Codex config remains inspectable as unsupported content");

        let package = graph
            .packages
            .values()
            .next()
            .expect("one unsupported package is present");
        assert_eq!(package.kind, PackageKind::Unsupported);
        assert_eq!(
            package.semantic_identity.as_str(),
            "unsupported:codex-config/.codex/config.toml"
        );
        assert_eq!(package.package_root().as_str(), ".codex");
        let payload = package
            .frontend_payload
            .as_ref()
            .expect("Codex config retains its structured native payload");
        assert_eq!(payload.namespace, "codex.config");
        assert_eq!(
            payload.fields.get("custom-field"),
            Some(&serde_json::Value::String("preserve-me".to_owned()))
        );
        assert!(matches!(
            &package.semantic_item,
            crate::SemanticItem::Unsupported { native_kind } if native_kind == "codex-config"
        ));
        assert_eq!(graph.diagnostics.len(), 1);
        assert_eq!(graph.diagnostics[0].code, "unsupported-semantic");
    }

    #[test]
    fn ingests_codex_openai_yaml_sidecar_under_codex_and_agents_skills_roots() {
        let sidecar_yaml = r##"
interface:
  display_name: "Code Reviewer"
  short_description: "Reviews pull requests and diffs."
  icon_small: "./assets/icon.svg"
  brand_color: "#10A37F"
  default_prompt: "Review the current git staged changes."

dependencies:
  tools:
    - type: "mcp"
      value: "github"
      description: "GitHub MCP server"
    - "Bash(git *)"

policy:
  allow_implicit_invocation: false
"##;

        for skill_root in [
            ".codex/skills/code-reviewer",
            ".agents/skills/code-reviewer",
        ] {
            let observations = [
                observation(
                    &format!("{skill_root}/SKILL.md"),
                    "---\nname: code-reviewer\ndescription: Reviews pull requests.\n---\n# Code Reviewer\n\nCheck diffs.\n",
                    false,
                ),
                observation(
                    &format!("{skill_root}/agents/openai.yaml"),
                    sidecar_yaml,
                    false,
                ),
                observation(
                    &format!("{skill_root}/assets/icon.svg"),
                    "<svg/>",
                    false,
                ),
            ];

            let compilation =
                compile_native(&observations).expect("Codex skill with sidecar compiles");
            assert_eq!(
                compilation.dispositions,
                vec![
                    NativeObservationDisposition::PackageContent,
                    NativeObservationDisposition::PackageContent,
                    NativeObservationDisposition::PackageContent,
                ],
                "agents/openai.yaml must be classified as PackageContent in {skill_root}"
            );
            assert!(
                compilation.diagnostics.is_empty(),
                "no unrecognized-native-file warning for agents/openai.yaml"
            );

            let graph = compilation.into_graph().unwrap();
            assert_eq!(graph.packages.len(), 1);
            let package = graph.packages.values().next().unwrap();
            assert_eq!(package.kind, PackageKind::Skill);
            assert_eq!(package.semantic_identity.as_str(), "skill:code-reviewer");
            assert_eq!(package.package_root().as_str(), skill_root);
            assert!(
                !package
                    .resources
                    .contains_key(&crate::ResourcePath::parse("agents/openai.yaml").unwrap()),
                "agents/openai.yaml must be consumed into SkillMetadata rather than left as an opaque resource"
            );
            assert!(package
                .resources
                .contains_key(&crate::ResourcePath::parse("assets/icon.svg").unwrap()));

            let SemanticItem::Skill { metadata, .. } = &package.semantic_item else {
                panic!("expected skill semantic item");
            };
            assert_eq!(
                metadata,
                &SkillMetadata {
                    display_name: Some("Code Reviewer".to_owned()),
                    allowed_tools: Some(vec!["mcp:github".to_owned(), "Bash(git *)".to_owned()]),
                    paths: None,
                    disable_model_invocation: Some(true),
                    user_invocable: None,
                    context: None,
                    agent: None,
                    icon: Some("./assets/icon.svg".to_owned()),
                    color: Some("#10A37F".to_owned()),
                    default_prompt: Some("Review the current git staged changes.".to_owned()),
                }
            );
        }
    }

    #[test]
    fn round_trips_codex_skill_with_openai_yaml_sidecar_and_policy_flags() {
        let sidecar = CodexOpenAiSidecar {
            interface: Some(CodexSidecarInterface {
                display_name: Some("Release Manager".to_owned()),
                short_description: None,
                icon_small: Some("./assets/release.svg".to_owned()),
                icon_large: None,
                brand_color: Some("#2563EB".to_owned()),
                default_prompt: Some("Prepare release notes.".to_owned()),
            }),
            dependencies: Some(CodexSidecarDependencies {
                tools: Some(vec![
                    CodexSidecarToolEntry::Structured(CodexSidecarToolDependency {
                        tool_type: Some("mcp".to_owned()),
                        value: "linear".to_owned(),
                        description: Some("Linear tracker".to_owned()),
                        transport: None,
                        url: None,
                    }),
                    CodexSidecarToolEntry::Structured(CodexSidecarToolDependency {
                        tool_type: Some("tool".to_owned()),
                        value: "Bash(cargo test *)".to_owned(),
                        description: None,
                        transport: None,
                        url: None,
                    }),
                ]),
            }),
            policy: Some(CodexSidecarPolicy {
                allow_implicit_invocation: Some(true),
            }),
        };

        let sidecar_yaml = serde_yaml::to_string(&sidecar).unwrap();
        let skill_md =
            "---\nname: release-manager\ndescription: Manage releases.\n---\n# Release\n";

        let graph = parse_graph(&[
            observation(".agents/skills/release-manager/SKILL.md", skill_md, false),
            observation(
                ".agents/skills/release-manager/agents/openai.yaml",
                sidecar_yaml.as_bytes(),
                false,
            ),
        ])
        .unwrap();

        let json = graph.to_canonical_json().unwrap();
        let from_json = crate::CompilationGraph::from_json(&json).unwrap();
        assert_eq!(from_json, graph);

        let toml_text = graph.to_toml().unwrap();
        let from_toml = crate::CompilationGraph::from_toml(&toml_text).unwrap();
        assert_eq!(from_toml, graph);

        let pkg = from_json.packages.values().next().unwrap();
        let SemanticItem::Skill { metadata, .. } = &pkg.semantic_item else {
            panic!("expected skill semantic item");
        };
        assert_eq!(metadata.disable_model_invocation, Some(false));
        assert_eq!(
            metadata.allowed_tools,
            Some(vec![
                "mcp:linear".to_owned(),
                "Bash(cargo test *)".to_owned()
            ])
        );

        let re_emitted_sidecar = CodexOpenAiSidecar {
            interface: Some(CodexSidecarInterface {
                display_name: metadata.display_name.clone(),
                short_description: None,
                icon_small: metadata.icon.clone(),
                icon_large: None,
                brand_color: metadata.color.clone(),
                default_prompt: metadata.default_prompt.clone(),
            }),
            dependencies: metadata
                .allowed_tools
                .as_ref()
                .map(|tools| CodexSidecarDependencies {
                    tools: Some(
                        tools
                            .iter()
                            .map(|tool| {
                                if let Some(mcp_val) = tool.strip_prefix("mcp:") {
                                    CodexSidecarToolEntry::Structured(CodexSidecarToolDependency {
                                        tool_type: Some("mcp".to_owned()),
                                        value: mcp_val.to_owned(),
                                        description: None,
                                        transport: None,
                                        url: None,
                                    })
                                } else {
                                    CodexSidecarToolEntry::Name(tool.clone())
                                }
                            })
                            .collect(),
                    ),
                }),
            policy: metadata
                .disable_model_invocation
                .map(|disabled| CodexSidecarPolicy {
                    allow_implicit_invocation: Some(!disabled),
                }),
        };

        let re_ingested = parse_graph(&[
            observation(".agents/skills/release-manager/SKILL.md", skill_md, false),
            observation(
                ".agents/skills/release-manager/agents/openai.yaml",
                serde_yaml::to_string(&re_emitted_sidecar).unwrap(),
                false,
            ),
        ])
        .unwrap();
        assert_eq!(re_ingested, graph);
    }

    #[test]
    fn rejects_unknown_fields_and_invalid_tools_in_codex_openai_yaml() {
        let skill_md = "---\nname: strict-skill\ndescription: Strict test.\n---\n# Strict\n";

        for bad_sidecar in [
            "unknown_top_level: true\n",
            "interface:\n  unknown_ui_key: true\n",
            "policy:\n  allow_implicit_invocation: true\n  extra_policy: false\n",
            "dependencies:\n  tools:\n    - type: mcp\n      value: github\n      evil: true\n",
            "dependencies:\n  tools:\n    - type: mcp\n      value: \"   \"\n",
        ] {
            let err = parse_graph(&[
                observation(".codex/skills/strict-skill/SKILL.md", skill_md, false),
                observation(
                    ".codex/skills/strict-skill/agents/openai.yaml",
                    bad_sidecar,
                    false,
                ),
            ])
            .expect_err("malformed or unknown-field sidecar must be rejected");
            assert!(
                !err.to_string().is_empty(),
                "expected descriptive error for {bad_sidecar}"
            );
        }
    }
}
