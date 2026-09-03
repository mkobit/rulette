use rulette::{compile_graph, inputs::observe_path, DecoderSelection, PackageKind};
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/v0_1")
        .join(name)
}

#[test]
fn native_fixture_trees_auto_detect_and_preserve_package_boundaries() {
    let cases = [
        ("codex", "codex", true, false),
        ("claude", "claude", true, true),
        ("cursor", "cursor", true, true),
        ("opencode", "opencode", true, true),
        ("antigravity", "antigravity", true, true),
    ];

    for (name, frontend, requires_skill, requires_unsupported) in cases {
        let observations = observe_path(fixture(name)).expect("fixture tree is safe to observe");
        let graph = compile_graph(&observations, DecoderSelection::Auto)
            .expect("fixture tree auto-detects and compiles as a graph");

        assert!(
            graph
                .packages
                .values()
                .all(|package| package.provenance.frontend == frontend),
            "{name} packages retain their frontend provenance"
        );
        assert!(
            graph
                .packages
                .values()
                .any(|package| package.kind == PackageKind::Rule),
            "{name} exposes a portable rule package"
        );
        if requires_skill {
            assert!(
                graph
                    .packages
                    .values()
                    .any(|package| package.kind == PackageKind::Skill),
                "{name} preserves its native skill package boundary"
            );
        }
        if requires_unsupported {
            assert!(
                graph
                    .packages
                    .values()
                    .any(|package| package.kind == PackageKind::Unsupported),
                "{name} retains nonportable native semantics as unsupported packages"
            );
        }
    }
}
