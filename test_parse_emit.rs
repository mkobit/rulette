use std::fs;

fn main() {
    let json = serde_json::json!({
        "allowed-tools": "[\"bash\", \"read\", \"write\"]"
    });

    let yaml = serde_yaml::to_string(&json).unwrap();
    println!("{}", yaml);
}
