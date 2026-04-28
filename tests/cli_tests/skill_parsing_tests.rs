use serde_json::Value;
use std::process::Command;

#[test]
fn test_complex_skill_directory_parsing() {
    let skill_dir = "tests/fixtures/agent-skills/complex-skill";
    let cargo_bin = assert_cmd::cargo::cargo_bin("rulette");

    let output = Command::new(&cargo_bin)
        .arg("transform")
        .arg(skill_dir)
        .arg("--to")
        .arg("ir-json")
        .output()
        .expect("Failed to execute rulette");

    assert!(output.status.success());
    let out_str = std::str::from_utf8(&output.stdout).unwrap();
    let json: Value = serde_json::from_str(out_str).unwrap();
    let entities = json.get("entities").unwrap().as_array().unwrap();

    // Verify that the skill itself is parsed.
    let has_skill = entities.iter().any(|e| e["kind"] == "skill" && e["metadata"]["name"] == "complex-skill");
    assert!(has_skill, "Should have parsed complex-skill");

    // Semantic refinement: Scripts are NOT hooks by default.
    // They are implementation assets. For now, they should not appear as top-level Hook entities.
    let has_hook = entities.iter().any(|e| e["kind"] == "hook" && e["metadata"]["name"] == "pre-commit.sh");
    assert!(!has_hook, "Scripts should NOT be parsed as hooks by default (semantic refinement)");
}
