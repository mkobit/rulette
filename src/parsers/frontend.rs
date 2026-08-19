use crate::agent_skills::{Skill, SkillMetadata};
use crate::cli::formats::InputFormat;
use crate::{
    Activation, ActivationMode, Entity, McpServer, McpServerConfig, McpServerMetadata, Rule,
    RuleMetadata, RuletteDocument,
};
use anyhow::Result;
use std::collections::BTreeMap as HashMap;
use std::path::{Component, Path, PathBuf};

pub fn parse(input: &str, format: InputFormat, filename: Option<&str>) -> Result<RuletteDocument> {
    tracing::info!("Parsing input as format: {:?}", format);
    match format {
        InputFormat::Auto => {
            if input.trim_start().starts_with('{') {
                if let Ok(mut doc) = serde_json::from_str::<RuletteDocument>(input) {
                    doc.ir_version = "0.1".to_string();
                    return Ok(doc);
                }
                if input.contains("\"permissions\"")
                    || input.contains("\"allowManagedPermissionRulesOnly\"")
                    || input.contains("\"hooks\"")
                {
                    return Ok(RuletteDocument {
                        ir_version: "0.1".to_string(),
                        entities: parse_claude_settings(input, filename)?,
                    });
                }
                if input.contains("\"mcpServers\"") {
                    if let Ok(entities) = parse_claude_settings(input, filename) {
                        return Ok(RuletteDocument {
                            ir_version: "0.1".to_string(),
                            entities,
                        });
                    }
                    return Ok(RuletteDocument {
                        ir_version: "0.1".to_string(),
                        entities: parse_cursor_mcp(input)?,
                    });
                }
            }
            let entities = if input.starts_with("---\n") || input.starts_with("---\r\n") {
                if input.contains("name:") && input.contains("description:") {
                    match parse_agent_skills(input, filename) {
                        Ok(skill) => vec![Entity::Skill(skill)],
                        Err(_) => match parse_cursor_mdc(input, filename) {
                            Ok(rule) => vec![Entity::Rule(rule)],
                            Err(e) if e.to_string().contains("ignored") => vec![],
                            Err(e) => return Err(e),
                        },
                    }
                } else {
                    match parse_cursor_mdc(input, filename) {
                        Ok(rule) => vec![Entity::Rule(rule)],
                        Err(e) if e.to_string().contains("ignored") => vec![],
                        Err(e) => return Err(e),
                    }
                }
            } else {
                match parse_claude(input, filename) {
                    Ok(rule) => vec![Entity::Rule(rule)],
                    Err(e) if e.to_string().contains("ignored") => vec![],
                    Err(e) => return Err(e),
                }
            };
            let mut doc = RuletteDocument {
                ir_version: "0.1".to_string(),
                entities,
            };
            inject_source_file(&mut doc, filename);
            Ok(doc)
        }
        InputFormat::IrJson => {
            let mut doc: RuletteDocument = serde_json::from_str(input)?;
            doc.ir_version = "0.1".to_string();
            inject_source_file(&mut doc, filename);
            Ok(doc)
        }
        InputFormat::IrToml => {
            let mut doc: RuletteDocument = toml::from_str(input)?;
            doc.ir_version = "0.1".to_string();
            inject_source_file(&mut doc, filename);
            Ok(doc)
        }
        _ => {
            let entities = match format {
                InputFormat::SkillMd | InputFormat::AgentSkills => {
                    vec![Entity::Skill(parse_agent_skills(input, filename)?)]
                }
                InputFormat::CursorMdc => match parse_cursor_mdc(input, filename) {
                    Ok(rule) => vec![Entity::Rule(rule)],
                    Err(e) if e.to_string().contains("ignored") => vec![],
                    Err(e) => return Err(e),
                },
                InputFormat::CursorMcp => parse_cursor_mcp(input)?,
                InputFormat::ClaudeSettings => parse_claude_settings(input, filename)?,
                InputFormat::Claude
                | InputFormat::Codex
                | InputFormat::Copilot
                | InputFormat::Windsurf
                | InputFormat::CursorLegacy => match parse_claude(input, filename) {
                    Ok(rule) => vec![Entity::Rule(rule)],
                    Err(e) if e.to_string().contains("ignored") => vec![],
                    Err(e) => return Err(e),
                },
                InputFormat::Gemini => parse_gemini(input, filename)?,
                _ => unreachable!(),
            };
            let mut doc = RuletteDocument {
                ir_version: "0.1".to_string(),
                entities,
            };
            inject_source_file(&mut doc, filename);
            Ok(doc)
        }
    }
}

fn inject_source_file(doc: &mut RuletteDocument, filename: Option<&str>) {
    if let Some(f) = filename {
        for entity in &mut doc.entities {
            match entity {
                Entity::Rule(rule) => {
                    rule.metadata.extra.insert(
                        "rulette:source_file".to_string(),
                        serde_json::Value::String(f.to_string()),
                    );
                }
                Entity::Skill(skill) => {
                    skill.metadata.extra.insert(
                        "rulette:source_file".to_string(),
                        serde_json::Value::String(f.to_string()),
                    );
                }
                Entity::Agent(agent) => {
                    agent.metadata.extra.insert(
                        "rulette:source_file".to_string(),
                        serde_json::Value::String(f.to_string()),
                    );
                }
                Entity::McpServer(mcp) => {
                    mcp.metadata.extra.insert(
                        "rulette:source_file".to_string(),
                        serde_json::Value::String(f.to_string()),
                    );
                }
                Entity::Hook(hook) => {
                    hook.metadata.extra.insert(
                        "rulette:source_file".to_string(),
                        serde_json::Value::String(f.to_string()),
                    );
                }
                Entity::Permissions(perms) => {
                    perms.metadata.extra.insert(
                        "rulette:source_file".to_string(),
                        serde_json::Value::String(f.to_string()),
                    );
                }
            }
        }
    }
}

fn parse_gemini(input: &str, filename: Option<&str>) -> Result<Vec<Entity>> {
    if let Ok(subagent) = super::gemini::GeminiSubAgent::parse(input) {
        let mut extra = subagent.metadata.extra.clone();
        if let Some(kind) = subagent.metadata.kind {
            extra.insert("kind".to_string(), serde_json::Value::String(kind));
        }
        if let Some(mcp) = subagent.metadata.mcp_servers {
            if let Ok(mcp_val) = serde_json::to_value(mcp) {
                extra.insert("mcpServers".to_string(), mcp_val);
            }
        }
        if let Some(temperature) = subagent.metadata.temperature {
            extra.insert(
                "temperature".to_string(),
                serde_json::Value::Number(serde_json::Number::from_f64(temperature).unwrap()),
            );
        }
        if let Some(max_turns) = subagent.metadata.max_turns {
            extra.insert(
                "max_turns".to_string(),
                serde_json::Value::Number(max_turns.into()),
            );
        }
        if let Some(timeout_mins) = subagent.metadata.timeout_mins {
            extra.insert(
                "timeout_mins".to_string(),
                serde_json::Value::Number(timeout_mins.into()),
            );
        }

        let agent_metadata = crate::AgentMetadata {
            name: subagent.metadata.name,
            description: Some(subagent.metadata.description),
            tool_access: None,
            agent_tools: subagent.metadata.tools,
            models: subagent.metadata.model.map(|m| vec![m]),
            extra,
        };

        Ok(vec![Entity::Agent(crate::Agent {
            metadata: agent_metadata,
            body: subagent.system_prompt,
        })])
    } else {
        Ok(vec![Entity::Rule(parse_claude(input, filename)?)])
    }
}

fn parse_agent_skills(input: &str, filename: Option<&str>) -> Result<Skill> {
    if let Some(f) = filename {
        if f.to_lowercase().ends_with("readme.md") {
            return Err(anyhow::anyhow!("README.md is ignored"));
        }
    }
    let (frontmatter, body) = extract_frontmatter(input);
    let mut metadata = SkillMetadata {
        name: "unnamed-skill".to_string(),
        description: "No description provided".to_string(),
        version: None,
        license: None,
        compatibility: None,
        metadata: HashMap::new(),
        allowed_tools: None,
        extra: HashMap::new(),
    };

    if let Some(fm) = frontmatter {
        #[derive(serde::Deserialize)]
        struct FmParse {
            name: Option<String>,
            description: Option<String>,
            version: Option<String>,
            license: Option<String>,
            compatibility: Option<String>,
            #[serde(rename = "allowed-tools")]
            allowed_tools: Option<serde_yaml::Value>,
            #[serde(flatten)]
            extra: HashMap<String, serde_json::Value>,
        }
        match serde_yaml::from_str::<FmParse>(fm) {
            Ok(parsed_fm) => {
                if let Some(name) = parsed_fm.name {
                    metadata.name = name;
                }
                if let Some(desc) = parsed_fm.description {
                    metadata.description = desc;
                }
                metadata.version = parsed_fm.version;
                metadata.license = parsed_fm.license;
                metadata.compatibility = parsed_fm.compatibility;
                if let Some(at) = parsed_fm.allowed_tools {
                    metadata.allowed_tools = Some(match at {
                        serde_yaml::Value::String(s) => serde_json::Value::String(s),
                        serde_yaml::Value::Sequence(seq) => {
                            let json_seq: Vec<serde_json::Value> = seq
                                .into_iter()
                                .filter_map(|v| {
                                    v.as_str().map(|s| serde_json::Value::String(s.to_string()))
                                })
                                .collect();
                            serde_json::Value::Array(json_seq)
                        }
                        _ => serde_json::Value::String(
                            serde_yaml::to_string(&at)
                                .unwrap_or_default()
                                .trim()
                                .to_string(),
                        ),
                    });
                }
                metadata.extra = parsed_fm.extra;
            }
            Err(e) => {
                eprintln!("Warning: Failed to parse agent-skills frontmatter: {}", e);
            }
        }
    }
    if metadata.name == "unnamed-skill" {
        if let Some(name) = extract_name_from_filename(filename) {
            metadata.name = name;
        }
    }
    if metadata.description == "No description provided" {
        if let Some(desc) = extract_description_from_body(body) {
            metadata.description = desc;
        }
    }

    metadata.validate()?;

    Ok(Skill {
        metadata,
        body: body.to_string(),
    })
}

fn parse_cursor_mdc(input: &str, filename: Option<&str>) -> Result<Rule> {
    if let Some(f) = filename {
        if f.to_lowercase().ends_with("readme.md") {
            return Err(anyhow::anyhow!("README.md is ignored"));
        }
    }
    let (frontmatter, body) = extract_frontmatter(input);
    let mut metadata = RuleMetadata::default();

    if let Some(fm) = frontmatter {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum GlobsValue {
            Single(String),
            Many(Vec<String>),
        }
        impl GlobsValue {
            fn into_vec(self) -> Vec<String> {
                match self {
                    // Cursor's real .mdc convention is a single comma-separated scalar.
                    GlobsValue::Single(s) => s
                        .split(',')
                        .map(|g| g.trim().to_string())
                        .filter(|g| !g.is_empty())
                        .collect(),
                    GlobsValue::Many(v) => v,
                }
            }
        }
        #[derive(serde::Deserialize)]
        struct FmParse {
            description: Option<String>,
            globs: Option<GlobsValue>,
            #[serde(rename = "alwaysApply")]
            always_apply: Option<bool>,
            #[serde(flatten)]
            extra: HashMap<String, serde_json::Value>,
        }
        if let Ok(parsed_fm) = serde_yaml::from_str::<FmParse>(fm) {
            metadata.description = parsed_fm.description;
            metadata.extra = parsed_fm.extra;
            metadata.activation = activation_from_cursor(
                parsed_fm.always_apply,
                parsed_fm.globs.map(GlobsValue::into_vec),
            )
            .map(crate::TargetOverrides::Bare);
        }
    }
    if !metadata.extra.contains_key("name") {
        if let Some(name) = extract_name_from_filename(filename) {
            metadata
                .extra
                .insert("name".to_string(), serde_json::Value::String(name));
        }
    }
    if metadata.description.is_none() {
        if let Some(desc) = extract_description_from_body(body) {
            metadata.description = Some(desc);
        }
    }

    Ok(Rule {
        metadata,
        body: body.to_string(),
    })
}

/// Maps Cursor's `alwaysApply`/`globs` frontmatter pair onto the typed
/// activation model, mirroring Cursor's own rule-type resolution: `alwaysApply:
/// true` wins outright, glob patterns imply auto-attach, and the absence of
/// both means the rule is manually referenced.
fn activation_from_cursor(
    always_apply: Option<bool>,
    globs: Option<Vec<String>>,
) -> Option<Activation> {
    if always_apply.is_none() && globs.is_none() {
        return None;
    }
    if always_apply == Some(true) {
        // Cursor ignores `globs` when `alwaysApply` is true, but a rule can
        // still declare both; preserve the globs for fidelity even though
        // `Always` mode doesn't consult them.
        return Some(Activation {
            mode: vec![ActivationMode::Always],
            globs: globs.filter(|g| !g.is_empty()),
            pattern: None,
            description: None,
        });
    }
    if let Some(globs) = globs.filter(|g| !g.is_empty()) {
        return Some(Activation {
            mode: vec![ActivationMode::Glob],
            globs: Some(globs),
            pattern: None,
            description: None,
        });
    }
    Some(Activation {
        mode: vec![ActivationMode::Manual],
        globs: None,
        pattern: None,
        description: None,
    })
}

fn parse_claude_settings(input: &str, filename: Option<&str>) -> Result<Vec<Entity>> {
    use crate::translate::claude_v1::{ClaudeMcpConfig, ClaudeV1};
    use crate::translate::Translator;

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ClaudeSettingsFile {
        #[serde(default)]
        mcp_servers: Option<HashMap<String, ClaudeMcpConfig>>,
        #[serde(default)]
        hooks: Option<HashMap<String, serde_json::Value>>,
        #[serde(flatten)]
        extra: HashMap<String, serde_json::Value>,
    }

    let parsed: ClaudeSettingsFile = serde_json::from_str(input)?;
    let mut entities = Vec::new();
    let translator = ClaudeV1;

    if let Some(mcp_servers) = parsed.mcp_servers {
        for (name, config) in mcp_servers {
            entities.push(Entity::McpServer(translator.translate_mcp(&name, &config)?));
        }
    }

    if let Some(hooks) = parsed.hooks {
        for (name, hook_data) in hooks {
            entities.push(Entity::Hook(
                translator.translate_hook(&name, &hook_data, filename)?,
            ));
        }
    }

    if !parsed.extra.is_empty() {
        entities.push(Entity::Permissions(crate::Permissions {
            metadata: crate::PermissionsMetadata {
                name: None,
                tool_access: None,
                settings_overrides: None,
                extra: parsed.extra,
            },
        }));
    }

    Ok(entities)
}

/// Infers Codex's `rulette:directory-scope` extension key from a real nested
/// `AGENTS.md` file's location, mirroring what `emitters::codex` already does
/// in reverse (grouping entities by this same key back into nested files at
/// emit time). Only triggers for files literally named `AGENTS.md`
/// (case-insensitive) -- Codex's own namesake -- so unrelated markdown files
/// routed through the same fallback parser (CLAUDE.md,
/// copilot-instructions.md, etc.) are unaffected.
///
/// `emitters::codex` rejects an absolute `rulette:directory-scope`, so an
/// absolute filename (e.g. from an absolute input path on the command line)
/// is made relative to the current directory first. Rulette has no
/// independent notion of "repo root" beyond that; a path outside the current
/// directory yields no inferred scope rather than an invalid absolute one.
fn relativize_to_cwd(parent: &Path, cwd: &Path) -> Option<PathBuf> {
    if let Ok(rel) = parent.strip_prefix(cwd) {
        return Some(rel.to_path_buf());
    }

    let parent_comps: Vec<_> = parent.components().collect();
    let cwd_comps: Vec<_> = cwd.components().collect();

    if parent_comps.len() >= cwd_comps.len() {
        let matches = parent_comps
            .iter()
            .zip(cwd_comps.iter())
            .all(|(p, c)| match (p, c) {
                (Component::Prefix(p_prefix), Component::Prefix(c_prefix)) => p_prefix
                    .as_os_str()
                    .to_string_lossy()
                    .eq_ignore_ascii_case(&c_prefix.as_os_str().to_string_lossy()),
                (Component::Normal(p_name), Component::Normal(c_name)) => {
                    if cfg!(windows) {
                        p_name
                            .to_string_lossy()
                            .eq_ignore_ascii_case(&c_name.to_string_lossy())
                    } else {
                        p_name == c_name
                    }
                }
                _ => p == c,
            });

        if matches {
            let rel: PathBuf = parent_comps[cwd_comps.len()..]
                .iter()
                .map(|c| c.as_os_str())
                .collect();
            return Some(rel);
        }
    }

    if let (Ok(canon_parent), Ok(canon_cwd)) =
        (std::fs::canonicalize(parent), std::fs::canonicalize(cwd))
    {
        if let Ok(rel) = canon_parent.strip_prefix(&canon_cwd) {
            return Some(rel.to_path_buf());
        }
    }

    None
}

fn infer_codex_directory_scope(filename: Option<&str>) -> Option<String> {
    let f = filename?;
    let path = Path::new(f);
    let base = path.file_name()?.to_str()?;
    if !base.eq_ignore_ascii_case("AGENTS.md") {
        return None;
    }
    let parent = path.parent()?;
    let relative_parent: PathBuf = if parent.is_absolute() || parent.has_root() {
        let cwd = std::env::current_dir().ok()?;
        relativize_to_cwd(parent, &cwd)?
    } else {
        parent.to_path_buf()
    };
    let scope: PathBuf = relative_parent
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .collect();
    if scope.as_os_str().is_empty() {
        None
    } else {
        Some(scope.to_string_lossy().replace('\\', "/"))
    }
}

fn parse_claude(input: &str, filename: Option<&str>) -> Result<Rule> {
    if let Some(f) = filename {
        if f.to_lowercase().ends_with("readme.md") {
            return Err(anyhow::anyhow!("README.md is ignored"));
        }
    }
    // CLAUDE.md generally doesn't use frontmatter natively in the same structured way
    let mut metadata = RuleMetadata::default();
    if let Some(name) = extract_name_from_filename(filename) {
        metadata
            .extra
            .insert("name".to_string(), serde_json::Value::String(name));
    }
    if let Some(desc) = extract_description_from_body(input) {
        metadata.description = Some(desc);
    }
    if let Some(scope) = infer_codex_directory_scope(filename) {
        metadata.extra.insert(
            "rulette:directory-scope".to_string(),
            serde_json::Value::String(scope),
        );
    }
    Ok(Rule {
        metadata,
        body: input.to_string(),
    })
}

fn parse_cursor_mcp(input: &str) -> Result<Vec<Entity>> {
    #[derive(serde::Deserialize)]
    struct CursorMcpFile {
        #[serde(rename = "mcpServers")]
        mcp_servers: HashMap<String, CursorMcpConfig>,
    }

    #[derive(serde::Deserialize)]
    struct CursorMcpConfig {
        command: String,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    }

    let parsed: CursorMcpFile = serde_json::from_str(input)?;
    let mut entities = Vec::new();

    for (name, config) in parsed.mcp_servers {
        entities.push(Entity::McpServer(McpServer {
            metadata: McpServerMetadata {
                name,
                extra: HashMap::new(),
            },
            config: McpServerConfig {
                command: config.command,
                args: config.args,
                env: config.env,
            },
        }));
    }

    Ok(entities)
}

fn extract_frontmatter(input: &str) -> (Option<&str>, &str) {
    if input.starts_with("---\n") || input.starts_with("---\r\n") {
        let start_offset = if input.starts_with("---\r\n") { 5 } else { 4 };
        if let Some(end_idx_rel) = input[start_offset..].find("---") {
            let end_idx = start_offset + end_idx_rel;
            let frontmatter = input[start_offset..end_idx].trim();

            // Closing --- is at end_idx..end_idx+3
            let mut body_start = end_idx + 3;
            if input[body_start..].starts_with('\n') {
                body_start += 1;
            } else if input[body_start..].starts_with("\r\n") {
                body_start += 2;
            }

            return (Some(frontmatter), &input[body_start..]);
        }
    }
    (None, input)
}

fn extract_name_from_filename(filename: Option<&str>) -> Option<String> {
    filename
        .and_then(|f| Path::new(f).file_stem())
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

fn extract_description_from_body(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("---") {
            // Take first non-empty, non-heading line
            // Limit to 100 chars
            let truncated = if trimmed.len() > 100 {
                &trimmed[..100]
            } else {
                trimmed
            };
            return Some(truncated.to_string());
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_claude_settings() {
        let json = r#"{
  "permissions": {
    "disableBypassPermissionsMode": "disable"
  },
  "mcpServers": {
    "test-server": {
      "command": "echo",
      "args": ["hello"],
      "env": {}
    }
  },
  "hooks": {
    "PreToolUse": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "python3 script.py"
          }
        ]
      }
    ]
  },
  "strictKnownMarketplaces": []
}"#;

        let doc = parse(json, InputFormat::Auto, None).unwrap();
        assert_eq!(doc.entities.len(), 3);

        let mut has_mcp = false;
        let mut has_permissions = false;
        let mut has_hooks = false;

        for entity in doc.entities {
            match entity {
                Entity::McpServer(mcp) => {
                    assert_eq!(mcp.metadata.name, "test-server");
                    assert_eq!(mcp.config.command, "echo");
                    assert_eq!(mcp.config.args, vec!["hello"]);
                    has_mcp = true;
                }
                Entity::Permissions(perms) => {
                    assert!(perms.metadata.extra.contains_key("permissions"));
                    assert!(perms.metadata.extra.contains_key("strictKnownMarketplaces"));
                    has_permissions = true;
                }
                Entity::Hook(hook) => {
                    assert_eq!(hook.metadata.name, "PreToolUse");
                    has_hooks = true;
                }
                _ => panic!("Expected McpServer, Hook, or Permissions entity"),
            }
        }

        assert!(has_mcp);
        assert!(has_permissions);
        assert!(has_hooks);
    }

    #[test]
    fn test_parse_claude_settings_strict_fixture() {
        let json = r#"{
  "permissions": {
    "disableBypassPermissionsMode": "disable",
    "ask": [
      "Bash"
    ],
    "deny": [
      "WebSearch",
      "WebFetch"
    ]
  },
  "allowManagedPermissionRulesOnly": true,
  "allowManagedHooksOnly": true,
  "strictKnownMarketplaces": [],
  "sandbox": {
    "autoAllowBashIfSandboxed": false,
    "excludedCommands": [],
    "network": {
      "allowUnixSockets": [],
      "allowAllUnixSockets": false,
      "allowLocalBinding": false,
      "allowedDomains": [],
      "httpProxyPort": null,
      "socksProxyPort": null
    },
    "enableWeakerNestedSandbox": false
  }
}"#;

        let doc = parse(json, InputFormat::Auto, None).unwrap();
        assert_eq!(doc.entities.len(), 1);

        match &doc.entities[0] {
            Entity::Permissions(perms) => {
                assert!(perms.metadata.extra.contains_key("permissions"));
                assert!(perms
                    .metadata
                    .extra
                    .contains_key("allowManagedPermissionRulesOnly"));
                assert!(perms.metadata.extra.contains_key("sandbox"));
            }
            _ => panic!("Expected Permissions entity"),
        }
    }

    #[test]
    fn test_parse_cursor_mcp() {
        let json = r#"{
            "mcpServers": {
                "filesystem": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/project"],
                    "env": {
                        "FOO": "bar"
                    }
                }
            }
        }"#;

        let doc = parse(json, InputFormat::Auto, None).unwrap();
        assert_eq!(doc.entities.len(), 1);

        match &doc.entities[0] {
            Entity::McpServer(mcp) => {
                assert_eq!(mcp.metadata.name, "filesystem");
                assert_eq!(mcp.config.command, "npx");
                assert_eq!(
                    mcp.config.args,
                    vec![
                        "-y",
                        "@modelcontextprotocol/server-filesystem",
                        "/home/user/project"
                    ]
                );
                assert_eq!(mcp.config.env.get("FOO").unwrap(), "bar");
            }
            _ => panic!("Expected McpServer entity"),
        }
    }

    #[test]
    fn test_parse_claude_settings_mcp_and_hooks() {
        let json = r#"{
            "mcpServers": {
                "filesystem": {
                    "command": "npx",
                    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/project"],
                    "env": {}
                }
            },
            "hooks": {
                "PreToolUse": [
                    {
                        "hooks": [
                            {
                                "type": "command",
                                "command": "python3 script.py"
                            }
                        ]
                    }
                ]
            }
        }"#;

        let doc = parse(json, InputFormat::Auto, None).unwrap();
        assert_eq!(doc.entities.len(), 2);

        let mut has_mcp = false;
        let mut has_hooks = false;

        for entity in doc.entities {
            match entity {
                Entity::McpServer(mcp) => {
                    assert_eq!(mcp.metadata.name, "filesystem");
                    has_mcp = true;
                }
                Entity::Hook(hook) => {
                    assert_eq!(hook.metadata.name, "PreToolUse");
                    has_hooks = true;
                }
                _ => panic!("Expected McpServer or Hook entity"),
            }
        }

        assert!(has_mcp);
        assert!(has_hooks);
    }

    #[test]
    fn test_parse_gemini_format() {
        let content = "---\nname: security-auditor\ndescription: test desc\nkind: local\ntools:\n  - grep\nmodel: gemini-pro\n---\n\nYou are a security auditor.";
        let doc = parse(content, InputFormat::Gemini, None).unwrap();
        assert_eq!(doc.entities.len(), 1);

        match &doc.entities[0] {
            Entity::Agent(agent) => {
                assert_eq!(agent.metadata.name, "security-auditor");
                assert_eq!(agent.metadata.description.as_deref(), Some("test desc"));
                assert_eq!(agent.metadata.agent_tools.as_ref().unwrap().len(), 1);
                assert_eq!(agent.metadata.models.as_ref().unwrap()[0], "gemini-pro");
                assert_eq!(
                    agent.metadata.extra.get("kind").unwrap().as_str().unwrap(),
                    "local"
                );
                assert_eq!(agent.body, "You are a security auditor.");
            }
            _ => panic!("Expected Agent entity"),
        }
    }

    #[test]
    fn test_parse_gemini_fallback() {
        let content = "Just a regular rule with no valid subagent frontmatter.";
        let doc = parse(content, InputFormat::Gemini, Some("test_file")).unwrap();
        assert_eq!(doc.entities.len(), 1);

        match &doc.entities[0] {
            Entity::Rule(rule) => {
                assert_eq!(
                    rule.metadata.extra.get("name").unwrap().as_str().unwrap(),
                    "test_file"
                );
                assert_eq!(rule.body, content);
            }
            _ => panic!("Expected Rule entity fallback"),
        }
    }

    #[test]
    fn test_parse_cursor_mdc_always_apply_promotes_to_activation() {
        let content = "---\ndescription: always on\nalwaysApply: true\n---\nBody.";
        let rule = parse_cursor_mdc(content, None).unwrap();
        let activation = rule.metadata.activation.expect("activation should be set");
        let resolved = activation.resolve("cursor-mdc");
        assert_eq!(resolved.mode, vec![crate::ActivationMode::Always]);
        assert_eq!(resolved.globs, None);
        assert!(!rule.metadata.extra.contains_key("alwaysApply"));
    }

    #[test]
    fn test_parse_cursor_mdc_globs_promote_to_activation() {
        let content = "---\ndescription: ts only\nglobs: \"src/**/*.ts,src/**/*.tsx\"\nalwaysApply: false\n---\nBody.";
        let rule = parse_cursor_mdc(content, None).unwrap();
        let activation = rule.metadata.activation.expect("activation should be set");
        let resolved = activation.resolve("cursor-mdc");
        assert_eq!(resolved.mode, vec![crate::ActivationMode::Glob]);
        assert_eq!(
            resolved.globs,
            Some(vec!["src/**/*.ts".to_string(), "src/**/*.tsx".to_string()])
        );
        assert!(!rule.metadata.extra.contains_key("globs"));
        assert!(!rule.metadata.extra.contains_key("alwaysApply"));
    }

    #[test]
    fn test_parse_cursor_mdc_manual_when_no_globs_or_always_apply() {
        let content = "---\ndescription: manual only\nalwaysApply: false\n---\nBody.";
        let rule = parse_cursor_mdc(content, None).unwrap();
        let activation = rule.metadata.activation.expect("activation should be set");
        let resolved = activation.resolve("cursor-mdc");
        assert_eq!(resolved.mode, vec![crate::ActivationMode::Manual]);
    }

    #[test]
    fn test_parse_cursor_mdc_no_activation_fields_leaves_activation_none() {
        let content = "---\ndescription: no activation info\n---\nBody.";
        let rule = parse_cursor_mdc(content, None).unwrap();
        assert!(rule.metadata.activation.is_none());
    }

    #[test]
    fn test_infer_codex_directory_scope_from_nested_agents_md() {
        assert_eq!(
            infer_codex_directory_scope(Some("rules/src/backend/AGENTS.md")),
            Some("rules/src/backend".to_string())
        );
    }

    #[test]
    fn test_infer_codex_directory_scope_none_for_top_level_agents_md() {
        assert_eq!(infer_codex_directory_scope(Some("AGENTS.md")), None);
        assert_eq!(infer_codex_directory_scope(Some("./AGENTS.md")), None);
    }

    #[test]
    fn test_infer_codex_directory_scope_none_for_non_agents_md_filename() {
        assert_eq!(
            infer_codex_directory_scope(Some("src/backend/CLAUDE.md")),
            None
        );
    }

    #[test]
    fn test_infer_codex_directory_scope_none_when_no_filename() {
        assert_eq!(infer_codex_directory_scope(None), None);
    }

    #[test]
    fn test_infer_codex_directory_scope_absolute_outside_cwd_yields_none() {
        // An absolute path unrelated to the current directory (e.g. a temp
        // dir elsewhere on disk) must not produce an absolute scope value --
        // emitters::codex rejects those outright. Falling back to no
        // inferred scope is the safe degradation.
        assert_eq!(
            infer_codex_directory_scope(Some("/definitely/not/cwd/src/backend/AGENTS.md")),
            None
        );
    }

    #[test]
    fn test_infer_codex_directory_scope_absolute_under_cwd_is_relativized() {
        let cwd = std::env::current_dir().unwrap();
        let absolute = cwd.join("src/backend/AGENTS.md");
        assert_eq!(
            infer_codex_directory_scope(Some(absolute.to_str().unwrap())),
            Some("src/backend".to_string())
        );
    }

    #[test]
    fn test_parse_claude_nested_agents_md_sets_directory_scope() {
        let rule = parse_claude("Backend rules.", Some("rules/src/backend/AGENTS.md")).unwrap();
        assert_eq!(
            rule.metadata
                .extra
                .get("rulette:directory-scope")
                .and_then(|v| v.as_str()),
            Some("rules/src/backend")
        );
    }

    #[test]
    fn test_parse_claude_top_level_agents_md_has_no_directory_scope() {
        let rule = parse_claude("Top-level rules.", Some("AGENTS.md")).unwrap();
        assert!(!rule.metadata.extra.contains_key("rulette:directory-scope"));
    }
}
