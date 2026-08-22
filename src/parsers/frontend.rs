use crate::cli::formats::InputFormat;
use crate::inputs::ArtifactObservation;
use crate::CompilationGraph;
use anyhow::{Context, Result};
use std::collections::BTreeSet;

/// Compiles raw observations through one documented v0.1 harness frontend.
pub fn compile_graph(
    observations: &[ArtifactObservation],
    format: InputFormat,
) -> Result<CompilationGraph> {
    let format = if format == InputFormat::Auto {
        detect_graph_format(observations)?
    } else {
        format
    };
    match format {
        InputFormat::Codex => super::codex::parse_graph(observations),
        InputFormat::Claude => super::claude::parse_graph(observations),
        InputFormat::CursorMdc => super::cursor::compile_cursor_graph(observations),
        InputFormat::Opencode => super::opencode::compile_opencode_graph(observations),
        InputFormat::Antigravity => super::antigravity::compile_antigravity_graph(observations),
        InputFormat::GraphJson => {
            compile_graph_interchange(observations, CompilationGraph::from_json)
        }
        InputFormat::GraphToml => {
            compile_graph_interchange(observations, CompilationGraph::from_toml)
        }
        InputFormat::Auto => unreachable!("auto detection resolves to one core frontend"),
    }
}

fn compile_graph_interchange(
    observations: &[ArtifactObservation],
    parse: fn(&str) -> Result<CompilationGraph>,
) -> Result<CompilationGraph> {
    let [observation] = observations else {
        anyhow::bail!("graph interchange requires exactly one input artifact");
    };
    let input = std::str::from_utf8(&observation.bytes)
        .context("graph interchange input must be valid UTF-8")?;
    parse(input)
}

fn detect_graph_format(observations: &[ArtifactObservation]) -> Result<InputFormat> {
    let mut candidates = BTreeSet::new();
    for observation in observations {
        let path = observation.source_path.as_str();
        if has_path_component(path, ".cursor") {
            candidates.insert(InputFormat::CursorMdc);
        }
        if has_path_component(path, ".opencode")
            || matches!(
                path.rsplit('/').next(),
                Some("opencode.json" | "opencode.jsonc")
            )
        {
            candidates.insert(InputFormat::Opencode);
        }
        if has_path_component(path, ".claude")
            || matches!(path.rsplit('/').next(), Some("CLAUDE.md" | ".mcp.json"))
        {
            candidates.insert(InputFormat::Claude);
        }
        if has_path_component(path, ".agents") || has_path_component(path, ".agent") {
            candidates.insert(InputFormat::Antigravity);
        }
        if has_path_component(path, ".codex")
            || matches!(path.rsplit('/').next(), Some("AGENTS.md"))
        {
            candidates.insert(InputFormat::Codex);
        }
    }

    match candidates.len() {
        1 => Ok(*candidates
            .iter()
            .next()
            .expect("one graph frontend candidate is present")),
        0 => anyhow::bail!("could not auto-detect a core graph frontend from the input paths"),
        _ => anyhow::bail!(
            "auto-detection found multiple core graph frontends; use --from to select one explicitly"
        ),
    }
}

fn has_path_component(path: &str, component: &str) -> bool {
    path.split('/').any(|item| item == component)
}

#[cfg(test)]
mod tests {
    use super::compile_graph;
    use crate::cli::formats::InputFormat;
    use crate::inputs::{ArtifactObservation, InputOrigin};
    use crate::{
        CompilationGraph, Package, Resource, ResourceContent, ResourcePath, SemanticIdentity,
        SourceProvenance,
    };

    fn graph_fixture() -> CompilationGraph {
        CompilationGraph::new([Package::rule(
            SemanticIdentity::parse("rule:repository-guidance").unwrap(),
            SourceProvenance::new("codex", "AGENTS.md").unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("AGENTS.md").unwrap(),
                ResourceContent::Text("Use the graph compiler.\n".to_owned()),
                false,
            ),
        )
        .unwrap()])
        .unwrap()
    }

    #[test]
    fn graph_compilation_dispatches_to_the_explicit_codex_frontend() {
        let observations = [
            ArtifactObservation::new(
                b"Follow the repository guidance.\n".to_vec(),
                "AGENTS.md",
                false,
                InputOrigin::Filesystem,
                "fixtures/project",
                None,
            )
            .unwrap(),
            ArtifactObservation::new(
                b"This is Claude guidance.\n".to_vec(),
                "CLAUDE.md",
                false,
                InputOrigin::Filesystem,
                "fixtures/project",
                None,
            )
            .unwrap(),
        ];

        let graph = compile_graph(&observations, InputFormat::Codex).unwrap();

        assert_eq!(graph.packages.len(), 1);
        let package = graph.packages.values().next().unwrap();
        assert_eq!(package.provenance.frontend, "codex");
        assert_eq!(package.semantic_identity.as_str(), "rule:AGENTS.md");
    }

    #[test]
    fn graph_compilation_auto_detects_a_cursor_rule_layout() {
        let observations = [ArtifactObservation::new(
            b"---\ndescription: Rust conventions\nalwaysApply: true\n---\nUse rustfmt.\n".to_vec(),
            ".cursor/rules/rust.mdc",
            false,
            InputOrigin::Filesystem,
            "fixtures/cursor-project",
            None,
        )
        .unwrap()];

        let graph = compile_graph(&observations, InputFormat::Auto).unwrap();

        assert_eq!(graph.packages.len(), 1);
        let package = graph.packages.values().next().unwrap();
        assert_eq!(package.provenance.frontend, "cursor");
        assert_eq!(package.semantic_identity.as_str(), "rule:rust");
    }

    #[test]
    fn graph_json_reingests_a_validated_compilation_graph() {
        let expected = graph_fixture();
        let observations = [ArtifactObservation::new(
            expected.to_canonical_json().unwrap().into_bytes(),
            "graph.json",
            false,
            InputOrigin::Filesystem,
            "fixtures/graph.json",
            None,
        )
        .unwrap()];

        let actual = compile_graph(&observations, InputFormat::GraphJson).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn graph_toml_reingests_a_validated_compilation_graph() {
        let expected = graph_fixture();
        let observations = [ArtifactObservation::new(
            expected.to_toml().unwrap().into_bytes(),
            "graph.toml",
            false,
            InputOrigin::Filesystem,
            "fixtures/graph.toml",
            None,
        )
        .unwrap()];

        let actual = compile_graph(&observations, InputFormat::GraphToml).unwrap();

        assert_eq!(actual, expected);
    }
}
