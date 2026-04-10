use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CursorSkill {
    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub globs: Option<String>,

    #[serde(rename = "alwaysApply", skip_serializing_if = "Option::is_none")]
    pub always_apply: Option<bool>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_generation() {
        let schema = schemars::schema_for!(CursorSkill);
        let schema_json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(schema_json.contains("\"title\": \"CursorSkill\""));
        assert!(schema_json.contains("\"description\""));
    }
}
