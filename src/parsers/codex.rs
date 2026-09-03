use crate::inputs::ArtifactObservation;
use crate::parsers::frontend::{NativeCompilation, NativeFrontend, NativeObservationDisposition};
use crate::{
    FrontendPayload, Package, PackageKind, PackageRoot, Resource, ResourceContent, ResourcePath,
    SemanticIdentity, SemanticItem, SourceProvenance,
};
use anyhow::{bail, Context, Result};
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
    let mut ordered_observations: Vec<_> = observations.iter().enumerate().collect();
    ordered_observations
        .sort_by(|left, right| observation_key(left.1).cmp(&observation_key(right.1)));

    let mut packages = Vec::new();
    let mut dispositions = vec![None; observations.len()];
    let mut skill_members: BTreeMap<SkillGroupKey, Vec<(usize, &ArtifactObservation)>> =
        BTreeMap::new();

    for (index, observation) in ordered_observations {
        let path = observation.source_path.as_str();
        if let Some(root) = skill_root(path) {
            skill_members
                .entry(SkillGroupKey::from_observation(root, observation))
                .or_default()
                .push((index, observation));
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
        if let Some(primary) = members
            .iter()
            .find(|(_, observation)| {
                skill_member(observation.source_path.as_str()) == Some("SKILL.md")
            })
            .map(|(_, observation)| *observation)
        {
            let member_observations = members
                .iter()
                .map(|(_, observation)| *observation)
                .collect::<Vec<_>>();
            packages.push(skill_package(&key.root, primary, &member_observations)?);
            for (index, _) in members {
                dispositions[index] = Some(NativeObservationDisposition::PackageContent);
            }
        } else {
            for (index, _) in members {
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
    members: &[&ArtifactObservation],
) -> Result<Package> {
    let primary_text =
        std::str::from_utf8(&primary.bytes).context("Codex SKILL.md must be valid UTF-8")?;
    let document = parse_skill_document(primary_text)?;
    let primary_path = ResourcePath::parse("SKILL.md")?;
    let mut resources = BTreeMap::new();

    for member in members {
        let member_path = skill_member(member.source_path.as_str())
            .expect("skill grouping only contains paths below its skill root");
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
            frontend_payload: payload,
        },
        resources,
        None,
    )
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

fn skill_root(path: &str) -> Option<&str> {
    let suffix = path.strip_prefix(".codex/skills/")?;
    let (name, member) = suffix.split_once('/')?;
    (!name.is_empty() && !member.is_empty()).then(|| &path[..".codex/skills/".len() + name.len()])
}

fn skill_member(path: &str) -> Option<&str> {
    let suffix = path.strip_prefix(".codex/skills/")?;
    let (_, member) = suffix.split_once('/')?;
    (!member.is_empty()).then_some(member)
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
}
