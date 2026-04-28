use crate::{Hook, McpServer};
use anyhow::Result;

pub mod claude_v1;

pub trait Converter {
    fn translate_hook(&self, name: &str, data: &serde_json::Value) -> Result<Hook>;
    fn translate_mcp(&self, name: &str, config: &claude_v1::ClaudeMcpConfig) -> Result<McpServer>;
}

pub fn get_converter(spec: &str) -> Option<Box<dyn Converter>> {
    match spec {
        "claude-v1" => Some(Box::new(claude_v1::ClaudeV1)),
        _ => None,
    }
}
