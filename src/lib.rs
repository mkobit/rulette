pub mod agent_skills;
pub mod backend;
pub mod claude;
pub mod cli;
pub mod codex;
pub mod cursor;
pub mod frontend;
pub mod gemini;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RuletteDocument {
    pub entities: Vec<Entity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind")]
pub enum Entity {
    #[serde(rename = "rule")]
    Rule(Rule),
    #[serde(rename = "skill")]
    Skill(agent_skills::Skill),
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Rule {
    pub metadata: RuleMetadata,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct RuleMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
