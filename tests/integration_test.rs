mod cli_tests;

#[cfg(test)]
mod data_input_tests {
    use std::path::Path;

    #[test]
    fn test_claude_code_fixture_is_available() {
        let dir = std::env::var("FIXTURE_CLAUDE_CODE_DIR")
            .unwrap_or_else(|_| env!("FIXTURE_CLAUDE_CODE_DIR").to_string());
        let path = Path::new(&dir);
        assert!(path.exists(), "Claude code fixture directory should exist");
        assert!(
            path.join("README.md").exists(),
            "Claude code fixture should contain README.md"
        );
    }

    #[test]
    fn test_conductor_fixture_is_available() {
        let dir = std::env::var("FIXTURE_CONDUCTOR_DIR")
            .unwrap_or_else(|_| env!("FIXTURE_CONDUCTOR_DIR").to_string());
        let path = Path::new(&dir);
        assert!(path.exists(), "Conductor fixture directory should exist");
    }

    #[test]
    fn test_agency_agents_fixture_is_available() {
        let dir = std::env::var("FIXTURE_AGENCY_AGENTS_DIR")
            .unwrap_or_else(|_| env!("FIXTURE_AGENCY_AGENTS_DIR").to_string());
        let path = Path::new(&dir);
        assert!(
            path.exists(),
            "Agency agents fixture directory should exist"
        );
    }
}
