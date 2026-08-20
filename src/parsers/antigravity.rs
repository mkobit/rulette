use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap as HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AntigravityTrigger {
    AlwaysOn,
    Glob,
    Manual,
    ModelDecision,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum GlobsValue {
    Single(String),
    Many(Vec<String>),
}

impl GlobsValue {
    pub fn into_vec(self) -> Vec<String> {
        match self {
            GlobsValue::Single(s) => s
                .split(',')
                .map(|g| g.trim().to_string())
                .filter(|g| !g.is_empty())
                .collect(),
            GlobsValue::Many(v) => v,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AntigravityRuleFrontmatter {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<AntigravityTrigger>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub globs: Option<GlobsValue>,

    #[serde(rename = "rulette:activation", skip_serializing_if = "Option::is_none")]
    pub activation: Option<crate::TargetOverrides<crate::Activation>>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_antigravity_frontmatter_serialization_and_deserialization() {
        let yaml = r#"
description: Rust conventions
trigger: glob
globs:
  - "**/*.rs"
  - "**/Cargo.toml"
"#;
        let parsed: AntigravityRuleFrontmatter = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(parsed.description.as_deref(), Some("Rust conventions"));
        assert_eq!(parsed.trigger, Some(AntigravityTrigger::Glob));
        assert_eq!(
            parsed.globs.map(|g| g.into_vec()),
            Some(vec!["**/*.rs".to_string(), "**/Cargo.toml".to_string()])
        );
    }

    #[test]
    fn test_antigravity_trigger_modes_deserialization() {
        let triggers = [
            ("trigger: always_on\n", AntigravityTrigger::AlwaysOn),
            ("trigger: glob\n", AntigravityTrigger::Glob),
            ("trigger: manual\n", AntigravityTrigger::Manual),
            ("trigger: model_decision\n", AntigravityTrigger::ModelDecision),
        ];

        for (yaml, expected) in triggers {
            let parsed: AntigravityRuleFrontmatter = serde_yaml::from_str(yaml).unwrap();
            assert_eq!(parsed.trigger, Some(expected));
        }
    }

    #[test]
    fn test_antigravity_schema_generation() {
        let schema = schemars::schema_for!(AntigravityRuleFrontmatter);
        let schema_json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(schema_json.contains("\"title\": \"AntigravityRuleFrontmatter\""));
        assert!(schema_json.contains("\"trigger\""));
        assert!(schema_json.contains("\"always_on\""));
        assert!(schema_json.contains("\"model_decision\""));
    }
}
