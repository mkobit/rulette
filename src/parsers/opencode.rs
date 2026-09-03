use crate::inputs::ArtifactObservation;
use crate::ir::graph::{
    FrontendPayload, Package, PackageKind, PackageRoot, Resource, ResourceContent, ResourcePath,
    SemanticIdentity, SemanticItem, SourceProvenance,
};
use crate::parsers::frontend::{NativeCompilation, NativeFrontend, NativeObservationDisposition};
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Compiles ordered OpenCode observations into portable rule and skill
/// packages while retaining native configuration, MCP, and agent semantics as
/// opaque unsupported packages.
#[cfg(test)]
fn compile_opencode_graph(inputs: &[ArtifactObservation]) -> Result<crate::CompilationGraph> {
    compile_native(inputs)?.into_graph()
}

/// Compiles OpenCode observations while recording the disposition of each input.
pub(crate) fn compile_native(inputs: &[ArtifactObservation]) -> Result<NativeCompilation> {
    let mut packages = Vec::new();
    let mut dispositions = vec![None; inputs.len()];
    let mut skill_groups: BTreeMap<SkillGroupKey, Vec<usize>> = BTreeMap::new();
    for (index, observation) in inputs.iter().enumerate() {
        if let Some((root, _)) = skill_location(&observation.source_path) {
            skill_groups
                .entry(SkillGroupKey::from_observation(&root, observation))
                .or_default()
                .push(index);
        }
    }

    let mut consumed = vec![false; inputs.len()];
    for (key, members) in skill_groups {
        if let Some(package) = compile_skill_package(&key.root, &members, inputs)? {
            for index in members {
                consumed[index] = true;
                dispositions[index] = Some(NativeObservationDisposition::PackageContent);
            }
            packages.push(package);
        }
    }

    for (index, observation) in inputs.iter().enumerate() {
        if consumed[index] {
            continue;
        }
        let path = observation.source_path.as_str();
        if is_opencode_config(path) {
            packages.extend(compile_config(observation)?);
            dispositions[index] = Some(NativeObservationDisposition::RetainedUnsupportedContent);
        } else if is_agent_path(path) {
            packages.push(unsupported_package(
                observation,
                "opencode-agent",
                "agent-markdown",
            )?);
            dispositions[index] = Some(NativeObservationDisposition::RetainedUnsupportedContent);
        } else if is_markdown(path) && !file_name(path).eq_ignore_ascii_case("readme.md") {
            packages.push(compile_rule(observation)?);
            dispositions[index] = Some(NativeObservationDisposition::PackageContent);
        } else {
            dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
        }
    }

    NativeCompilation::new(
        NativeFrontend::Opencode,
        inputs,
        packages,
        dispositions
            .into_iter()
            .collect::<Option<Vec<NativeObservationDisposition>>>()
            .expect("OpenCode parser classifies every observation"),
    )
}

/// A native skill package can only contain members from one explicit input.
///
/// `input_label` is validated as content-safe at observation construction and
/// `root` comes from a normalized resource path.
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

fn compile_rule(observation: &ArtifactObservation) -> Result<Package> {
    let source = text(observation, "OpenCode rule")?;
    let primary_path = ResourcePath::parse(file_name(observation.source_path.as_str()))?;
    let mut resources = BTreeMap::new();
    resources.insert(
        primary_path.clone(),
        Resource::primary_instruction(
            primary_path.clone(),
            ResourceContent::Text(source.to_owned()),
            observation.executable,
        ),
    );
    Package::new(
        PackageKind::Rule,
        SemanticIdentity::parse(format!(
            "rule:{}",
            file_stem(observation.source_path.as_str())
        ))?,
        provenance(observation)?,
        parent_root(&observation.source_path)?,
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

fn compile_skill_package(
    root: &str,
    members: &[usize],
    inputs: &[ArtifactObservation],
) -> Result<Option<Package>> {
    let primary_index = members.iter().copied().find(|index| {
        relative_to_root(&inputs[*index].source_path, root).as_deref() == Some("SKILL.md")
    });
    let Some(primary_index) = primary_index else {
        return Ok(None);
    };
    let primary = &inputs[primary_index];
    let (frontmatter, body) = split_frontmatter(text(primary, "OpenCode skill")?);
    let parsed = frontmatter
        .map(serde_yaml::from_str::<SkillFrontmatter>)
        .transpose()
        .context("could not parse OpenCode skill frontmatter")?
        .unwrap_or_default();
    let name = parsed
        .name
        .unwrap_or_else(|| skill_name_from_root(root).to_owned());
    let description = parsed
        .description
        .ok_or_else(|| anyhow!("OpenCode skill `{name}` is missing a discovery description"))?;
    let mut resources = BTreeMap::new();
    for index in members {
        let observation = &inputs[*index];
        let path = ResourcePath::parse(
            relative_to_root(&observation.source_path, root)
                .expect("skill member shares its group root"),
        )?;
        let resource = if *index == primary_index {
            Resource::primary_instruction(
                path.clone(),
                ResourceContent::Text(body.to_owned()),
                observation.executable,
            )
        } else {
            Resource::opaque(
                path.clone(),
                resource_content(observation),
                observation.executable,
            )
        };
        if resources.insert(path.clone(), resource).is_some() {
            bail!("duplicate OpenCode skill resource `{}`", path.as_str());
        }
    }
    Package::new(
        PackageKind::Skill,
        SemanticIdentity::parse(format!("skill:{name}"))?,
        provenance(primary)?,
        PackageRoot::parse(root)?,
        SemanticItem::Skill {
            primary_instruction: ResourcePath::parse("SKILL.md")?,
            description,
            frontend_payload: payload("opencode.skill-frontmatter", parsed.extra),
        },
        resources,
        None,
    )
    .map(Some)
}

#[derive(Default, Deserialize)]
struct SkillFrontmatter {
    name: Option<String>,
    description: Option<String>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

fn compile_config(observation: &ArtifactObservation) -> Result<Vec<Package>> {
    let source = text(observation, "OpenCode configuration")?;
    let config: serde_json::Value =
        json5::from_str(source).context("could not parse OpenCode configuration")?;
    let object = config
        .as_object()
        .ok_or_else(|| anyhow!("OpenCode configuration must be a JSON object"))?;
    let mut packages = vec![unsupported_package(
        observation,
        "opencode-configuration",
        "configuration",
    )?];
    if let Some(mcp) = object.get("mcp").and_then(serde_json::Value::as_object) {
        for name in mcp.keys() {
            packages.push(unsupported_package(observation, "opencode-mcp", name)?);
        }
    }
    if let Some(agents) = object.get("agent").and_then(serde_json::Value::as_object) {
        for name in agents.keys() {
            packages.push(unsupported_package(
                observation,
                "opencode-agent",
                &format!("inline-{name}"),
            )?);
        }
    }
    if object.contains_key("permission") {
        packages.push(unsupported_package(
            observation,
            "opencode-permission",
            "permission",
        )?);
    }
    Ok(packages)
}

fn unsupported_package(
    observation: &ArtifactObservation,
    native_kind: &str,
    label: &str,
) -> Result<Package> {
    let path = ResourcePath::parse(file_name(observation.source_path.as_str()))?;
    let mut resources = BTreeMap::new();
    resources.insert(
        path.clone(),
        Resource::opaque(path, resource_content(observation), observation.executable),
    );
    Package::new(
        PackageKind::Unsupported,
        SemanticIdentity::parse(format!(
            "unsupported:{native_kind}-{}-{}",
            semantic_path(&observation.source_path),
            semantic_component(label)
        ))?,
        provenance(observation)?,
        parent_root(&observation.source_path)?,
        SemanticItem::Unsupported {
            native_kind: native_kind.to_owned(),
        },
        resources,
        None,
    )
}

fn provenance(observation: &ArtifactObservation) -> Result<SourceProvenance> {
    let mut provenance = SourceProvenance::new("opencode", &observation.provenance.input_label)?;
    provenance.archive_member = observation.provenance.archive_member.clone();
    Ok(provenance)
}

fn resource_content(observation: &ArtifactObservation) -> ResourceContent {
    match String::from_utf8(observation.bytes.clone()) {
        Ok(text) => ResourceContent::Text(text),
        Err(_) => ResourceContent::Bytes(observation.bytes.clone()),
    }
}

fn payload(
    namespace: &str,
    fields: BTreeMap<String, serde_json::Value>,
) -> Option<FrontendPayload> {
    (!fields.is_empty()).then_some(FrontendPayload {
        namespace: namespace.to_owned(),
        fields,
    })
}

fn text<'a>(observation: &'a ArtifactObservation, kind: &str) -> Result<&'a str> {
    std::str::from_utf8(&observation.bytes).with_context(|| {
        format!(
            "{kind} `{}` must be UTF-8",
            observation.source_path.as_str()
        )
    })
}

fn split_frontmatter(input: &str) -> (Option<&str>, &str) {
    let Some(remainder) = input
        .strip_prefix("---\n")
        .or_else(|| input.strip_prefix("---\r\n"))
    else {
        return (None, input);
    };
    let Some(end) = remainder.find("---") else {
        return (None, input);
    };
    let body = &remainder[end + 3..];
    let body = body
        .strip_prefix("\r\n")
        .or_else(|| body.strip_prefix('\n'))
        .unwrap_or(body);
    (Some(remainder[..end].trim()), body)
}

fn skill_location(path: &ResourcePath) -> Option<(String, String)> {
    let components: Vec<_> = path.as_str().split('/').collect();
    let skills = components
        .iter()
        .rposition(|component| *component == "skills")?;
    let name = *components.get(skills + 1)?;
    let resource = components.get(skills + 2..)?.join("/");
    (!resource.is_empty()).then(|| (components[..skills + 2].join("/"), name.to_owned()))
}

fn relative_to_root(path: &ResourcePath, root: &str) -> Option<String> {
    path.as_str()
        .strip_prefix(root)
        .and_then(|rest| rest.strip_prefix('/'))
        .map(ToOwned::to_owned)
}

fn skill_name_from_root(root: &str) -> &str {
    root.rsplit('/').next().expect("skill roots have a name")
}

fn parent_root(path: &ResourcePath) -> Result<PackageRoot> {
    path.as_str()
        .rsplit_once('/')
        .map(|(parent, _)| PackageRoot::parse(parent))
        .unwrap_or_else(|| Ok(PackageRoot::root()))
}

fn file_name(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .expect("resource paths are non-empty")
}

fn file_stem(path: &str) -> &str {
    file_name(path)
        .rsplit_once('.')
        .map_or(file_name(path), |(stem, _)| stem)
}

fn is_opencode_config(path: &str) -> bool {
    matches!(file_name(path), "opencode.json" | "opencode.jsonc")
}

fn is_agent_path(path: &str) -> bool {
    is_markdown(path) && path.split('/').any(|component| component == "agents")
}

fn is_markdown(path: &str) -> bool {
    matches!(
        file_name(path)
            .rsplit_once('.')
            .map(|(_, extension)| extension),
        Some("md") | Some("mdc")
    )
}

fn semantic_path(path: &ResourcePath) -> String {
    semantic_component(path.as_str())
}

fn semantic_component(value: &str) -> String {
    let value: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    if value.is_empty() {
        "unit".to_owned()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::{ArtifactObservation, InputOrigin};
    use crate::ir::graph::{PackageKind, ResourceContent, SemanticItem};

    fn observation(path: &str, bytes: impl AsRef<[u8]>, executable: bool) -> ArtifactObservation {
        observation_with_label(path, bytes, executable, "workspace")
    }

    fn observation_with_label(
        path: &str,
        bytes: impl AsRef<[u8]>,
        executable: bool,
        input_label: &str,
    ) -> ArtifactObservation {
        ArtifactObservation::new(
            bytes.as_ref().to_vec(),
            path,
            executable,
            InputOrigin::Filesystem,
            input_label,
            None,
        )
        .unwrap()
    }

    #[test]
    fn compiles_opencode_rules_and_skills_with_native_package_resources() {
        let graph = compile_opencode_graph(&[
            observation(".opencode/rust.md", "Use Rust idioms.\n", false),
            observation(
                ".opencode/skills/review/SKILL.md",
                "---\nname: review\ndescription: Review changes\nversion: 1\n---\n# Review\n",
                false,
            ),
            observation(
                ".opencode/skills/review/scripts/check",
                [0xff, 1, 2, 3],
                true,
            ),
        ])
        .unwrap();

        let rule = graph
            .packages
            .values()
            .find(|package| package.kind == PackageKind::Rule)
            .unwrap();
        assert_eq!(rule.semantic_identity.as_str(), "rule:rust");
        assert_eq!(rule.package_root.as_str(), ".opencode");

        let skill = graph
            .packages
            .values()
            .find(|package| package.kind == PackageKind::Skill)
            .unwrap();
        assert_eq!(skill.semantic_identity.as_str(), "skill:review");
        assert_eq!(skill.package_root.as_str(), ".opencode/skills/review");
        assert_eq!(
            skill.resources[&crate::ResourcePath::parse("scripts/check").unwrap()].content,
            ResourceContent::Bytes(vec![0xff, 1, 2, 3])
        );
        assert!(skill.resources[&crate::ResourcePath::parse("scripts/check").unwrap()].executable);
    }

    #[test]
    fn retains_opencode_configuration_mcp_agents_and_permissions_as_unsupported_packages() {
        let graph = compile_opencode_graph(&[observation(
            "opencode.json",
            r#"{
                "instructions": ["AGENTS.md"],
                "mcp": {"local": {"command": "npx"}},
                "agent": {"reviewer": {"prompt": "Review changes"}},
                "permission": {"edit": "deny"}
            }"#,
            false,
        )])
        .unwrap();

        assert_eq!(graph.packages.len(), 4);
        assert_eq!(
            graph
                .packages
                .values()
                .filter(|package| matches!(package.semantic_item, SemanticItem::Unsupported { .. }))
                .count(),
            4
        );
        assert_eq!(
            graph
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.code == "unsupported-semantic")
                .count(),
            4
        );
    }

    #[test]
    fn retains_opencode_agent_markdown_as_an_unsupported_native_package() {
        let graph = compile_opencode_graph(&[observation(
            ".opencode/agents/reviewer.md",
            "---\ndescription: Review changes\nmode: subagent\n---\nReview carefully.\n",
            false,
        )])
        .unwrap();

        let package = graph.packages.values().next().unwrap();
        assert!(matches!(
            package.semantic_item,
            SemanticItem::Unsupported { ref native_kind } if native_kind == "opencode-agent"
        ));
        assert_eq!(package.resources.len(), 1);
    }

    #[test]
    fn keeps_equal_root_opencode_skills_from_distinct_inputs_separate() {
        let observations = vec![
            observation_with_label(
                ".opencode/skills/review/SKILL.md",
                "---\nname: first-review\ndescription: First review\n---\n# First\n",
                false,
                "snapshots/first",
            ),
            observation_with_label(
                ".opencode/skills/review/scripts/first",
                "first companion",
                false,
                "snapshots/first",
            ),
            observation_with_label(
                ".opencode/skills/review/SKILL.md",
                "---\nname: second-review\ndescription: Second review\n---\n# Second\n",
                false,
                "snapshots/second",
            ),
            observation_with_label(
                ".opencode/skills/review/scripts/second",
                "second companion",
                false,
                "snapshots/second",
            ),
        ];
        let mut reversed = observations.clone();
        reversed.reverse();

        let graphs = [
            compile_opencode_graph(&observations).unwrap(),
            compile_opencode_graph(&reversed).unwrap(),
        ];
        assert_eq!(
            graphs[0].to_canonical_json().unwrap(),
            graphs[1].to_canonical_json().unwrap()
        );
        assert_eq!(graphs[0].packages.len(), 2);

        let first = graphs[0]
            .packages
            .values()
            .find(|package| package.semantic_identity.as_str() == "skill:first-review")
            .unwrap();
        assert!(first
            .resources
            .contains_key(&crate::ResourcePath::parse("scripts/first").unwrap()));
        assert!(!first
            .resources
            .contains_key(&crate::ResourcePath::parse("scripts/second").unwrap()));

        let second = graphs[0]
            .packages
            .values()
            .find(|package| package.semantic_identity.as_str() == "skill:second-review")
            .unwrap();
        assert!(second
            .resources
            .contains_key(&crate::ResourcePath::parse("scripts/second").unwrap()));
        assert!(!second
            .resources
            .contains_key(&crate::ResourcePath::parse("scripts/first").unwrap()));
    }

    #[test]
    fn reports_equal_root_opencode_skill_collisions_stably_across_input_orders() {
        let observations = vec![
            observation_with_label(
                ".opencode/skills/review/SKILL.md",
                "---\nname: review\ndescription: First review\n---\n# First\n",
                false,
                "snapshots/first",
            ),
            observation_with_label(
                ".opencode/skills/review/SKILL.md",
                "---\nname: review\ndescription: Second review\n---\n# Second\n",
                false,
                "snapshots/second",
            ),
        ];
        let mut reversed = observations.clone();
        reversed.reverse();

        let errors = [
            compile_opencode_graph(&observations)
                .unwrap_err()
                .to_string(),
            compile_opencode_graph(&reversed).unwrap_err().to_string(),
        ];
        assert_eq!(errors[0], errors[1]);
        assert!(errors[0].contains("semantic identity `skill:review`"));
    }

    #[test]
    fn rejects_duplicate_opencode_skill_resource_paths() {
        let error = compile_opencode_graph(&[
            observation(
                ".opencode/skills/review/SKILL.md",
                "---\nname: review\ndescription: Review changes\n---\n# First\n",
                false,
            ),
            observation(
                ".opencode/skills/review/SKILL.md",
                "---\nname: review\ndescription: Review changes\n---\n# Second\n",
                false,
            ),
        ])
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("duplicate OpenCode skill resource `SKILL.md`"));
    }
}
