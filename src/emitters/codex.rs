use super::{CapabilityEntry, CoverageStatus, Emitter};
use crate::{Entity, RuletteDocument};
use anyhow::{anyhow, Result};
use std::collections::BTreeMap as HashMap;
use std::path::{Component, Path, PathBuf};

pub struct CodexEmitter;

const DIRECTORY_SCOPE_KEY: &str = "rulette:directory-scope";

/// Reads `rulette:directory-scope` from an entity's extra map. Returns the
/// scope as a relative path with no `..` traversal, or None if unscoped.
fn directory_scope(extra: &HashMap<String, serde_json::Value>) -> Result<Option<String>> {
    let Some(value) = extra.get(DIRECTORY_SCOPE_KEY) else {
        return Ok(None);
    };
    let scope = value
        .as_str()
        .ok_or_else(|| anyhow!("{} must be a string", DIRECTORY_SCOPE_KEY))?;
    let path = Path::new(scope);
    if path.is_absolute() || path.components().any(|c| c == Component::ParentDir) {
        anyhow::bail!(
            "Invalid {} '{}': must be a relative path with no '..' segments",
            DIRECTORY_SCOPE_KEY,
            scope
        );
    }
    Ok(Some(scope.to_string()))
}

fn agents_md_path(scope: &Option<String>) -> PathBuf {
    match scope {
        Some(scope) => Path::new(scope).join("AGENTS.md"),
        None => PathBuf::from("AGENTS.md"),
    }
}

impl Emitter for CodexEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        tracing::debug!(
            "Emitting document with {} entities (strict={})",
            doc.entities.len(),
            strict
        );
        let mut grouped: HashMap<Option<String>, String> = HashMap::new();
        for entity in &doc.entities {
            match entity {
                crate::Entity::Hook(hook) => {
                    if strict {
                        return Err(anyhow!("Lossy conversion: Hook to Codex drops metadata"));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Hook '{}' to Codex drops metadata",
                            hook.metadata.name
                        );
                    }
                }
                crate::Entity::Agent(agent) => {
                    if strict {
                        return Err(anyhow!("Lossy conversion: Agent to Codex drops metadata"));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Agent '{}' to Codex drops metadata",
                            agent.metadata.name
                        );
                    }
                }
                crate::Entity::Permissions(perms) => {
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: Permissions to Codex drops metadata"
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Permissions '{}' to Codex drops metadata",
                            perms.metadata.name.as_deref().unwrap_or("(unnamed)")
                        );
                    }
                }
                Entity::Rule(rule) => {
                    let scope = directory_scope(&rule.metadata.extra)?;
                    let output = grouped.entry(scope).or_default();
                    output.push_str(&rule.body);
                    output.push_str("\n\n");
                }
                Entity::McpServer(mcp) => {
                    if strict {
                        return Err(anyhow::anyhow!(
                            "Lossy conversion: McpServer to target format drops metadata"
                        ));
                    } else {
                        eprintln!("Warning: Lossy conversion: McpServer '{}' to target format drops metadata", mcp.metadata.name);
                    }
                }
                Entity::Skill(skill) => {
                    if strict {
                        return Err(anyhow!("Lossy conversion: Skill to Codex drops metadata"));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: Skill '{}' to Codex drops metadata",
                            skill.metadata.name
                        );
                    }
                    let scope = directory_scope(&skill.metadata.extra)?;
                    let output = grouped.entry(scope).or_default();
                    output.push_str(&skill.body);
                    output.push_str("\n\n");
                }
            }
        }
        let mut map = HashMap::new();
        for (scope, output) in grouped {
            if !output.is_empty() {
                map.insert(agents_md_path(&scope), output.trim_end().to_string());
            }
        }
        Ok(map)
    }

    fn capabilities(&self, doc: &RuletteDocument) -> Vec<CapabilityEntry> {
        // directory_scope() validates rulette:directory-scope and can fail
        // for a malformed value (absolute path, ".." traversal). emit()
        // propagates that as a hard error via `?` regardless of `strict`;
        // capabilities() has no error channel, so an invalid scope is
        // reported as Dropped for that entity instead of aborting the whole
        // coverage probe -- it plainly isn't representable at this target.
        let raw: Vec<CapabilityEntry> = doc
            .entities
            .iter()
            .map(|entity| match entity {
                Entity::Rule(rule) => match directory_scope(&rule.metadata.extra) {
                    Ok(_) => CapabilityEntry::supported(entity),
                    Err(e) => CapabilityEntry::lossy_or_dropped(
                        entity,
                        CoverageStatus::Dropped,
                        e.to_string(),
                    ),
                },
                Entity::Skill(skill) => match directory_scope(&skill.metadata.extra) {
                    Ok(_) => CapabilityEntry::lossy_or_dropped(
                        entity,
                        CoverageStatus::Lossy,
                        format!(
                            "Lossy conversion: Skill '{}' to Codex drops metadata",
                            skill.metadata.name
                        ),
                    ),
                    Err(e) => CapabilityEntry::lossy_or_dropped(
                        entity,
                        CoverageStatus::Dropped,
                        e.to_string(),
                    ),
                },
                Entity::Hook(hook) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Hook '{}' to Codex drops metadata",
                        hook.metadata.name
                    ),
                ),
                Entity::Agent(agent) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Agent '{}' to Codex drops metadata",
                        agent.metadata.name
                    ),
                ),
                Entity::Permissions(perms) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: Permissions '{}' to Codex drops metadata",
                        perms.metadata.name.as_deref().unwrap_or("(unnamed)")
                    ),
                ),
                Entity::McpServer(mcp) => CapabilityEntry::lossy_or_dropped(
                    entity,
                    CoverageStatus::Dropped,
                    format!(
                        "Lossy conversion: McpServer '{}' to target format drops metadata",
                        mcp.metadata.name
                    ),
                ),
            })
            .collect();
        super::aggregate_capabilities(raw)
    }
}
