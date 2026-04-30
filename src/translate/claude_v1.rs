use super::Translator;
use crate::{
    Hook, HookEvent, HookEventKind, HookMetadata, McpServer, McpServerConfig, McpServerMetadata,
};
use anyhow::Result;
use serde::Deserialize;
use std::collections::BTreeMap as HashMap;

pub struct ClaudeV1;

#[derive(Deserialize)]
pub struct ClaudeMcpConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl ClaudeV1 {
    pub fn translate_hook(
        &self,
        name: &str,
        data: &serde_json::Value,
        filename: Option<&str>,
    ) -> Result<Hook> {
        let mut event = None;
        let kind = match name {
            "PreToolUse" => Some(HookEventKind::PreToolUse),
            "PostToolUse" => Some(HookEventKind::PostToolUse),
            "Notification" => Some(HookEventKind::Notification),
            "Stop" => Some(HookEventKind::Stop),
            "SubagentStop" => Some(HookEventKind::SubagentStop),
            _ => None,
        };

        if let Some(k) = kind {
            if let Some(commands) = data
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|obj| obj.get("hooks"))
                .and_then(|h| h.as_array())
            {
                for cmd_obj in commands {
                    if let Some(cmd) = cmd_obj.get("command").and_then(|c| c.as_str()) {
                        event = Some(HookEvent {
                            event: k.clone(),
                            matcher: None,
                            command: Some(cmd.to_string()),
                        });
                        break;
                    }
                }
            }
        }

        let mut extra = HashMap::new();
        extra.insert(name.to_string(), data.clone());
        if let Some(f) = filename {
            extra.insert(
                "rulette:source_file".to_string(),
                serde_json::Value::String(f.to_string()),
            );
        }

        Ok(Hook {
            metadata: HookMetadata {
                name: name.to_string(),
                hook_event: event,
                extra,
            },
        })
    }
}

impl Translator for ClaudeV1 {
    fn translate_hook(&self, name: &str, data: &serde_json::Value) -> Result<Hook> {
        self.translate_hook(name, data, None)
    }

    fn translate_mcp(&self, name: &str, config: &ClaudeMcpConfig) -> Result<McpServer> {
        Ok(McpServer {
            metadata: McpServerMetadata {
                name: name.to_string(),
                extra: HashMap::new(),
            },
            config: McpServerConfig {
                command: config.command.clone(),
                args: config.args.clone(),
                env: config.env.clone(),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HookEventKind;
    use serde_json::json;

    #[test]
    fn test_translate_hook_pre_tool_use() {
        let hook_data = json!([
            {
                "hooks": [
                    {
                        "type": "command",
                        "command": "echo 'hello'"
                    }
                ]
            }
        ]);

        let hook = ClaudeV1
            .translate_hook("PreToolUse", &hook_data, None)
            .unwrap();
        assert_eq!(hook.metadata.name, "PreToolUse");
        let event = hook.metadata.hook_event.unwrap();
        assert_eq!(event.event, HookEventKind::PreToolUse);
        assert_eq!(event.command.unwrap(), "echo 'hello'");
    }

    #[test]
    fn test_translate_mcp_server() {
        let config = ClaudeMcpConfig {
            command: "npx".to_string(),
            args: vec!["test".to_string()],
            env: HashMap::new(),
        };

        let mcp = ClaudeV1.translate_mcp("my-server", &config).unwrap();
        assert_eq!(mcp.metadata.name, "my-server");
        assert_eq!(mcp.config.command, "npx");
        assert_eq!(mcp.config.args, vec!["test"]);
    }
}
