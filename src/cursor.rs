use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CursorMdc {
    pub frontmatter: CursorMdcFrontmatter,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CursorMdcFrontmatter {
    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub globs: Option<Vec<String>>,

    #[serde(rename = "alwaysApply", skip_serializing_if = "Option::is_none")]
    pub always_apply: Option<bool>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_cursor_mdc_frontmatter() {
        let valid = CursorMdcFrontmatter {
            description: "TypeScript conventions for this project".to_string(),
            globs: Some(vec!["src/**/*.ts".to_string(), "src/**/*.tsx".to_string()]),
            always_apply: Some(false),
            extra: HashMap::new(),
        };

        let json = serde_json::to_string(&valid).unwrap();
        let deserialized: CursorMdcFrontmatter = serde_json::from_str(&json).unwrap();
        assert_eq!(valid.description, deserialized.description);
        assert_eq!(valid.globs, deserialized.globs);
        assert_eq!(valid.always_apply, deserialized.always_apply);
    }

    #[test]
    fn test_schema_generation() {
        let schema = schemars::schema_for!(CursorMdcFrontmatter);
        let schema_json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(schema_json.contains("\"title\": \"CursorMdcFrontmatter\""));
        assert!(schema_json.contains("\"description\""));
    }
}
