use crate::inputs::ArtifactObservation;
use crate::{ActivationMode, PortableActivation};
use crate::{
    CompilationGraph, FrontendPayload, Package, PackageKind, PackageRoot, Resource,
    ResourceContent, ResourcePath, SemanticIdentity, SemanticItem, SourceProvenance,
    TargetActivation,
};
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

/// Compiles documented Claude project files into the package-aware graph.
///
/// The caller supplies observations in source order.
/// Sorting them again keeps standalone use deterministic and leaves all
/// package-resource ordering to the graph's ordered maps.
pub fn parse_graph(observations: &[ArtifactObservation]) -> Result<CompilationGraph> {
    let mut observations: Vec<_> = observations.iter().collect();
    observations.sort_by(|left, right| observation_key(left).cmp(&observation_key(right)));

    let mut packages = Vec::new();
    let mut skill_members: BTreeMap<SkillGroupKey, Vec<&ArtifactObservation>> = BTreeMap::new();

    for observation in observations {
        let path = observation.source_path.as_str();
        if let Some(root) = skill_root(path) {
            skill_members
                .entry(SkillGroupKey::from_observation(root, observation))
                .or_default()
                .push(observation);
        } else if is_rule_path(path) {
            packages.push(rule_package(observation)?);
        } else if is_agent_path(path) {
            packages.push(unsupported_package(
                observation,
                "agent",
                None,
                markdown_frontmatter_payload(observation, "claude.agent-frontmatter")?,
            )?);
        } else if path == ".claude/settings.json" {
            packages.extend(settings_packages(observation)?);
        } else if path == ".mcp.json" {
            packages.extend(mcp_packages(observation)?);
        }
    }

    for (key, members) in skill_members {
        if let Some(primary) = members
            .iter()
            .copied()
            .find(|observation| skill_member(observation.source_path.as_str()) == Some("SKILL.md"))
        {
            packages.push(skill_package(&key.root, primary, &members)?);
        }
    }

    CompilationGraph::new(packages)
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

fn is_rule_path(path: &str) -> bool {
    path == "CLAUDE.md"
        || path.ends_with("/CLAUDE.md")
        || (path.starts_with(".claude/rules/") && path.ends_with(".md"))
}

fn is_agent_path(path: &str) -> bool {
    path.starts_with(".claude/agents/") && path.ends_with(".md")
}

fn rule_package(observation: &ArtifactObservation) -> Result<Package> {
    let source_path = observation.source_path.as_str();
    let (root, name) = split_parent(source_path)?;
    let primary_path = ResourcePath::parse(name)?;
    let source = std::str::from_utf8(&observation.bytes)
        .context("Claude instruction files must be valid UTF-8")?;
    let document = if source_path.starts_with(".claude/rules/") {
        parse_rule_document(source)?
    } else {
        RuleDocument::plain(source)
    };
    let mut resources = BTreeMap::new();
    resources.insert(
        primary_path.clone(),
        Resource::primary_instruction(
            primary_path.clone(),
            ResourceContent::Text(document.body),
            observation.executable,
        ),
    );
    Package::new(
        PackageKind::Rule,
        SemanticIdentity::parse(format!("rule:{source_path}"))?,
        provenance(observation)?,
        PackageRoot::parse(root)?,
        SemanticItem::Rule {
            primary_instruction: primary_path,
            description: None,
            activation: document.activation,
            frontend_payload: document.frontmatter,
        },
        resources,
        None,
    )
}

struct RuleDocument {
    body: String,
    activation: Option<TargetActivation>,
    frontmatter: Option<FrontendPayload>,
}

impl RuleDocument {
    fn plain(body: &str) -> Self {
        Self {
            body: body.to_owned(),
            activation: None,
            frontmatter: None,
        }
    }
}

fn parse_rule_document(input: &str) -> Result<RuleDocument> {
    if !input.starts_with("---\n") && !input.starts_with("---\r\n") {
        return Ok(RuleDocument::plain(input));
    }

    let (frontmatter, body) = split_frontmatter(input)?;
    let mut fields: BTreeMap<String, serde_json::Value> =
        serde_yaml::from_str(frontmatter).context("Claude rule frontmatter must be valid YAML")?;
    let activation = fields
        .remove("paths")
        .map(activation_from_paths)
        .transpose()?;
    let frontmatter = (!fields.is_empty()).then(|| FrontendPayload {
        namespace: "claude.rule-frontmatter".to_owned(),
        fields,
    });
    Ok(RuleDocument {
        body: body.to_owned(),
        activation,
        frontmatter,
    })
}

fn activation_from_paths(value: serde_json::Value) -> Result<TargetActivation> {
    let globs = match value {
        serde_json::Value::String(path) => vec![path],
        serde_json::Value::Array(paths) => paths
            .into_iter()
            .map(|path| {
                path.as_str()
                    .map(ToOwned::to_owned)
                    .context("Claude rule paths entries must be strings")
            })
            .collect::<Result<Vec<_>>>()?,
        _ => bail!("Claude rule paths must be a string or an array of strings"),
    };
    if globs.is_empty() || globs.iter().any(|glob| glob.is_empty()) {
        bail!("Claude rule paths must contain at least one non-empty glob")
    }
    Ok(TargetActivation::Bare(PortableActivation {
        mode: vec![ActivationMode::Glob],
        globs: Some(globs),
        pattern: None,
        description: None,
    }))
}

fn skill_package(
    root: &str,
    primary: &ArtifactObservation,
    members: &[&ArtifactObservation],
) -> Result<Package> {
    let primary_text =
        std::str::from_utf8(&primary.bytes).context("Claude SKILL.md must be valid UTF-8")?;
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
            bail!("duplicate Claude skill resource `{}`", path.as_str());
        }
    }

    let payload = (!document.frontmatter.is_empty()).then(|| FrontendPayload {
        namespace: "claude.skill-frontmatter".to_owned(),
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

fn settings_packages(observation: &ArtifactObservation) -> Result<Vec<Package>> {
    let parsed: serde_json::Value = serde_json::from_slice(&observation.bytes)
        .context("Claude settings.json must be valid JSON")?;
    let fields = parsed
        .as_object()
        .context("Claude settings.json must contain a JSON object")?;
    let mut packages = Vec::new();

    if let Some(mcp_servers) = fields.get("mcpServers") {
        let mcp_servers = mcp_servers
            .as_object()
            .context("Claude settings mcpServers must be an object")?;
        for (name, config) in mcp_servers {
            packages.push(unsupported_package(
                observation,
                "mcp-server",
                Some(name),
                Some(value_payload(
                    "claude.settings.mcp-server",
                    "config",
                    config,
                )),
            )?);
        }
    }
    if let Some(hooks) = fields.get("hooks") {
        let hooks = hooks
            .as_object()
            .context("Claude settings hooks must be an object")?;
        for (name, config) in hooks {
            packages.push(unsupported_package(
                observation,
                "hook",
                Some(name),
                Some(value_payload("claude.settings.hook", "config", config)),
            )?);
        }
    }
    if let Some(permissions) = fields.get("permissions") {
        packages.push(unsupported_package(
            observation,
            "permissions",
            None,
            Some(value_payload(
                "claude.settings.permissions",
                "config",
                permissions,
            )),
        )?);
    }
    for (name, config) in fields
        .iter()
        .filter(|(name, _)| !matches!(name.as_str(), "mcpServers" | "hooks" | "permissions"))
    {
        packages.push(unsupported_package(
            observation,
            "settings",
            Some(name),
            Some(value_payload("claude.settings", "value", config)),
        )?);
    }
    if packages.is_empty() {
        packages.push(unsupported_package(
            observation,
            "settings",
            None,
            Some(FrontendPayload {
                namespace: "claude.settings".to_owned(),
                fields: BTreeMap::new(),
            }),
        )?);
    }
    Ok(packages)
}

fn mcp_packages(observation: &ArtifactObservation) -> Result<Vec<Package>> {
    let parsed: serde_json::Value = serde_json::from_slice(&observation.bytes)
        .context("Claude .mcp.json must be valid JSON")?;
    let fields = parsed
        .as_object()
        .context("Claude .mcp.json must contain a JSON object")?;
    let mcp_servers = fields
        .get("mcpServers")
        .and_then(serde_json::Value::as_object)
        .context("Claude .mcp.json must contain an mcpServers object")?;
    let mut packages = Vec::new();
    for (name, config) in mcp_servers {
        packages.push(unsupported_package(
            observation,
            "mcp-server",
            Some(name),
            Some(value_payload("claude.mcp.mcp-server", "config", config)),
        )?);
    }
    if packages.is_empty() {
        packages.push(unsupported_package(
            observation,
            "mcp-server",
            None,
            Some(FrontendPayload {
                namespace: "claude.mcp".to_owned(),
                fields: BTreeMap::new(),
            }),
        )?);
    }
    for (name, value) in fields
        .iter()
        .filter(|(name, _)| name.as_str() != "mcpServers")
    {
        packages.push(unsupported_package(
            observation,
            "mcp-config",
            Some(name),
            Some(value_payload("claude.mcp.config", "value", value)),
        )?);
    }
    Ok(packages)
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

fn value_payload(namespace: &str, key: &str, value: &serde_json::Value) -> FrontendPayload {
    let mut fields = BTreeMap::new();
    fields.insert(key.to_owned(), value.clone());
    FrontendPayload {
        namespace: namespace.to_owned(),
        fields,
    }
}

fn markdown_frontmatter_payload(
    observation: &ArtifactObservation,
    namespace: &str,
) -> Result<Option<FrontendPayload>> {
    let input = std::str::from_utf8(&observation.bytes)
        .context("Claude agent markdown must be valid UTF-8")?;
    if !input.starts_with("---\n") && !input.starts_with("---\r\n") {
        return Ok(None);
    }
    let (frontmatter, _) = split_frontmatter(input)?;
    let fields: BTreeMap<String, serde_json::Value> =
        serde_yaml::from_str(frontmatter).context("Claude agent frontmatter must be valid YAML")?;
    Ok(Some(FrontendPayload {
        namespace: namespace.to_owned(),
        fields,
    }))
}

fn provenance(observation: &ArtifactObservation) -> Result<SourceProvenance> {
    let mut provenance = SourceProvenance::new("claude", &observation.provenance.input_label)?;
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
    let suffix = path.strip_prefix(".claude/skills/")?;
    let (name, member) = suffix.split_once('/')?;
    (!name.is_empty() && !member.is_empty()).then(|| &path[..".claude/skills/".len() + name.len()])
}

fn skill_member(path: &str) -> Option<&str> {
    let suffix = path.strip_prefix(".claude/skills/")?;
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
        .context("Claude SKILL.md frontmatter must be valid YAML")?;
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
    use crate::{
        ActivationMode, PackageKind, ResourceContent, ResourcePath, ResourceRole, SemanticItem,
        TargetActivation,
    };

    fn observation(path: &str, bytes: impl Into<Vec<u8>>, executable: bool) -> ArtifactObservation {
        observation_with_label(path, bytes, executable, "fixtures/claude-project")
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
            "fixtures/claude-project.tar",
            Some(ResourcePath::parse(path).unwrap()),
        )
        .expect("test archive observation is valid")
    }

    fn package_by_identity<'a>(
        graph: &'a crate::CompilationGraph,
        identity: &str,
    ) -> &'a crate::Package {
        graph
            .packages
            .values()
            .find(|package| package.semantic_identity.as_str() == identity)
            .unwrap_or_else(|| panic!("package `{identity}` is present"))
    }

    #[test]
    fn parses_project_and_rules_directory_markdown_as_rule_packages() {
        let graph = parse_graph(&[
            observation("CLAUDE.md", "Project conventions.\n", false),
            observation(
                ".claude/rules/rust.md",
                "Prefer Result over unwrap.\n",
                false,
            ),
        ])
        .expect("Claude rule files parse into a graph");

        let project = package_by_identity(&graph, "rule:CLAUDE.md");
        assert_eq!(project.kind, PackageKind::Rule);
        assert_eq!(project.package_root().as_str(), ".");
        assert_eq!(project.provenance.frontend, "claude");
        assert_eq!(project.provenance.input_label, "fixtures/claude-project");
        let project_resource = project
            .resources
            .get(&ResourcePath::parse("CLAUDE.md").unwrap())
            .expect("project rule retains its instruction");
        assert_eq!(project_resource.role, ResourceRole::PrimaryInstruction);
        assert_eq!(
            project_resource.content,
            ResourceContent::Text("Project conventions.\n".to_owned())
        );

        let rule = package_by_identity(&graph, "rule:.claude/rules/rust.md");
        assert_eq!(rule.kind, PackageKind::Rule);
        assert_eq!(rule.package_root().as_str(), ".claude/rules");
        assert!(rule
            .resources
            .contains_key(&ResourcePath::parse("rust.md").unwrap()));
    }

    #[test]
    fn parses_claude_rule_paths_frontmatter_as_activation_and_payload() {
        let graph = parse_graph(&[observation(
            ".claude/rules/api.md",
            "---\npaths:\n  - src/api/**/*.rs\nowner: platform\n---\n# API rules\n\nValidate every request.\n",
            false,
        )])
        .expect("path-scoped Claude rule parses into a graph");

        let package = package_by_identity(&graph, "rule:.claude/rules/api.md");
        let SemanticItem::Rule {
            activation: Some(TargetActivation::Bare(activation)),
            frontend_payload: Some(payload),
            ..
        } = &package.semantic_item
        else {
            panic!("rule retains paths activation and unknown frontmatter payload");
        };
        assert_eq!(activation.mode, vec![ActivationMode::Glob]);
        assert_eq!(activation.globs, Some(vec!["src/api/**/*.rs".to_owned()]));
        assert_eq!(payload.namespace, "claude.rule-frontmatter");
        assert_eq!(
            payload.fields.get("owner"),
            Some(&serde_json::Value::String("platform".to_owned()))
        );
        let primary = package
            .resources
            .get(&ResourcePath::parse("api.md").unwrap())
            .expect("rule retains its primary instruction");
        assert_eq!(
            primary.content,
            ResourceContent::Text("# API rules\n\nValidate every request.\n".to_owned())
        );
    }

    #[test]
    fn parses_claude_skill_with_companion_resources_and_frontmatter_payload() {
        let graph = parse_graph(&[
            observation(
                ".claude/skills/release-workflow/SKILL.md",
                "---\nname: release-workflow\ndescription: Prepare a release.\nuser-invocable: false\n---\n# Release\n\nShip it.\n",
                false,
            ),
            observation(
                ".claude/skills/release-workflow/scripts/check.sh",
                vec![0, 1, 2, 3],
                true,
            ),
        ])
        .expect("Claude skill parses into a graph");

        let package = package_by_identity(&graph, "skill:release-workflow");
        assert_eq!(package.kind, PackageKind::Skill);
        assert_eq!(
            package.package_root().as_str(),
            ".claude/skills/release-workflow"
        );
        let primary = package
            .resources
            .get(&ResourcePath::parse("SKILL.md").unwrap())
            .expect("skill entrypoint is retained");
        assert_eq!(primary.role, ResourceRole::PrimaryInstruction);
        assert_eq!(
            primary.content,
            ResourceContent::Text("# Release\n\nShip it.\n".to_owned())
        );
        let companion = package
            .resources
            .get(&ResourcePath::parse("scripts/check.sh").unwrap())
            .expect("skill companion is retained");
        assert_eq!(companion.role, ResourceRole::Opaque);
        assert_eq!(companion.content, ResourceContent::Bytes(vec![0, 1, 2, 3]));
        assert!(companion.executable);

        let SemanticItem::Skill {
            frontend_payload: Some(payload),
            ..
        } = &package.semantic_item
        else {
            panic!("skill retains its native frontmatter payload");
        };
        assert_eq!(payload.namespace, "claude.skill-frontmatter");
        assert_eq!(
            payload.fields.get("user-invocable"),
            Some(&serde_json::Value::Bool(false))
        );
    }

    #[test]
    fn rejects_same_claude_skill_identity_from_distinct_inputs() {
        let error = parse_graph(&[
            observation_with_label(
                ".claude/skills/release-workflow/SKILL.md",
                "---\nname: release-workflow\ndescription: Prepare a release.\n---\n# First\n",
                false,
                "fixtures/first",
            ),
            observation_with_label(
                ".claude/skills/release-workflow/SKILL.md",
                "---\nname: release-workflow\ndescription: Prepare a release.\n---\n# Second\n",
                false,
                "fixtures/second",
            ),
        ])
        .expect_err("separate source packages with the same skill identity must collide");

        assert!(error
            .to_string()
            .contains("duplicate semantic identity `skill:release-workflow`"));
    }

    #[test]
    fn groups_claude_skill_members_from_one_tar_input() {
        let graph = parse_graph(&[
            archive_observation(
                ".claude/skills/release-workflow/SKILL.md",
                "---\nname: release-workflow\ndescription: Prepare a release.\n---\n# Release\n",
                false,
            ),
            archive_observation(
                ".claude/skills/release-workflow/scripts/check.sh",
                vec![0, 1, 2, 3],
                true,
            ),
        ])
        .expect("one archive skill stays one package");

        let package = graph.packages.values().next().unwrap();
        assert_eq!(graph.packages.len(), 1);
        assert!(package
            .resources
            .contains_key(&ResourcePath::parse("scripts/check.sh").unwrap()));
        assert_eq!(
            package
                .provenance
                .archive_member
                .as_ref()
                .map(ResourcePath::as_str),
            Some(".claude/skills/release-workflow/SKILL.md")
        );
    }

    #[test]
    fn classifies_claude_agents_settings_and_mcp_as_unsupported_packages() {
        let settings = br#"{
            "mcpServers": {"github": {"command": "npx"}},
            "hooks": {"PreToolUse": []},
            "permissions": {"allow": ["Bash"]},
            "env": {"CI": "1"}
        }"#;
        let graph = parse_graph(&[
            observation(
                ".claude/agents/reviewer.md",
                "---\nname: reviewer\n---\nReview changes.\n",
                false,
            ),
            observation(".claude/settings.json", settings.as_slice(), false),
            observation(
                ".mcp.json",
                br#"{"mcpServers":{"project":{"command":"node"}},"custom-setting":"preserve-me"}"#
                    .as_slice(),
                false,
            ),
        ])
        .expect("nonportable Claude files remain inspectable");

        for (identity, native_kind) in [
            ("unsupported:agent/.claude/agents/reviewer.md", "agent"),
            (
                "unsupported:mcp-server/.claude/settings.json/github",
                "mcp-server",
            ),
            ("unsupported:hook/.claude/settings.json/PreToolUse", "hook"),
            (
                "unsupported:permissions/.claude/settings.json",
                "permissions",
            ),
            ("unsupported:settings/.claude/settings.json/env", "settings"),
            ("unsupported:mcp-server/.mcp.json/project", "mcp-server"),
            (
                "unsupported:mcp-config/.mcp.json/custom-setting",
                "mcp-config",
            ),
        ] {
            let package = package_by_identity(&graph, identity);
            assert_eq!(package.kind, PackageKind::Unsupported);
            assert!(matches!(
                &package.semantic_item,
                SemanticItem::Unsupported { native_kind: actual } if actual == native_kind
            ));
            assert_eq!(package.resources.len(), 1);
            assert_eq!(
                package.resources.values().next().unwrap().role,
                ResourceRole::Opaque
            );
        }
        let agent = package_by_identity(&graph, "unsupported:agent/.claude/agents/reviewer.md");
        let agent_payload = agent
            .frontend_payload
            .as_ref()
            .expect("agent frontmatter remains in a native payload");
        assert_eq!(agent_payload.namespace, "claude.agent-frontmatter");
        assert_eq!(
            agent_payload.fields.get("name"),
            Some(&serde_json::Value::String("reviewer".to_owned()))
        );

        let mcp = package_by_identity(
            &graph,
            "unsupported:mcp-server/.claude/settings.json/github",
        );
        let mcp_payload = mcp
            .frontend_payload
            .as_ref()
            .expect("settings MCP retains its native payload");
        assert_eq!(mcp_payload.namespace, "claude.settings.mcp-server");
        assert_eq!(
            mcp_payload.fields.get("config"),
            Some(&serde_json::json!({"command": "npx"}))
        );
        let mcp_config =
            package_by_identity(&graph, "unsupported:mcp-config/.mcp.json/custom-setting");
        let mcp_config_payload = mcp_config
            .frontend_payload
            .as_ref()
            .expect("unknown MCP configuration remains in a native payload");
        assert_eq!(mcp_config_payload.namespace, "claude.mcp.config");
        assert_eq!(
            mcp_config_payload.fields.get("value"),
            Some(&serde_json::Value::String("preserve-me".to_owned()))
        );
        assert_eq!(graph.diagnostics.len(), 7);
    }
}
