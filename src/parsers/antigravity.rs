use crate::inputs::ArtifactObservation;
use crate::ir::graph::{
    FrontendPayload, Package, PackageKind, PackageRoot, PortableActivation, Resource,
    ResourceContent, ResourcePath, SemanticIdentity, SemanticItem, SourceProvenance,
    TargetActivation,
};
use crate::parsers::frontend::{NativeCompilation, NativeFrontend, NativeObservationDisposition};
use crate::ActivationMode;
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum AntigravityTrigger {
    AlwaysOn,
    Glob,
    Manual,
    ModelDecision,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum GlobsValue {
    Single(String),
    Many(Vec<String>),
}

impl GlobsValue {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            GlobsValue::Single(s) => s
                .split(',')
                .map(|g| g.trim().to_string())
                .filter(|g| !g.is_empty())
                .collect(),
            GlobsValue::Many(v) => v,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct AntigravityRuleFrontmatter {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    trigger: Option<AntigravityTrigger>,

    #[serde(skip_serializing_if = "Option::is_none")]
    globs: Option<GlobsValue>,

    #[serde(rename = "rulette:activation", skip_serializing_if = "Option::is_none")]
    pub activation: Option<TargetActivation>,

    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

/// Compiles ordered Antigravity observations into portable rule and skill
/// packages while retaining native agent and configuration units as opaque
/// unsupported packages.
#[cfg(test)]
fn compile_antigravity_graph(inputs: &[ArtifactObservation]) -> Result<crate::CompilationGraph> {
    compile_native(inputs)?.into_graph()
}

/// Compiles Antigravity observations while recording the disposition of each input.
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
        if is_agent_path(path) {
            packages.push(unsupported_package(
                observation,
                "antigravity-agent",
                "agent",
            )?);
            dispositions[index] = Some(NativeObservationDisposition::RetainedUnsupportedContent);
        } else if is_configuration(path) {
            packages.push(unsupported_package(
                observation,
                "antigravity-configuration",
                "configuration",
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
        NativeFrontend::Antigravity,
        inputs,
        packages,
        dispositions
            .into_iter()
            .collect::<Option<Vec<NativeObservationDisposition>>>()
            .expect("Antigravity parser classifies every observation"),
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
    let source = text(observation, "Antigravity rule")?;
    let (frontmatter, body) = split_frontmatter(source);
    let parsed = frontmatter
        .map(serde_yaml::from_str::<AntigravityRuleFrontmatter>)
        .transpose()
        .context("could not parse Antigravity rule frontmatter")?
        .unwrap_or(AntigravityRuleFrontmatter {
            description: None,
            trigger: None,
            globs: None,
            activation: None,
            extra: BTreeMap::new(),
        });
    let description = parsed.description;
    let globs = parsed.globs.map(GlobsValue::into_vec);
    let activation = parsed
        .activation
        .or_else(|| {
            parsed.trigger.map(|trigger| {
                TargetActivation::Bare(activation_from_trigger(
                    trigger,
                    globs.clone(),
                    description.clone(),
                ))
            })
        })
        .or_else(|| {
            globs.map(|globs| {
                TargetActivation::Bare(PortableActivation {
                    mode: vec![ActivationMode::Glob],
                    globs: Some(globs),
                    pattern: None,
                    description: None,
                })
            })
        });
    let primary_path = ResourcePath::parse(file_name(observation.source_path.as_str()))?;
    let mut resources = BTreeMap::new();
    resources.insert(
        primary_path.clone(),
        Resource::primary_instruction(
            primary_path.clone(),
            ResourceContent::Text(body.to_owned()),
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
            description,
            activation,
            frontend_payload: payload("antigravity.rule-frontmatter", parsed.extra),
        },
        resources,
        None,
    )
}

fn activation_from_trigger(
    trigger: AntigravityTrigger,
    globs: Option<Vec<String>>,
    description: Option<String>,
) -> PortableActivation {
    let (mode, description) = match trigger {
        AntigravityTrigger::AlwaysOn => (ActivationMode::Always, None),
        AntigravityTrigger::Glob => (ActivationMode::Glob, None),
        AntigravityTrigger::Manual => (ActivationMode::Manual, None),
        AntigravityTrigger::ModelDecision => (ActivationMode::Model, description),
    };
    PortableActivation {
        mode: vec![mode],
        globs: globs.filter(|values| !values.is_empty()),
        pattern: None,
        description,
    }
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
    let (frontmatter, body) = split_frontmatter(text(primary, "Antigravity skill")?);
    let parsed = frontmatter
        .map(serde_yaml::from_str::<SkillFrontmatter>)
        .transpose()
        .context("could not parse Antigravity skill frontmatter")?
        .unwrap_or_default();
    let name = parsed
        .name
        .unwrap_or_else(|| skill_name_from_root(root).to_owned());
    let description = parsed
        .description
        .ok_or_else(|| anyhow!("Antigravity skill `{name}` is missing a discovery description"))?;
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
            bail!("duplicate Antigravity skill resource `{}`", path.as_str());
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
            frontend_payload: payload("antigravity.skill-frontmatter", parsed.extra),
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
    let mut provenance = SourceProvenance::new("antigravity", &observation.provenance.input_label)?;
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

fn is_agent_path(path: &str) -> bool {
    is_markdown(path) && path.split('/').any(|component| component == "agents")
}

fn is_configuration(path: &str) -> bool {
    matches!(
        file_name(path)
            .rsplit_once('.')
            .map(|(_, extension)| extension),
        Some("json") | Some("jsonc") | Some("toml") | Some("yaml") | Some("yml")
    ) && path.split('/').any(|component| component == ".antigravity")
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
    use crate::ir::graph::{PackageKind, ResourceContent, SemanticItem, TargetActivation};

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
    fn test_antigravity_frontmatter_serialization_and_deserialization() {
        let yaml = r#"
description: Rust conventions
trigger: glob
globs:
  - "**/*.rs"
  - "**/Cargo.toml"
"#;
        let parsed: AntigravityRuleFrontmatter = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(parsed.description.as_deref(), Some("Rust conventions"));
        assert_eq!(parsed.trigger, Some(AntigravityTrigger::Glob));
        assert_eq!(
            parsed.globs.map(|g| g.into_vec()),
            Some(vec!["**/*.rs".to_string(), "**/Cargo.toml".to_string()])
        );
    }

    #[test]
    fn test_antigravity_trigger_modes_deserialization() {
        let triggers = [
            ("trigger: always_on\n", AntigravityTrigger::AlwaysOn),
            ("trigger: glob\n", AntigravityTrigger::Glob),
            ("trigger: manual\n", AntigravityTrigger::Manual),
            (
                "trigger: model_decision\n",
                AntigravityTrigger::ModelDecision,
            ),
        ];

        for (yaml, expected) in triggers {
            let parsed: AntigravityRuleFrontmatter = serde_yaml::from_str(yaml).unwrap();
            assert_eq!(parsed.trigger, Some(expected));
        }
    }

    #[test]
    fn compiles_antigravity_rules_with_portable_trigger_activation() {
        let graph = compile_antigravity_graph(&[observation(
            ".antigravity/rust.md",
            "---\ndescription: Rust conventions\ntrigger: model_decision\nnative: retained\n---\nUse Result values.\n",
            false,
        )])
        .unwrap();

        let package = graph.packages.values().next().unwrap();
        assert_eq!(package.kind, PackageKind::Rule);
        assert_eq!(package.semantic_identity.as_str(), "rule:rust");
        assert_eq!(package.package_root.as_str(), ".antigravity");
        let SemanticItem::Rule {
            activation,
            frontend_payload,
            ..
        } = &package.semantic_item
        else {
            panic!("Antigravity markdown must compile to a rule package")
        };
        assert!(matches!(
            activation,
            Some(TargetActivation::Bare(activation))
                if activation.mode == vec![crate::ActivationMode::Model]
                    && activation.description.as_deref() == Some("Rust conventions")
        ));
        assert_eq!(
            frontend_payload.as_ref().unwrap().fields["native"],
            serde_json::json!("retained")
        );
    }

    #[test]
    fn compiles_antigravity_skill_packages_with_opaque_resources() {
        let graph = compile_antigravity_graph(&[
            observation(
                ".antigravity/skills/review/SKILL.md",
                "---\nname: review\ndescription: Review changes\n---\n# Review\n",
                false,
            ),
            observation(".antigravity/skills/review/bin/check", [0xff, 0x00], true),
        ])
        .unwrap();

        let package = graph.packages.values().next().unwrap();
        assert_eq!(package.kind, PackageKind::Skill);
        assert_eq!(package.package_root.as_str(), ".antigravity/skills/review");
        assert_eq!(
            package.resources[&crate::ResourcePath::parse("bin/check").unwrap()].content,
            ResourceContent::Bytes(vec![0xff, 0x00])
        );
    }

    #[test]
    fn retains_antigravity_agent_configuration_as_an_unsupported_native_package() {
        let graph = compile_antigravity_graph(&[observation(
            ".antigravity/agents/reviewer.md",
            "Review changes.\n",
            false,
        )])
        .unwrap();

        let package = graph.packages.values().next().unwrap();
        assert!(matches!(
            package.semantic_item,
            SemanticItem::Unsupported { ref native_kind } if native_kind == "antigravity-agent"
        ));
        assert_eq!(package.resources.len(), 1);
    }

    #[test]
    fn keeps_equal_root_antigravity_skills_from_distinct_inputs_separate() {
        let observations = vec![
            observation_with_label(
                ".antigravity/skills/review/SKILL.md",
                "---\nname: first-review\ndescription: First review\n---\n# First\n",
                false,
                "snapshots/first",
            ),
            observation_with_label(
                ".antigravity/skills/review/scripts/first",
                "first companion",
                false,
                "snapshots/first",
            ),
            observation_with_label(
                ".antigravity/skills/review/SKILL.md",
                "---\nname: second-review\ndescription: Second review\n---\n# Second\n",
                false,
                "snapshots/second",
            ),
            observation_with_label(
                ".antigravity/skills/review/scripts/second",
                "second companion",
                false,
                "snapshots/second",
            ),
        ];
        let mut reversed = observations.clone();
        reversed.reverse();

        let graphs = [
            compile_antigravity_graph(&observations).unwrap(),
            compile_antigravity_graph(&reversed).unwrap(),
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
    fn reports_equal_root_antigravity_skill_collisions_stably_across_input_orders() {
        let observations = vec![
            observation_with_label(
                ".antigravity/skills/review/SKILL.md",
                "---\nname: review\ndescription: First review\n---\n# First\n",
                false,
                "snapshots/first",
            ),
            observation_with_label(
                ".antigravity/skills/review/SKILL.md",
                "---\nname: review\ndescription: Second review\n---\n# Second\n",
                false,
                "snapshots/second",
            ),
        ];
        let mut reversed = observations.clone();
        reversed.reverse();

        let errors = [
            compile_antigravity_graph(&observations)
                .unwrap_err()
                .to_string(),
            compile_antigravity_graph(&reversed)
                .unwrap_err()
                .to_string(),
        ];
        assert_eq!(errors[0], errors[1]);
        assert!(errors[0].contains("semantic identity `skill:review`"));
    }

    #[test]
    fn rejects_duplicate_antigravity_skill_resource_paths() {
        let error = compile_antigravity_graph(&[
            observation(
                ".antigravity/skills/review/SKILL.md",
                "---\nname: review\ndescription: Review changes\n---\n# First\n",
                false,
            ),
            observation(
                ".antigravity/skills/review/SKILL.md",
                "---\nname: review\ndescription: Review changes\n---\n# Second\n",
                false,
            ),
        ])
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("duplicate Antigravity skill resource `SKILL.md`"));
    }
}
