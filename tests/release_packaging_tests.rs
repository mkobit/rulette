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
