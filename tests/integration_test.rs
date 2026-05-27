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
