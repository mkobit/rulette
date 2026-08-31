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
