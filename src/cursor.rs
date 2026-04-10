use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CursorRule {
    pub metadata: CursorMetadata,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CursorMetadata {
    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub globs: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_generation() {
        let schema = schemars::schema_for!(CursorRule);
        let schema_json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(schema_json.contains("\"title\": \"CursorRule\""));
        assert!(schema_json.contains("\"metadata\""));
        assert!(schema_json.contains("\"body\""));
    }
}
