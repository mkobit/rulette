use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PLUGIN_MANIFEST_SCHEMA_V1: &str =
    "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json";
pub const MCP_CONFIG_SCHEMA_V1: &str = "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json";

fn validate_manifest_schema<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s != PLUGIN_MANIFEST_SCHEMA_V1 {
        return Err(serde::de::Error::custom(format!(
            "invalid $schema: expected `{PLUGIN_MANIFEST_SCHEMA_V1}`, found `{s}`"
        )));
    }
    Ok(s)
}

fn validate_mcp_config_schema<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s != MCP_CONFIG_SCHEMA_V1 {
        return Err(serde::de::Error::custom(format!(
            "invalid $schema: expected `{MCP_CONFIG_SCHEMA_V1}`, found `{s}`"
        )));
    }
    Ok(s)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct PluginAuthorWireV1 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifestWireV1 {
    #[serde(rename = "$schema", deserialize_with = "validate_manifest_schema")]
    pub schema: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<PluginAuthorWireV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<BTreeMap<String, serde_json::Value>>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl PluginManifestWireV1 {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            schema: PLUGIN_MANIFEST_SCHEMA_V1.to_string(),
            name: name.into(),
            version: None,
            description: None,
            author: None,
            homepage: None,
            repository: None,
            license: None,
            keywords: None,
            extensions: None,
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpServerWireV1 {
    #[serde(rename = "stdio")]
    Stdio {
        command: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        env: BTreeMap<String, String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cwd: Option<String>,
    },
    #[serde(rename = "streamable-http")]
    StreamableHttp {
        url: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        headers: BTreeMap<String, String>,
    },
    #[serde(rename = "sse")]
    Sse {
        url: String,
        #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
        headers: BTreeMap<String, String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpConfigWireV1 {
    #[serde(rename = "$schema", deserialize_with = "validate_mcp_config_schema")]
    pub schema: String,
    #[serde(rename = "mcpServers", default)]
    pub mcp_servers: BTreeMap<String, McpServerWireV1>,
}

impl McpConfigWireV1 {
    pub fn new(mcp_servers: BTreeMap<String, McpServerWireV1>) -> Self {
        Self {
            schema: MCP_CONFIG_SCHEMA_V1.to_string(),
            mcp_servers,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SkillFrontmatterWire {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra: BTreeMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_manifest_wire() {
        let json = serde_json::json!({
            "$schema": PLUGIN_MANIFEST_SCHEMA_V1,
            "name": "minimal-plugin"
        });

        let wire: PluginManifestWireV1 = serde_json::from_value(json).unwrap();
        assert_eq!(wire.schema, PLUGIN_MANIFEST_SCHEMA_V1);
        assert_eq!(wire.name, "minimal-plugin");
        assert!(wire.version.is_none());
        assert!(wire.description.is_none());
        assert!(wire.author.is_none());
        assert!(wire.homepage.is_none());
        assert!(wire.repository.is_none());
        assert!(wire.license.is_none());
        assert!(wire.keywords.is_none());
        assert!(wire.extensions.is_none());
        assert!(wire.extra.is_empty());
    }

    #[test]
    fn parses_manifest_with_all_wire_fields_and_extensions() {
        let json = serde_json::json!({
            "$schema": PLUGIN_MANIFEST_SCHEMA_V1,
            "name": "full-plugin",
            "version": "1.2.3",
            "description": "Comprehensive plugin test",
            "author": {
                "name": "Test Author",
                "email": "author@example.com",
                "url": "https://example.com/author"
            },
            "homepage": "https://example.com",
            "repository": "https://github.com/example/full-plugin",
            "license": "MIT",
            "keywords": ["ai", "assistant"],
            "extensions": {
                "com.vendor.client": {
                    "setting_a": true,
                    "count": 10
                }
            }
        });

        let wire: PluginManifestWireV1 = serde_json::from_value(json).unwrap();
        assert_eq!(wire.name, "full-plugin");
        assert_eq!(wire.version.as_deref(), Some("1.2.3"));
        assert_eq!(
            wire.description.as_deref(),
            Some("Comprehensive plugin test")
        );
        let author = wire.author.unwrap();
        assert_eq!(author.name.as_deref(), Some("Test Author"));
        assert_eq!(author.email.as_deref(), Some("author@example.com"));
        assert_eq!(author.url.as_deref(), Some("https://example.com/author"));
        assert_eq!(wire.homepage.as_deref(), Some("https://example.com"));
        assert_eq!(
            wire.repository.as_deref(),
            Some("https://github.com/example/full-plugin")
        );
        assert_eq!(wire.license.as_deref(), Some("MIT"));
        assert_eq!(
            wire.keywords.as_deref(),
            Some(&["ai".to_string(), "assistant".to_string()][..])
        );
        let ext = wire.extensions.unwrap();
        assert!(ext.contains_key("com.vendor.client"));
        assert!(wire.extra.is_empty());
    }

    #[test]
    fn captures_unknown_fields_in_manifest_extra() {
        let json = serde_json::json!({
            "$schema": PLUGIN_MANIFEST_SCHEMA_V1,
            "name": "extra-plugin",
            "custom_field": "custom_value",
            "nested_extra": {
                "flag": true
            }
        });

        let wire: PluginManifestWireV1 = serde_json::from_value(json).unwrap();
        assert_eq!(wire.name, "extra-plugin");
        assert_eq!(wire.extra.len(), 2);
        assert_eq!(
            wire.extra.get("custom_field"),
            Some(&serde_json::json!("custom_value"))
        );
        assert_eq!(
            wire.extra.get("nested_extra"),
            Some(&serde_json::json!({ "flag": true }))
        );
    }

    #[test]
    fn rejects_invalid_or_missing_manifest_schema() {
        let invalid_schema = serde_json::json!({
            "$schema": "https://example.com/invalid/schema.json",
            "name": "bad-schema"
        });
        assert!(serde_json::from_value::<PluginManifestWireV1>(invalid_schema).is_err());

        let missing_schema = serde_json::json!({
            "name": "no-schema"
        });
        assert!(serde_json::from_value::<PluginManifestWireV1>(missing_schema).is_err());
    }

    #[test]
    fn rejects_missing_manifest_name() {
        let missing_name = serde_json::json!({
            "$schema": PLUGIN_MANIFEST_SCHEMA_V1
        });
        assert!(serde_json::from_value::<PluginManifestWireV1>(missing_name).is_err());
    }

    #[test]
    fn author_rejects_unknown_fields() {
        let json = serde_json::json!({
            "name": "Valid",
            "extra_author_field": "disallowed"
        });
        assert!(serde_json::from_value::<PluginAuthorWireV1>(json).is_err());
    }

    #[test]
    fn parses_and_serializes_mcp_config_with_all_transports() {
        let mut servers = BTreeMap::new();
        servers.insert(
            "stdio-srv".to_string(),
            McpServerWireV1::Stdio {
                command: "./bin/srv".to_string(),
                args: vec!["--flag".to_string()],
                env: BTreeMap::from([("ENV_VAR".to_string(), "val".to_string())]),
                cwd: Some("./".to_string()),
            },
        );
        servers.insert(
            "http-srv".to_string(),
            McpServerWireV1::StreamableHttp {
                url: "https://example.com/mcp".to_string(),
                headers: BTreeMap::from([(
                    "Authorization".to_string(),
                    "Bearer token".to_string(),
                )]),
            },
        );
        servers.insert(
            "sse-srv".to_string(),
            McpServerWireV1::Sse {
                url: "https://example.com/sse".to_string(),
                headers: BTreeMap::new(),
            },
        );

        let config = McpConfigWireV1::new(servers);
        let serialized = serde_json::to_string(&config).unwrap();
        let parsed: McpConfigWireV1 = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn mcp_config_rejects_invalid_or_missing_schema() {
        let invalid = serde_json::json!({
            "$schema": "https://example.com/invalid/mcp.json",
            "mcpServers": {}
        });
        assert!(serde_json::from_value::<McpConfigWireV1>(invalid).is_err());

        let missing = serde_json::json!({
            "mcpServers": {}
        });
        assert!(serde_json::from_value::<McpConfigWireV1>(missing).is_err());
    }

    #[test]
    fn mcp_config_rejects_unknown_fields() {
        let json = serde_json::json!({
            "$schema": MCP_CONFIG_SCHEMA_V1,
            "mcpServers": {},
            "unexpected": "disallowed"
        });
        assert!(serde_json::from_value::<McpConfigWireV1>(json).is_err());
    }

    #[test]
    fn parses_skill_frontmatter_wire() {
        let yaml = "name: skill-name\ndescription: A useful skill.\ncustom_meta: 123\n";
        let parsed: SkillFrontmatterWire = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(parsed.name.as_deref(), Some("skill-name"));
        assert_eq!(parsed.description.as_deref(), Some("A useful skill."));
        assert_eq!(
            parsed.extra.get("custom_meta"),
            Some(&serde_json::json!(123))
        );
    }
}
