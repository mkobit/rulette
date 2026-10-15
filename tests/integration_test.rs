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

#[cfg(test)]
mod docker_sandbox_tests {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    #[test]
    fn docker_sandbox_guidance_documents_direct_agent_invocation_and_environment_semantics() {
        let content = fs::read_to_string("AGENTS.md").expect("failed to read AGENTS.md");
        let sandbox_section = content
            .split("## Docker sandbox execution")
            .nth(1)
            .expect("missing Docker sandbox execution section")
            .split("## Inputs")
            .next()
            .expect("missing Inputs section");

        assert!(
            sandbox_section.contains(
                "sbx env exec .sbx/sbxenv.yaml -- codex exec --model <model> \"<prompt>\""
            ),
            "Codex delegation must document direct sbx env exec with explicit --model"
        );
        assert!(
            sandbox_section.contains("sbx env exec .sbx/sbxenv.agy.yaml -- agy --model <model> --dangerously-skip-permissions --print \"<prompt>\""),
            "AGY delegation must document direct sbx env exec with explicit --model and --dangerously-skip-permissions"
        );
        assert!(
            sandbox_section.contains("configures the sandbox environment integration")
                && sandbox_section.contains("does not launch the agent automatically")
                && sandbox_section.contains("sbx env exec` command runs only the explicit command"),
            "Guidance must distinguish agent environment configuration from command launching"
        );
        assert!(
            sandbox_section.contains("Reserve shell wrappers (`sh -c` or `sh -lc`) only for commands that require shell syntax or login-shell PATH setup."),
            "Guidance must reserve shell wrappers for commands requiring shell syntax or login-shell PATH"
        );
        assert!(
            !sandbox_section.contains("sh -c 'codex"),
            "Codex invocation must not use sh -c wrapper"
        );
        assert!(
            !sandbox_section.contains("sh -c 'agy"),
            "AGY invocation must not use sh -c wrapper"
        );
        assert!(
            !sandbox_section.contains("< /dev/null"),
            "Direct agent invocations must not redirect /dev/null"
        );
    }

    #[test]
    #[cfg(unix)]
    fn docker_sandbox_direct_invocation_smoke_returns_predictably() {
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        let bin_dir = temp_dir.path().join("bin");
        fs::create_dir_all(&bin_dir).expect("failed to create bin dir");

        let make_executable = |name: &str, script: &str| {
            let path = bin_dir.join(name);
            fs::write(&path, script).expect("failed to write script");
            let mut perms = fs::metadata(&path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms).expect("failed to set permissions");
        };

        // Mock sbx: dispatches direct execution after '--' without shell wrappers
        make_executable(
            "sbx",
            r#"#!/bin/sh
set -eu
if [ "$1" = "env" ] && [ "$2" = "exec" ]; then
    shift 2
    env_file="$1"
    shift
    if [ "$1" != "--" ]; then
        echo "expected -- delimiter" >&2
        exit 1
    fi
    shift
    exec "$@"
fi
echo "unsupported sbx arguments: $*" >&2
exit 1
"#,
        );

        // Mock codex: validates direct CLI arguments and model flag
        make_executable(
            "codex",
            r#"#!/bin/sh
set -eu
if [ "$1" = "exec" ] && [ "$2" = "--model" ]; then
    echo "codex: model=$3 prompt=$4"
    exit 0
fi
echo "unexpected codex arguments: $*" >&2
exit 1
"#,
        );

        // Mock agy: validates direct CLI arguments, model flag, and permission bypass
        make_executable(
            "agy",
            r#"#!/bin/sh
set -eu
if [ "$1" = "--model" ] && [ "$3" = "--dangerously-skip-permissions" ] && [ "$4" = "--print" ]; then
    echo "agy: model=$2 prompt=$5"
    exit 0
fi
echo "unexpected agy arguments: $*" >&2
exit 1
"#,
        );

        let system_path = std::env::var("PATH").unwrap_or_default();
        let test_path = format!("{}:{}", bin_dir.display(), system_path);

        // Test direct Codex invocation
        let codex_output = Command::new(bin_dir.join("sbx"))
            .args([
                "env",
                "exec",
                ".sbx/sbxenv.yaml",
                "--",
                "codex",
                "exec",
                "--model",
                "gpt-5",
                "hello codex",
            ])
            .env_remove("BASH_ENV")
            .env("PATH", &test_path)
            .output()
            .expect("failed to run direct codex delegation");

        assert!(
            codex_output.status.success(),
            "direct codex invocation failed: {}",
            String::from_utf8_lossy(&codex_output.stderr)
        );
        let codex_stdout = String::from_utf8_lossy(&codex_output.stdout);
        assert_eq!(codex_stdout.trim(), "codex: model=gpt-5 prompt=hello codex");

        // Test direct AGY invocation
        let agy_output = Command::new(bin_dir.join("sbx"))
            .args([
                "env",
                "exec",
                ".sbx/sbxenv.agy.yaml",
                "--",
                "agy",
                "--model",
                "gemini-3.8-flash-medium",
                "--dangerously-skip-permissions",
                "--print",
                "hello agy",
            ])
            .env_remove("BASH_ENV")
            .env("PATH", &test_path)
            .output()
            .expect("failed to run direct agy delegation");

        assert!(
            agy_output.status.success(),
            "direct agy invocation failed: {}",
            String::from_utf8_lossy(&agy_output.stderr)
        );
        let agy_stdout = String::from_utf8_lossy(&agy_output.stdout);
        assert_eq!(
            agy_stdout.trim(),
            "agy: model=gemini-3.8-flash-medium prompt=hello agy"
        );
    }
}
