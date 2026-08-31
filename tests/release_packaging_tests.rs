#[test]
fn release_packaging_tasks_keep_the_static_linux_artifact_contract() {
    let mise = std::fs::read_to_string("mise.toml").unwrap();
    for required in [
        "[tasks.\"release:package\"]",
        "[tasks.\"release:smoke\"]",
        "x86_64-unknown-linux-musl",
        "scripts/package-static-release.sh",
        "scripts/verify-static-release.sh",
    ] {
        assert!(mise.contains(required), "missing {required}");
    }
}

#[test]
fn release_workflow_smokes_the_exact_verified_artifact_before_creating_a_release() {
    let workflow = std::fs::read_to_string(".github/workflows/release.yml").unwrap();
    for required in [
        "tags: ['v*']",
        "permissions:\n  contents: write",
        "musl-tools binutils file",
        "rustup target add x86_64-unknown-linux-musl",
        "mise run check",
        "mise run spec-validate",
        "cargo llvm-cov",
        "cargo audit",
        "mise run release:package",
        "mise run release:smoke",
        "gh release create",
        "${GITHUB_REF_NAME}",
        "dist/rulette-${GITHUB_REF_NAME}-x86_64-unknown-linux-musl.tar.gz",
        "dist/rulette-${GITHUB_REF_NAME}-x86_64-unknown-linux-musl.tar.gz.sha256",
        "v${package_version}",
    ] {
        assert!(workflow.contains(required), "missing {required}");
    }
    assert!(
        !workflow.contains("dist/*.tar.gz"),
        "release upload must name the verified archive exactly"
    );
}

#[test]
fn package_script_builds_locked_musl_and_writes_archive_checksum() {
    let script = std::fs::read_to_string("scripts/package-static-release.sh").unwrap();
    for required in [
        "set -euo pipefail",
        "cargo build --locked --release --target x86_64-unknown-linux-musl",
        "tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner",
        "gzip -n",
        "sha256sum",
        "rulette-v${release_version}-x86_64-unknown-linux-musl.tar.gz",
    ] {
        assert!(script.contains(required), "missing {required}");
    }
}

#[test]
fn smoke_script_requires_checksum_static_linkage_and_runtime_commands() {
    let script = std::fs::read_to_string("scripts/verify-static-release.sh").unwrap();
    for required in [
        "snapshot_archive",
        "snapshot_checksum",
        "cp --",
        "cmp --silent",
        "sha256sum --check",
        "tar -tzf",
        "file --brief",
        "readelf --dynamic",
        "ldd",
        "--version",
        "schema --to graph",
    ] {
        assert!(script.contains(required), "missing {required}");
    }
}

#[cfg(unix)]
fn verifier_script() -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap()
        .join("scripts/verify-static-release.sh")
}

#[cfg(unix)]
fn write_checksum(archive: &std::path::Path, archive_name: &str) {
    use std::process::Command;

    let checksum = Command::new("sha256sum").arg(archive).output().unwrap();
    assert!(checksum.status.success());
    let checksum = String::from_utf8(checksum.stdout)
        .unwrap()
        .replace(archive.to_str().unwrap(), archive_name);
    std::fs::write(format!("{}.sha256", archive.display()), checksum).unwrap();
}

#[cfg(unix)]
fn assert_verifier_rejects(archive: &std::path::Path) {
    use std::process::Command;

    assert!(!Command::new(verifier_script())
        .arg(archive)
        .status()
        .unwrap()
        .success());
}

#[cfg(unix)]
#[test]
fn smoke_script_rejects_checksum_mismatch() {
    let temporary = tempfile::tempdir().unwrap();
    let archive = temporary.path().join("rulette.tar.gz");
    std::fs::write(&archive, "not an archive").unwrap();
    std::fs::write(
        format!("{}.sha256", archive.display()),
        "0  rulette.tar.gz\n",
    )
    .unwrap();

    assert_verifier_rejects(&archive);
}

#[cfg(unix)]
#[test]
fn smoke_script_rejects_a_foreign_checksum_sidecar() {
    let temporary = tempfile::tempdir().unwrap();
    let archive = temporary.path().join("rulette.tar.gz");
    std::fs::write(&archive, "not an archive").unwrap();
    write_checksum(&archive, "different.tar.gz");

    assert_verifier_rejects(&archive);
}

#[cfg(unix)]
#[test]
fn smoke_script_rejects_a_malformed_checksum_sidecar() {
    let temporary = tempfile::tempdir().unwrap();
    let archive = temporary.path().join("rulette.tar.gz");
    std::fs::write(&archive, "not an archive").unwrap();
    std::fs::write(format!("{}.sha256", archive.display()), "not a checksum\n").unwrap();

    assert_verifier_rejects(&archive);
}

#[cfg(unix)]
#[test]
fn smoke_script_rejects_an_extra_archive_member() {
    use std::process::Command;

    let temporary = tempfile::tempdir().unwrap();
    let archive = temporary.path().join("rulette.tar.gz");
    std::fs::write(temporary.path().join("rulette"), "binary").unwrap();
    std::fs::write(temporary.path().join("extra"), "extra").unwrap();
    assert!(Command::new("tar")
        .args(["-C", temporary.path().to_str().unwrap(), "-czf"])
        .arg(&archive)
        .args(["rulette", "extra"])
        .status()
        .unwrap()
        .success());
    write_checksum(&archive, "rulette.tar.gz");

    assert_verifier_rejects(&archive);
}

#[cfg(unix)]
#[test]
fn smoke_script_rejects_a_traversal_archive_member() {
    use std::process::Command;

    let temporary = tempfile::tempdir().unwrap();
    let archive = temporary.path().join("rulette.tar.gz");
    std::fs::write(temporary.path().join("rulette"), "binary").unwrap();
    assert!(Command::new("tar")
        .args([
            "-C",
            temporary.path().to_str().unwrap(),
            "--transform=s#rulette#../rulette#",
            "-czf",
        ])
        .arg(&archive)
        .arg("rulette")
        .status()
        .unwrap()
        .success());
    write_checksum(&archive, "rulette.tar.gz");

    assert_verifier_rejects(&archive);
}

#[cfg(unix)]
#[test]
fn smoke_script_rejects_a_symlink_archive_member() {
    use std::os::unix::fs::symlink;
    use std::process::Command;

    let temporary = tempfile::tempdir().unwrap();
    let archive = temporary.path().join("rulette.tar.gz");
    std::fs::write(temporary.path().join("target"), "binary").unwrap();
    symlink("target", temporary.path().join("rulette")).unwrap();
    assert!(Command::new("tar")
        .args(["-C", temporary.path().to_str().unwrap(), "-czf"])
        .arg(&archive)
        .arg("rulette")
        .status()
        .unwrap()
        .success());
    write_checksum(&archive, "rulette.tar.gz");

    assert_verifier_rejects(&archive);
}

#[cfg(unix)]
#[test]
fn smoke_script_rejects_a_dynamic_elf_archive() {
    use std::process::Command;

    let temporary = tempfile::tempdir().unwrap();
    let archive = temporary.path().join("rulette.tar.gz");
    let binary = std::path::Path::new(env!("CARGO_BIN_EXE_rulette"));
    let script = std::env::current_dir()
        .unwrap()
        .join("scripts/verify-static-release.sh");

    assert!(Command::new("tar")
        .args(["-C", binary.parent().unwrap().to_str().unwrap(), "-czf"])
        .arg(&archive)
        .arg(binary.file_name().unwrap())
        .status()
        .unwrap()
        .success());
    let checksum = Command::new("sha256sum").arg(&archive).output().unwrap();
    assert!(checksum.status.success());
    let checksum = String::from_utf8(checksum.stdout)
        .unwrap()
        .replace(archive.to_str().unwrap(), "rulette.tar.gz");
    std::fs::write(format!("{}.sha256", archive.display()), checksum).unwrap();

    assert!(!Command::new(script)
        .arg(archive)
        .status()
        .unwrap()
        .success());
}

#[cfg(unix)]
#[test]
fn package_script_writes_a_verified_executable_archive_and_cleans_staging() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let temporary = tempfile::tempdir().unwrap();
    let repository = temporary.path().join("repository");
    let scripts = repository.join("scripts");
    let tools = temporary.path().join("tools");
    let caller = temporary.path().join("caller");
    std::fs::create_dir_all(&scripts).unwrap();
    std::fs::create_dir_all(&tools).unwrap();
    std::fs::create_dir_all(&caller).unwrap();

    let script_path = scripts.join("package-static-release.sh");
    std::fs::copy("scripts/package-static-release.sh", &script_path).unwrap();
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

    let cargo_path = tools.join("cargo");
    std::fs::write(
        &cargo_path,
        r#"#!/usr/bin/env bash
set -euo pipefail
[[ "$PWD" == "$EXPECTED_REPOSITORY" ]]
if [[ "$1" == "pkgid" ]]; then
    printf '%s\n' 'rulette@0.1.0'
    exit 0
fi
mkdir -p target/x86_64-unknown-linux-musl/release
printf '%s\n' 'fixture binary' > target/x86_64-unknown-linux-musl/release/rulette
chmod 0644 target/x86_64-unknown-linux-musl/release/rulette
"#,
    )
    .unwrap();
    std::fs::set_permissions(&cargo_path, std::fs::Permissions::from_mode(0o755)).unwrap();

    let path = format!("{}:{}", tools.display(), std::env::var("PATH").unwrap());
    let output = Command::new(&script_path)
        .current_dir(&caller)
        .env("PATH", &path)
        .env("EXPECTED_REPOSITORY", &repository)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let archive = repository.join("dist/rulette-v0.1.0-x86_64-unknown-linux-musl.tar.gz");
    let checksum = repository.join("dist/rulette-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256");
    assert!(archive.is_file());
    assert!(checksum.is_file());
    assert!(Command::new("sha256sum")
        .arg("-c")
        .arg(checksum.file_name().unwrap())
        .current_dir(archive.parent().unwrap())
        .status()
        .unwrap()
        .success());
    assert!(
        Command::new("tar")
            .args(["-tzf", archive.to_str().unwrap()])
            .output()
            .unwrap()
            .stdout
            == b"rulette\n"
    );
    let mode = Command::new("tar")
        .args(["-tvzf", archive.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(mode.status.success());
    assert!(String::from_utf8_lossy(&mode.stdout).starts_with("-rwxr-xr-x"));
    assert!(std::fs::read_dir(archive.parent().unwrap())
        .unwrap()
        .all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("package-root")
        }));

    let tar_path = tools.join("tar");
    std::fs::write(&tar_path, "#!/usr/bin/env bash\nexit 1\n").unwrap();
    std::fs::set_permissions(&tar_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(&archive, "stale archive").unwrap();
    std::fs::write(&checksum, "stale checksum\n").unwrap();
    let output = Command::new(&script_path)
        .current_dir(&caller)
        .env("PATH", &path)
        .env("EXPECTED_REPOSITORY", &repository)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!archive.exists());
    assert!(!checksum.exists());
    assert!(std::fs::read_dir(archive.parent().unwrap())
        .unwrap()
        .all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("package-root")
        }));
}
