mod cli_tests;

#[cfg(test)]
mod main_tests {
    use assert_cmd::prelude::*;
    use std::process::Command;

    #[test]
    fn test_main_schema_command() {
        let mut cmd = Command::cargo_bin("rulette").unwrap();
        cmd.arg("schema");
        cmd.assert().success();
    }

    #[test]
    fn test_main_schema_all_transform_targets() {
        for format in [
            "claude",
            "cursor-mdc",
            "cursor-mcp",
            "codex",
            "windsurf",
            "copilot",
            "gemini",
            "agent-skills",
        ] {
            let mut cmd = Command::cargo_bin("rulette").unwrap();
            cmd.arg("schema").arg("--to").arg(format);
            cmd.assert()
                .success()
                .stdout(predicates::str::contains("$schema"));
        }
    }

    #[test]
    fn test_main_schema_all_extension_keys() {
        for key in [
            "rulette:activation",
            "rulette:hook-event",
            "rulette:tool-access",
            "rulette:agent-tools",
            "rulette:models",
            "rulette:directory-scope",
            "rulette:settings-overrides",
        ] {
            let mut cmd = Command::cargo_bin("rulette").unwrap();
            cmd.arg("schema").arg("--extension").arg(key);
            cmd.assert()
                .success()
                .stdout(predicates::str::contains("$schema"));
        }
    }

    #[test]
    fn test_main_schema_activation_covers_bare_and_wrapped() {
        let mut cmd = Command::cargo_bin("rulette").unwrap();
        cmd.arg("schema").arg("--extension").arg("rulette:activation");
        let assert = cmd.assert().success();
        let output = assert.get_output();
        let json_str = String::from_utf8(output.stdout.clone()).unwrap();
        let val: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Verify JSON Schema structure
        assert!(val.get("$schema").is_some());
        assert_eq!(val.get("title").and_then(|v| v.as_str()), Some("TargetOverrides"));

        // Verify anyOf contains wrapped object (default + overrides) and bare Activation ref
        let any_of = val.get("anyOf").and_then(|v| v.as_array()).expect("expected anyOf array");
        assert_eq!(any_of.len(), 2);
        assert!(val.get("$defs").and_then(|d| d.get("Activation")).is_some());
    }

    #[test]
    fn test_main_inspect_command() {
        let mut cmd = Command::cargo_bin("rulette").unwrap();
        cmd.arg("inspect").arg("-");

        use std::io::Write;
        let mut child = cmd.stdin(std::process::Stdio::piped()).spawn().unwrap();
        let mut stdin = child.stdin.take().unwrap();
        std::thread::spawn(move || {
            stdin.write_all(b"{\"entities\": []}").unwrap();
        });

        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
    }

    #[test]
    fn test_main_inspect_dry_run_command() {
        let mut cmd = Command::cargo_bin("rulette").unwrap();
        cmd.arg("inspect").arg("-").arg("--to").arg("claude");

        use std::io::Write;
        let mut child = cmd.stdin(std::process::Stdio::piped()).spawn().unwrap();
        let mut stdin = child.stdin.take().unwrap();
        std::thread::spawn(move || {
            stdin.write_all(b"{\"entities\": []}").unwrap();
        });

        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
    }

    #[test]
    fn test_main_inspect_quiet_suppresses_output() {
        let mut cmd = Command::cargo_bin("rulette").unwrap();
        cmd.arg("-q")
            .arg("inspect")
            .arg("-")
            .arg("--to")
            .arg("claude");

        use std::io::Write;
        let mut child = cmd.stdin(std::process::Stdio::piped()).spawn().unwrap();
        let mut stdin = child.stdin.take().unwrap();
        std::thread::spawn(move || {
            stdin.write_all(b"{\"entities\": []}").unwrap();
        });

        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        assert!(output.stdout.is_empty());
    }
}

#[cfg(test)]
mod data_input_tests {
    use std::path::Path;

    #[test]
    fn test_claude_code_fixture_is_available() {
        let dir = env!("FIXTURE_CLAUDE_CODE_DIR");
        let path = Path::new(dir);
        assert!(path.exists(), "Claude code fixture directory should exist");
        assert!(
            path.join("README.md").exists(),
            "Claude code fixture should contain README.md"
        );
    }

    #[test]
    fn test_conductor_fixture_is_available() {
        let dir = env!("FIXTURE_CONDUCTOR_DIR");
        let path = Path::new(dir);
        assert!(path.exists(), "Conductor fixture directory should exist");
    }

    #[test]
    fn test_agency_agents_fixture_is_available() {
        let dir = env!("FIXTURE_AGENCY_AGENTS_DIR");
        let path = Path::new(dir);
        assert!(
            path.exists(),
            "Agency agents fixture directory should exist"
        );
    }

    #[test]
    fn test_mattpocock_skills_fixture_is_available() {
        let dir = env!("FIXTURE_MATTPOCOCK_SKILLS_DIR");
        let path = Path::new(dir);
        assert!(
            path.exists(),
            "Matt Pocock skills fixture directory should exist"
        );
    }
}
