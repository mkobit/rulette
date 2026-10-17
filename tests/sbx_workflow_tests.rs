#![cfg(unix)]

use std::fs;
use std::io::Write;
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

struct Runtime {
    root: tempfile::TempDir,
    bin: PathBuf,
    checkout: PathBuf,
    scratch: PathBuf,
}

fn executable(path: &Path, script: &str) {
    fs::write(path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

impl Runtime {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let bin = root.path().join("bin");
        let checkout = root.path().join("shared checkout");
        let scratch = root.path().join("scratch");
        for directory in [&bin, &checkout, &scratch] {
            fs::create_dir(directory).unwrap();
        }
        fs::create_dir(checkout.join(".sbx")).unwrap();
        fs::write(checkout.join("Cargo.toml"), "").unwrap();
        fs::write(checkout.join("AGENTS.md"), "").unwrap();
        for tool in [
            "mise", "cargo", "rustc", "bun", "bd", "file", "cc", "musl-gcc", "git", "rg",
        ] {
            executable(&bin.join(tool), "#!/bin/sh\nprintf 'mock version\\n'\n");
        }
        executable(
            &bin.join("rustup"),
            "#!/bin/sh\nprintf 'llvm-tools-x86_64-unknown-linux-gnu\\n'\n",
        );
        let agent_script = r#"#!/bin/sh
set -eu
if [ "$1" = --version ]; then
    printf 'mock agent version\n'
    exit 0
fi
case "${0##*/}" in
    codex)
        [ "$#" -eq 7 ]
        [ "$1" = exec ] && [ "$2" = --model ] && [ "$3" = "$MOCK_MODEL" ]
        [ "$4" = --disable ] && [ "$5" = hooks ]
        [ "$6" = --dangerously-bypass-approvals-and-sandbox ]
        prompt=$7
        ;;
    agy)
        [ "$#" -eq 5 ]
        [ "$1" = --model ] && [ "$2" = "$MOCK_MODEL" ]
        [ "$3" = --dangerously-skip-permissions ] && [ "$4" = --print ]
        prompt=$5
        ;;
esac
[ "$prompt" = "$MOCK_PROMPT" ]
if [ "${MOCK_REQUIRE_EOF:-0}" = 1 ]; then
    if IFS= read -r unexpected_input; then
        printf 'expected EOF on stdin\n' >&2
        exit 43
    fi
fi
if [ "${MOCK_EXIT:-0}" -ne 0 ]; then
    printf 'mock dispatch failed\n' >&2
    exit "$MOCK_EXIT"
fi
printf '%s' "$prompt" > result.txt
printf 'changed=result.txt checks=mock-only blockers=none\n'
"#;
        for agent in ["codex", "agy"] {
            executable(&bin.join(agent), agent_script);
        }
        executable(
            &bin.join("sbx"),
            r#"#!/bin/sh
set -eu
[ "$1" = env ] && [ "$2" = exec ] && [ "$4" = -- ]
case "$3" in
    .sbx/sbxenv.yaml|.sbx/sbxenv.agy.yaml) ;;
    *) exit 2 ;;
esac
shift 4
exec "$@"
"#,
        );
        Self {
            root,
            bin,
            checkout,
            scratch,
        }
    }

    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(&self.checkout)
            .env_remove("BASH_ENV")
            .env_remove("ENV")
            .env("PATH", format!("{}:/usr/bin:/bin", self.bin.display()))
            .env("TMPDIR", &self.scratch);
        command
    }

    fn preflight(&self, agent: &str, expected: &Path) -> Output {
        self.command("sh")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join(".sbx/kit/check-runtime.sh"))
            .arg(agent)
            .arg(expected)
            .output()
            .unwrap()
    }

    fn assert_clean(&self) {
        assert_eq!(fs::read_dir(&self.scratch).unwrap().count(), 0);
    }
}

#[test]
fn mocked_dispatch_covers_both_agents_from_preflight_through_handoff() {
    for agent in ["codex", "agy"] {
        let runtime = Runtime::new();
        let preflight = runtime.preflight(agent, &runtime.checkout);
        assert!(
            preflight.status.success(),
            "{}",
            String::from_utf8_lossy(&preflight.stderr)
        );
        assert!(String::from_utf8_lossy(&preflight.stdout).contains("SBX preflight passed"));
        runtime.assert_clean();

        let prompt = "one prompt with spaces\nand $(not-a-command)";
        let mut command = runtime.command("sbx");
        command.args([
            "env",
            "exec",
            if agent == "codex" {
                ".sbx/sbxenv.yaml"
            } else {
                ".sbx/sbxenv.agy.yaml"
            },
            "--",
            agent,
        ]);
        if agent == "codex" {
            command.args([
                "exec",
                "--model",
                "mock-model",
                "--disable",
                "hooks",
                "--dangerously-bypass-approvals-and-sandbox",
            ]);
        } else {
            command.args([
                "--model",
                "mock-model",
                "--dangerously-skip-permissions",
                "--print",
            ]);
        }
        command
            .arg(prompt)
            .env("MOCK_MODEL", "mock-model")
            .env("MOCK_PROMPT", prompt);
        let output = command.output().unwrap();
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("changed=result.txt"));
        assert_eq!(
            fs::read_to_string(runtime.checkout.join("result.txt")).unwrap(),
            prompt
        );
        fs::remove_file(runtime.checkout.join("result.txt")).unwrap();

        let failed = command.env("MOCK_EXIT", "42").output().unwrap();
        assert_eq!(failed.status.code(), Some(42));
        assert!(String::from_utf8_lossy(&failed.stderr).contains("mock dispatch failed"));
        assert!(!runtime.checkout.join("result.txt").exists());
        runtime.assert_clean();
    }
}

#[test]
fn documented_codex_stdin_fallback_preserves_arguments_eof_and_status() {
    let guidance = fs::read_to_string("AGENTS.md").unwrap();
    let workflow = fs::read_to_string("docs/2026-10-15-sbx-delegation.md").unwrap();
    let fallback = workflow
        .lines()
        .find(|line| line.starts_with("sbx env exec .sbx/sbxenv.yaml -- sh -c "))
        .unwrap();
    assert!(guidance.lines().any(|line| line == fallback));

    for status in [0, 42] {
        let runtime = Runtime::new();
        let model = "mock model 'quoted' $(not-a-command)";
        let prompt = "one prompt with 'quotes' and \"quotes\"\nand $(not-a-command); $HOME";
        let mut child = runtime
            .command("sh")
            .args(["-c", fallback])
            .env("model", model)
            .env("prompt", prompt)
            .env("MOCK_MODEL", model)
            .env("MOCK_PROMPT", prompt)
            .env("MOCK_EXIT", status.to_string())
            .env("MOCK_REQUIRE_EOF", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        input.write_all(b"host stdin remains open\n").unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(status),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        if status == 0 {
            assert_eq!(
                fs::read_to_string(runtime.checkout.join("result.txt")).unwrap(),
                prompt
            );
        } else {
            assert!(String::from_utf8_lossy(&output.stderr).contains("mock dispatch failed"));
            assert!(!runtime.checkout.join("result.txt").exists());
        }
        runtime.assert_clean();
    }
}

#[test]
fn runtime_rejects_wrong_workspace_agent_and_broken_tool() {
    let runtime = Runtime::new();
    let wrong = runtime.preflight("codex", runtime.root.path());
    assert!(!wrong.status.success());
    assert!(String::from_utf8_lossy(&wrong.stderr).contains("Workspace mismatch"));
    let unknown = runtime.preflight("unknown", &runtime.checkout);
    assert!(!unknown.status.success());
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("Unsupported agent"));
    let relative = runtime.preflight("codex", Path::new("."));
    assert!(!relative.status.success());
    assert!(String::from_utf8_lossy(&relative.stderr).contains("must be absolute"));
    executable(&runtime.bin.join("cargo"), "#!/bin/sh\nexit 23\n");
    let broken = runtime.preflight("codex", &runtime.checkout);
    assert!(!broken.status.success());
    assert!(String::from_utf8_lossy(&broken.stderr).contains("Cannot execute: cargo"));
    runtime.assert_clean();
}

#[test]
fn runtime_reports_missing_tools_and_components() {
    let runtime = Runtime::new();
    fs::remove_file(runtime.bin.join("agy")).unwrap();
    let missing = runtime.preflight("agy", &runtime.checkout);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("Missing tool: agy"));
    executable(&runtime.bin.join("rustup"), "#!/bin/sh\nexit 0\n");
    let component = runtime.preflight("codex", &runtime.checkout);
    assert!(!component.status.success());
    assert!(String::from_utf8_lossy(&component.stderr).contains("Missing Rust llvm-tools"));
    executable(
        &runtime.bin.join("rustup"),
        "#!/bin/sh\n[ \"$1\" = --version ] && exit 0\nprintf 'llvm-tools-x86_64-unknown-linux-gnu\\n'\nexit 9\n",
    );
    let failed_list = runtime.preflight("codex", &runtime.checkout);
    assert!(!failed_list.status.success());
    assert!(String::from_utf8_lossy(&failed_list.stderr)
        .contains("Cannot list installed Rust components"));
    runtime.assert_clean();
}

#[test]
fn runtime_cleans_partial_setup_and_interruption() {
    for (script, expected_status) in [
        ("#!/bin/sh\nexit 17\n", 17),
        ("#!/bin/sh\nkill -TERM \"$PPID\"\n", 143),
    ] {
        let runtime = Runtime::new();
        fs::write(runtime.scratch.join("keep"), "existing data").unwrap();
        executable(&runtime.bin.join("chmod"), script);
        let output = runtime.preflight("codex", &runtime.checkout);
        assert_eq!(output.status.code(), Some(expected_status));
        assert_eq!(fs::read_dir(&runtime.scratch).unwrap().count(), 1);
        assert_eq!(
            fs::read_to_string(runtime.scratch.join("keep")).unwrap(),
            "existing data"
        );
    }
}

#[test]
fn runtime_reports_cleanup_failure_without_hiding_original_status() {
    for original_status in [0, 17] {
        let runtime = Runtime::new();
        executable(&runtime.bin.join("rm"), "#!/bin/sh\nexit 19\n");
        if original_status != 0 {
            executable(&runtime.bin.join("chmod"), "#!/bin/sh\nexit 17\n");
        }
        let output = runtime.preflight("codex", &runtime.checkout);
        assert_eq!(output.status.code(), Some(original_status.max(1)));
        assert!(String::from_utf8_lossy(&output.stderr).contains("cleanup failed"));
    }
}

#[test]
fn runtime_reports_temporary_directory_setup_failure() {
    let runtime = Runtime::new();
    executable(&runtime.bin.join("mktemp"), "#!/bin/sh\nexit 11\n");
    let output = runtime.preflight("codex", &runtime.checkout);
    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Cannot create temporary directory"));
    runtime.assert_clean();
}

#[test]
fn kit_path_setup_executes_tools_is_idempotent_and_preserves_collisions() {
    let runtime = Runtime::new();
    let kit: serde_yaml::Value =
        serde_yaml::from_str(&fs::read_to_string(".sbx/kit/spec.yaml").unwrap()).unwrap();
    let install = kit["setup"]["install"].as_sequence().unwrap();
    let setup = install.last().unwrap()["command"]
        .as_str()
        .unwrap()
        .replace("/usr/local/bin", runtime.bin.to_str().unwrap())
        .replace(
            "/home/agent/.local/share/mise/shims",
            runtime.root.path().join("shims").to_str().unwrap(),
        );
    let tools = [
        "bun",
        "bd",
        "cargo",
        "cargo-clippy",
        "cargo-fmt",
        "rustc",
        "rustfmt",
        "rustup",
    ];
    fs::create_dir(runtime.root.path().join("shims")).unwrap();
    for tool in tools {
        let target = runtime.bin.join(tool);
        if target.exists() {
            fs::remove_file(&target).unwrap();
        }
        symlink(
            runtime.bin.join("mise"),
            runtime.root.path().join("shims").join(tool),
        )
        .unwrap();
    }
    for _attempt in 0..2 {
        let output = runtime.command("sh").args(["-c", &setup]).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        for tool in tools {
            assert!(runtime
                .command(tool)
                .arg("--version")
                .status()
                .unwrap()
                .success());
        }
    }
    fs::remove_file(runtime.bin.join("cargo")).unwrap();
    executable(&runtime.bin.join("cargo"), "#!/bin/sh\nexit 29\n");
    let collision = runtime.command("sh").args(["-c", &setup]).output().unwrap();
    assert!(!collision.status.success());
    assert!(String::from_utf8_lossy(&collision.stderr).contains("Refuse to replace"));
    assert_eq!(runtime.command("cargo").status().unwrap().code(), Some(29));
}

#[test]
fn environment_contract_keeps_shared_checkout_without_host_oauth_import() {
    for path in [".sbx/sbxenv.yaml", ".sbx/sbxenv.agy.yaml"] {
        let environment: serde_yaml::Value =
            serde_yaml::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(environment["workspace"]["clone"].as_bool(), Some(false));
        assert_eq!(environment["workspace"]["path"].as_str(), Some(".."));
        assert!(environment.get("additionalWorkspaces").is_none());
    }
    let kit = fs::read_to_string(".sbx/kit/spec.yaml").unwrap();
    assert!(kit.contains("build-essential file musl-tools ripgrep"));
    assert!(kit.contains("rustup component add rustfmt clippy llvm-tools"));
    assert!(!kit.contains("antigravity-oauth-token"));
    let mise = fs::read_to_string("mise.toml").unwrap();
    let configuration: toml::Value = toml::from_str(&mise).unwrap();
    let check = configuration["tasks"]["check"].to_string();
    assert!(!check.contains("sbx"));
}
