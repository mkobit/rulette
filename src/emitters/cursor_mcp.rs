use super::Emitter;
use crate::{Entity, RuletteDocument};
use anyhow::{anyhow, Result};
use std::collections::BTreeMap as HashMap;
use std::path::PathBuf;

pub struct CursorMcpEmitter;

#[derive(serde::Serialize)]
struct CursorMcpServerConfig<'a> {
    command: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    args: &'a Vec<String>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    env: &'a HashMap<String, String>,
}

#[derive(serde::Serialize)]
struct CursorMcpFile<'a> {
    #[serde(rename = "mcpServers")]
    mcp_servers: HashMap<&'a str, CursorMcpServerConfig<'a>>,
}

/// Owned, schema-only mirror of [`CursorMcpFile`] for `rulette schema --to cursor-mcp`.
/// The emitter itself uses borrowed fields for zero-copy serialization, which
/// doesn't play well with schema generation, so this describes the same shape.
#[derive(serde::Serialize, schemars::JsonSchema)]
pub struct CursorMcpFileSchema {
    #[serde(rename = "mcpServers")]
    pub mcp_servers: HashMap<String, CursorMcpServerSchema>,
}

#[derive(serde::Serialize, schemars::JsonSchema)]
pub struct CursorMcpServerSchema {
    pub command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

impl Emitter for CursorMcpEmitter {
    fn emit(&self, doc: &RuletteDocument, strict: bool) -> Result<HashMap<PathBuf, String>> {
        tracing::debug!(
            "Emitting document with {} entities (strict={})",
            doc.entities.len(),
            strict
        );
        let mut mcp_servers = HashMap::new();

        for entity in &doc.entities {
            match entity {
                Entity::McpServer(mcp) => {
                    if !mcp.metadata.extra.is_empty() {
                        if strict {
                            return Err(anyhow!(
                                "Lossy conversion: McpServer '{}' extra metadata to Cursor MCP drops fields",
                                mcp.metadata.name
                            ));
                        } else {
                            eprintln!(
                                "Warning: Lossy conversion: McpServer '{}' extra metadata to Cursor MCP drops fields",
                                mcp.metadata.name
                            );
                        }
                    }
                    mcp_servers.insert(
                        mcp.metadata.name.as_str(),
                        CursorMcpServerConfig {
                            command: &mcp.config.command,
                            args: &mcp.config.args,
                            env: &mcp.config.env,
                        },
                    );
                }
                Entity::Rule(_)
                | Entity::Skill(_)
                | Entity::Hook(_)
                | Entity::Agent(_)
                | Entity::Permissions(_) => {
                    let kind = entity_kind_label(entity);
                    if strict {
                        return Err(anyhow!(
                            "Lossy conversion: {} to Cursor MCP drops metadata",
                            kind
                        ));
                    } else {
                        eprintln!(
                            "Warning: Lossy conversion: {} to Cursor MCP drops metadata",
                            kind
                        );
                    }
                }
            }
        }

        let mut map = HashMap::new();
        if !mcp_servers.is_empty() {
            let file = CursorMcpFile { mcp_servers };
            map.insert(
                PathBuf::from("mcp.json"),
                serde_json::to_string_pretty(&file)?,
            );
        }
        Ok(map)
    }
}

fn entity_kind_label(entity: &Entity) -> &'static str {
    match entity {
        Entity::Rule(_) => "Rule",
        Entity::Skill(_) => "Skill",
        Entity::McpServer(_) => "McpServer",
        Entity::Hook(_) => "Hook",
        Entity::Agent(_) => "Agent",
        Entity::Permissions(_) => "Permissions",
    }
}
