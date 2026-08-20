use crate::cli::formats::{InputFormat, OutputFormat};
use crate::cli::io::read_inputs;
use crate::emitters::{
    entity_kind_kebab, AgentSkillsEmitter, AntigravityEmitter, ClaudeEmitter, CodexEmitter,
    CopilotEmitter, CursorEmitter, CursorMcpEmitter, Emitter, GeminiEmitter, WindsurfEmitter,
};
use crate::parsers::parse;
use crate::pipeline;
use crate::{Entity, RuletteDocument};
use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap as HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct TransformArgs {
    /// Input files or directories (or "-" for stdin). Defaults to stdin only
    /// when neither this nor --config's `inputs` is set.
    pub input: Vec<String>,

    /// Source format (auto-detected if omitted)
    #[arg(long, value_enum, default_value_t = InputFormat::Auto)]
    pub from: InputFormat,

    /// Target output format
    #[arg(long, value_enum)]
    pub to: Option<OutputFormat>,

    /// Output path (file or directory) or multiple targets via format:path
    #[arg(short, long)]
    pub out: Vec<String>,

    /// Override name metadata for parsed entities
    #[arg(long)]
    pub name: Option<String>,

    /// Override description metadata for parsed entities
    #[arg(long)]
    pub description: Option<String>,

    /// Keep only rules matching expression (e.g., 'license == "MIT"')
    #[arg(long)]
    pub filter: Option<String>,

    /// Remove rules matching expression
    #[arg(long)]
    pub exclude: Option<String>,

    /// Rename a metadata field value (from=to)
    #[arg(long)]
    pub rename: Option<String>,

    /// Set a metadata field (field=value)
    #[arg(long)]
    pub set: Option<String>,

    /// Load a transform-config file (.toml/.json/.jsonc/.json5); composes
    /// with and can be overridden by other CLI flags
    #[arg(long)]
    pub config: Option<String>,

    /// Report drift without writing; exits non-zero if any target would be created or updated
    #[arg(long)]
    pub check: bool,
}

fn default_scope() -> String {
    "project".to_string()
}

const VALID_SCOPES: &[&str] = &["project", "user", "enterprise", "local"];
const VALID_ENTITY_KINDS: &[&str] = &[
    "rule",
    "skill",
    "mcp-server",
    "hook",
    "agent",
    "permissions",
];

/// A declarative transform-config file: a literal transcription of a
/// `transform` invocation (inputs, pipeline, outputs). Never auto-discovered
/// -- only loaded via an explicit `--config <path>`, and never itself
/// produces state.
#[derive(Deserialize, Serialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
struct TransformConfigFile {
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    pipeline: Vec<PipelineStep>,
    #[serde(default)]
    outputs: Vec<OutputEntry>,
}

/// One ordered pipeline step. The default (externally-tagged) serde shape of
/// this enum is `{"filter": "..."}` / `{"exclude": "..."}` / etc. -- exactly
/// the transform-config file's documented `pipeline` shape, with no custom
/// (de)serialization code needed.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
enum PipelineStep {
    Filter(String),
    Exclude(String),
    Rename(String),
    Set(String),
}

/// One `outputs` entry: a single output destination plus its per-output
/// entity-kind allow/deny lists and strictness override.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct OutputEntry {
    target: OutputFormat,
    #[serde(default = "default_scope")]
    scope: String,
    path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entities: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    drop: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    strict: Option<bool>,
}

impl OutputEntry {
    fn validate(&self) -> Result<()> {
        if !VALID_SCOPES.contains(&self.scope.as_str()) {
            anyhow::bail!(
                "Invalid scope '{}' in transform-config output entry; must be one of {:?}",
                self.scope,
                VALID_SCOPES
            );
        }
        for (field_name, tokens) in [("entities", &self.entities), ("drop", &self.drop)] {
            if let Some(tokens) = tokens {
                for token in tokens {
                    if !VALID_ENTITY_KINDS.contains(&token.as_str()) {
                        anyhow::bail!(
                            "Invalid entity kind '{}' in transform-config output '{}'; must be one of {:?}",
                            token,
                            field_name,
                            VALID_ENTITY_KINDS
                        );
                    }
                }
            }
        }
        Ok(())
    }
}

impl TransformConfigFile {
    /// Loads and validates a transform-config file, dispatching on extension:
    /// `.toml` -> TOML, `.json` -> strict JSON, `.jsonc`/`.json5`/anything
    /// else -> JSON5 (a strict superset of "JSON plus comments and trailing
    /// commas"), all against this one schema. A single parse attempt, not a
    /// fallback chain across parsers.
    fn load(path: &str) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        let config: TransformConfigFile = match ext.as_deref() {
            Some("toml") => toml::from_str(&content)?,
            Some("json") => serde_json::from_str(&content)?,
            _ => json5::from_str(&content).map_err(|e| anyhow::anyhow!("{}", e))?,
        };

        for output in &config.outputs {
            output.validate()?;
        }

        Ok(config)
    }
}

pub struct OutputTarget {
    pub format: OutputFormat,
    pub path: Option<String>,
    pub entities: Option<Vec<String>>,
    pub drop: Option<Vec<String>>,
    pub strict: Option<bool>,
}

impl From<&OutputEntry> for OutputTarget {
    fn from(entry: &OutputEntry) -> Self {
        OutputTarget {
            format: entry.target,
            // "-" means stdout, matching the CLI's `-o format:-` convention
            // (`parse_targets` below) -- a config output entry shouldn't
            // need a different sentinel than the flag syntax it composes with.
            path: if entry.path.is_empty() || entry.path == "-" {
                None
            } else {
                Some(entry.path.clone())
            },
            entities: entry.entities.clone(),
            drop: entry.drop.clone(),
            strict: entry.strict,
        }
    }
}

pub fn resolve_output_path(_to: &OutputFormat, out: Option<&String>) -> Option<PathBuf> {
    if let Some(path) = out {
        return Some(PathBuf::from(path));
    }

    None
}

pub fn parse_targets(
    out_args: &[String],
    to_arg: Option<OutputFormat>,
) -> Result<Vec<OutputTarget>> {
    let mut targets = Vec::new();

    if out_args.is_empty() {
        if let Some(format) = to_arg {
            targets.push(OutputTarget {
                format,
                path: None,
                entities: None,
                drop: None,
                strict: None,
            });
            return Ok(targets);
        } else {
            // Default to IrJson to stdout
            targets.push(OutputTarget {
                format: OutputFormat::IrJson,
                path: None,
                entities: None,
                drop: None,
                strict: None,
            });
            return Ok(targets);
        }
    }

    for arg in out_args {
        let parts: Vec<&str> = arg.splitn(2, ':').collect();
        if parts.len() == 2 {
            let format_str = parts[0];
            let path_str = parts[1];

            let format_opt = match format_str {
                "claude" => Some(OutputFormat::Claude),
                "cursor-mdc" => Some(OutputFormat::CursorMdc),
                "cursor-mcp" => Some(OutputFormat::CursorMcp),
                "codex" => Some(OutputFormat::Codex),
                "windsurf" => Some(OutputFormat::Windsurf),
                "copilot" => Some(OutputFormat::Copilot),
                "gemini" => Some(OutputFormat::Gemini),
                "antigravity" => Some(OutputFormat::Antigravity),
                "agent-skills" => Some(OutputFormat::AgentSkills),
                "ir-json" => Some(OutputFormat::IrJson),
                "ir-toml" => Some(OutputFormat::IrToml),
                "transform-config" => Some(OutputFormat::TransformConfig),
                _ => None,
            };

            if let Some(format) = format_opt {
                targets.push(OutputTarget {
                    format,
                    path: if path_str.is_empty() || path_str == "-" {
                        None
                    } else {
                        Some(path_str.to_string())
                    },
                    entities: None,
                    drop: None,
                    strict: None,
                });
                continue;
            }
        }

        if let Some(format) = to_arg {
            targets.push(OutputTarget {
                format,
                path: if arg == "-" {
                    None
                } else {
                    Some(arg.to_string())
                },
                entities: None,
                drop: None,
                strict: None,
            });
        } else {
            // If it's not format:path and no --to, maybe it's just a path for IrJson?
            // The instruction says: "Default output should be IrJson to stdout if no --to or --out is provided."
            // If --out is provided, we should probably follow the old logic or refine it.
            // Old logic bailed if no --to.
            anyhow::bail!(
                "Could not parse {} as format:path and no --to format specified",
                arg
            );
        }
    }

    Ok(targets)
}

/// One path-convention table entry: a matcher, the target it implies, and
/// that target's default scaffold path.
type ToolPathConvention = (fn(&Path) -> bool, OutputFormat, &'static str);

/// Ordered, most-specific-first table mapping an input path's naming
/// convention to the target it scaffolds and that target's default output
/// path. Deliberately covers only the targets this capability currently
/// recognizes (codex, claude, cursor-mdc, cursor-mcp) -- not
/// windsurf/copilot/gemini/agent-skills, deferred per the source design's
/// "Scope of tools".
const TOOL_PATH_CONVENTIONS: &[ToolPathConvention] = &[
    (
        |p| p.file_name().and_then(|f| f.to_str()) == Some("AGENTS.md"),
        OutputFormat::Codex,
        "AGENTS.md",
    ),
    (
        |p| p.file_name().and_then(|f| f.to_str()) == Some("CLAUDE.md"),
        OutputFormat::Claude,
        ".claude/",
    ),
    (
        |p| has_path_component(p, ".claude"),
        OutputFormat::Claude,
        ".claude/",
    ),
    // More specific first: an mcp.json path under .cursor must not fall
    // through to the generic .cursor matcher below it, or every Cursor MCP
    // config would be misclassified as a cursor-mdc rules directory.
    (
        |p| {
            p.file_name().and_then(|f| f.to_str()) == Some("mcp.json")
                && has_path_component(p, ".cursor")
        },
        OutputFormat::CursorMcp,
        ".cursor/mcp.json",
    ),
    (
        |p| has_path_component(p, ".cursor"),
        OutputFormat::CursorMdc,
        ".cursor/rules/",
    ),
    (
        |p| has_path_component(p, ".antigravity"),
        OutputFormat::Antigravity,
        ".antigravity/",
    ),
];

fn has_path_component(path: &Path, name: &str) -> bool {
    path.components().any(|c| c.as_os_str() == name)
}

/// Infers scaffold `outputs` entries from the invocation's own resolved
/// input paths (never from parsed content -- see design.md Decision 6).
/// Returns the inferred outputs, deduplicated by target, plus the list of
/// input paths that matched no known convention.
fn scaffold_outputs(resolved_inputs: &[String]) -> (Vec<OutputEntry>, Vec<String>) {
    let mut outputs: Vec<OutputEntry> = Vec::new();
    let mut unmatched: Vec<String> = Vec::new();

    for input in resolved_inputs {
        let path = Path::new(input);
        match TOOL_PATH_CONVENTIONS
            .iter()
            .find(|(matcher, _, _)| matcher(path))
        {
            Some((_, format, default_path)) => {
                if !outputs.iter().any(|o| o.target == *format) {
                    outputs.push(OutputEntry {
                        target: *format,
                        scope: default_scope(),
                        path: default_path.to_string(),
                        entities: None,
                        drop: None,
                        strict: None,
                    });
                }
            }
            None => unmatched.push(input.clone()),
        }
    }

    (outputs, unmatched)
}

/// Applies a target's `entities` allow-list and `drop` deny-list to a set of
/// entities. A target with both `None` is unfiltered -- unchanged from
/// today's shared-document behavior.
fn entities_for_target(entities: &[Entity], target: &OutputTarget) -> Vec<Entity> {
    if target.entities.is_none() && target.drop.is_none() {
        return entities.to_vec();
    }

    entities
        .iter()
        .filter(|entity| {
            let kind = entity_kind_kebab(entity);
            let allowed = target
                .entities
                .as_ref()
                .map(|allow| allow.iter().any(|a| a == kind))
                .unwrap_or(true);
            let dropped = target
                .drop
                .as_ref()
                .map(|deny| deny.iter().any(|d| d == kind))
                .unwrap_or(false);
            allowed && !dropped
        })
        .cloned()
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteStatus {
    Created,
    Updated,
    Unchanged,
}

enum Written {
    Created(PathBuf),
    Updated {
        path: PathBuf,
        original_content: String,
    },
}

enum PlannedItem {
    Stdout {
        header: Option<String>,
        content: String,
    },
    File {
        path: PathBuf,
        content: String,
        status: WriteStatus,
        original_content: Option<String>,
    },
}

/// What's at a target path before it's touched, checked via `symlink_metadata`
/// so a symlink is classified by the link itself rather than what it points to.
enum ExistingTarget {
    Absent,
    Regular(String),
    Unreadable,
    NonRegular,
}

fn classify_existing_target(path: &Path) -> ExistingTarget {
    match fs::symlink_metadata(path) {
        Err(_) => ExistingTarget::Absent,
        Ok(meta) => {
            if !meta.is_file() {
                ExistingTarget::NonRegular
            } else {
                match fs::read_to_string(path) {
                    Ok(content) => ExistingTarget::Regular(content),
                    Err(_) => ExistingTarget::Unreadable,
                }
            }
        }
    }
}

impl TransformArgs {
    pub fn execute(&self, strict: bool, quiet: bool) -> Result<()> {
        // Config must load before inputs are read/resolved: `resolved_inputs`
        // below needs `config.inputs` to apply the CLI-vs-config precedence
        // rule (design.md Decision 3).
        let config = match &self.config {
            Some(path) => TransformConfigFile::load(path)?,
            None => TransformConfigFile::default(),
        };

        if !config.inputs.is_empty() && !self.input.is_empty() {
            anyhow::bail!(
                "Both --config's `inputs` and positional CLI inputs are set; specify inputs in only one place"
            );
        }
        let resolved_inputs: Vec<String> = if !self.input.is_empty() {
            self.input.clone()
        } else if !config.inputs.is_empty() {
            config.inputs.clone()
        } else {
            vec!["-".to_string()]
        };

        let mut combined_entities = vec![];

        let inputs = read_inputs(&resolved_inputs)?;
        for input_file in inputs {
            let doc = parse(
                &input_file.content,
                self.from,
                input_file.filename.as_deref(),
            )?;
            combined_entities.extend(doc.entities);
        }

        // Apply metadata overrides (from Parse/Convert)
        for entity in &mut combined_entities {
            match entity {
                crate::Entity::Hook(_)
                | crate::Entity::Agent(_)
                | crate::Entity::Permissions(_) => {}
                crate::Entity::Rule(rule) => {
                    if let Some(name) = &self.name {
                        rule.metadata
                            .extra
                            .insert("name".to_string(), serde_json::Value::String(name.clone()));
                    }
                    if let Some(desc) = &self.description {
                        rule.metadata.description = Some(desc.clone());
                    }
                }
                crate::Entity::Skill(skill) => {
                    if let Some(name) = &self.name {
                        skill.metadata.name = name.clone();
                    }
                    if let Some(desc) = &self.description {
                        skill.metadata.description = desc.clone();
                    }
                }
                crate::Entity::McpServer(mcp) => {
                    if let Some(name) = &self.name {
                        mcp.metadata.name = name.clone();
                    }
                }
            }
        }

        // Effective pipeline: config's steps run first, in file order; CLI
        // pipeline flags compose (append) after them, in the fixed order
        // filter, exclude, rename, set (design.md Decision 4). Outputs, by
        // contrast, are all-or-nothing CLI-wins-if-present, resolved below.
        let mut effective_pipeline: Vec<PipelineStep> = config.pipeline.clone();
        if let Some(expr) = &self.filter {
            effective_pipeline.push(PipelineStep::Filter(expr.clone()));
        }
        if let Some(expr) = &self.exclude {
            effective_pipeline.push(PipelineStep::Exclude(expr.clone()));
        }
        if let Some(expr) = &self.rename {
            effective_pipeline.push(PipelineStep::Rename(expr.clone()));
        }
        if let Some(expr) = &self.set {
            effective_pipeline.push(PipelineStep::Set(expr.clone()));
        }

        for step in &effective_pipeline {
            match step {
                PipelineStep::Filter(expr) => {
                    let parsed = pipeline::FilterExpr::parse(expr)?;
                    combined_entities.retain(|entity| parsed.matches(entity));
                }
                PipelineStep::Exclude(expr) => {
                    let parsed = pipeline::FilterExpr::parse(expr)?;
                    combined_entities.retain(|entity| !parsed.matches(entity));
                }
                PipelineStep::Rename(expr) => {
                    let parts: Vec<&str> = expr.split('=').collect();
                    if parts.len() == 2 {
                        let from = parts[0].trim();
                        let to = parts[1].trim();
                        for entity in &mut combined_entities {
                            pipeline::rename_field(entity, from, to);
                        }
                    }
                }
                PipelineStep::Set(expr) => {
                    let parts: Vec<&str> = expr.split('=').collect();
                    if parts.len() == 2 {
                        let key = parts[0].trim();
                        let val = parts[1].trim();
                        for entity in &mut combined_entities {
                            pipeline::set_field(entity, key, val);
                        }
                    }
                }
            }
        }

        // Outputs: any CLI -o/--to replaces the config's `outputs` entirely.
        let run_targets: Vec<OutputTarget> = if !self.out.is_empty() || self.to.is_some() {
            parse_targets(&self.out, self.to)?
        } else if !config.outputs.is_empty() {
            config.outputs.iter().map(OutputTarget::from).collect()
        } else {
            parse_targets(&[], None)?
        };

        // Strict Identity Collision Detection
        {
            let mut seen: HashMap<String, &Entity> = HashMap::new();

            for entity in &combined_entities {
                let name = match entity {
                    Entity::Rule(rule) => rule
                        .metadata
                        .extra
                        .get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    Entity::Skill(skill) => Some(skill.metadata.name.clone()),
                    Entity::McpServer(mcp) => Some(mcp.metadata.name.clone()),
                    Entity::Hook(hook) => Some(hook.metadata.name.clone()),
                    Entity::Agent(agent) => Some(agent.metadata.name.clone()),
                    Entity::Permissions(perms) => perms.metadata.name.clone(),
                };

                let filename = match entity {
                    Entity::Rule(rule) => rule
                        .metadata
                        .extra
                        .get("rulette:source_file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    Entity::Skill(skill) => skill
                        .metadata
                        .extra
                        .get("rulette:source_file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    Entity::McpServer(mcp) => mcp
                        .metadata
                        .extra
                        .get("rulette:source_file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    Entity::Hook(hook) => hook
                        .metadata
                        .extra
                        .get("rulette:source_file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    Entity::Agent(agent) => agent
                        .metadata
                        .extra
                        .get("rulette:source_file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    Entity::Permissions(perms) => perms
                        .metadata
                        .extra
                        .get("rulette:source_file")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                };

                let id = if let (Some(n), Some(f)) = (name, filename) {
                    format!("{}:{}", f, n)
                } else {
                    match entity {
                        Entity::Rule(rule) => rule
                            .metadata
                            .extra
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        Entity::Skill(skill) => Some(skill.metadata.name.clone()),
                        Entity::McpServer(mcp) => Some(mcp.metadata.name.clone()),
                        Entity::Hook(hook) => Some(hook.metadata.name.clone()),
                        Entity::Agent(agent) => Some(agent.metadata.name.clone()),
                        Entity::Permissions(perms) => perms.metadata.name.clone(),
                    }
                    .unwrap_or_else(|| {
                        serde_json::to_string(&entity).unwrap_or_else(|_| "unknown".to_string())
                    })
                };

                if let Some(_existing) = seen.get(&id) {
                    anyhow::bail!("Identity collision detected: entity '{}' already exists. Rulette requires unique identities across all inputs.", id);
                }
                seen.insert(id, entity);
            }
        }

        let doc = RuletteDocument {
            ir_version: "0.1".to_string(),
            entities: combined_entities,
        };

        // Emission logic
        let mut generated_outputs = Vec::new();

        for target in run_targets {
            let effective_strict = target.strict.unwrap_or(strict);
            let target_entities = entities_for_target(&doc.entities, &target);
            let target_doc = RuletteDocument {
                ir_version: doc.ir_version.clone(),
                entities: target_entities,
            };

            let output_map = match target.format {
                OutputFormat::Claude => ClaudeEmitter.emit(&target_doc, effective_strict)?,
                OutputFormat::CursorMdc => CursorEmitter.emit(&target_doc, effective_strict)?,
                OutputFormat::CursorMcp => CursorMcpEmitter.emit(&target_doc, effective_strict)?,
                OutputFormat::AgentSkills => {
                    AgentSkillsEmitter.emit(&target_doc, effective_strict)?
                }
                OutputFormat::Copilot => CopilotEmitter.emit(&target_doc, effective_strict)?,
                OutputFormat::Windsurf => WindsurfEmitter.emit(&target_doc, effective_strict)?,
                OutputFormat::Gemini => GeminiEmitter.emit(&target_doc, effective_strict)?,
                OutputFormat::Antigravity => {
                    AntigravityEmitter.emit(&target_doc, effective_strict)?
                }
                OutputFormat::Codex => CodexEmitter.emit(&target_doc, effective_strict)?,
                OutputFormat::IrJson => {
                    let mut map = HashMap::new();
                    map.insert(
                        PathBuf::from("ir.json"),
                        serde_json::to_string_pretty(&target_doc)?,
                    );
                    map
                }
                OutputFormat::IrToml => {
                    let mut map = HashMap::new();
                    map.insert(PathBuf::from("ir.toml"), toml::to_string(&target_doc)?);
                    map
                }
                OutputFormat::JsonSchema => {
                    let mut map = HashMap::new();
                    let schema = schemars::schema_for!(crate::RuletteDocument);
                    map.insert(
                        PathBuf::from("schema.json"),
                        serde_json::to_string_pretty(&schema)?,
                    );
                    map
                }
                OutputFormat::TransformConfig => {
                    let (outputs, unmatched) = scaffold_outputs(&resolved_inputs);
                    for path in &unmatched {
                        eprintln!(
                            "Warning: no known tool convention matched input path '{}'; it is included in the generated inputs but contributes no output entry",
                            path
                        );
                    }
                    let manifest = TransformConfigFile {
                        inputs: resolved_inputs.clone(),
                        pipeline: Vec::new(),
                        outputs,
                    };
                    let out_ext = target
                        .path
                        .as_deref()
                        .and_then(|p| Path::new(p).extension())
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_lowercase());
                    let content = if out_ext.as_deref() == Some("toml") {
                        toml::to_string(&manifest)?
                    } else {
                        serde_json::to_string_pretty(&manifest)?
                    };
                    let mut map = HashMap::new();
                    map.insert(PathBuf::from("rulette.transform.jsonc"), content);
                    map
                }
            };
            generated_outputs.push((target, output_map));
        }

        let check = self.check;

        // Flatten every target's rendered output into an ordered plan, classifying
        // each file target's existing content against disk before any writes happen.
        // This keeps render / compare / write as distinct phases: a target whose
        // existing content can't be read or isn't a regular file aborts here, before
        // any target in the invocation is written.
        let mut items: Vec<PlannedItem> = Vec::new();

        for (target, output_map) in &generated_outputs {
            let base_path = resolve_output_path(&target.format, target.path.as_ref());

            // A multi-file emitter (e.g. Codex, grouping directory-scoped
            // entities into several nested `AGENTS.md` files) is normally
            // anchored by treating an existing/implied `base` directory as
            // the container for every entry. But when `base` itself names
            // one of the emitted files directly -- e.g. `-o codex:AGENTS.md`
            // with no pre-existing `AGENTS.md` directory -- anchoring there
            // would nest the root file under a directory named after itself
            // (`AGENTS.md/AGENTS.md`). Detect that case and anchor at
            // `base`'s parent instead, so the root file lands at `base` and
            // sibling nested files resolve alongside it.
            let base_path = base_path.map(|base| {
                if !base.is_dir() && output_map.len() > 1 {
                    if let Some(file_name) = base.file_name() {
                        let base_is_a_root_entry = output_map.keys().any(|k| {
                            k.as_os_str() == file_name
                                && k.parent().map(|p| p.as_os_str().is_empty()).unwrap_or(true)
                        });
                        if base_is_a_root_entry {
                            return base
                                .parent()
                                .map(|p| p.to_path_buf())
                                .unwrap_or_else(|| PathBuf::from(""));
                        }
                    }
                }
                base
            });

            let mut sorted_paths: Vec<_> = output_map.keys().collect();
            sorted_paths.sort();

            for rel_path in sorted_paths {
                let content = output_map[rel_path].clone();

                let Some(ref base) = base_path else {
                    let header = if output_map.len() > 1 {
                        Some(format!("--- {} ---", rel_path.display()))
                    } else {
                        None
                    };
                    items.push(PlannedItem::Stdout { header, content });
                    continue;
                };

                let mut path = base.clone();
                if path.is_dir() || path.extension().is_none() || output_map.len() > 1 {
                    path.push(rel_path);
                }

                let (status, original_content) = match classify_existing_target(&path) {
                    ExistingTarget::Absent => (WriteStatus::Created, None),
                    ExistingTarget::Regular(existing) => {
                        if existing == content {
                            (WriteStatus::Unchanged, None)
                        } else {
                            (WriteStatus::Updated, Some(existing))
                        }
                    }
                    ExistingTarget::Unreadable => {
                        anyhow::bail!(
                            "Cannot read existing target {} to compare content; aborting before any writes",
                            path.display()
                        );
                    }
                    ExistingTarget::NonRegular => {
                        anyhow::bail!(
                            "Existing target {} is not a regular file (a symlink or a directory); aborting before any writes",
                            path.display()
                        );
                    }
                };

                items.push(PlannedItem::File {
                    path,
                    content,
                    status,
                    original_content,
                });
            }
        }

        if check
            && !items
                .iter()
                .any(|item| matches!(item, PlannedItem::File { .. }))
        {
            anyhow::bail!(
                "--check requires at least one output file target (-o); no target resolves to a file path"
            );
        }

        // Track files written this run so a mid-run failure can be rolled back,
        // keeping multi-target emission all-or-nothing on disk. Created paths are
        // removed on rollback; Updated paths are restored to their pre-write content.
        let mut written: Vec<Written> = Vec::new();
        let mut write_error: Option<anyhow::Error> = None;

        for item in &items {
            match item {
                PlannedItem::Stdout { header, content } => {
                    if !check {
                        if let Some(header) = header {
                            println!("{}", header);
                        }
                        println!("{}", content);
                    }
                }
                PlannedItem::File {
                    path,
                    content,
                    status,
                    original_content,
                } => {
                    match status {
                        WriteStatus::Unchanged => {}
                        WriteStatus::Created | WriteStatus::Updated => {
                            if !check {
                                if let Some(parent) = path.parent() {
                                    if let Err(e) = fs::create_dir_all(parent) {
                                        write_error = Some(e.into());
                                        break;
                                    }
                                }
                                if let Err(e) = fs::write(path, content) {
                                    write_error = Some(e.into());
                                    break;
                                }
                                written.push(if matches!(status, WriteStatus::Created) {
                                    Written::Created(path.clone())
                                } else {
                                    Written::Updated {
                                        path: path.clone(),
                                        original_content: original_content
                                            .clone()
                                            .unwrap_or_default(),
                                    }
                                });
                            }
                        }
                    }

                    if !quiet {
                        let label = match status {
                            WriteStatus::Created => "Created",
                            WriteStatus::Updated => "Updated",
                            WriteStatus::Unchanged => "Unchanged",
                        };
                        println!("{} {}", label, path.display());
                    }
                }
            }
        }

        if let Some(e) = write_error {
            for w in written.iter().rev() {
                match w {
                    Written::Created(path) => {
                        let _ = fs::remove_file(path);
                    }
                    Written::Updated {
                        path,
                        original_content,
                    } => {
                        let _ = fs::write(path, original_content);
                    }
                }
            }
            return Err(e);
        }

        if check {
            let has_drift = items.iter().any(|item| {
                matches!(
                    item,
                    PlannedItem::File {
                        status: WriteStatus::Created | WriteStatus::Updated,
                        ..
                    }
                )
            });
            if has_drift {
                anyhow::bail!("Drift detected: one or more targets would be created or updated");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn write_temp(suffix: &str, content: &str) -> tempfile::TempPath {
        let mut f = tempfile::Builder::new().suffix(suffix).tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.into_temp_path()
    }

    const EQUIVALENT_CONFIG_JSON: &str = r#"{
        "inputs": ["./rules/"],
        "pipeline": [{"filter": "status == \"stable\""}],
        "outputs": [{"target": "claude", "path": ".claude/"}]
    }"#;

    #[test]
    fn test_load_toml_json_jsonc_json5_equivalent() {
        let toml_content = r#"
inputs = ["./rules/"]
[[pipeline]]
filter = "status == \"stable\""
[[outputs]]
target = "claude"
path = ".claude/"
"#;
        let jsonc_content = r#"{
            // a comment
            "inputs": ["./rules/"],
            "pipeline": [{"filter": "status == \"stable\""}],
            "outputs": [{"target": "claude", "path": ".claude/"}],
        }"#;
        let json5_content = r#"{
            inputs: ["./rules/"],
            pipeline: [{filter: "status == \"stable\""}],
            outputs: [{target: "claude", path: ".claude/"}],
        }"#;

        let toml_path = write_temp(".toml", toml_content);
        let json_path = write_temp(".json", EQUIVALENT_CONFIG_JSON);
        let jsonc_path = write_temp(".jsonc", jsonc_content);
        let json5_path = write_temp(".json5", json5_content);

        let toml_cfg = TransformConfigFile::load(toml_path.to_str().unwrap()).unwrap();
        let json_cfg = TransformConfigFile::load(json_path.to_str().unwrap()).unwrap();
        let jsonc_cfg = TransformConfigFile::load(jsonc_path.to_str().unwrap()).unwrap();
        let json5_cfg = TransformConfigFile::load(json5_path.to_str().unwrap()).unwrap();

        for cfg in [&toml_cfg, &json_cfg, &jsonc_cfg, &json5_cfg] {
            assert_eq!(cfg.inputs, vec!["./rules/".to_string()]);
            assert_eq!(cfg.outputs.len(), 1);
            assert_eq!(cfg.outputs[0].target, OutputFormat::Claude);
            assert_eq!(cfg.outputs[0].path, ".claude/");
        }
    }

    #[test]
    fn test_load_strict_json_rejects_comments() {
        let content = r#"{
            // not allowed in strict json
            "inputs": []
        }"#;
        let path = write_temp(".json", content);
        assert!(TransformConfigFile::load(path.to_str().unwrap()).is_err());
    }

    #[test]
    fn test_load_unrecognized_extension_falls_back_to_json5() {
        let path = write_temp(".cfg", EQUIVALENT_CONFIG_JSON);
        let cfg = TransformConfigFile::load(path.to_str().unwrap()).unwrap();
        assert_eq!(cfg.inputs, vec!["./rules/".to_string()]);
    }

    #[test]
    fn test_load_unrecognized_extension_surfaces_json5_error() {
        let path = write_temp(".cfg", "not valid json5 {{{");
        let err = TransformConfigFile::load(path.to_str().unwrap()).unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_load_old_flat_shape_fails_with_unknown_field() {
        let content = r#"{"filter": "status == \"stable\""}"#;
        let path = write_temp(".jsonc", content);
        let err = TransformConfigFile::load(path.to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("unknown field"),
            "expected an unknown-field error, got: {}",
            err
        );
    }

    #[test]
    fn test_output_entry_rejects_unknown_scope() {
        let entry = OutputEntry {
            target: OutputFormat::Claude,
            scope: "team".to_string(),
            path: ".claude/".to_string(),
            entities: None,
            drop: None,
            strict: None,
        };
        assert!(entry.validate().is_err());
    }

    #[test]
    fn test_output_entry_rejects_unknown_entity_kind() {
        let entry = OutputEntry {
            target: OutputFormat::Claude,
            scope: default_scope(),
            path: ".claude/".to_string(),
            entities: Some(vec!["rules".to_string()]),
            drop: None,
            strict: None,
        };
        assert!(entry.validate().is_err());

        let entry_drop = OutputEntry {
            drop: Some(vec!["hooks".to_string()]),
            ..entry
        };
        assert!(entry_drop.validate().is_err());
    }

    #[test]
    fn test_output_entry_accepts_valid_scope_and_entity_kinds() {
        let entry = OutputEntry {
            target: OutputFormat::Claude,
            scope: "user".to_string(),
            path: ".claude/".to_string(),
            entities: Some(vec!["rule".to_string(), "skill".to_string()]),
            drop: Some(vec!["hook".to_string()]),
            strict: Some(true),
        };
        assert!(entry.validate().is_ok());
    }

    #[test]
    fn test_output_entry_dash_path_becomes_stdout_target() {
        let entry = OutputEntry {
            target: OutputFormat::IrJson,
            scope: default_scope(),
            path: "-".to_string(),
            entities: None,
            drop: None,
            strict: None,
        };
        let target = OutputTarget::from(&entry);
        assert!(
            target.path.is_none(),
            "a config output entry's path of \"-\" must mean stdout, matching -o format:-"
        );
    }

    #[test]
    fn test_scaffold_outputs_known_layout() {
        let inputs = vec![
            ".claude/".to_string(),
            ".cursor/".to_string(),
            "AGENTS.md".to_string(),
        ];
        let (outputs, unmatched) = scaffold_outputs(&inputs);
        assert!(unmatched.is_empty());
        let targets: Vec<OutputFormat> = outputs.iter().map(|o| o.target).collect();
        assert!(targets.contains(&OutputFormat::Claude));
        assert!(targets.contains(&OutputFormat::CursorMdc));
        assert!(targets.contains(&OutputFormat::Codex));
    }

    #[test]
    fn test_scaffold_outputs_distinguishes_cursor_mdc_and_mcp() {
        let inputs = vec![
            ".cursor/rules/typescript.mdc".to_string(),
            ".cursor/mcp.json".to_string(),
        ];
        let (outputs, unmatched) = scaffold_outputs(&inputs);
        assert!(unmatched.is_empty());
        let targets: Vec<OutputFormat> = outputs.iter().map(|o| o.target).collect();
        assert!(targets.contains(&OutputFormat::CursorMdc));
        assert!(targets.contains(&OutputFormat::CursorMcp));
        assert_eq!(targets.len(), 2);
    }

    #[test]
    fn test_scaffold_outputs_bare_claude_md() {
        let inputs = vec!["CLAUDE.md".to_string()];
        let (outputs, unmatched) = scaffold_outputs(&inputs);
        assert!(unmatched.is_empty());
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].target, OutputFormat::Claude);
    }

    #[test]
    fn test_scaffold_outputs_dedups_nested_codex_files() {
        let inputs = vec!["AGENTS.md".to_string(), "src/backend/AGENTS.md".to_string()];
        let (outputs, unmatched) = scaffold_outputs(&inputs);
        assert!(unmatched.is_empty());
        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].target, OutputFormat::Codex);
    }

    #[test]
    fn test_scaffold_outputs_unmatched_path_is_preserved_but_not_output() {
        let inputs = vec![".".to_string()];
        let (outputs, unmatched) = scaffold_outputs(&inputs);
        assert!(outputs.is_empty());
        assert_eq!(unmatched, vec![".".to_string()]);
    }

    #[test]
    fn test_entities_for_target_allow_list() {
        let entities = sample_entities();
        let target = OutputTarget {
            format: OutputFormat::Claude,
            path: None,
            entities: Some(vec!["rule".to_string()]),
            drop: None,
            strict: None,
        };
        let filtered = entities_for_target(&entities, &target);
        assert_eq!(filtered.len(), 1);
        assert!(matches!(filtered[0], Entity::Rule(_)));
    }

    #[test]
    fn test_entities_for_target_deny_list() {
        let entities = sample_entities();
        let target = OutputTarget {
            format: OutputFormat::Claude,
            path: None,
            entities: None,
            drop: Some(vec!["skill".to_string()]),
            strict: None,
        };
        let filtered = entities_for_target(&entities, &target);
        assert_eq!(filtered.len(), 1);
        assert!(matches!(filtered[0], Entity::Rule(_)));
    }

    #[test]
    fn test_entities_for_target_unfiltered_when_unset() {
        let entities = sample_entities();
        let target = OutputTarget {
            format: OutputFormat::Claude,
            path: None,
            entities: None,
            drop: None,
            strict: None,
        };
        let filtered = entities_for_target(&entities, &target);
        assert_eq!(filtered.len(), entities.len());
    }

    fn sample_entities() -> Vec<Entity> {
        vec![
            Entity::Rule(crate::Rule {
                metadata: crate::RuleMetadata::default(),
                body: "rule body".to_string(),
            }),
            Entity::Skill(crate::agent_skills::Skill {
                metadata: crate::agent_skills::SkillMetadata {
                    name: "test-skill".to_string(),
                    description: "desc".to_string(),
                    version: None,
                    license: None,
                    compatibility: None,
                    metadata: HashMap::new(),
                    allowed_tools: None,
                    extra: HashMap::new(),
                },
                body: "skill body".to_string(),
            }),
        ]
    }
}
