use assert_cmd::Command;
use serde_json::json;
use tempfile::NamedTempFile;

#[test]
fn test_hook_eval_antigravity_auto_approve() {
    let policy_json = json!({
        "default_mode": "ask",
        "rules": [
            { "tool": "bash", "pattern": "git *", "action": "allow" },
            { "tool": "bash", "pattern": "npm test", "action": "allow" },
            { "tool": "bash", "pattern": "rm *", "action": "deny" }
        ]
    });
    let mut policy_file = NamedTempFile::new().unwrap();
    serde_json::to_writer(&mut policy_file, &policy_json).unwrap();

    let input_payload = json!({
        "toolCall": {
            "name": "run_command",
            "args": {
                "CommandLine": "git status && npm test"
            }
        }
    });

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("hook-eval")
        .arg("--policy")
        .arg(policy_file.path())
        .write_stdin(serde_json::to_string(&input_payload).unwrap())
        .assert();

    let output = assert.success().get_output().stdout.clone();
    let resp: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(resp["decision"], "auto_approve");
    assert_eq!(
        resp["permissionOverrides"],
        json!(["command(git status && npm test)"])
    );
}

#[test]
fn test_hook_eval_antigravity_deny_compound() {
    let policy_json = json!({
        "default_mode": "ask",
        "rules": [
            { "tool": "bash", "pattern": "git *", "action": "allow" },
            { "tool": "bash", "pattern": "rm *", "action": "deny" }
        ]
    });
    let mut policy_file = NamedTempFile::new().unwrap();
    serde_json::to_writer(&mut policy_file, &policy_json).unwrap();

    let input_payload = json!({
        "toolCall": {
            "name": "run_command",
            "args": {
                "CommandLine": "git status && rm -rf /"
            }
        }
    });

    let mut cmd = Command::cargo_bin("rulette").unwrap();
    let assert = cmd
        .arg("hook-eval")
        .arg("--policy")
        .arg(policy_file.path())
        .write_stdin(serde_json::to_string(&input_payload).unwrap())
        .assert();

    let output = assert.success().get_output().stdout.clone();
    let resp: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(resp["decision"], "deny");
}

#[test]
fn test_hook_eval_cursor_allow_and_deny_exit_code_2() {
    let policy_json = json!({
        "default_mode": "ask",
        "rules": [
            { "tool": "bash", "pattern": "npm test", "action": "allow" },
            { "tool": "bash", "pattern": "rm *", "action": "deny" }
        ]
    });
    let mut policy_file = NamedTempFile::new().unwrap();
    serde_json::to_writer(&mut policy_file, &policy_json).unwrap();

    // Allow case: exit code 0
    let allow_payload = json!({
        "command": "npm test"
    });
    let mut cmd_allow = Command::cargo_bin("rulette").unwrap();
    let assert_allow = cmd_allow
        .arg("hook-eval")
        .arg("--policy")
        .arg(policy_file.path())
        .write_stdin(serde_json::to_string(&allow_payload).unwrap())
        .assert();
    let output_allow = assert_allow.success().get_output().stdout.clone();
    let resp_allow: serde_json::Value = serde_json::from_slice(&output_allow).unwrap();
    assert_eq!(resp_allow["permission"], "allow");

    // Deny case: exit code 2
    let deny_payload = json!({
        "command": "rm -rf /tmp/cache"
    });
    let mut cmd_deny = Command::cargo_bin("rulette").unwrap();
    let assert_deny = cmd_deny
        .arg("hook-eval")
        .arg("--policy")
        .arg(policy_file.path())
        .write_stdin(serde_json::to_string(&deny_payload).unwrap())
        .assert();
    let output_deny = assert_deny.code(2).get_output().stdout.clone();
    let resp_deny: serde_json::Value = serde_json::from_slice(&output_deny).unwrap();
    assert_eq!(resp_deny["permission"], "deny");
}

#[test]
fn test_hook_eval_claude_codex_format() {
    let policy_json = json!({
        "default_mode": "ask",
        "rules": [
            { "tool": "bash", "pattern": "cargo test", "action": "allow" },
            { "tool": "bash", "pattern": "git push *", "action": "ask" },
            { "tool": "bash", "pattern": "rm *", "action": "deny" }
        ]
    });
    let mut policy_file = NamedTempFile::new().unwrap();
    serde_json::to_writer(&mut policy_file, &policy_json).unwrap();

    // Allow
    let allow_payload = json!({
        "tool_name": "Bash",
        "tool_input": {
            "command": "cargo test"
        }
    });
    let mut cmd_allow = Command::cargo_bin("rulette").unwrap();
    let assert_allow = cmd_allow
        .arg("hook-eval")
        .arg("--policy")
        .arg(policy_file.path())
        .write_stdin(serde_json::to_string(&allow_payload).unwrap())
        .assert();
    let output_allow = assert_allow.success().get_output().stdout.clone();
    let resp_allow: serde_json::Value = serde_json::from_slice(&output_allow).unwrap();
    assert_eq!(
        resp_allow["hookSpecificOutput"]["permissionDecision"],
        "allow"
    );

    // Ask
    let ask_payload = json!({
        "tool_name": "Bash",
        "tool_input": {
            "command": "git push origin main"
        }
    });
    let mut cmd_ask = Command::cargo_bin("rulette").unwrap();
    let assert_ask = cmd_ask
        .arg("hook-eval")
        .arg("--policy")
        .arg(policy_file.path())
        .write_stdin(serde_json::to_string(&ask_payload).unwrap())
        .assert();
    let output_ask = assert_ask.success().get_output().stdout.clone();
    let resp_ask: serde_json::Value = serde_json::from_slice(&output_ask).unwrap();
    assert_eq!(resp_ask["hookSpecificOutput"]["permissionDecision"], "ask");

    // Deny
    let deny_payload = json!({
        "tool_name": "Bash",
        "tool_input": {
            "command": "rm -rf /"
        }
    });
    let mut cmd_deny = Command::cargo_bin("rulette").unwrap();
    let assert_deny = cmd_deny
        .arg("hook-eval")
        .arg("--policy")
        .arg(policy_file.path())
        .write_stdin(serde_json::to_string(&deny_payload).unwrap())
        .assert();
    let output_deny = assert_deny.success().get_output().stdout.clone();
    let resp_deny: serde_json::Value = serde_json::from_slice(&output_deny).unwrap();
    assert_eq!(
        resp_deny["hookSpecificOutput"]["permissionDecision"],
        "deny"
    );
}
