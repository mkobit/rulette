use crate::inputs::ArtifactObservation;
use crate::ir::graph::{
    CompilationGraph, FrontendPayload, GraphDiagnostic, Package, PackageKind, PackageRoot,
    PortableActivation, Resource, ResourceContent, ResourcePath, SemanticIdentity, SemanticItem,
    SourceProvenance, TargetActivation,
};
use crate::ActivationMode;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Compiles ordered Cursor observations into portable packages and retained
/// unsupported native units without accessing caller paths.
pub fn compile_cursor_graph(inputs: &[ArtifactObservation]) -> Result<CompilationGraph> {
    let mut packages = Vec::new();
    let mut warnings = Vec::new();
    let mut skill_groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    for (index, observation) in inputs.iter().enumerate() {
        if let Some((root, _)) = skill_location(&observation.source_path) {
            skill_groups.entry(root).or_default().push(index);
        }
    }

    let mut consumed = vec![false; inputs.len()];
    for (root, members) in skill_groups {
        if let Some(package) = compile_skill_package("cursor", &root, &members, inputs)? {
            for index in members {
                consumed[index] = true;
            }
            packages.push(package);
        }
    }

    for (index, observation) in inputs.iter().enumerate() {
        if consumed[index] {
            continue;
        }
        let path = observation.source_path.as_str();
        if path.ends_with(".mdc") {
            packages.push(compile_rule(observation)?);
        } else if file_name(path) == "mcp.json" && contains_path_component(path, ".cursor") {
            packages.push(unsupported_package(observation, "cursor-mcp", "mcp")?);
        } else if contains_path_component(path, "agents") {
            packages.push(unsupported_package(observation, "cursor-agent", "agent")?);
        } else if is_cursor_configuration(path) {
            packages.push(unsupported_package(
                observation,
                "cursor-configuration",
                "configuration",
            )?);
        } else {
            warnings.push(unrecognized_warning("cursor", observation));
        }
    }

    graph_with_warnings(packages, warnings)
}

#[derive(Deserialize)]
struct CursorRuleFrontmatter {
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    globs: Option<CursorGlobs>,
    #[serde(rename = "alwaysApply", default)]
    always_apply: Option<bool>,
    #[serde(rename = "rulette:activation", default)]
    activation: Option<TargetActivation>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CursorGlobs {
    Single(String),
    Many(Vec<String>),
}

impl CursorGlobs {
    fn into_vec(self) -> Vec<String> {
        match self {
            Self::Single(globs) => globs
                .split(',')
                .map(str::trim)
                .filter(|glob| !glob.is_empty())
                .map(ToOwned::to_owned)
                .collect(),
            Self::Many(globs) => globs,
        }
    }
}

fn compile_rule(observation: &ArtifactObservation) -> Result<Package> {
    let source = text(observation, "Cursor rule")?;
    let (frontmatter, body) = split_frontmatter(source);
    let parsed = frontmatter
        .map(serde_yaml::from_str::<CursorRuleFrontmatter>)
        .transpose()
        .context("could not parse Cursor rule frontmatter")?
        .unwrap_or(CursorRuleFrontmatter {
            description: None,
            globs: None,
            always_apply: None,
            activation: None,
            extra: BTreeMap::new(),
        });
    let primary_path = ResourcePath::parse(file_name(observation.source_path.as_str()))?;
    let resource = Resource::primary_instruction(
        primary_path.clone(),
        ResourceContent::Text(body.to_owned()),
        observation.executable,
    );
    let mut resources = BTreeMap::new();
    resources.insert(primary_path.clone(), resource);
    let activation = parsed.activation.or_else(|| {
        activation_from_cursor(parsed.always_apply, parsed.globs.map(CursorGlobs::into_vec))
    });
    Package::new(
        PackageKind::Rule,
        SemanticIdentity::parse(format!(
            "rule:{}",
            file_stem(observation.source_path.as_str())
        ))?,
        provenance("cursor", observation)?,
        parent_root(&observation.source_path)?,
        SemanticItem::Rule {
            primary_instruction: primary_path,
            description: parsed.description,
            activation,
            frontend_payload: payload("cursor.rule-frontmatter", parsed.extra),
        },
        resources,
        None,
    )
}

fn activation_from_cursor(
    always_apply: Option<bool>,
    globs: Option<Vec<String>>,
) -> Option<TargetActivation> {
    if always_apply.is_none() && globs.is_none() {
        return None;
    }
    let mode = if always_apply == Some(true) {
        ActivationMode::Always
    } else if globs.as_ref().is_some_and(|values| !values.is_empty()) {
        ActivationMode::Glob
    } else {
        ActivationMode::Manual
    };
    Some(TargetActivation::Bare(PortableActivation {
        mode: vec![mode],
        globs: globs.filter(|values| !values.is_empty()),
        pattern: None,
        description: None,
    }))
}

fn compile_skill_package(
    frontend: &str,
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
    let (frontmatter, body) = split_frontmatter(text(primary, "Cursor skill")?);
    let parsed = frontmatter
        .map(serde_yaml::from_str::<SkillFrontmatter>)
        .transpose()
        .context("could not parse Cursor skill frontmatter")?
        .unwrap_or_default();
    let name = parsed
        .name
        .unwrap_or_else(|| skill_name_from_root(root).to_owned());
    let description = parsed
        .description
        .ok_or_else(|| anyhow!("Cursor skill `{name}` is missing a discovery description"))?;
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
        resources.insert(path, resource);
    }
    let primary_path = ResourcePath::parse("SKILL.md")?;
    Package::new(
        PackageKind::Skill,
        SemanticIdentity::parse(format!("skill:{name}"))?,
        provenance(frontend, primary)?,
        PackageRoot::parse(root)?,
        SemanticItem::Skill {
            primary_instruction: primary_path,
            description,
            frontend_payload: payload("cursor.skill-frontmatter", parsed.extra),
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
            "unsupported:{native_kind}-{}-{label}",
            semantic_path(&observation.source_path)
        ))?,
        provenance("cursor", observation)?,
        parent_root(&observation.source_path)?,
        SemanticItem::Unsupported {
            native_kind: native_kind.to_owned(),
        },
        resources,
        None,
    )
}

fn graph_with_warnings(
    packages: Vec<Package>,
    mut warnings: Vec<GraphDiagnostic>,
) -> Result<CompilationGraph> {
    let mut graph = CompilationGraph::new(packages)?;
    warnings.sort();
    graph.diagnostics.splice(0..0, warnings);
    graph.validate()?;
    Ok(graph)
}

fn unrecognized_warning(frontend: &str, observation: &ArtifactObservation) -> GraphDiagnostic {
    GraphDiagnostic {
        severity: crate::DiagnosticSeverity::Warning,
        code: "unrecognized-native-file".to_owned(),
        message: format!(
            "{frontend} frontend did not recognize `{}` as a native package member",
            observation.source_path.as_str()
        ),
        package_id: None,
    }
}

fn provenance(frontend: &str, observation: &ArtifactObservation) -> Result<SourceProvenance> {
    let mut provenance = SourceProvenance::new(frontend, &observation.provenance.input_label)?;
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

fn semantic_path(path: &ResourcePath) -> String {
    path.as_str()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn contains_path_component(path: &str, expected: &str) -> bool {
    path.split('/').any(|component| component == expected)
}

fn is_cursor_configuration(path: &str) -> bool {
    matches!(
        file_name(path),
        "settings.json" | "hooks.json" | "permissions.json"
    ) && contains_path_component(path, ".cursor")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::{ArtifactObservation, InputOrigin};
    use crate::ir::graph::{PackageKind, ResourceContent, SemanticItem, TargetActivation};

    fn observation(path: &str, bytes: impl AsRef<[u8]>, executable: bool) -> ArtifactObservation {
        ArtifactObservation::new(
            bytes.as_ref().to_vec(),
            path,
            executable,
            InputOrigin::Filesystem,
            "workspace",
            None,
        )
        .unwrap()
    }

    #[test]
    fn compiles_cursor_rules_with_portable_activation_and_frontmatter_payload() {
        let graph = compile_cursor_graph(&[observation(
            ".cursor/rules/rust.mdc",
            "---\ndescription: Use Rust idioms\nalwaysApply: true\nglobs: src/**/*.rs\ncustom: retained\n---\nPrefer Result over panic.\n",
            false,
        )])
        .unwrap();

        let package = graph.packages.values().next().unwrap();
        assert_eq!(package.kind, PackageKind::Rule);
        assert_eq!(package.semantic_identity.as_str(), "rule:rust");
        assert_eq!(package.package_root.as_str(), ".cursor/rules");
        assert_eq!(
            package.resources[&crate::ResourcePath::parse("rust.mdc").unwrap()].content,
            ResourceContent::Text("Prefer Result over panic.\n".to_string())
        );
        let SemanticItem::Rule {
            description,
            activation,
            frontend_payload,
            ..
        } = &package.semantic_item
        else {
            panic!("Cursor MDC must compile to a rule package")
        };
        assert_eq!(description.as_deref(), Some("Use Rust idioms"));
        assert!(matches!(
            activation,
            Some(TargetActivation::Bare(activation))
                if activation.mode == vec![crate::ActivationMode::Always]
                    && activation.globs == Some(vec!["src/**/*.rs".to_string()])
        ));
        assert_eq!(
            frontend_payload.as_ref().unwrap().fields["custom"],
            serde_json::json!("retained")
        );
    }

    #[test]
    fn compiles_cursor_skill_packages_with_opaque_binary_resources() {
        let graph = compile_cursor_graph(&[
            observation(
                ".cursor/skills/review/SKILL.md",
                "---\nname: review\ndescription: Review changes\n---\n# Review\n",
                false,
            ),
            observation(
                ".cursor/skills/review/scripts/check.sh",
                [0xff, b'E', b'L', b'F'],
                true,
            ),
        ])
        .unwrap();

        let package = graph.packages.values().next().unwrap();
        assert_eq!(package.kind, PackageKind::Skill);
        assert_eq!(package.semantic_identity.as_str(), "skill:review");
        assert_eq!(package.package_root.as_str(), ".cursor/skills/review");
        assert_eq!(
            package.resources[&crate::ResourcePath::parse("scripts/check.sh").unwrap()].content,
            ResourceContent::Bytes(vec![0xff, b'E', b'L', b'F'])
        );
        assert!(
            package.resources[&crate::ResourcePath::parse("scripts/check.sh").unwrap()].executable
        );
    }

    #[test]
    fn retains_cursor_mcp_configuration_as_an_unsupported_native_package() {
        let graph = compile_cursor_graph(&[observation(
            ".cursor/mcp.json",
            r#"{"mcpServers":{"local":{"command":"npx"}}}"#,
            false,
        )])
        .unwrap();

        let package = graph.packages.values().next().unwrap();
        assert!(matches!(
            package.semantic_item,
            SemanticItem::Unsupported { ref native_kind } if native_kind == "cursor-mcp"
        ));
        assert_eq!(package.resources.len(), 1);
        assert!(graph
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "unsupported-semantic"));
    }

    #[test]
    fn rejects_cursor_rule_identity_collisions() {
        let error = compile_cursor_graph(&[
            observation(".cursor/rules/a/rust.mdc", "first", false),
            observation(".cursor/rules/b/rust.mdc", "second", false),
        ])
        .unwrap_err();

        assert!(error.to_string().contains("semantic identity"));
    }
}
