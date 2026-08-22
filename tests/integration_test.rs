mod cli_tests;

#[cfg(test)]
mod main_tests {
    use assert_cmd::prelude::*;
    use std::process::Command;

    #[test]
    fn schema_defaults_to_the_compilation_graph() {
        let mut cmd = Command::cargo_bin("rulette").unwrap();
        cmd.arg("schema");
        cmd.assert()
            .success()
            .stdout(predicates::str::contains("CompilationGraph"));
    }

    #[test]
    fn inspect_accepts_a_core_native_tree() {
        let mut cmd = Command::cargo_bin("rulette").unwrap();
        cmd.arg("-q")
            .arg("inspect")
            .arg("tests/fixtures/v0_1/codex")
            .arg("--to")
            .arg("claude");
        cmd.assert().success().stdout(predicates::str::is_empty());
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
