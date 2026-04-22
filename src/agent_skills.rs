use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Skill {
    pub metadata: SkillMetadata,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<String>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,

    #[serde(rename = "allowed-tools", skip_serializing_if = "Option::is_none")]
    pub allowed_tools: Option<serde_json::Value>,

    #[serde(flatten)]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    InvalidNameLength,
    InvalidNameCharacters,
    InvalidNameEdges,
    InvalidNameConsecutiveHyphens,
    InvalidDescriptionLength,
    InvalidCompatibilityLength,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidNameLength => write!(f, "name length must be between 1 and 64 characters"),
            Self::InvalidNameCharacters => write!(
                f,
                "name may only contain lowercase alphanumeric characters and hyphens"
            ),
            Self::InvalidNameEdges => write!(f, "name must not start or end with a hyphen"),
            Self::InvalidNameConsecutiveHyphens => {
                write!(f, "name must not contain consecutive hyphens")
            }
            Self::InvalidDescriptionLength => write!(
                f,
                "description length must be between 1 and 1024 characters"
            ),
            Self::InvalidCompatibilityLength => {
                write!(f, "compatibility length must not exceed 500 characters")
            }
        }
    }
}

impl std::error::Error for ValidationError {}

impl SkillMetadata {
    pub fn validate(&self) -> Result<(), ValidationError> {
        let name_len = self.name.chars().count();
        if !(1..=64).contains(&name_len) {
            return Err(ValidationError::InvalidNameLength);
        }

        if !self
            .name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(ValidationError::InvalidNameCharacters);
        }

        if self.name.starts_with('-') || self.name.ends_with('-') {
            return Err(ValidationError::InvalidNameEdges);
        }

        if self.name.contains("--") {
            return Err(ValidationError::InvalidNameConsecutiveHyphens);
        }

        let desc_len = self.description.chars().count();
        if !(1..=1024).contains(&desc_len) {
            return Err(ValidationError::InvalidDescriptionLength);
        }

        if let Some(compat) = &self.compatibility {
            let compat_len = compat.chars().count();
            if compat_len > 500 {
                return Err(ValidationError::InvalidCompatibilityLength);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_skill_metadata() {
        let valid = SkillMetadata {
            name: "pdf-processing".to_string(),
            description: "Extract PDF text, fill forms, merge files. Use when handling PDFs."
                .to_string(),
            version: Some("1.0".to_string()),
            license: Some("Apache-2.0".to_string()),
            compatibility: Some("Designed for Claude Code".to_string()),
            metadata: {
                let mut map = HashMap::new();
                map.insert("author".to_string(), "example-org".to_string());
                map
            },
            allowed_tools: Some(serde_json::Value::String(
                "Bash(git:*) Bash(jq:*) Read".to_string(),
            )),
            extra: HashMap::new(),
        };

        assert!(valid.validate().is_ok());

        let json = serde_json::to_string(&valid).unwrap();
        let deserialized: SkillMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(valid.name, deserialized.name);
        assert_eq!(valid.description, deserialized.description);
    }

    #[test]
    fn test_invalid_names() {
        let mut meta = SkillMetadata {
            name: "".to_string(),
            description: "valid description".to_string(),
            version: None,
            license: None,
            compatibility: None,
            metadata: HashMap::new(),
            allowed_tools: None,
            extra: HashMap::new(),
        };
        assert_eq!(meta.validate(), Err(ValidationError::InvalidNameLength));

        meta.name = "PDF-Processing".to_string();
        assert_eq!(meta.validate(), Err(ValidationError::InvalidNameCharacters));

        meta.name = "-pdf".to_string();
        assert_eq!(meta.validate(), Err(ValidationError::InvalidNameEdges));

        meta.name = "pdf-".to_string();
        assert_eq!(meta.validate(), Err(ValidationError::InvalidNameEdges));

        meta.name = "pdf--processing".to_string();
        assert_eq!(
            meta.validate(),
            Err(ValidationError::InvalidNameConsecutiveHyphens)
        );

        meta.name = "a".repeat(65);
        assert_eq!(meta.validate(), Err(ValidationError::InvalidNameLength));
    }

    #[test]
    fn test_invalid_description() {
        let mut meta = SkillMetadata {
            name: "valid-name".to_string(),
            description: "".to_string(),
            version: None,
            license: None,
            compatibility: None,
            metadata: HashMap::new(),
            allowed_tools: None,
            extra: HashMap::new(),
        };
        assert_eq!(
            meta.validate(),
            Err(ValidationError::InvalidDescriptionLength)
        );

        meta.description = "a".repeat(1025);
        assert_eq!(
            meta.validate(),
            Err(ValidationError::InvalidDescriptionLength)
        );
    }

    #[test]
    fn test_invalid_compatibility() {
        let meta = SkillMetadata {
            name: "valid-name".to_string(),
            description: "valid description".to_string(),
            version: None,
            license: None,
            compatibility: Some("a".repeat(501)),
            metadata: HashMap::new(),
            allowed_tools: None,
            extra: HashMap::new(),
        };
        assert_eq!(
            meta.validate(),
            Err(ValidationError::InvalidCompatibilityLength)
        );
    }

    #[test]
    fn test_schema_generation() {
        let schema = schemars::schema_for!(SkillMetadata);
        let schema_json = serde_json::to_string_pretty(&schema).unwrap();
        assert!(schema_json.contains("\"title\": \"SkillMetadata\""));
        assert!(schema_json.contains("\"name\""));
        assert!(schema_json.contains("\"description\""));
    }
}
