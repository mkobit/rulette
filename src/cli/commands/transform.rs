use crate::cli::formats::InputFormat;
use crate::emitters::lowering::{
    lower, CapabilityFinding, CapabilitySeverity, LoweringOptions, NativeTarget,
};
use crate::inputs::{observe_path, observe_stdin, ArtifactObservation};
use crate::publication::{mapping_for, PublicationScope};
use crate::{compile_graph, pipeline, CompilationGraph, PackageId};
use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Args, Debug)]
pub struct TransformArgs {
    /// Native input files or directories, or `-` for standard input.
    ///
    /// Stdin is used when neither these inputs nor config inputs are supplied.
    pub input: Vec<String>,

    /// Source frontend, auto-detected when omitted.
    #[arg(long, value_enum, default_value_t = InputFormat::Auto)]
    pub from: InputFormat,

    /// Select one package by its exact graph package ID.
    #[arg(long)]
    pub select: Vec<String>,

    /// Analyze and lower a native target as `format` or `format@scope`.
    ///
    /// Project is the default scope.
    #[arg(long)]
    pub target: Vec<String>,

    /// Accept reported representational loss for requested native targets.
    #[arg(long)]
    pub allow_lossy: bool,

    /// Load one explicit, selection-only transform configuration file.
    #[arg(long)]
    pub config: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct TargetRequest {
    target: NativeTarget,
    scope: PublicationScope,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
struct TransformConfigFile {
    #[serde(default)]
    inputs: Vec<String>,
    #[serde(default)]
    targets: Vec<TransformConfigTarget>,
    #[serde(default)]
    select: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct TransformConfigTarget {
    target: String,
    #[serde(default = "default_scope")]
    scope: String,
}

fn default_scope() -> String {
    "project".to_owned()
}

impl TransformConfigFile {
    fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let extension = path.extension().and_then(|value| value.to_str());
        let config: Self = match extension {
            Some("toml") => toml::from_str(&content)?,
            Some("json") => serde_json::from_str(&content)?,
            _ => json5::from_str(&content).map_err(|error| anyhow::anyhow!(error.to_string()))?,
        };
        if config
            .select
            .windows(2)
            .any(|pair| pair[0].as_str() >= pair[1].as_str())
        {
            anyhow::bail!("transform configuration `select` must be strictly sorted");
        }
        Ok(config)
    }
}

impl TransformArgs {
    pub fn execute(&self, quiet: bool) -> Result<()> {
        let config = self
            .config
            .as_deref()
            .map(TransformConfigFile::load)
            .transpose()?
            .unwrap_or_default();
        let inputs = resolve_inputs(&self.input, &config.inputs)?;
        let selector_strings = resolve_selectors(&self.select, &config.select)?;
        let targets = resolve_targets(&self.target, &config.targets)?;
        if self.allow_lossy && targets.is_empty() {
            anyhow::bail!("--allow-lossy requires at least one --target");
        }

        let graph = compile_graph(&observe_inputs(&inputs)?, self.from)?;
        let selectors = resolve_package_ids(&graph, &selector_strings)?;
        let selected_graph = pipeline::select_packages(&graph, &selectors)?;

        for target in targets {
            let plan = lower(
                &selected_graph,
                target.target,
                if self.allow_lossy {
                    LoweringOptions::allow_lossy()
                } else {
                    LoweringOptions::strict()
                },
            )?;
            if self.allow_lossy {
                render_accepted_losses(&plan.findings)?;
            }
        }

        if !quiet {
            print!("{}", selected_graph.to_canonical_json()?);
        }
        Ok(())
    }
}

fn resolve_inputs(cli_inputs: &[String], config_inputs: &[String]) -> Result<Vec<String>> {
    if !cli_inputs.is_empty() && !config_inputs.is_empty() {
        anyhow::bail!("inputs may be supplied by the command line or --config, not both");
    }
    if !cli_inputs.is_empty() {
        return Ok(cli_inputs.to_vec());
    }
    if !config_inputs.is_empty() {
        return Ok(config_inputs.to_vec());
    }
    Ok(vec!["-".to_owned()])
}

fn resolve_selectors(cli_selectors: &[String], config_selectors: &[String]) -> Result<Vec<String>> {
    if !cli_selectors.is_empty() && !config_selectors.is_empty() {
        anyhow::bail!("--select and transform configuration `select` may not be combined");
    }
    Ok(if cli_selectors.is_empty() {
        config_selectors.to_vec()
    } else {
        cli_selectors.to_vec()
    })
}

fn resolve_targets(
    cli_targets: &[String],
    config_targets: &[TransformConfigTarget],
) -> Result<Vec<TargetRequest>> {
    let mut targets = if cli_targets.is_empty() {
        config_targets
            .iter()
            .map(|target| parse_target_request(&format!("{}@{}", target.target, target.scope)))
            .collect::<Result<Vec<_>>>()?
    } else {
        cli_targets
            .iter()
            .map(|target| parse_target_request(target))
            .collect::<Result<Vec<_>>>()?
    };
    targets.sort_unstable();
    targets.dedup();
    for target in &targets {
        mapping_for(target.target, target.scope)?;
    }
    Ok(targets)
}

fn parse_target_request(value: &str) -> Result<TargetRequest> {
    let (target, scope) = match value.split_once('@') {
        Some((target, scope)) if !scope.contains('@') => (target, scope),
        Some(_) => anyhow::bail!("target `{value}` must use at most one `@` scope separator"),
        None => (value, "project"),
    };
    let target = parse_native_target(target)?;
    let scope = match scope {
        "project" => PublicationScope::Project,
        "user" => PublicationScope::User,
        _ => {
            anyhow::bail!("unsupported v0.1 publication scope `{scope}`; expected project or user")
        }
    };
    Ok(TargetRequest { target, scope })
}

pub(crate) fn parse_native_target(value: &str) -> Result<NativeTarget> {
    match value {
        "codex" => Ok(NativeTarget::Codex),
        "opencode" => Ok(NativeTarget::OpenCode),
        "claude" => Ok(NativeTarget::Claude),
        "cursor" => Ok(NativeTarget::Cursor),
        "antigravity" => Ok(NativeTarget::Antigravity),
        _ => anyhow::bail!(
            "unsupported v0.1 target `{value}`; expected codex, opencode, claude, cursor, or antigravity"
        ),
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

fn resolve_package_ids(graph: &CompilationGraph, selectors: &[String]) -> Result<Vec<PackageId>> {
    selectors
        .iter()
        .map(|selector| {
            graph
                .packages
                .keys()
                .find(|identifier| identifier.as_str() == selector)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("unknown package ID `{selector}`"))
        })
        .collect()
}

#[derive(Serialize)]
struct AcceptedLoss<'a> {
    target: &'a str,
    package_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    resource_path: Option<&'a str>,
    severity: &'a str,
    reason_code: &'a str,
    reason: &'a str,
    provenance: &'a crate::SourceProvenance,
}

fn render_accepted_losses(findings: &[CapabilityFinding]) -> Result<()> {
    for finding in findings
        .iter()
        .filter(|finding| finding.severity != CapabilitySeverity::Supported)
    {
        let severity = match finding.severity {
            CapabilitySeverity::Supported => "supported",
            CapabilitySeverity::Lossy => "lossy",
            CapabilitySeverity::Dropped => "dropped",
        };
        eprintln!(
            "{}",
            serde_json::to_string(&AcceptedLoss {
                target: finding.target.as_str(),
                package_id: finding.package_id.as_str(),
                resource_path: finding.resource_path.as_ref().map(|path| path.as_str()),
                severity,
                reason_code: finding.reason_code.as_str(),
                reason: &finding.reason,
                provenance: &finding.provenance,
            })?
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_target_request, resolve_inputs, TransformConfigFile};
    use crate::publication::PublicationScope;

    #[test]
    fn target_defaults_to_project_scope() {
        let target = parse_target_request("codex").unwrap();
        assert_eq!(target.scope, PublicationScope::Project);
    }

    #[test]
    fn command_and_config_inputs_are_exclusive() {
        assert!(resolve_inputs(&["one".to_owned()], &["two".to_owned()]).is_err());
    }

    #[test]
    fn config_selectors_must_be_strictly_sorted() {
        let config: TransformConfigFile = toml::from_str("select = [\"b\", \"a\"]").unwrap();
        assert!(config.select.windows(2).any(|pair| pair[0] >= pair[1]));
    }
}
