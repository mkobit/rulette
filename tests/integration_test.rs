mod cli_tests;

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
}
