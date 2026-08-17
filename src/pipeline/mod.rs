use crate::Entity;
use anyhow::Result;

/// A parsed `<field> == "<value>"` filter/exclude expression.
///
/// Matching is scoped strictly to the entity's `metadata` and
/// `metadata.extra` fields -- it never falls back to scanning the entity's
/// raw serialized JSON (including the body), which could otherwise produce
/// a spurious match against unrelated text that happens to contain the
/// expression as a substring.
pub struct FilterExpr {
    key: String,
    value: String,
}

impl FilterExpr {
    pub fn parse(expr: &str) -> Result<Self> {
        let parts: Vec<&str> = expr.splitn(2, "==").collect();
        let [field, value] = parts.as_slice() else {
            anyhow::bail!(
                "Invalid filter expression '{}': expected '<field> == \"<value>\"'",
                expr
            );
        };
        let key = field.trim().to_string();
        if key.is_empty() {
            anyhow::bail!("Invalid filter expression '{}': empty field name", expr);
        }
        let value = value
            .trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .to_string();
        Ok(Self { key, value })
    }

    pub fn matches(&self, entity: &Entity) -> bool {
        let Ok(json_val) = serde_json::to_value(entity) else {
            return false;
        };

        // "kind" is the entity's top-level discriminant (rule, skill,
        // mcp-server, hook, agent, permissions), not a metadata field.
        if self.key == "kind" {
            return json_val.get("kind").and_then(|v| v.as_str()) == Some(self.value.as_str());
        }

        let Some(metadata) = json_val.get("metadata") else {
            return false;
        };
        if let Some(field) = metadata.get(&self.key) {
            if field.as_str() == Some(self.value.as_str()) {
                return true;
            }
        }
        if let Some(extra) = metadata.get("extra") {
            if let Some(field) = extra.get(&self.key) {
                if field.as_str() == Some(self.value.as_str()) {
                    return true;
                }
            }
        }
        false
    }
}

pub fn rename_field(entity: &mut Entity, from: &str, to: &str) {
    match entity {
        crate::Entity::Hook(_) | crate::Entity::Agent(_) | crate::Entity::Permissions(_) => {}
        Entity::Rule(rule) => {
            if let Some(val) = rule.metadata.extra.remove(from) {
                rule.metadata.extra.insert(to.to_string(), val);
            }
        }
        Entity::Skill(skill) => {
            if let Some(val) = skill.metadata.extra.remove(from) {
                skill.metadata.extra.insert(to.to_string(), val);
            }
        }
        Entity::McpServer(mcp) => {
            if let Some(val) = mcp.metadata.extra.remove(from) {
                mcp.metadata.extra.insert(to.to_string(), val);
            }
        }
    }
}

pub fn set_field(entity: &mut Entity, key: &str, value: &str) {
    let json_val = serde_json::Value::String(value.to_string());
    match entity {
        crate::Entity::Hook(_) | crate::Entity::Agent(_) | crate::Entity::Permissions(_) => {}
        Entity::Rule(rule) => {
            rule.metadata.extra.insert(key.to_string(), json_val);
        }
        Entity::Skill(skill) => {
            skill.metadata.extra.insert(key.to_string(), json_val);
        }
        Entity::McpServer(mcp) => {
            mcp.metadata.extra.insert(key.to_string(), json_val);
        }
    }
}
