use crate::cli::commands::transform::parse_native_target;
use crate::cli::formats::InputFormat;
use crate::emitters::lowering::{
    lower, CapabilityFinding, CapabilitySeverity, LoweringOptions, NativeTarget,
};
use crate::inputs::{observe_path, observe_stdin, ArtifactObservation};
use crate::{compile_graph, CompilationGraph, PackageKind};
use anyhow::Result;
use clap::Args;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io;

#[derive(Args, Debug)]
pub struct InspectArgs {
    /// Native input files or directories, or `-` for standard input.
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Source frontend, auto-detected when omitted.
    #[arg(long, value_enum, default_value_t = InputFormat::Auto)]
    pub from: InputFormat,

    /// Analyze one core target without publishing native artifacts.
    #[arg(short, long, conflicts_with = "coverage")]
    pub to: Option<String>,

    /// Compute the core-target capability matrix for observed package kinds.
    #[arg(long, conflicts_with = "to")]
    pub coverage: bool,

    /// Render coverage as JSON.
    #[arg(long, requires = "coverage")]
    pub json: bool,

    /// Fail coverage when any observed package kind is lossy or dropped.
    #[arg(long, requires = "coverage")]
    pub strict: bool,
}

const CORE_TARGETS: [NativeTarget; 5] = [
    NativeTarget::Codex,
    NativeTarget::OpenCode,
    NativeTarget::Claude,
    NativeTarget::Cursor,
    NativeTarget::Antigravity,
];

#[derive(Clone)]
struct CoverageCell {
    target: NativeTarget,
    package_kind: String,
    finding: CapabilityFinding,
}

#[derive(Serialize)]
struct CoverageJsonEntry<'a> {
    target: &'a str,
    package_kind: &'a str,
    status: &'a str,
    package_id: &'a str,
    provenance: &'a crate::SourceProvenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource_path: Option<&'a str>,
    reason_code: &'a str,
    reason: &'a str,
}

#[derive(Serialize)]
struct FindingJson<'a> {
    target: &'a str,
    package_id: &'a str,
    provenance: &'a crate::SourceProvenance,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource_path: Option<&'a str>,
    status: &'a str,
    reason_code: &'a str,
    reason: &'a str,
}

impl InspectArgs {
    pub fn execute(&self, quiet: bool) -> Result<()> {
        let graph = compile_graph(&observe_inputs(&self.input)?, self.from)?;
        if !quiet && !self.json {
            println!("=== Compilation graph ===");
            print!("{}", graph.to_canonical_json()?);
        }

        if self.coverage {
            return self.execute_coverage(&graph, quiet);
        }
        if let Some(target) = &self.to {
            let target = parse_native_target(target)?;
            let plan = lower(&graph, target, LoweringOptions::allow_lossy())?;
            if !quiet {
                println!("=== Capability findings for {} ===", target.as_str());
                for finding in &plan.findings {
                    println!("{}", serde_json::to_string(&finding_json(finding))?);
                }
            }
        }
        Ok(())
    }

    fn execute_coverage(&self, graph: &CompilationGraph, quiet: bool) -> Result<()> {
        let cells = coverage_cells(graph)?;
        let has_loss = cells
            .iter()
            .any(|cell| cell.finding.severity != CapabilitySeverity::Supported);
        if self.json {
            let output: Vec<_> = cells
                .iter()
                .map(|cell| CoverageJsonEntry {
                    target: cell.target.as_str(),
                    package_kind: &cell.package_kind,
                    status: severity_name(cell.finding.severity),
                    package_id: cell.finding.package_id.as_str(),
                    provenance: &cell.finding.provenance,
                    resource_path: cell
                        .finding
                        .resource_path
                        .as_ref()
                        .map(|path| path.as_str()),
                    reason_code: cell.finding.reason_code.as_str(),
                    reason: &cell.finding.reason,
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else if !quiet {
            println!("=== Coverage matrix ===");
            println!("package_kind target       status     package_id reason_code provenance");
            for cell in &cells {
                println!(
                    "{:<12} {:<12} {:<10} {} {} {}:{}",
                    cell.package_kind,
                    cell.target.as_str(),
                    severity_name(cell.finding.severity),
                    cell.finding.package_id.as_str(),
                    cell.finding.reason_code.as_str(),
                    cell.finding.provenance.frontend,
                    cell.finding.provenance.input_label,
                );
            }
        }
        if self.strict && has_loss {
            anyhow::bail!(
                "Coverage check failed: one or more graph package kinds are Lossy or Dropped"
            );
        }
        Ok(())
    }
}

fn observe_inputs(inputs: &[String]) -> Result<Vec<ArtifactObservation>> {
    let mut observations = Vec::new();
    let mut saw_stdin = false;
    for input in inputs {
        if input == "-" {
            if saw_stdin {
                anyhow::bail!("standard input may be supplied only once");
            }
            saw_stdin = true;
            observations.extend(observe_stdin(io::stdin().lock())?);
        } else {
            observations.extend(observe_path(input)?);
        }
    }
    Ok(observations)
}

fn coverage_cells(graph: &CompilationGraph) -> Result<Vec<CoverageCell>> {
    let mut matrix: BTreeMap<(NativeTarget, String), CapabilityFinding> = BTreeMap::new();
    for target in CORE_TARGETS {
        let plan = lower(graph, target, LoweringOptions::allow_lossy())?;
        for package in graph.packages.values() {
            let kind = package_kind_name(&package.kind).to_owned();
            let candidate = plan
                .findings
                .iter()
                .filter(|finding| finding.package_id == package.id)
                .max_by(|left, right| {
                    left.severity
                        .cmp(&right.severity)
                        .then_with(|| right.id.cmp(&left.id))
                })
                .expect("lowering reports a package finding for every validated graph package")
                .clone();
            let key = (target, kind);
            match matrix.get(&key) {
                Some(current)
                    if current.severity > candidate.severity
                        || (current.severity == candidate.severity
                            && current.id <= candidate.id) => {}
                _ => {
                    matrix.insert(key, candidate);
                }
            }
        }
    }
    Ok(matrix
        .into_iter()
        .map(|((target, package_kind), finding)| CoverageCell {
            target,
            package_kind,
            finding,
        })
        .collect())
}

fn finding_json(finding: &CapabilityFinding) -> FindingJson<'_> {
    FindingJson {
        target: finding.target.as_str(),
        package_id: finding.package_id.as_str(),
        provenance: &finding.provenance,
        resource_path: finding.resource_path.as_ref().map(|path| path.as_str()),
        status: severity_name(finding.severity),
        reason_code: finding.reason_code.as_str(),
        reason: &finding.reason,
    }
}

fn package_kind_name(kind: &PackageKind) -> &'static str {
    match kind {
        PackageKind::Rule => "rule",
        PackageKind::Skill => "skill",
        PackageKind::Unsupported => "unsupported",
    }
}

fn severity_name(severity: CapabilitySeverity) -> &'static str {
    match severity {
        CapabilitySeverity::Supported => "supported",
        CapabilitySeverity::Lossy => "lossy",
        CapabilitySeverity::Dropped => "dropped",
    }
}
