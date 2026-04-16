use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
struct Meta {
    pub name: String,
    #[serde(rename = "rulette:hook-event", skip_serializing_if = "Option::is_none")]
    pub hook_event: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

fn main() {
    let json = r#"{"name": "test", "rulette:hook-event": "foo", "other": 123}"#;
    let m: Meta = serde_json::from_str(json).unwrap();
    println!("{:?}", m);
    println!("{}", serde_json::to_string(&m).unwrap());
}
