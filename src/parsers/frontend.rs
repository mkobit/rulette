use crate::inputs::{ArtifactObservation, InputOrigin};
use crate::parsers::aggregation::{construct_graph, AggregationCandidate};
use crate::{CompilationGraph, DiagnosticSeverity, GraphDiagnostic};
use anyhow::{Context, Result};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NativeFrontend {
    Codex,
    Claude,
    CursorMdc,
    Opencode,
    Antigravity,
}

impl NativeFrontend {
    fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::CursorMdc => "cursor-mdc",
            Self::Opencode => "opencode",
            Self::Antigravity => "antigravity",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DecoderSelection {
    Auto,
    Native(NativeFrontend),
    GraphJson,
    GraphToml,
}

/// The non-fatal disposition assigned by a native frontend to one input observation.
///
/// A successful native compilation records one disposition per observation in
/// the same order as its input slice.
/// Malformed recognized input instead returns a fatal error before a
/// `NativeCompilation` audit is produced.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeObservationDisposition {
    PackageContent,
    RetainedUnsupportedContent,
    UnrecognizedWarning,
}

/// A successful native frontend result together with its complete observation audit.
///
/// Malformed recognized input is represented by the enclosing `Result` error.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeCompilation {
    pub candidates: Vec<AggregationCandidate>,
    pub diagnostics: Vec<GraphDiagnostic>,
    pub dispositions: Vec<NativeObservationDisposition>,
}

impl NativeCompilation {
    pub(crate) fn new(
        frontend: NativeFrontend,
        observations: &[ArtifactObservation],
        packages: Vec<crate::Package>,
        dispositions: Vec<NativeObservationDisposition>,
    ) -> Result<Self> {
        if dispositions.len() != observations.len() {
            anyhow::bail!(
                "{} frontend left {} observations without a classification",
                frontend.as_str(),
                observations.len().saturating_sub(dispositions.len())
            );
        }
        let mut diagnostics = packages
            .iter()
            .filter_map(|package| match &package.semantic_item {
                crate::SemanticItem::Unsupported { native_kind } => Some(GraphDiagnostic {
                    severity: DiagnosticSeverity::UnsupportedSemantic,
                    code: "unsupported-semantic".to_owned(),
                    message: format!(
                        "{} frontend recognized unsupported native {}",
                        package.provenance.frontend, native_kind
                    ),
                    package_id: Some(package.id.clone()),
                }),
                _ => None,
            })
            .collect::<Vec<_>>();
        diagnostics.extend(
            observations
                .iter()
                .zip(&dispositions)
                .filter(|(_, disposition)| {
                    **disposition == NativeObservationDisposition::UnrecognizedWarning
                })
                .map(|(observation, _)| GraphDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    code: "unrecognized-native-file".to_owned(),
                    message: format!(
                        "{} frontend did not recognize `{}` as a native package member",
                        frontend.as_str(),
                        observation.source_path.as_str()
                    ),
                    package_id: None,
                }),
        );
        diagnostics.sort_by(|left, right| {
            (
                &left.severity,
                &left.code,
                left.package_id.as_ref(),
                &left.message,
            )
                .cmp(&(
                    &right.severity,
                    &right.code,
                    right.package_id.as_ref(),
                    &right.message,
                ))
        });
        let candidates = packages
            .into_iter()
            .map(|package| {
                let observation = observations
                    .iter()
                    .find(|observation| {
                        observation.provenance.input_label == package.provenance.input_label
                    })
                    .context("native package provenance did not match a supplied explicit input")?;
                AggregationCandidate::new(package, &observation.provenance)
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            candidates,
            diagnostics,
            dispositions,
        })
    }

    pub(crate) fn into_graph(self) -> Result<CompilationGraph> {
        construct_graph(self.candidates, self.diagnostics)
    }
}

/// Compiles raw observations through one documented v0.1 harness frontend.
pub fn compile_graph(
    observations: &[ArtifactObservation],
    selection: DecoderSelection,
) -> Result<CompilationGraph> {
    let selection = resolve_decoder_selection(observations, selection)?;
    match selection {
        DecoderSelection::Native(frontend) => compile_native_graph(observations, frontend),
        DecoderSelection::GraphJson => {
            compile_graph_interchange(observations, CompilationGraph::from_json)
        }
        DecoderSelection::GraphToml => {
            compile_graph_interchange(observations, CompilationGraph::from_toml)
        }
        DecoderSelection::Auto => unreachable!("auto detection resolves to one core frontend"),
    }
}

fn resolve_decoder_selection(
    observations: &[ArtifactObservation],
    selection: DecoderSelection,
) -> Result<DecoderSelection> {
    match selection {
        DecoderSelection::Auto => {
            if observations
                .iter()
                .any(|observation| observation.provenance.input_label == "stdin")
            {
                anyhow::bail!("standard input requires an explicit --from decoder selection");
            }
            detect_graph_format(observations)
        }
        DecoderSelection::Native(frontend) => {
            validate_native_observations(observations, frontend)?;
            Ok(selection)
        }
        DecoderSelection::GraphJson | DecoderSelection::GraphToml => {
            validate_interchange_observations(observations)?;
            Ok(selection)
        }
    }
}

fn compile_native_graph(
    observations: &[ArtifactObservation],
    frontend: NativeFrontend,
) -> Result<CompilationGraph> {
    compile_native_frontend(observations, frontend)?.into_graph()
}

/// Compiles one selected native frontend without decoder preflight.
///
/// The result records an explicit disposition for every observation and rejects
/// source sets that produce no packages.
/// Malformed recognized input returns an error before any successful audit.
pub fn compile_native_frontend(
    observations: &[ArtifactObservation],
    frontend: NativeFrontend,
) -> Result<NativeCompilation> {
    let compilation = match frontend {
        NativeFrontend::Codex => super::codex::compile_native(observations),
        NativeFrontend::Claude => super::claude::compile_native(observations),
        NativeFrontend::CursorMdc => super::cursor::compile_native(observations),
        NativeFrontend::Opencode => super::opencode::compile_native(observations),
        NativeFrontend::Antigravity => super::antigravity::compile_native(observations),
    }?;
    if compilation.candidates.is_empty() {
        anyhow::bail!(
            "unsupported source syntax: {} frontend produced no packages",
            frontend.as_str()
        );
    }
    Ok(compilation)
}

fn validate_native_observations(
    observations: &[ArtifactObservation],
    frontend: NativeFrontend,
) -> Result<()> {
    if observations
        .iter()
        .any(|observation| observation.origin == InputOrigin::Stdin)
    {
        anyhow::bail!(
            "plain native standard input is unsupported; supply a tar or gzip-compressed tar archive"
        );
    }

    if !interchange_formats(observations).is_empty() {
        anyhow::bail!("native and graph interchange inputs cannot be combined");
    }

    let foreign_frontends = native_frontends(observations)
        .into_iter()
        .filter(|candidate| *candidate != frontend)
        .collect::<BTreeSet<_>>();
    if !foreign_frontends.is_empty() {
        let frontends = foreign_frontends
            .into_iter()
            .map(NativeFrontend::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        anyhow::bail!(
            "explicit native frontend `{}` cannot decode observations recognized as another native frontend: {frontends}",
            frontend.as_str()
        );
    }
    Ok(())
}

fn validate_interchange_observations(observations: &[ArtifactObservation]) -> Result<()> {
    if observations.iter().any(|observation| {
        interchange_format(observation).is_none()
            && !native_frontend_candidates(observation).is_empty()
    }) {
        anyhow::bail!("native and graph interchange inputs cannot be combined");
    }
    let interchange_formats = interchange_formats(observations);
    if interchange_formats.len() > 1 {
        anyhow::bail!("graph JSON and graph TOML inputs cannot be combined");
    }
    Ok(())
}

fn compile_graph_interchange(
    observations: &[ArtifactObservation],
    parse: fn(&str) -> Result<CompilationGraph>,
) -> Result<CompilationGraph> {
    let graphs = observations
        .iter()
        .map(|observation| {
            let input = std::str::from_utf8(&observation.bytes)
                .context("graph interchange input must be valid UTF-8")?;
            parse(input).with_context(|| {
                format!(
                    "could not decode graph interchange input `{}`",
                    observation.provenance.input_label
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let mut diagnostics = Vec::new();
    let mut candidates = Vec::new();
    for (observation, graph) in observations.iter().zip(graphs) {
        diagnostics.extend(graph.diagnostics);
        candidates.extend(
            graph
                .packages
                .into_values()
                .map(|package| AggregationCandidate::new(package, &observation.provenance))
                .collect::<Result<Vec<_>>>()?,
        );
    }

    super::aggregation::construct_graph(candidates, diagnostics)
}

fn detect_graph_format(observations: &[ArtifactObservation]) -> Result<DecoderSelection> {
    let interchange_formats = interchange_formats(observations);
    let native_frontends = native_frontends(observations);
    if !interchange_formats.is_empty() && !native_frontends.is_empty() {
        anyhow::bail!("native and graph interchange inputs cannot be combined");
    }
    if !interchange_formats.is_empty() {
        anyhow::bail!("graph interchange requires an explicit --from decoder selection");
    }
    let candidates = native_frontends
        .into_iter()
        .map(DecoderSelection::Native)
        .collect::<BTreeSet<_>>();

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

fn native_frontends(observations: &[ArtifactObservation]) -> BTreeSet<NativeFrontend> {
    observations
        .iter()
        .flat_map(native_frontend_candidates)
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum InterchangeFormat {
    Json,
    Toml,
}

fn interchange_formats(observations: &[ArtifactObservation]) -> BTreeSet<InterchangeFormat> {
    observations.iter().filter_map(interchange_format).collect()
}

fn interchange_format(observation: &ArtifactObservation) -> Option<InterchangeFormat> {
    let input = std::str::from_utf8(&observation.bytes).ok()?;
    if CompilationGraph::from_json(input).is_ok() {
        Some(InterchangeFormat::Json)
    } else if CompilationGraph::from_toml(input).is_ok() {
        Some(InterchangeFormat::Toml)
    } else {
        None
    }
}

fn native_frontend_candidates(observation: &ArtifactObservation) -> BTreeSet<NativeFrontend> {
    let path = observation.source_path.as_str();
    let file_name = path.rsplit('/').next();
    let mut candidates = BTreeSet::new();
    if has_path_component(path, ".cursor") || path.ends_with(".mdc") {
        candidates.insert(NativeFrontend::CursorMdc);
    }
    if has_path_component(path, ".opencode")
        || matches!(file_name, Some("opencode.json" | "opencode.jsonc"))
    {
        candidates.insert(NativeFrontend::Opencode);
    }
    if has_path_component(path, ".claude") || matches!(file_name, Some("CLAUDE.md" | ".mcp.json")) {
        candidates.insert(NativeFrontend::Claude);
    }
    if has_path_component(path, ".antigravity")
        || has_path_component(path, ".agents")
        || has_path_component(path, ".agent")
    {
        candidates.insert(NativeFrontend::Antigravity);
    }
    if has_path_component(path, ".codex") || matches!(file_name, Some("AGENTS.md")) {
        candidates.insert(NativeFrontend::Codex);
    }
    candidates
}

fn has_path_component(path: &str, component: &str) -> bool {
    path.split('/').any(|item| item == component)
}

#[cfg(test)]
mod tests {
    use super::{compile_graph, compile_native_frontend, DecoderSelection, NativeFrontend};
    use crate::inputs::{ArtifactObservation, InputOrigin};
    use crate::{
        CompilationGraph, DiagnosticSeverity, GraphDiagnostic, Package, Resource, ResourceContent,
        ResourcePath, SemanticIdentity, SourceProvenance,
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

    fn graph_fixture_named(name: &str) -> CompilationGraph {
        CompilationGraph::new([Package::rule(
            SemanticIdentity::parse(format!("rule:{name}")).unwrap(),
            SourceProvenance::new("codex", format!("snapshots/{name}/AGENTS.md")).unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("AGENTS.md").unwrap(),
                ResourceContent::Text(format!("Use the {name} graph compiler.\n")),
                false,
            ),
        )
        .unwrap()])
        .unwrap()
    }

    fn graph_observation(
        graph: &CompilationGraph,
        source_path: &str,
        input_label: &str,
        format: DecoderSelection,
    ) -> ArtifactObservation {
        let bytes = match format {
            DecoderSelection::GraphJson => graph.to_canonical_json().unwrap().into_bytes(),
            DecoderSelection::GraphToml => graph.to_toml().unwrap().into_bytes(),
            _ => panic!("graph observations require an explicit graph decoder"),
        };
        ArtifactObservation::new(
            bytes,
            source_path,
            false,
            InputOrigin::Filesystem,
            input_label,
            None,
        )
        .unwrap()
    }

    #[test]
    fn graph_compilation_dispatches_to_the_explicit_codex_frontend() {
        let observations = [ArtifactObservation::new(
            b"Follow the repository guidance.\n".to_vec(),
            "AGENTS.md",
            false,
            InputOrigin::Filesystem,
            "fixtures/project",
            None,
        )
        .unwrap()];

        let graph = compile_graph(
            &observations,
            DecoderSelection::Native(NativeFrontend::Codex),
        )
        .unwrap();

        assert_eq!(graph.packages.len(), 1);
        let package = graph.packages.values().next().unwrap();
        assert_eq!(package.provenance.frontend, "codex");
        assert_eq!(package.semantic_identity.as_str(), "rule:AGENTS.md");
    }

    #[test]
    fn native_compilation_records_a_disposition_for_every_observation() {
        let observations = [
            ArtifactObservation::new(
                b"Follow the repository guidance.\n".to_vec(),
                "AGENTS.md",
                false,
                InputOrigin::Filesystem,
                "fixtures/codex",
                None,
            )
            .unwrap(),
            ArtifactObservation::new(
                b"Safe but unrelated text.\n".to_vec(),
                "notes.txt",
                false,
                InputOrigin::Filesystem,
                "fixtures/codex",
                None,
            )
            .unwrap(),
        ];

        let compilation = compile_native_frontend(&observations, NativeFrontend::Codex).unwrap();

        assert_eq!(compilation.dispositions.len(), observations.len());
        assert_eq!(
            compilation.dispositions[0],
            super::NativeObservationDisposition::PackageContent
        );
        assert_eq!(
            compilation.dispositions[1],
            super::NativeObservationDisposition::UnrecognizedWarning
        );
        assert_eq!(
            compilation
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>(),
            ["codex frontend did not recognize `notes.txt` as a native package member"]
        );
    }

    #[test]
    fn every_native_frontend_classifies_retained_and_unrecognized_observations() {
        let fixtures = [
            (
                NativeFrontend::Codex,
                "AGENTS.md",
                b"Follow the repository guidance.\n".as_slice(),
                ".codex/config.toml",
                b"model = \"gpt-5\"\n".as_slice(),
            ),
            (
                NativeFrontend::Claude,
                "CLAUDE.md",
                b"Follow the repository guidance.\n".as_slice(),
                ".mcp.json",
                br#"{"mcpServers":{}}"#.as_slice(),
            ),
            (
                NativeFrontend::CursorMdc,
                ".cursor/rules/rule.mdc",
                b"Follow the repository guidance.\n".as_slice(),
                ".cursor/mcp.json",
                br#"{}"#.as_slice(),
            ),
            (
                NativeFrontend::Opencode,
                ".opencode/rules/rule.md",
                b"Follow the repository guidance.\n".as_slice(),
                "opencode.json",
                br#"{}"#.as_slice(),
            ),
            (
                NativeFrontend::Antigravity,
                ".agent/rules/rule.md",
                b"Follow the repository guidance.\n".as_slice(),
                ".antigravity/settings.json",
                br#"{}"#.as_slice(),
            ),
        ];

        for (frontend, package_path, package_bytes, retained_path, retained_bytes) in fixtures {
            let observations = [
                ArtifactObservation::new(
                    package_bytes.to_vec(),
                    package_path,
                    false,
                    InputOrigin::Filesystem,
                    "fixtures/native",
                    None,
                )
                .unwrap(),
                ArtifactObservation::new(
                    retained_bytes.to_vec(),
                    retained_path,
                    false,
                    InputOrigin::Filesystem,
                    "fixtures/native",
                    None,
                )
                .unwrap(),
                ArtifactObservation::new(
                    b"Safe but unrelated text.\n".to_vec(),
                    "notes.txt",
                    false,
                    InputOrigin::Filesystem,
                    "fixtures/native",
                    None,
                )
                .unwrap(),
            ];

            let compilation = compile_native_frontend(&observations, frontend).unwrap();

            assert_eq!(
                compilation.dispositions,
                [
                    super::NativeObservationDisposition::PackageContent,
                    super::NativeObservationDisposition::RetainedUnsupportedContent,
                    super::NativeObservationDisposition::UnrecognizedWarning,
                ],
                "{frontend:?} must classify all observations"
            );
            assert!(!compilation.candidates.is_empty());
            assert_eq!(
                compilation
                    .diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.code == "unrecognized-native-file")
                    .count(),
                1
            );
        }
    }

    #[test]
    fn native_warning_diagnostics_are_independent_of_observation_order() {
        let recognized = ArtifactObservation::new(
            b"Follow the repository guidance.\n".to_vec(),
            "AGENTS.md",
            false,
            InputOrigin::Filesystem,
            "fixtures/codex",
            None,
        )
        .unwrap();
        let first = ArtifactObservation::new(
            b"first\n".to_vec(),
            "z-last.txt",
            false,
            InputOrigin::Filesystem,
            "fixtures/codex",
            None,
        )
        .unwrap();
        let second = ArtifactObservation::new(
            b"second\n".to_vec(),
            "a-first.txt",
            false,
            InputOrigin::Filesystem,
            "fixtures/codex",
            None,
        )
        .unwrap();

        let forward = compile_native_frontend(
            &[recognized.clone(), first.clone(), second.clone()],
            NativeFrontend::Codex,
        )
        .unwrap();
        let reverse =
            compile_native_frontend(&[second, recognized, first], NativeFrontend::Codex).unwrap();

        assert_eq!(forward.diagnostics, reverse.diagnostics);
        assert_eq!(
            forward
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.as_str())
                .collect::<Vec<_>>(),
            [
                "codex frontend did not recognize `a-first.txt` as a native package member",
                "codex frontend did not recognize `z-last.txt` as a native package member",
            ]
        );
    }

    #[test]
    fn native_frontends_reject_warning_only_source_sets() {
        for frontend in [
            NativeFrontend::Codex,
            NativeFrontend::Claude,
            NativeFrontend::CursorMdc,
            NativeFrontend::Opencode,
            NativeFrontend::Antigravity,
        ] {
            let observations = [ArtifactObservation::new(
                b"Safe but unrelated text.\n".to_vec(),
                "notes.txt",
                false,
                InputOrigin::Filesystem,
                "fixtures/unknown",
                None,
            )
            .unwrap()];

            let error =
                compile_graph(&observations, DecoderSelection::Native(frontend)).unwrap_err();

            assert!(
                error.to_string().contains("unsupported source syntax"),
                "{frontend:?} must reject a warning-only source set"
            );
        }
    }

    #[test]
    fn native_frontend_api_rejects_empty_and_warning_only_source_sets() {
        for frontend in [
            NativeFrontend::Codex,
            NativeFrontend::Claude,
            NativeFrontend::CursorMdc,
            NativeFrontend::Opencode,
            NativeFrontend::Antigravity,
        ] {
            let empty_error = compile_native_frontend(&[], frontend).unwrap_err();
            assert!(
                empty_error
                    .to_string()
                    .contains("unsupported source syntax"),
                "{frontend:?} must reject empty source sets"
            );

            let observations = [ArtifactObservation::new(
                b"Safe but unrelated text.\n".to_vec(),
                "notes.txt",
                false,
                InputOrigin::Filesystem,
                "fixtures/unknown",
                None,
            )
            .unwrap()];
            let warning_error = compile_native_frontend(&observations, frontend).unwrap_err();
            assert!(
                warning_error
                    .to_string()
                    .contains("unsupported source syntax"),
                "{frontend:?} must reject warning-only source sets"
            );
        }
    }

    #[test]
    fn native_frontend_api_preserves_retained_unsupported_content_as_a_package() {
        let observations = [ArtifactObservation::new(
            b"model = \"gpt-5\"\n".to_vec(),
            ".codex/config.toml",
            false,
            InputOrigin::Filesystem,
            "fixtures/codex",
            None,
        )
        .unwrap()];

        let compilation = compile_native_frontend(&observations, NativeFrontend::Codex).unwrap();

        assert_eq!(compilation.candidates.len(), 1);
        assert_eq!(
            compilation.dispositions,
            [super::NativeObservationDisposition::RetainedUnsupportedContent]
        );
    }

    #[test]
    fn malformed_recognized_content_fails_before_a_successful_native_audit() {
        let fixtures = [
            (NativeFrontend::Codex, "AGENTS.md", b"\xff".as_slice()),
            (NativeFrontend::Claude, "CLAUDE.md", b"\xff".as_slice()),
            (
                NativeFrontend::CursorMdc,
                ".cursor/rules/rule.mdc",
                b"---\n: invalid\n---\nrule\n".as_slice(),
            ),
            (NativeFrontend::Opencode, "opencode.json", b"{".as_slice()),
            (
                NativeFrontend::Antigravity,
                ".agent/rules/rule.md",
                b"---\n: invalid\n---\nrule\n".as_slice(),
            ),
        ];

        for (frontend, path, bytes) in fixtures {
            let observations = [ArtifactObservation::new(
                bytes.to_vec(),
                path,
                false,
                InputOrigin::Filesystem,
                "fixtures/malformed",
                None,
            )
            .unwrap()];

            assert!(
                compile_native_frontend(&observations, frontend).is_err(),
                "{frontend:?} must reject malformed recognized content"
            );
        }
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

        let graph = compile_graph(&observations, DecoderSelection::Auto).unwrap();

        assert_eq!(graph.packages.len(), 1);
        let package = graph.packages.values().next().unwrap();
        assert_eq!(package.provenance.frontend, "cursor");
        assert_eq!(package.semantic_identity.as_str(), "rule:rust");
    }

    #[test]
    fn graph_compilation_auto_detects_canonical_antigravity_rule_and_configuration_layouts() {
        for (path, bytes) in [
            (
                ".antigravity/rules/rule.md",
                b"Follow the repository guidance.\n".as_slice(),
            ),
            (".antigravity/settings.json", b"{}".as_slice()),
        ] {
            let observations = [ArtifactObservation::new(
                bytes.to_vec(),
                path,
                false,
                InputOrigin::Filesystem,
                "fixtures/antigravity",
                None,
            )
            .unwrap()];

            let graph = compile_graph(&observations, DecoderSelection::Auto).unwrap();

            assert_eq!(graph.packages.len(), 1, "{path}");
            assert_eq!(
                graph.packages.values().next().unwrap().provenance.frontend,
                "antigravity",
                "{path}"
            );
        }
    }

    #[test]
    fn graph_compilation_auto_rejects_multiple_native_frontend_families() {
        let observations = [
            ArtifactObservation::new(
                b"Follow the repository guidance.\n".to_vec(),
                "AGENTS.md",
                false,
                InputOrigin::Filesystem,
                "fixtures/codex",
                None,
            )
            .unwrap(),
            ArtifactObservation::new(
                b"Follow the repository guidance.\n".to_vec(),
                "CLAUDE.md",
                false,
                InputOrigin::Filesystem,
                "fixtures/claude",
                None,
            )
            .unwrap(),
        ];

        let error = compile_graph(&observations, DecoderSelection::Auto).unwrap_err();

        assert!(error.to_string().contains("multiple core graph frontends"));
    }

    #[test]
    fn explicit_native_selection_rejects_a_known_foreign_frontend_observation() {
        let observations = [
            ArtifactObservation::new(
                b"Follow the repository guidance.\n".to_vec(),
                "AGENTS.md",
                false,
                InputOrigin::Filesystem,
                "fixtures/codex",
                None,
            )
            .unwrap(),
            ArtifactObservation::new(
                b"Follow the repository guidance.\n".to_vec(),
                "CLAUDE.md",
                false,
                InputOrigin::Filesystem,
                "fixtures/claude",
                None,
            )
            .unwrap(),
        ];

        let error = compile_graph(
            &observations,
            DecoderSelection::Native(NativeFrontend::Codex),
        )
        .unwrap_err();

        assert!(error.to_string().contains("claude"));
    }

    #[test]
    fn explicit_native_selection_rejects_canonical_antigravity_rules_and_configuration() {
        for (path, bytes) in [
            (
                ".antigravity/rules/rule.md",
                b"Follow the repository guidance.\n".as_slice(),
            ),
            (".antigravity/settings.json", b"{}".as_slice()),
        ] {
            let observations = [ArtifactObservation::new(
                bytes.to_vec(),
                path,
                false,
                InputOrigin::Filesystem,
                "fixtures/antigravity",
                None,
            )
            .unwrap()];

            let error = compile_graph(
                &observations,
                DecoderSelection::Native(NativeFrontend::Codex),
            )
            .unwrap_err();

            assert!(error.to_string().contains("antigravity"), "{path}");
        }
    }

    #[test]
    fn graph_interchange_requires_explicit_selection() {
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

        let error = compile_graph(&observations, DecoderSelection::Auto).unwrap_err();

        assert!(error
            .to_string()
            .contains("graph interchange requires an explicit --from"));
    }

    #[test]
    fn auto_rejects_native_and_neutral_path_graph_interchange() {
        let graph = graph_fixture();
        let observations = [
            ArtifactObservation::new(
                b"Follow the repository guidance.\n".to_vec(),
                "AGENTS.md",
                false,
                InputOrigin::Filesystem,
                "fixtures/codex",
                None,
            )
            .unwrap(),
            ArtifactObservation::new(
                graph.to_canonical_json().unwrap().into_bytes(),
                "snapshot.data",
                false,
                InputOrigin::Filesystem,
                "fixtures/graph",
                None,
            )
            .unwrap(),
        ];

        let error = compile_graph(&observations, DecoderSelection::Auto).unwrap_err();

        assert!(error.to_string().contains("native and graph interchange"));
    }

    #[test]
    fn explicit_native_rejects_neutral_path_graph_interchange() {
        let graph = graph_fixture();
        let observations = [
            ArtifactObservation::new(
                b"Follow the repository guidance.\n".to_vec(),
                "AGENTS.md",
                false,
                InputOrigin::Filesystem,
                "fixtures/codex",
                None,
            )
            .unwrap(),
            ArtifactObservation::new(
                graph.to_canonical_json().unwrap().into_bytes(),
                "snapshot.data",
                false,
                InputOrigin::Filesystem,
                "fixtures/graph",
                None,
            )
            .unwrap(),
        ];

        let error = compile_graph(
            &observations,
            DecoderSelection::Native(NativeFrontend::Codex),
        )
        .unwrap_err();

        assert!(error.to_string().contains("native and graph interchange"));
    }

    #[test]
    fn explicit_graph_selection_rejects_a_known_native_frontend_observation() {
        let graph = graph_fixture();
        let graph_observation = graph_observation(
            &graph,
            "snapshot.json",
            "snapshots/graph.json",
            DecoderSelection::GraphJson,
        );
        let native_observation = ArtifactObservation::new(
            b"Follow the repository guidance.\n".to_vec(),
            "AGENTS.md",
            false,
            InputOrigin::Filesystem,
            "snapshots/native",
            None,
        )
        .unwrap();

        let error = compile_graph(
            &[graph_observation, native_observation],
            DecoderSelection::GraphJson,
        )
        .unwrap_err();

        assert!(error.to_string().contains("native and graph interchange"));
    }

    #[test]
    fn auto_rejects_graph_interchange_bytes_at_a_native_layout() {
        let expected = graph_fixture();
        let expected_json = expected.to_canonical_json().unwrap();
        let observations = [ArtifactObservation::new(
            expected_json.clone().into_bytes(),
            "AGENTS.md",
            false,
            InputOrigin::Filesystem,
            "fixtures/codex",
            None,
        )
        .unwrap()];

        let error = compile_graph(&observations, DecoderSelection::Auto).unwrap_err();

        assert!(error.to_string().contains("native and graph interchange"));
    }

    #[test]
    fn arbitrary_json_is_not_recognized_as_graph_interchange() {
        let observations = [
            ArtifactObservation::new(
                b"Follow the repository guidance.\n".to_vec(),
                "AGENTS.md",
                false,
                InputOrigin::Filesystem,
                "fixtures/codex",
                None,
            )
            .unwrap(),
            ArtifactObservation::new(
                br#"{"unrelated": true}"#.to_vec(),
                "notes.json",
                false,
                InputOrigin::Filesystem,
                "fixtures/unknown",
                None,
            )
            .unwrap(),
        ];

        let graph = compile_graph(
            &observations,
            DecoderSelection::Native(NativeFrontend::Codex),
        )
        .unwrap();

        assert_eq!(graph.packages.len(), 1);
    }

    #[test]
    fn auto_detects_a_standalone_cursor_mdc_rule() {
        let observations = [ArtifactObservation::new(
            b"Use rustfmt.\n".to_vec(),
            "rust.mdc",
            false,
            InputOrigin::Filesystem,
            "fixtures/cursor",
            None,
        )
        .unwrap()];

        let graph = compile_graph(&observations, DecoderSelection::Auto).unwrap();

        assert_eq!(graph.packages.len(), 1);
        assert_eq!(
            graph.packages.values().next().unwrap().provenance.frontend,
            "cursor"
        );
    }

    #[test]
    fn explicit_native_rejects_a_standalone_cursor_mdc_observation() {
        let observations = [
            ArtifactObservation::new(
                b"Follow the repository guidance.\n".to_vec(),
                "AGENTS.md",
                false,
                InputOrigin::Filesystem,
                "fixtures/codex",
                None,
            )
            .unwrap(),
            ArtifactObservation::new(
                b"Use rustfmt.\n".to_vec(),
                "rust.mdc",
                false,
                InputOrigin::Filesystem,
                "fixtures/cursor",
                None,
            )
            .unwrap(),
        ];

        let error = compile_graph(
            &observations,
            DecoderSelection::Native(NativeFrontend::Codex),
        )
        .unwrap_err();

        assert!(error.to_string().contains("cursor-mdc"));
    }

    #[test]
    fn auto_rejects_tar_stdin_before_native_detection() {
        let observations = [ArtifactObservation::new(
            b"Follow the repository guidance.\n".to_vec(),
            "AGENTS.md",
            false,
            InputOrigin::Tar,
            "stdin",
            Some(ResourcePath::parse("AGENTS.md").unwrap()),
        )
        .unwrap()];

        let error = compile_graph(&observations, DecoderSelection::Auto).unwrap_err();

        assert!(error
            .to_string()
            .contains("standard input requires an explicit --from"));
    }

    #[test]
    fn auto_rejects_gzip_tar_stdin_before_native_detection() {
        let observations = [ArtifactObservation::new(
            b"Follow the repository guidance.\n".to_vec(),
            "AGENTS.md",
            false,
            InputOrigin::GzipTar,
            "stdin",
            Some(ResourcePath::parse("AGENTS.md").unwrap()),
        )
        .unwrap()];

        let error = compile_graph(&observations, DecoderSelection::Auto).unwrap_err();

        assert!(error
            .to_string()
            .contains("standard input requires an explicit --from"));
    }

    #[test]
    fn explicit_native_selection_rejects_plain_stdin() {
        let observations = [ArtifactObservation::new(
            b"Follow the repository guidance.\n".to_vec(),
            "stdin",
            false,
            InputOrigin::Stdin,
            "stdin",
            None,
        )
        .unwrap()];

        let error = compile_graph(
            &observations,
            DecoderSelection::Native(NativeFrontend::Codex),
        )
        .unwrap_err();

        assert!(error.to_string().contains("plain native standard input"));
    }

    #[test]
    fn explicit_native_selection_rejects_an_unknown_source_when_no_packages_result() {
        let observations = [ArtifactObservation::new(
            b"Unrelated text.\n".to_vec(),
            "notes.txt",
            false,
            InputOrigin::Filesystem,
            "fixtures/unknown",
            None,
        )
        .unwrap()];

        let error = compile_graph(
            &observations,
            DecoderSelection::Native(NativeFrontend::Codex),
        )
        .unwrap_err();

        assert!(error.to_string().contains("unsupported source syntax"));
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

        let actual = compile_graph(&observations, DecoderSelection::GraphJson).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn explicit_graph_json_decodes_a_document_named_as_a_native_codex_file() {
        let expected = graph_fixture();
        let observations = [graph_observation(
            &expected,
            "AGENTS.md",
            "snapshots/graph.json",
            DecoderSelection::GraphJson,
        )];

        let actual = compile_graph(&observations, DecoderSelection::GraphJson).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn explicit_graph_toml_decodes_a_document_below_a_native_cursor_layout() {
        let expected = graph_fixture();
        let observations = [graph_observation(
            &expected,
            ".cursor/rules/snapshot.mdc",
            "snapshots/graph.toml",
            DecoderSelection::GraphToml,
        )];

        let actual = compile_graph(&observations, DecoderSelection::GraphToml).unwrap();

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

        let actual = compile_graph(&observations, DecoderSelection::GraphToml).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn graph_json_aggregates_multiple_validated_documents() {
        let first = graph_fixture_named("first");
        let second = graph_fixture_named("second");
        let observations = [
            graph_observation(
                &first,
                "first.json",
                "snapshots/first.json",
                DecoderSelection::GraphJson,
            ),
            graph_observation(
                &second,
                "second.json",
                "snapshots/second.json",
                DecoderSelection::GraphJson,
            ),
        ];

        let graph = compile_graph(&observations, DecoderSelection::GraphJson).unwrap();

        assert_eq!(graph.packages.len(), 2);
        assert_eq!(
            graph.to_canonical_json().unwrap(),
            CompilationGraph::new([
                first.packages.values().next().unwrap().clone(),
                second.packages.values().next().unwrap().clone(),
            ])
            .unwrap()
            .to_canonical_json()
            .unwrap()
        );
    }

    #[test]
    fn graph_toml_aggregates_multiple_validated_documents() {
        let first = graph_fixture_named("first");
        let second = graph_fixture_named("second");
        let observations = [
            graph_observation(
                &first,
                "first.toml",
                "snapshots/first.toml",
                DecoderSelection::GraphToml,
            ),
            graph_observation(
                &second,
                "second.toml",
                "snapshots/second.toml",
                DecoderSelection::GraphToml,
            ),
        ];

        let graph = compile_graph(&observations, DecoderSelection::GraphToml).unwrap();

        assert_eq!(graph.packages.len(), 2);
    }

    #[test]
    fn graph_interchange_rejects_an_invalid_document_before_aggregation() {
        let valid = graph_fixture_named("valid");
        let mut unsupported_version: serde_json::Value =
            serde_json::from_str(&valid.to_canonical_json().unwrap()).unwrap();
        unsupported_version["graph_version"] = serde_json::json!("1.0");
        let observations = [
            graph_observation(
                &valid,
                "valid.json",
                "snapshots/valid.json",
                DecoderSelection::GraphJson,
            ),
            ArtifactObservation::new(
                serde_json::to_vec(&unsupported_version).unwrap(),
                "invalid.json",
                false,
                InputOrigin::Filesystem,
                "snapshots/invalid.json",
                None,
            )
            .unwrap(),
        ];

        let error = compile_graph(&observations, DecoderSelection::GraphJson).unwrap_err();

        assert!(error
            .chain()
            .any(|cause| cause.to_string().contains("unsupported graph version")));
    }

    #[test]
    fn graph_interchange_rejects_a_schema_invalid_document_before_aggregation() {
        let valid = graph_fixture_named("valid");
        let observations = [
            graph_observation(
                &valid,
                "valid.toml",
                "snapshots/valid.toml",
                DecoderSelection::GraphToml,
            ),
            ArtifactObservation::new(
                format!("{}unexpected = true\n", valid.to_toml().unwrap()).into_bytes(),
                "invalid.toml",
                false,
                InputOrigin::Filesystem,
                "snapshots/invalid.toml",
                None,
            )
            .unwrap(),
        ];

        let error = compile_graph(&observations, DecoderSelection::GraphToml).unwrap_err();

        assert!(error
            .chain()
            .any(|cause| cause.to_string().contains("unexpected")));
    }

    #[test]
    fn graph_interchange_merges_diagnostics_and_packages_independently_of_input_order() {
        let mut first = graph_fixture_named("first");
        first.diagnostics = vec![GraphDiagnostic {
            severity: DiagnosticSeverity::Warning,
            code: "z-last".to_owned(),
            message: "last diagnostic".to_owned(),
            package_id: None,
        }];
        let mut second = graph_fixture_named("second");
        second.diagnostics = vec![GraphDiagnostic {
            severity: DiagnosticSeverity::Warning,
            code: "a-first".to_owned(),
            message: "first diagnostic".to_owned(),
            package_id: None,
        }];
        let first_observation = graph_observation(
            &first,
            "first.json",
            "snapshots/first.json",
            DecoderSelection::GraphJson,
        );
        let second_observation = graph_observation(
            &second,
            "second.json",
            "snapshots/second.json",
            DecoderSelection::GraphJson,
        );

        let forward = compile_graph(
            &[first_observation.clone(), second_observation.clone()],
            DecoderSelection::GraphJson,
        )
        .unwrap();
        let reverse = compile_graph(
            &[second_observation, first_observation],
            DecoderSelection::GraphJson,
        )
        .unwrap();

        assert_eq!(
            forward.to_canonical_json().unwrap(),
            reverse.to_canonical_json().unwrap()
        );
        assert_eq!(forward.diagnostics, reverse.diagnostics);
        assert_eq!(
            forward
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>(),
            ["a-first", "z-last"]
        );
        let serialized = forward.to_canonical_json().unwrap();
        assert!(!serialized.contains("snapshots/first.json"));
        assert!(!serialized.contains("snapshots/second.json"));
        let serialized = forward.to_toml().unwrap();
        assert!(!serialized.contains("snapshots/first.json"));
        assert!(!serialized.contains("snapshots/second.json"));
    }

    #[test]
    fn graph_toml_merges_diagnostics_and_packages_independently_of_input_order() {
        let mut first = graph_fixture_named("first");
        first.diagnostics = vec![GraphDiagnostic {
            severity: DiagnosticSeverity::Warning,
            code: "z-last".to_owned(),
            message: "last diagnostic".to_owned(),
            package_id: None,
        }];
        let mut second = graph_fixture_named("second");
        second.diagnostics = vec![GraphDiagnostic {
            severity: DiagnosticSeverity::Warning,
            code: "a-first".to_owned(),
            message: "first diagnostic".to_owned(),
            package_id: None,
        }];
        let first_observation = graph_observation(
            &first,
            "first.toml",
            "snapshots/first.toml",
            DecoderSelection::GraphToml,
        );
        let second_observation = graph_observation(
            &second,
            "second.toml",
            "snapshots/second.toml",
            DecoderSelection::GraphToml,
        );

        let forward = compile_graph(
            &[first_observation.clone(), second_observation.clone()],
            DecoderSelection::GraphToml,
        )
        .unwrap();
        let reverse = compile_graph(
            &[second_observation, first_observation],
            DecoderSelection::GraphToml,
        )
        .unwrap();

        assert_eq!(
            forward.to_canonical_json().unwrap(),
            reverse.to_canonical_json().unwrap()
        );
        assert_eq!(forward.to_toml().unwrap(), reverse.to_toml().unwrap());
        assert_eq!(forward.diagnostics, reverse.diagnostics);
    }

    #[test]
    fn graph_interchange_rejects_mixed_json_and_toml_documents() {
        let graph = graph_fixture_named("mixed");
        let observations = [
            graph_observation(
                &graph,
                "snapshot.json",
                "snapshots/snapshot.json",
                DecoderSelection::GraphJson,
            ),
            graph_observation(
                &graph,
                "snapshot.toml",
                "snapshots/snapshot.toml",
                DecoderSelection::GraphToml,
            ),
        ];

        let error = compile_graph(&observations, DecoderSelection::GraphJson).unwrap_err();

        assert!(error
            .to_string()
            .contains("graph JSON and graph TOML inputs cannot be combined"));
    }
}
