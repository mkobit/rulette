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
fn release_notes_document_static_portability_and_explicit_publication_safety() {
    let notes = std::fs::read_to_string("docs/releases/v0.1.0.md").unwrap();
    for required in [
        "x86_64-unknown-linux-musl",
        "fully static",
        "no runtime dependencies",
        "--stage",
        "--apply",
        "--expect-plan-sha256",
        "--allow-project-root",
        "strict by default",
        "--allow-lossy",
        "no registry",
        "no fetch subsystem",
    ] {
        assert!(notes.contains(required), "missing {required}");
    }

    assert!(
        !notes.contains("SHA256"),
        "digest example must not be a placeholder"
    );
    assert!(
        !notes.contains("..."),
        "digest example must not contain an ellipsis"
    );
    assert!(
        notes.contains("Copy the actual digest from the staging output."),
        "missing staging digest guidance"
    );
    let digest = notes
        .lines()
        .find_map(|line| {
            line.split_whitespace()
                .collect::<Vec<_>>()
                .windows(2)
                .find(|arguments| arguments[0] == "--expect-plan-sha256")
                .map(|arguments| arguments[1])
        })
        .expect("missing --expect-plan-sha256 value");
    assert!(
        digest.starts_with("sha256_"),
        "digest must have sha256_ prefix"
    );
    assert_eq!(
        digest.len(),
        71,
        "digest must contain 64 hexadecimal digits"
    );
    assert!(
        digest["sha256_".len()..]
            .chars()
            .all(|character| character.is_ascii_hexdigit()),
        "digest must contain only hexadecimal digits after sha256_"
    );
}

#[test]
fn release_workflow_smokes_the_exact_verified_artifact_before_creating_a_release() {
    let workflow = std::fs::read_to_string(".github/workflows/release.yml").unwrap();
    for required in [
        "tags: ['v*']",
        "permissions: {}",
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
fn release_workflow_orders_validation_packaging_smoke_and_publication() {
    let workflow = std::fs::read_to_string(".github/workflows/release.yml").unwrap();
    let positions = workflow_step_positions(&workflow);

    for step in [
        "Validate release tag",
        "mise run check",
        "mise run spec-validate",
        "cargo llvm-cov",
        "cargo audit",
        "mise run release:package",
    ] {
        assert!(
            positions[step] < positions["mise run release:smoke"],
            "{step} must run before the smoke test"
        );
    }
    assert!(
        positions["mise run release:smoke"] < positions["actions/upload-artifact"],
        "the smoke test must run before the verified artifact upload"
    );
    assert!(
        positions["actions/upload-artifact"] < positions["publish:"],
        "the upload must precede the dependent publish job"
    );
    assert!(
        positions["publish:"] < positions["actions/download-artifact"],
        "the publish job must download the verified artifact"
    );
    assert!(
        positions["actions/download-artifact"] < positions["gh release create"],
        "the download must precede release creation"
    );
}

fn workflow_step_positions(workflow: &str) -> std::collections::BTreeMap<&'static str, usize> {
    [
        ("Validate release tag", "Validate release tag"),
        ("mise run check", "mise run check"),
        ("mise run spec-validate", "mise run spec-validate"),
        ("cargo llvm-cov", "cargo llvm-cov"),
        ("cargo audit", "run: cargo audit"),
        ("mise run release:package", "mise run release:package"),
        ("mise run release:smoke", "mise run release:smoke"),
        ("actions/upload-artifact", "actions/upload-artifact"),
        ("publish:", "publish:"),
        ("actions/download-artifact", "actions/download-artifact"),
        ("gh release create", "gh release create"),
    ]
    .into_iter()
    .map(|(name, step)| {
        let position = workflow
            .find(step)
            .unwrap_or_else(|| panic!("missing workflow step {step}"));
        (name, position)
    })
    .collect()
}

#[test]
fn release_workflow_transfers_only_verified_artifacts_to_a_dependent_publish_job() {
    let workflow = std::fs::read_to_string(".github/workflows/release.yml").unwrap();
    let validate_job = workflow.find("  validate-package:").unwrap();
    let publish_job = workflow.find("  publish:").unwrap();
    let smoke = workflow
        .find("VERIFIED_RELEASE_DIR=verified-release")
        .unwrap();
    let upload = workflow.find("actions/upload-artifact").unwrap();
    let download = workflow.find("actions/download-artifact").unwrap();
    let release = workflow.find("gh release create").unwrap();

    assert!(validate_job < smoke);
    assert!(smoke < upload);
    assert!(upload < publish_job);
    assert!(publish_job < download);
    assert!(download < release);
    assert!(workflow[validate_job..publish_job].contains("contents: read"));
    assert!(workflow[publish_job..].contains("needs: validate-package"));
    assert!(workflow[publish_job..].contains("contents: write"));
    assert!(workflow[validate_job..publish_job].contains("verified-release/"));
    assert!(workflow[publish_job..].contains("verified-release/"));
    assert!(!workflow[publish_job..].contains("dist/"));
}

#[test]
fn package_script_builds_locked_musl_and_writes_archive_checksum() {
    let script = std::fs::read_to_string("scripts/package-static-release.sh").unwrap();
    for required in [
        "set -euo pipefail",
        "cargo build --locked --release --target x86_64-unknown-linux-musl",
        "tar --sort=name --mtime=@0 --owner=0 --group=0 --numeric-owner",
        "GNU tar",
        "gzip -n",
        "sha256sum",
        "rulette-v${release_version}-x86_64-unknown-linux-musl.tar.gz",
        "cargo-package-version.sh",
    ] {
        assert!(script.contains(required), "missing {required}");
    }
}

#[test]
fn release_entrypoints_extract_versions_from_modern_cargo_package_identifiers() {
    for path in [
        "scripts/package-static-release.sh",
        "mise.toml",
        ".github/workflows/release.yml",
    ] {
        let entrypoint = std::fs::read_to_string(path).unwrap();

        assert!(
            entrypoint.contains("cargo-package-version.sh"),
            "{path} must use the shared checked version extractor"
        );
        assert!(
            !entrypoint.contains("cargo pkgid"),
            "{path} must not independently parse cargo pkgid"
        );
    }
}

#[test]
fn markdown_lint_excludes_superpowers_scratch_directory() {
    let configuration = std::fs::read_to_string(".markdownlint-cli2.jsonc").unwrap();

    assert!(
        configuration.contains("!.superpowers"),
        "markdown lint must exclude the .superpowers scratch directory"
    );
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
        "readelf -l",
        "readelf --dynamic",
        "ldd",
        "--version",
        "schema --to graph",
    ] {
        assert!(script.contains(required), "missing {required}");
    }
}

#[cfg(target_os = "linux")]
fn verifier_script() -> std::path::PathBuf {
    std::env::current_dir()
        .unwrap()
        .join("scripts/verify-static-release.sh")
}

#[cfg(target_os = "linux")]
fn write_checksum(archive: &std::path::Path, archive_name: &str) {
    use std::process::Command;

    let checksum = Command::new("sha256sum").arg(archive).output().unwrap();
    assert!(checksum.status.success());
    let checksum = String::from_utf8(checksum.stdout)
        .unwrap()
        .replace(archive.to_str().unwrap(), archive_name);
    std::fs::write(format!("{}.sha256", archive.display()), checksum).unwrap();
}

#[cfg(target_os = "linux")]
fn write_hostile_archive(
    archive: &std::path::Path,
    member_path: &str,
    entry_type: tar::EntryType,
    link_name: Option<&str>,
) {
    use std::fs::File;
    use std::io::Cursor;

    let encoder = flate2::write::GzEncoder::new(
        File::create(archive).unwrap(),
        flate2::Compression::default(),
    );
    let mut builder = tar::Builder::new(encoder);
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(entry_type);
    header.set_mode(0o755);
    if let Some(link_name) = link_name {
        header.set_size(0);
        builder
            .append_link(&mut header, member_path, link_name)
            .unwrap();
    } else {
        header.set_size(b"binary".len() as u64);
        header.as_mut_bytes()[..member_path.len()].copy_from_slice(member_path.as_bytes());
        header.set_cksum();
        builder.append(&header, Cursor::new(b"binary")).unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap();
}

#[cfg(target_os = "linux")]
fn assert_verifier_rejects(archive: &std::path::Path) {
    use std::process::Command;

    assert!(!Command::new(verifier_script())
        .arg(archive)
        .status()
        .unwrap()
        .success());
}

#[cfg(target_os = "linux")]
#[test]
fn smoke_script_exports_the_verified_snapshot_even_if_the_source_changes_after_snapshot() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("rulette.tar.gz");
    let staging = temporary.path().join("staging");
    let tools = temporary.path().join("tools");
    let verified = temporary.path().join("verified-release");
    std::fs::create_dir_all(&staging).unwrap();
    std::fs::create_dir_all(&tools).unwrap();
    std::fs::write(staging.join("rulette"), "#!/usr/bin/env bash\nexit 0\n").unwrap();
    std::fs::set_permissions(
        staging.join("rulette"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    assert!(Command::new("tar")
        .args(["-C", staging.to_str().unwrap(), "-czf"])
        .arg(&source)
        .arg("rulette")
        .status()
        .unwrap()
        .success());
    write_checksum(&source, "rulette.tar.gz");
    let original_archive = std::fs::read(&source).unwrap();
    let original_checksum = std::fs::read(format!("{}.sha256", source.display())).unwrap();

    for (name, body) in [
        (
            "file",
            "#!/usr/bin/env bash\nprintf tampered > \"$TAMPER_ARCHIVE\"\nprintf 'tampered\\n' > \"$TAMPER_ARCHIVE.sha256\"\nprintf 'ELF 64-bit LSB executable, statically linked\\n'\n",
        ),
        ("readelf", "#!/usr/bin/env bash\nexit 0\n"),
        ("ldd", "#!/usr/bin/env bash\nprintf 'not a dynamic executable\\n'\n"),
    ] {
        let tool = tools.join(name);
        std::fs::write(&tool, body).unwrap();
        std::fs::set_permissions(tool, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    let path = format!("{}:{}", tools.display(), std::env::var("PATH").unwrap());
    let output = Command::new(verifier_script())
        .arg(&source)
        .env("PATH", path)
        .env("TAMPER_ARCHIVE", &source)
        .env("VERIFIED_RELEASE_DIR", &verified)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_ne!(std::fs::read(&source).unwrap(), original_archive);
    assert_eq!(
        std::fs::read(verified.join("rulette.tar.gz")).unwrap(),
        original_archive
    );
    assert_eq!(
        std::fs::read(verified.join("rulette.tar.gz.sha256")).unwrap(),
        original_checksum
    );
}

#[cfg(target_os = "linux")]
#[test]
fn smoke_script_accepts_static_pie_and_rejects_an_elf_interpreter() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    for (program_headers, dynamic_section, expected_success) in [
        ("Elf file type is DYN\n", "Dynamic section at offset 0x0\n", true),
        (
            "Elf file type is DYN\n  INTERP         0x000000\n",
            "Dynamic section at offset 0x0\n",
            false,
        ),
        (
            "Elf file type is DYN\n",
            "Dynamic section at offset 0x0\n 0x0000000000000001 (NEEDED) Shared library: [libc.so.6]\n",
            false,
        ),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let archive = temporary.path().join("rulette.tar.gz");
        let staging = temporary.path().join("staging");
        let tools = temporary.path().join("tools");
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::create_dir_all(&tools).unwrap();
        std::fs::write(staging.join("rulette"), "#!/usr/bin/env bash\nexit 0\n").unwrap();
        std::fs::set_permissions(
            staging.join("rulette"),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(Command::new("tar")
            .args(["-C", staging.to_str().unwrap(), "-czf"])
            .arg(&archive)
            .arg("rulette")
            .status()
            .unwrap()
            .success());
        write_checksum(&archive, "rulette.tar.gz");

        for (name, body) in [
            (
                "file",
                "#!/usr/bin/env bash\nprintf 'ELF 64-bit LSB pie executable, static-pie linked\\n'\n"
                    .to_owned(),
            ),
            (
                "readelf",
                format!(
                    "#!/usr/bin/env bash\nif [[ \"$1\" == \"-l\" ]]; then\n    printf '%s' '{}'\nelse\n    printf '%s' '{}'\nfi\n",
                    program_headers, dynamic_section
                ),
            ),
            ("ldd", "#!/usr/bin/env bash\nprintf 'statically linked\\n'\n".to_owned()),
        ] {
            let tool = tools.join(name);
            std::fs::write(tool, body).unwrap();
            std::fs::set_permissions(tools.join(name), std::fs::Permissions::from_mode(0o755))
                .unwrap();
        }

        let path = format!("{}:{}", tools.display(), std::env::var("PATH").unwrap());
        let output = Command::new(verifier_script())
            .arg(&archive)
            .env("PATH", path)
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            expected_success,
            "{program_headers}{dynamic_section}"
        );
    }
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
#[test]
fn smoke_script_rejects_a_foreign_checksum_sidecar() {
    let temporary = tempfile::tempdir().unwrap();
    let archive = temporary.path().join("rulette.tar.gz");
    std::fs::write(&archive, "not an archive").unwrap();
    write_checksum(&archive, "different.tar.gz");

    assert_verifier_rejects(&archive);
}

#[cfg(target_os = "linux")]
#[test]
fn smoke_script_rejects_a_malformed_checksum_sidecar() {
    let temporary = tempfile::tempdir().unwrap();
    let archive = temporary.path().join("rulette.tar.gz");
    std::fs::write(&archive, "not an archive").unwrap();
    std::fs::write(format!("{}.sha256", archive.display()), "not a checksum\n").unwrap();

    assert_verifier_rejects(&archive);
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
#[test]
fn smoke_script_rejects_a_traversal_archive_member() {
    let temporary = tempfile::tempdir().unwrap();
    let archive = temporary.path().join("rulette.tar.gz");
    write_hostile_archive(&archive, "../rulette", tar::EntryType::Regular, None);
    write_checksum(&archive, "rulette.tar.gz");

    assert_verifier_rejects(&archive);
}

#[cfg(target_os = "linux")]
#[test]
fn smoke_script_rejects_a_symlink_archive_member() {
    let temporary = tempfile::tempdir().unwrap();
    let archive = temporary.path().join("rulette.tar.gz");
    write_hostile_archive(&archive, "rulette", tar::EntryType::Symlink, Some("target"));
    write_checksum(&archive, "rulette.tar.gz");

    assert_verifier_rejects(&archive);
}

#[cfg(target_os = "linux")]
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

#[cfg(target_os = "linux")]
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
    assert!(
        !repository.join("dist").exists(),
        "the package script must create an absent dist directory"
    );

    let script_path = scripts.join("package-static-release.sh");
    std::fs::copy("scripts/package-static-release.sh", &script_path).unwrap();
    let version_script_path = scripts.join("cargo-package-version.sh");
    std::fs::copy("scripts/cargo-package-version.sh", &version_script_path).unwrap();
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::set_permissions(&version_script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

    let cargo_path = tools.join("cargo");
    std::fs::write(
        &cargo_path,
        r#"#!/usr/bin/env bash
set -euo pipefail
[[ "$PWD" == "$EXPECTED_REPOSITORY" ]]
if [[ "$1" == "pkgid" ]]; then
    printf '%s\n' 'path+file:///repository#0.1.0'
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
    let output = Command::new("bash")
        .arg(&script_path)
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
    let published_names = std::fs::read_dir(archive.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        published_names,
        std::collections::BTreeSet::from([
            "rulette-v0.1.0-x86_64-unknown-linux-musl.tar.gz".to_owned(),
            "rulette-v0.1.0-x86_64-unknown-linux-musl.tar.gz.sha256".to_owned(),
        ])
    );
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
    let output = Command::new("bash")
        .arg(&script_path)
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

#[cfg(target_os = "linux")]
#[test]
fn package_script_rejects_a_malformed_cargo_package_identifier() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    let temporary = tempfile::tempdir().unwrap();
    let repository = temporary.path().join("repository");
    let scripts = repository.join("scripts");
    let tools = temporary.path().join("tools");
    std::fs::create_dir_all(&scripts).unwrap();
    std::fs::create_dir_all(&tools).unwrap();

    let script_path = scripts.join("package-static-release.sh");
    std::fs::copy("scripts/package-static-release.sh", &script_path).unwrap();
    let version_script_path = scripts.join("cargo-package-version.sh");
    std::fs::copy("scripts/cargo-package-version.sh", &version_script_path).unwrap();
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::set_permissions(&version_script_path, std::fs::Permissions::from_mode(0o755)).unwrap();

    let cargo_path = tools.join("cargo");
    std::fs::write(
        &cargo_path,
        "#!/usr/bin/env bash\nset -euo pipefail\nif [[ \"$1\" == \"pkgid\" ]]; then\n    printf '%s\\n' 'malformed-package-id'\n    exit 0\nfi\nexit 1\n",
    )
    .unwrap();
    std::fs::set_permissions(&cargo_path, std::fs::Permissions::from_mode(0o755)).unwrap();

    let path = format!("{}:{}", tools.display(), std::env::var("PATH").unwrap());
    let output = Command::new("bash")
        .arg(&script_path)
        .env("PATH", path)
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("could not determine package version"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!repository.join("dist").exists());
}

#[cfg(unix)]
#[test]
fn cargo_package_version_script_accepts_only_cargo_compatible_semver_identifiers() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::Command;

    for (version, valid) in [
        ("1.2.3..", false),
        ("1.2.3.foo", false),
        ("1.2.3-.alpha", false),
        ("1.2.3-alpha..1", false),
        ("1.2.3-alpha.", false),
        ("1.2.3-01", false),
        ("1.2.3-01.2", false),
        ("1.2.3+", false),
        ("1.2.3++build", false),
        ("1.2.3+build+metadata", false),
        ("1.2.3-alpha+", false),
        ("1.2.3-alpha+build..1", false),
        ("1.2.3-alpha+build_01", false),
        ("01.2.3", false),
        ("1.02.3", false),
        ("1.2.03", false),
        ("1.2.3-alpha_1", false),
        ("0.1.0", true),
        ("1.2.3-0", true),
        ("1.2.3-1.2", true),
        ("1.2.3-alpha.1+build.7", true),
        ("1.2.3+build-01", true),
        ("1.2.3-alpha+build-01", true),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let tools = temporary.path().join("tools");
        std::fs::create_dir_all(&tools).unwrap();

        let script_path = temporary.path().join("cargo-package-version.sh");
        std::fs::copy("scripts/cargo-package-version.sh", &script_path).unwrap();
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let cargo_path = tools.join("cargo");
        std::fs::write(
            &cargo_path,
            "#!/usr/bin/env bash\nset -euo pipefail\nif [[ \"$1\" == \"pkgid\" ]]; then\n    printf 'path+file:///repository#%s\\n' \"$PACKAGE_VERSION\"\n    exit 0\nfi\nexit 1\n",
        )
        .unwrap();
        std::fs::set_permissions(&cargo_path, std::fs::Permissions::from_mode(0o755)).unwrap();

        let path = format!("{}:{}", tools.display(), std::env::var("PATH").unwrap());
        let output = Command::new("bash")
            .arg(&script_path)
            .env("PACKAGE_VERSION", version)
            .env("PATH", path)
            .output()
            .unwrap();

        assert_eq!(output.status.success(), valid, "{version}");
        if valid {
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                format!("{version}\n")
            );
        } else {
            assert!(
                String::from_utf8_lossy(&output.stderr)
                    .contains("could not determine package version"),
                "{version}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}
