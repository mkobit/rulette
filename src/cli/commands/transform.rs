use crate::cli::formats::InputFormat;
use crate::emitters::lowering::{
    lower, CapabilityFinding, CapabilitySeverity, LoweringOptions, NativeTarget,
};
use crate::inputs::{observe_path, observe_stdin, ArtifactObservation};
use crate::publication::{
    apply_plan, check_plan, check_sources, mapping_for, parse_plan_with_expected_digest, stage,
    ApplyOptions, AuthorizedRoot, DestinationState, PlanDigest, PlanOperationRequest,
    PublicationScope, ScopedAcceptedLoss, ScopedLowering, SourceCheckRequest, StageRequest,
    StageRoot,
};
use crate::{compile_graph, pipeline, CompilationGraph, PackageId};
use anyhow::{Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct DestinationDrift;

impl std::fmt::Display for DestinationDrift {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("destination check found drift")
    }
}

impl std::error::Error for DestinationDrift {}

#[derive(Args, Debug)]
pub struct TransformArgs {
    /// Native input files or directories, or `-` for standard input.
    ///
    /// Stdin is used when neither these inputs nor config inputs are supplied.
    #[arg(conflicts_with = "apply")]
    pub input: Vec<String>,

    /// Source frontend, auto-detected when omitted.
    #[arg(long, value_enum, default_value_t = InputFormat::Auto, conflicts_with = "apply")]
    pub from: InputFormat,

    /// Select one package by its exact graph package ID.
    #[arg(long, conflicts_with = "apply")]
    pub select: Vec<String>,

    /// Stage a native target as `format@scope`.
    ///
    #[arg(long, conflicts_with = "apply")]
    pub target: Vec<String>,

    /// Accept reported representational loss for requested native targets.
    #[arg(long, conflicts_with = "apply")]
    pub allow_lossy: bool,

    /// Write a self-contained publication plan to this new directory.
    #[arg(long, conflicts_with = "apply")]
    pub stage: Option<PathBuf>,

    /// Explicitly authorize the live project root for all project targets.
    #[arg(long, conflicts_with = "apply")]
    pub project_root: Option<PathBuf>,

    /// Explicitly authorize one user target root as `target=path`.
    #[arg(long, conflicts_with = "apply")]
    pub user_root: Vec<String>,

    /// Check destinations without creating a stage or applying a plan.
    #[arg(long)]
    pub check: bool,

    /// Apply the plan at `stage-dir/rulette.plan.json`.
    #[arg(long, value_name = "STAGE_DIR/rulette.plan.json")]
    pub apply: Option<PathBuf>,

    /// Require this SHA-256 digest before checking or applying a staged plan.
    #[arg(long, requires = "apply")]
    pub expect_plan_sha256: Option<String>,

    /// Explicitly authorize the live project root for plan operations.
    #[arg(long, requires = "apply")]
    pub allow_project_root: Option<PathBuf>,

    /// Explicitly authorize one plan user target root as `target=path`.
    #[arg(long, requires = "apply")]
    pub allow_user_root: Vec<String>,

    /// Allow an apply operation to replace conflicting destinations.
    #[arg(long, requires = "apply", conflicts_with = "check")]
    pub replace: bool,

    /// Load one explicit selection-and-target-only transform configuration file.
    #[arg(long, conflicts_with = "apply")]
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
        if self.apply.is_some() {
            return self.execute_plan_mode();
        }
        let config = self
            .config
            .as_deref()
            .map(TransformConfigFile::load)
            .transpose()?
            .unwrap_or_default();
        let inputs = resolve_inputs(&self.input, &config.inputs)?;
        let selector_strings = resolve_selectors(&self.select, &config.select)?;
        let targets = resolve_targets(&self.target, &config.targets)?;
        validate_source_mode(self, &targets)?;
        if self.allow_lossy && targets.is_empty() {
            anyhow::bail!("--allow-lossy requires at least one --target");
        }

        let graph = compile_graph(&observe_inputs(&inputs)?, self.from)?;
        let selectors = resolve_package_ids(&graph, &selector_strings)?;
        let selected_graph = pipeline::select_packages(&graph, &selectors)?;

        let lowerings = targets
            .iter()
            .map(|target| {
                lower(
                    &selected_graph,
                    target.target,
                    if self.allow_lossy {
                        LoweringOptions::allow_lossy()
                    } else {
                        LoweringOptions::strict()
                    },
                )
                .map(|plan| (target, plan))
            })
            .collect::<Result<Vec<_>>>()?;

        if self.check {
            let report = check_sources(SourceCheckRequest {
                graph: &selected_graph,
                lowerings: scoped_lowerings(&lowerings),
                roots: source_roots(self, &targets)?,
                accepted_losses: accepted_losses(&lowerings, self.allow_lossy),
            })?;
            render_check_report(&report.entries)?;
            if !quiet {
                print!("{}", selected_graph.to_canonical_json()?);
            }
            if report.is_clean() {
                return Ok(());
            }
            return Err(DestinationDrift.into());
        }

        if let Some(stage_dir) = &self.stage {
            let staged = stage(StageRequest {
                graph: &selected_graph,
                lowerings: scoped_lowerings(&lowerings),
                roots: stage_roots(self, &targets)?,
                accepted_losses: accepted_losses(&lowerings, self.allow_lossy),
                stage_dir,
            })?;
            if self.allow_lossy {
                for (_, plan) in &lowerings {
                    render_accepted_losses(&plan.findings)?;
                }
            }
            eprintln!("plan digest: {}", staged.plan_digest.as_str());
        }

        if !quiet {
            print!("{}", selected_graph.to_canonical_json()?);
        }
        Ok(())
    }

    fn execute_plan_mode(&self) -> Result<()> {
        validate_plan_mode(self)?;
        let expected_plan_digest = PlanDigest::parse(
            self.expect_plan_sha256
                .as_deref()
                .context("--expect-plan-sha256 is required with --apply")?,
        )?;
        let plan_path = self.apply.as_deref().expect("plan mode requires --apply");
        if plan_path.file_name().and_then(|name| name.to_str()) != Some("rulette.plan.json") {
            anyhow::bail!("--apply must name stage-dir/rulette.plan.json");
        }
        let stage_dir = plan_path.parent().unwrap_or_else(|| Path::new("."));
        let request = PlanOperationRequest {
            stage_dir,
            roots: plan_roots(self, stage_dir, &expected_plan_digest)?,
            expected_plan_digest,
        };
        if self.check {
            let report = check_plan(request)?;
            render_check_report(&report.entries)?;
            if report.is_clean() {
                return Ok(());
            }
            return Err(DestinationDrift.into());
        }
        let report = apply_plan(
            request,
            ApplyOptions {
                replace: self.replace,
            },
        )?;
        let mut entries = report
            .created
            .into_iter()
            .map(|entry| (entry, "created"))
            .chain(report.replaced.into_iter().map(|entry| (entry, "replaced")))
            .chain(
                report
                    .unchanged
                    .into_iter()
                    .map(|entry| (entry, "unchanged")),
            )
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.0.cmp(&right.0));
        for (entry, state) in entries {
            eprintln!("{state} {entry}");
        }
        Ok(())
    }
}

fn validate_source_mode(args: &TransformArgs, targets: &[TargetRequest]) -> Result<()> {
    if args.check && targets.is_empty() {
        anyhow::bail!("source --check requires at least one --target");
    }
    if args.stage.is_some() && targets.is_empty() {
        anyhow::bail!("--stage requires at least one --target");
    }
    if !targets.is_empty() && args.stage.is_none() && !args.check {
        anyhow::bail!("--target requires --stage unless --check is used");
    }
    if args.check && args.stage.is_some() {
        anyhow::bail!("--check may not be combined with --stage");
    }
    if args.check && args.allow_lossy {
        anyhow::bail!("--allow-lossy may not be combined with --check");
    }
    if targets
        .iter()
        .any(|target| target.scope == PublicationScope::Project)
        && args.project_root.is_none()
    {
        anyhow::bail!("--project-root is required for project targets");
    }
    if !args.user_root.is_empty() {
        let roots = parse_target_roots(&args.user_root, "--user-root")?;
        for target in roots.keys() {
            if !targets
                .iter()
                .any(|request| request.target == *target && request.scope == PublicationScope::User)
            {
                anyhow::bail!(
                    "--user-root authorizes an unrequested user target `{}`",
                    target.as_str()
                );
            }
        }
    }
    for target in targets
        .iter()
        .filter(|target| target.scope == PublicationScope::User)
    {
        if !parse_target_roots(&args.user_root, "--user-root")?.contains_key(&target.target) {
            anyhow::bail!(
                "--user-root {}=PATH is required for user targets",
                target.target.as_str()
            );
        }
    }
    Ok(())
}

fn validate_plan_mode(args: &TransformArgs) -> Result<()> {
    if !args.input.is_empty()
        || args.from != InputFormat::Auto
        || !args.select.is_empty()
        || !args.target.is_empty()
        || args.stage.is_some()
        || args.config.is_some()
        || args.allow_lossy
    {
        anyhow::bail!(
            "--apply may not be combined with source inputs or source compilation options"
        );
    }
    if args.replace && args.check {
        anyhow::bail!("--replace may not be combined with --check");
    }
    Ok(())
}

fn parse_target_roots(
    values: &[String],
    flag: &str,
) -> Result<std::collections::BTreeMap<NativeTarget, PathBuf>> {
    let mut roots = std::collections::BTreeMap::new();
    for value in values {
        let (target, path) = value
            .split_once('=')
            .context(format!("{flag} must use target=path"))?;
        let target = parse_native_target(target)?;
        if path.is_empty() || roots.insert(target, PathBuf::from(path)).is_some() {
            anyhow::bail!("{flag} must name each target exactly once");
        }
    }
    Ok(roots)
}

fn source_roots<'a>(
    args: &'a TransformArgs,
    targets: &[TargetRequest],
) -> Result<Vec<AuthorizedRoot<'a>>> {
    targets
        .iter()
        .map(|target| match target.scope {
            PublicationScope::Project => Ok(AuthorizedRoot {
                target: target.target,
                scope: target.scope,
                path: args
                    .project_root
                    .as_deref()
                    .expect("validated project root"),
            }),
            PublicationScope::User => Ok(AuthorizedRoot {
                target: target.target,
                scope: target.scope,
                path: find_user_root(&args.user_root, "--user-root", target.target)?
                    .expect("validated user root"),
            }),
        })
        .collect()
}

fn stage_roots<'a>(
    args: &'a TransformArgs,
    targets: &[TargetRequest],
) -> Result<Vec<StageRoot<'a>>> {
    targets
        .iter()
        .map(|target| match target.scope {
            PublicationScope::Project => Ok(StageRoot {
                target: target.target,
                scope: target.scope,
                path: args
                    .project_root
                    .as_deref()
                    .expect("validated project root"),
            }),
            PublicationScope::User => Ok(StageRoot {
                target: target.target,
                scope: target.scope,
                path: find_user_root(&args.user_root, "--user-root", target.target)?
                    .expect("validated user root"),
            }),
        })
        .collect()
}

fn plan_roots<'a>(
    args: &'a TransformArgs,
    stage_dir: &Path,
    expected_plan_digest: &PlanDigest,
) -> Result<Vec<AuthorizedRoot<'a>>> {
    let plan_bytes = std::fs::read(stage_dir.join("rulette.plan.json"))?;
    let plan = parse_plan_with_expected_digest(&plan_bytes, expected_plan_digest)?;
    let has_project_target = plan
        .mappings
        .keys()
        .any(|(_, scope)| *scope == PublicationScope::Project);
    if args.allow_project_root.is_some() && !has_project_target {
        anyhow::bail!("--allow-project-root authorizes no project target in the plan");
    }
    let mut roots = args
        .allow_user_root
        .iter()
        .map(|value| {
            let (target, path) = value
                .split_once('=')
                .context("--allow-user-root must use target=path")?;
            if path.is_empty() {
                anyhow::bail!("authority root path may not be empty");
            }
            Ok(AuthorizedRoot {
                target: parse_native_target(target)?,
                scope: PublicationScope::User,
                path: Path::new(path),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if let Some(path) = args.allow_project_root.as_deref() {
        for &(target, scope) in plan.mappings.keys() {
            if scope != PublicationScope::Project {
                continue;
            }
            roots.push(AuthorizedRoot {
                target,
                scope: PublicationScope::Project,
                path,
            });
        }
    }
    Ok(roots)
}

fn find_user_root<'a>(
    values: &'a [String],
    flag: &str,
    target: NativeTarget,
) -> Result<Option<&'a Path>> {
    for value in values {
        let (candidate, path) = value
            .split_once('=')
            .context(format!("{flag} must use target=path"))?;
        if parse_native_target(candidate)? == target {
            return Ok(Some(Path::new(path)));
        }
    }
    Ok(None)
}

fn scoped_lowerings<'a>(
    lowerings: &'a [(&'a TargetRequest, crate::emitters::lowering::LoweringPlan)],
) -> Vec<ScopedLowering<'a>> {
    lowerings
        .iter()
        .map(|(target, plan)| ScopedLowering {
            scope: target.scope,
            lowering: plan,
        })
        .collect()
}

fn accepted_losses<'a>(
    lowerings: &'a [(&'a TargetRequest, crate::emitters::lowering::LoweringPlan)],
    allow_lossy: bool,
) -> Vec<ScopedAcceptedLoss<'a>> {
    if !allow_lossy {
        return Vec::new();
    }
    lowerings
        .iter()
        .flat_map(|(target, plan)| {
            plan.findings
                .iter()
                .filter(move |finding| finding.severity != CapabilitySeverity::Supported)
                .map(move |finding| ScopedAcceptedLoss {
                    scope: target.scope,
                    finding,
                })
        })
        .collect()
}

fn render_check_report(entries: &[crate::publication::DestinationCheck]) -> Result<()> {
    let mut entries = entries.to_vec();
    entries.sort_by(|left, right| left.entry_id.cmp(&right.entry_id));
    for entry in entries {
        let state = match entry.state {
            DestinationState::Absent => "absent",
            DestinationState::Unchanged => "unchanged",
            DestinationState::Conflict => "conflict",
        };
        eprintln!("{} {state}", entry.entry_id);
    }
    Ok(())
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
        None => anyhow::bail!("target `{value}` must use the form format@scope"),
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
    fn target_requires_an_explicit_scope() {
        assert!(parse_target_request("codex").is_err());
        assert_eq!(
            parse_target_request("codex@project").unwrap().scope,
            PublicationScope::Project
        );
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
