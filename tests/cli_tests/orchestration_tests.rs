use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_multi_agent_orchestration_scenario() {
    let example_dir = "examples/multi-agent-orchestration/rules";
    let temp_dir = tempdir().unwrap();
    let claude_out = temp_dir.path().join("claude");
    let gemini_out = temp_dir.path().join("gemini");
    let cursor_out = temp_dir.path().join("cursor");

    fs::create_dir_all(&claude_out).unwrap();
    fs::create_dir_all(&gemini_out).unwrap();
    fs::create_dir_all(&cursor_out).unwrap();

    let cargo_bin = assert_cmd::cargo::cargo_bin("rulette");

    // 1. Compile stable rules for Claude and Gemini
    let status1 = Command::new(&cargo_bin)
        .arg("transform")
        .arg(example_dir)
        .arg("--filter")
        .arg("status == \"stable\"")
        .arg("--set")
        .arg("org=mycompany")
        .arg("-o")
        .arg(format!("claude:{}", claude_out.display()))
        .arg("-o")
        .arg(format!("gemini:{}", gemini_out.display()))
        .status()
        .expect("Failed to execute rulette for stable rules");

    assert!(status1.success());

    // Verify Claude output: Should contain security-auditor skill but NOT experimental-ui
    let claude_md = claude_out.join("CLAUDE.md");
    assert!(claude_md.exists());
    let claude_content = fs::read_to_string(claude_md).unwrap();
    assert!(claude_content.contains("# Security Auditor"));
    assert!(!claude_content.contains("# Experimental UI Rules"));

    // Verify Gemini output
    let gemini_md = gemini_out.join("GEMINI.md");
    assert!(gemini_md.exists());
    let gemini_content = fs::read_to_string(gemini_md).unwrap();
    assert!(gemini_content.contains("# Security Auditor"));

    // 2. Compile experimental rules for Cursor
    let status2 = Command::new(&cargo_bin)
        .arg("transform")
        .arg(example_dir)
        .arg("--filter")
        .arg("status == \"experimental\"")
        .arg("-o")
        .arg(format!("cursor-mdc:{}", cursor_out.display()))
        .status()
        .expect("Failed to execute rulette for experimental rules");

    assert!(status2.success());

    // Verify Cursor output: Should contain experimental-ui but NOT security-auditor
    let cursor_mdc = cursor_out.join("experimental-ui.mdc");
    assert!(cursor_mdc.exists());
    let cursor_content = fs::read_to_string(cursor_mdc).unwrap();
    assert!(cursor_content.contains("# Experimental UI Rules"));
    assert!(!cursor_content.contains("# Security Auditor"));
}
