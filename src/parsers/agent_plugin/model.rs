use super::wire::{
    McpConfigWireV1, McpServerWireV1, PluginAuthorWireV1, PluginManifestWireV1,
    MCP_CONFIG_SCHEMA_V1, PLUGIN_MANIFEST_SCHEMA_V1,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PluginAuthor {
    pub name: Option<String>,
    pub email: Option<String>,
    pub url: Option<String>,
}

impl From<PluginAuthorWireV1> for PluginAuthor {
    fn from(wire: PluginAuthorWireV1) -> Self {
        Self {
            name: wire.name,
            email: wire.email,
            url: wire.url,
        }
    }
}

impl From<PluginAuthor> for PluginAuthorWireV1 {
    fn from(author: PluginAuthor) -> Self {
        Self {
            name: author.name,
            email: author.email,
            url: author.url,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub author: Option<PluginAuthor>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub keywords: Vec<String>,
    pub extensions: BTreeMap<String, serde_json::Value>,
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl PluginManifest {
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        validate_manifest_name(&name)?;
        Ok(Self {
            name,
            version: None,
            description: None,
            author: None,
            homepage: None,
            repository: None,
            license: None,
            keywords: Vec::new(),
            extensions: BTreeMap::new(),
            extra: BTreeMap::new(),
        })
    }
}

impl TryFrom<PluginManifestWireV1> for PluginManifest {
    type Error = anyhow::Error;

    fn try_from(wire: PluginManifestWireV1) -> Result<Self, Self::Error> {
        if wire.schema != PLUGIN_MANIFEST_SCHEMA_V1 {
            bail!(
                "invalid manifest schema URL: expected `{PLUGIN_MANIFEST_SCHEMA_V1}`, found `{}`",
                wire.schema
            );
        }
        validate_manifest_name(&wire.name)?;
        Ok(Self {
            name: wire.name,
            version: wire.version,
            description: wire.description,
            author: wire.author.map(Into::into),
            homepage: wire.homepage,
            repository: wire.repository,
            license: wire.license,
            keywords: wire.keywords.unwrap_or_default(),
            extensions: wire.extensions.unwrap_or_default(),
            extra: wire.extra,
        })
    }
}

impl From<PluginManifest> for PluginManifestWireV1 {
    fn from(manifest: PluginManifest) -> Self {
        Self {
            schema: PLUGIN_MANIFEST_SCHEMA_V1.to_string(),
            name: manifest.name,
            version: manifest.version,
            description: manifest.description,
            author: manifest.author.map(Into::into),
            homepage: manifest.homepage,
            repository: manifest.repository,
            license: manifest.license,
            keywords: if manifest.keywords.is_empty() {
                None
            } else {
                Some(manifest.keywords)
            },
            extensions: if manifest.extensions.is_empty() {
                None
            } else {
                Some(manifest.extensions)
            },
            extra: manifest.extra,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpTransport {
    Stdio {
        command: String,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        cwd: Option<String>,
    },
    StreamableHttp {
        url: String,
        headers: BTreeMap<String, String>,
    },
    Sse {
        url: String,
        headers: BTreeMap<String, String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServer {
    pub transport: McpTransport,
}

impl McpServer {
    pub fn new(transport: McpTransport) -> Self {
        Self { transport }
    }

    pub fn stdio(
        command: impl Into<String>,
        args: Vec<String>,
        env: BTreeMap<String, String>,
        cwd: Option<String>,
    ) -> Result<Self> {
        let command = command.into();
        validate_command(&command)?;
        validate_server_env(&env)?;
        if let Some(ref cwd_val) = cwd {
            validate_cwd(cwd_val)?;
        }
        Ok(Self {
            transport: McpTransport::Stdio {
                command,
                args,
                env,
                cwd,
            },
        })
    }

    pub fn streamable_http(
        url: impl Into<String>,
        headers: BTreeMap<String, String>,
    ) -> Result<Self> {
        let url = url.into();
        if url.trim().is_empty() {
            bail!("streamable-http server url cannot be empty");
        }
        Ok(Self {
            transport: McpTransport::StreamableHttp { url, headers },
        })
    }

    pub fn sse(url: impl Into<String>, headers: BTreeMap<String, String>) -> Result<Self> {
        let url = url.into();
        if url.trim().is_empty() {
            bail!("sse server url cannot be empty");
        }
        Ok(Self {
            transport: McpTransport::Sse { url, headers },
        })
    }

    pub fn transport(&self) -> &McpTransport {
        &self.transport
    }
}

impl From<McpTransport> for McpServer {
    fn from(transport: McpTransport) -> Self {
        Self { transport }
    }
}

impl TryFrom<McpServerWireV1> for McpServer {
    type Error = anyhow::Error;

    fn try_from(wire: McpServerWireV1) -> Result<Self, Self::Error> {
        match wire {
            McpServerWireV1::Stdio {
                command,
                args,
                env,
                cwd,
            } => Self::stdio(command, args, env, cwd),
            McpServerWireV1::StreamableHttp { url, headers } => Self::streamable_http(url, headers),
            McpServerWireV1::Sse { url, headers } => Self::sse(url, headers),
        }
    }
}

impl From<McpServer> for McpServerWireV1 {
    fn from(server: McpServer) -> Self {
        match server.transport {
            McpTransport::Stdio {
                command,
                args,
                env,
                cwd,
            } => Self::Stdio {
                command,
                args,
                env,
                cwd,
            },
            McpTransport::StreamableHttp { url, headers } => Self::StreamableHttp { url, headers },
            McpTransport::Sse { url, headers } => Self::Sse { url, headers },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct McpConfiguration {
    pub mcp_servers: BTreeMap<String, McpServer>,
}

impl McpConfiguration {
    pub fn new(mcp_servers: BTreeMap<String, McpServer>) -> Self {
        Self { mcp_servers }
    }
}

impl TryFrom<McpConfigWireV1> for McpConfiguration {
    type Error = anyhow::Error;

    fn try_from(wire: McpConfigWireV1) -> Result<Self, Self::Error> {
        if wire.schema != MCP_CONFIG_SCHEMA_V1 {
            bail!(
                "invalid MCP config schema URL: expected `{MCP_CONFIG_SCHEMA_V1}`, found `{}`",
                wire.schema
            );
        }
        let mut mcp_servers = BTreeMap::new();
        for (name, server_wire) in wire.mcp_servers {
            let server = McpServer::try_from(server_wire)
                .with_context(|| format!("invalid configuration for MCP server `{name}`"))?;
            mcp_servers.insert(name, server);
        }
        Ok(Self { mcp_servers })
    }
}

impl From<McpConfiguration> for McpConfigWireV1 {
    fn from(config: McpConfiguration) -> Self {
        let mcp_servers = config
            .mcp_servers
            .into_iter()
            .map(|(k, v)| (k, McpServerWireV1::from(v)))
            .collect();
        Self {
            schema: MCP_CONFIG_SCHEMA_V1.to_string(),
            mcp_servers,
        }
    }
}

fn validate_manifest_name(name: &str) -> Result<()> {
    if name.trim().is_empty() {
        bail!("plugin manifest name cannot be empty");
    }
    Ok(())
}

fn validate_server_env(env: &BTreeMap<String, String>) -> Result<()> {
    for key in env.keys() {
        if key == "PLUGIN_ROOT" || key == "PLUGIN_DATA" {
            bail!("server env must not define `{key}`");
        }
    }
    Ok(())
}

pub fn validate_command(command: &str) -> Result<()> {
    if command.trim().is_empty() {
        bail!("command cannot be empty");
    }
    if command.contains('/') || command.contains('\\') || command.starts_with('.') {
        if !command.starts_with("./") {
            bail!("relative command path `{command}` must start with `./`");
        }
        let rest = &command[2..];
        if rest.is_empty() {
            bail!("command path `{command}` cannot be empty after `./`");
        }
        validate_rest_of_path(rest, command, "command")?;
    }
    Ok(())
}

pub fn validate_cwd(cwd: &str) -> Result<()> {
    if cwd.trim().is_empty() {
        bail!("cwd cannot be empty");
    }
    if cwd == "./" || cwd == "${PLUGIN_ROOT}" || cwd == "${PLUGIN_DATA}" {
        return Ok(());
    }
    if let Some(rest) = cwd.strip_prefix("${PLUGIN_ROOT}/") {
        validate_rest_of_path(rest, cwd, "cwd")?;
    } else if let Some(rest) = cwd.strip_prefix("${PLUGIN_DATA}/") {
        validate_rest_of_path(rest, cwd, "cwd")?;
    } else if let Some(rest) = cwd.strip_prefix("./") {
        validate_rest_of_path(rest, cwd, "cwd")?;
    } else {
        bail!("cwd `{cwd}` must start with `./`, `${{PLUGIN_ROOT}}`, or `${{PLUGIN_DATA}}`");
    }
    Ok(())
}

fn validate_rest_of_path(rest: &str, full_path: &str, field: &str) -> Result<()> {
    if rest.is_empty() {
        bail!("{field} `{full_path}` contains an empty component");
    }
    for component in rest.split('/') {
        if component.is_empty() {
            bail!("{field} `{full_path}` contains an empty component");
        }
        if component == ".." {
            bail!("{field} `{full_path}` must not contain `..`");
        }
        if component == "." {
            bail!("{field} `{full_path}` must not contain dot components");
        }
        if component.contains('\\') {
            bail!("{field} `{full_path}` must be slash-separated");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_valid_manifest_wire() {
        let json = serde_json::json!({
            "$schema": PLUGIN_MANIFEST_SCHEMA_V1,
            "name": "sample-plugin",
            "version": "1.0.0",
            "description": "A sample agent plugin",
            "author": {
                "name": "Alice",
                "email": "alice@example.com",
                "url": "https://example.com/alice"
            },
            "homepage": "https://example.com/plugin",
            "repository": "https://github.com/example/plugin",
            "license": "Apache-2.0",
            "keywords": ["agent", "plugin", "tools"],
            "extensions": {
                "com.example.client": {
                    "setting": true
                }
            }
        });

        let wire: PluginManifestWireV1 = serde_json::from_value(json).unwrap();
        assert_eq!(wire.name, "sample-plugin");
        assert_eq!(wire.version.as_deref(), Some("1.0.0"));
        assert_eq!(
            wire.keywords.as_deref(),
            Some(
                &[
                    "agent".to_string(),
                    "plugin".to_string(),
                    "tools".to_string()
                ][..]
            )
        );
        assert!(wire.extra.is_empty());

        let domain = PluginManifest::try_from(wire).unwrap();
        assert_eq!(domain.name, "sample-plugin");
        assert_eq!(domain.author.unwrap().name.as_deref(), Some("Alice"));
        assert_eq!(domain.keywords, vec!["agent", "plugin", "tools"]);
    }

    #[test]
    fn captures_unknown_manifest_fields_into_extra() {
        let json = serde_json::json!({
            "$schema": PLUGIN_MANIFEST_SCHEMA_V1,
            "name": "sample-plugin",
            "unknownField": 42,
            "vendorCustom": { "enabled": true }
        });

        let wire: PluginManifestWireV1 = serde_json::from_value(json).unwrap();
        assert_eq!(wire.name, "sample-plugin");
        assert_eq!(wire.extra.get("unknownField"), Some(&serde_json::json!(42)));
        assert_eq!(
            wire.extra.get("vendorCustom"),
            Some(&serde_json::json!({ "enabled": true }))
        );

        let domain = PluginManifest::try_from(wire).unwrap();
        assert_eq!(domain.extra.len(), 2);
    }

    #[test]
    fn rejects_invalid_manifest_schema_url() {
        let json = serde_json::json!({
            "$schema": "https://example.com/wrong/schema.json",
            "name": "sample-plugin"
        });

        let result = serde_json::from_value::<PluginManifestWireV1>(json);
        assert!(result.is_err(), "deserialization must validate $schema URL");
    }

    #[test]
    fn rejects_empty_manifest_name() {
        let wire = PluginManifestWireV1 {
            schema: PLUGIN_MANIFEST_SCHEMA_V1.to_string(),
            name: "   ".to_string(),
            version: None,
            description: None,
            author: None,
            homepage: None,
            repository: None,
            license: None,
            keywords: None,
            extensions: None,
            extra: BTreeMap::new(),
        };

        let err = PluginManifest::try_from(wire).unwrap_err();
        assert!(err.to_string().contains("manifest name cannot be empty"));
    }

    #[test]
    fn deserializes_valid_mcp_config_with_all_transports() {
        let json = serde_json::json!({
            "$schema": MCP_CONFIG_SCHEMA_V1,
            "mcpServers": {
                "local-stdio": {
                    "type": "stdio",
                    "command": "./bin/server",
                    "args": ["--port", "8080"],
                    "env": { "DEBUG": "1" },
                    "cwd": "./"
                },
                "bare-stdio": {
                    "type": "stdio",
                    "command": "node",
                    "args": ["server.js"]
                },
                "remote-http": {
                    "type": "streamable-http",
                    "url": "https://example.com/mcp",
                    "headers": { "Authorization": "Bearer secret" }
                },
                "remote-sse": {
                    "type": "sse",
                    "url": "https://example.com/sse",
                    "headers": { "X-API-Key": "key" }
                }
            }
        });

        let wire: McpConfigWireV1 = serde_json::from_value(json).unwrap();
        assert_eq!(wire.mcp_servers.len(), 4);

        let domain = McpConfiguration::try_from(wire).unwrap();
        assert_eq!(domain.mcp_servers.len(), 4);

        let local_stdio = domain.mcp_servers.get("local-stdio").unwrap();
        match local_stdio.transport() {
            McpTransport::Stdio {
                command,
                args,
                env,
                cwd,
            } => {
                assert_eq!(command, "./bin/server");
                assert_eq!(args, &["--port", "8080"]);
                assert_eq!(env.get("DEBUG").map(String::as_str), Some("1"));
                assert_eq!(cwd.as_deref(), Some("./"));
            }
            _ => panic!("expected Stdio transport"),
        }

        let remote_http = domain.mcp_servers.get("remote-http").unwrap();
        match remote_http.transport() {
            McpTransport::StreamableHttp { url, headers } => {
                assert_eq!(url, "https://example.com/mcp");
                assert_eq!(
                    headers.get("Authorization").map(String::as_str),
                    Some("Bearer secret")
                );
            }
            _ => panic!("expected StreamableHttp transport"),
        }

        let remote_sse = domain.mcp_servers.get("remote-sse").unwrap();
        match remote_sse.transport() {
            McpTransport::Sse { url, headers } => {
                assert_eq!(url, "https://example.com/sse");
                assert_eq!(headers.get("X-API-Key").map(String::as_str), Some("key"));
            }
            _ => panic!("expected Sse transport"),
        }
    }

    #[test]
    fn rejects_invalid_mcp_config_schema_url() {
        let json = serde_json::json!({
            "$schema": "https://example.com/invalid/mcp.json",
            "mcpServers": {}
        });

        let result = serde_json::from_value::<McpConfigWireV1>(json);
        assert!(result.is_err(), "deserialization must validate $schema URL");
    }

    #[test]
    fn rejects_server_env_defining_reserved_plugin_variables() {
        for forbidden in ["PLUGIN_ROOT", "PLUGIN_DATA"] {
            let mut env = BTreeMap::new();
            env.insert(forbidden.to_string(), "/custom/path".to_string());
            let result = McpServer::stdio("./server", vec![], env, None);
            assert!(
                result.is_err(),
                "must reject reserved environment variable {forbidden}"
            );
            assert!(result.unwrap_err().to_string().contains(forbidden));
        }
    }

    #[test]
    fn validates_command_paths() {
        // Valid commands
        assert!(validate_command("node").is_ok());
        assert!(validate_command("python3").is_ok());
        assert!(validate_command("./bin/server").is_ok());
        assert!(validate_command("./server.sh").is_ok());

        // Invalid commands
        assert!(validate_command("").is_err());
        assert!(validate_command("   ").is_err());
        assert!(
            validate_command("bin/server").is_err(),
            "relative command without leading ./"
        );
        assert!(validate_command("./").is_err(), "empty after ./");
        assert!(
            validate_command("./bin/../server").is_err(),
            "parent traversal .."
        );
        assert!(
            validate_command("../server").is_err(),
            "parent traversal .."
        );
        assert!(
            validate_command("./bin//server").is_err(),
            "empty component"
        );
        assert!(
            validate_command("./bin/").is_err(),
            "trailing empty component"
        );
    }

    #[test]
    fn validates_cwd_paths() {
        // Valid cwds
        assert!(validate_cwd("./").is_ok());
        assert!(validate_cwd("./sub").is_ok());
        assert!(validate_cwd("./sub/dir").is_ok());
        assert!(validate_cwd("${PLUGIN_ROOT}").is_ok());
        assert!(validate_cwd("${PLUGIN_ROOT}/dist").is_ok());
        assert!(validate_cwd("${PLUGIN_DATA}").is_ok());
        assert!(validate_cwd("${PLUGIN_DATA}/logs").is_ok());

        // Invalid cwds
        assert!(validate_cwd("").is_err());
        assert!(
            validate_cwd("sub/dir").is_err(),
            "must start with ./, ${{PLUGIN_ROOT}}, or ${{PLUGIN_DATA}}"
        );
        assert!(validate_cwd("./sub/../dir").is_err(), "parent traversal ..");
        assert!(validate_cwd("./sub//dir").is_err(), "empty component");
        assert!(validate_cwd("./sub/").is_err(), "empty component");
        assert!(
            validate_cwd("${PLUGIN_ROOT}/..").is_err(),
            "parent traversal .."
        );
        assert!(
            validate_cwd("${PLUGIN_ROOT}//foo").is_err(),
            "empty component"
        );
        assert!(validate_cwd("${PLUGIN_ROOT}/").is_err(), "empty component");
    }

    #[test]
    fn roundtrip_manifest_wire_and_domain() {
        let manifest = PluginManifest {
            name: "test-plugin".to_string(),
            version: Some("2.0.0".to_string()),
            description: Some("Description".to_string()),
            author: Some(PluginAuthor {
                name: Some("Bob".to_string()),
                email: None,
                url: None,
            }),
            homepage: None,
            repository: Some("https://github.com/test/repo".to_string()),
            license: Some("MIT".to_string()),
            keywords: vec!["test".to_string()],
            extensions: BTreeMap::new(),
            extra: BTreeMap::new(),
        };

        let wire = PluginManifestWireV1::from(manifest.clone());
        assert_eq!(wire.schema, PLUGIN_MANIFEST_SCHEMA_V1);
        let serialized = serde_json::to_string(&wire).unwrap();
        let parsed_wire: PluginManifestWireV1 = serde_json::from_str(&serialized).unwrap();
        let parsed_domain = PluginManifest::try_from(parsed_wire).unwrap();
        assert_eq!(manifest, parsed_domain);
    }

    #[test]
    fn roundtrip_mcp_config_wire_and_domain() {
        let mut servers = BTreeMap::new();
        servers.insert(
            "stdio-srv".to_string(),
            McpServer::stdio(
                "./server",
                vec!["--arg".to_string()],
                BTreeMap::new(),
                Some("./".to_string()),
            )
            .unwrap(),
        );
        servers.insert(
            "http-srv".to_string(),
            McpServer::streamable_http("https://api.example.com", BTreeMap::new()).unwrap(),
        );

        let config = McpConfiguration::new(servers);
        let wire = McpConfigWireV1::from(config.clone());
        assert_eq!(wire.schema, MCP_CONFIG_SCHEMA_V1);
        let serialized = serde_json::to_string(&wire).unwrap();
        let parsed_wire: McpConfigWireV1 = serde_json::from_str(&serialized).unwrap();
        let parsed_domain = McpConfiguration::try_from(parsed_wire).unwrap();
        assert_eq!(config, parsed_domain);
    }
}
