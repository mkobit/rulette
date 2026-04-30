use crate::Entity;

pub fn match_expr(entity: &Entity, expr: &str) -> bool {
    let parts: Vec<&str> = expr.split("==").collect();
    if parts.len() == 2 {
        let key = parts[0].trim();
        let val = parts[1].trim().trim_matches(|c| c == '"' || c == '\'');

        if let Ok(json_val) = serde_json::to_value(entity) {
            if let Some(metadata) = json_val.get("metadata") {
                if let Some(field) = metadata.get(key) {
                    if field.as_str() == Some(val) {
                        return true;
                    }
                }
                if let Some(extra) = metadata.get("extra") {
                    if let Some(field) = extra.get(key) {
                        if field.as_str() == Some(val) {
                            return true;
                        }
                    }
                }
            }
        }
    }

    if let Ok(json) = serde_json::to_string(entity) {
        if json.contains(expr) {
            return true;
        }
    }
    false
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
