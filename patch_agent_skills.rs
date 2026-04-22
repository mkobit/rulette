fn main() {
    let mut parsed: serde_yaml::Value = serde_yaml::from_str(r#"
allowed-tools: ["bash", "read", "write"]
"#).unwrap();
    println!("{:?}", parsed.get("allowed-tools").unwrap());
    println!("{}", serde_yaml::to_string(parsed.get("allowed-tools").unwrap()).unwrap());
}
