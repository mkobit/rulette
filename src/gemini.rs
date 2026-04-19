use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GeminiSubAgentMetadata {
    pub name: String,
    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<String>>,

    #[serde(rename = "mcpServers", skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<HashMap<String, serde_json::Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_mins: Option<u32>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GeminiSubAgent {
    pub metadata: GeminiSubAgentMetadata,
    pub system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GeminiSlashCommand {
    pub description: String,
    pub prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
/// See [Gemini API Documentation](https://ai.google.dev/api/rest/v1beta/tools#FunctionDeclaration) for details on the structure of Gemini tools.
pub struct GeminiSkill {
    pub name: String,
    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seeded: Option<bool>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,

    #[serde(rename = "allowed-tools", skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_generation() {
        let schema = schemars::schema_for!(GeminiSkill);
        let schema_json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(schema_json.contains("\"title\": \"GeminiSkill\""));
        assert!(schema_json.contains("\"name\""));
        assert!(schema_json.contains("\"description\""));
    }
}

#[cfg(test)]
mod slash_command_tests {
    use super::*;

    #[test]
    fn test_slash_command_schema_generation() {
        let schema = schemars::schema_for!(GeminiSlashCommand);
        let schema_json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(schema_json.contains("\"title\": \"GeminiSlashCommand\""));
        assert!(schema_json.contains("\"description\""));
        assert!(schema_json.contains("\"prompt\""));
    }
}

#[cfg(test)]
mod subagent_tests {
    use super::*;

    #[test]
    fn test_subagent_metadata_schema() {
        let schema = schemars::schema_for!(GeminiSubAgentMetadata);
        let schema_json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(schema_json.contains("\"title\": \"GeminiSubAgentMetadata\""));
        assert!(schema_json.contains("\"name\""));
        assert!(schema_json.contains("\"description\""));
        assert!(schema_json.contains("\"mcpServers\""));
    }
}

impl GeminiSubAgent {
    pub fn parse(content: &str) -> Result<Self, anyhow::Error> {
        // Implement parsing markdown with yaml frontmatter.
        // It starts with '---', ends with '---'.
        if !content.starts_with("---\n") && !content.starts_with("---\r\n") {
            return Err(anyhow::anyhow!("Missing YAML frontmatter start (---)"));
        }

        // Find the end of frontmatter
        let end_idx_opt = content[4..].find("\n---");
        if let Some(end_idx_rel) = end_idx_opt {
            let end_idx = end_idx_rel + 4;
            let yaml_content = &content[4..end_idx];

            // The body starts after the \n---\n or \n---\r\n
            let body_start = if content.len() > end_idx + 4 {
                if content[end_idx + 4..].starts_with("\r\n") {
                    end_idx + 6
                } else if content[end_idx + 4..].starts_with("\n") {
                    end_idx + 5
                } else {
                    end_idx + 4
                }
            } else {
                content.len()
            };

            let metadata: GeminiSubAgentMetadata = serde_yaml::from_str(yaml_content)?;
            let system_prompt = content[body_start..].trim_start().to_string();

            Ok(Self {
                metadata,
                system_prompt,
            })
        } else {
            Err(anyhow::anyhow!("Missing YAML frontmatter end (---)"))
        }
    }
}

#[cfg(test)]
mod parse_subagent_tests {
    use super::*;

    #[test]
    fn test_parse_subagent() {
        let content = "---\nname: security-auditor\ndescription: Specialized in finding security vulnerabilities in code.\nkind: local\ntools:\n  - read_file\n  - grep_search\nmodel: gemini-3-flash-preview\ntemperature: 0.2\nmax_turns: 10\n---\n\nYou are a ruthless Security Auditor. Your job is to analyze code for potential vulnerabilities.";
        let agent = GeminiSubAgent::parse(content).unwrap();
        assert_eq!(agent.metadata.name, "security-auditor");
        assert_eq!(
            agent.metadata.description,
            "Specialized in finding security vulnerabilities in code."
        );
        assert_eq!(agent.metadata.kind.as_deref(), Some("local"));
        assert_eq!(agent.metadata.tools.as_ref().unwrap().len(), 2);
        assert_eq!(
            agent.metadata.model.as_deref(),
            Some("gemini-3-flash-preview")
        );
        assert_eq!(agent.metadata.temperature, Some(0.2));
        assert_eq!(agent.metadata.max_turns, Some(10));
        assert_eq!(agent.system_prompt, "You are a ruthless Security Auditor. Your job is to analyze code for potential vulnerabilities.");
    }

    #[test]
    fn test_parse_invalid_subagent() {
        let content = "name: no-frontmatter\ndescription: test\n---\nbody";
        assert!(GeminiSubAgent::parse(content).is_err());
    }
}
