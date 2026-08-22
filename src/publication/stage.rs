//! Isolated, descriptor-relative publication staging.
//!
//! Staging writes only a self-contained artifact directory and canonical
//! plan. It does not inspect, modify, or apply any mapped live-root paths.

use crate::emitters::lowering::{CapabilityFinding, LoweringPlan, NativeTarget};
use crate::ir::graph::{CompilationGraph, PackageId, ResourcePath};
use crate::publication::fs::{open_root, PublicationRoot};
use crate::publication::{
    canonical_plan_json, mapping_for, ArtifactDescriptor, PlanDigest, PlanEntry, PlanLossFinding,
    PublicationPlan, PublicationScope, RootBinding, PLAN_VERSION,
};
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::candidate::{accepted_loss_findings, collect_scoped_lowerings, entry_identifier};

pub use super::candidate::{ScopedAcceptedLoss, ScopedLowering};

const PLAN_FILE: &str = "rulette.plan.json";
const ARTIFACTS_DIRECTORY: &str = "artifacts";
static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// An explicitly authorized live root for one target and publication scope.
pub struct StageRoot<'a> {
    pub target: NativeTarget,
    pub scope: PublicationScope,
    pub path: &'a Path,
}

/// Inputs required to prepare one isolated staged-publication directory.
pub struct StageRequest<'a> {
    pub graph: &'a CompilationGraph,
    pub lowerings: Vec<ScopedLowering<'a>>,
    pub roots: Vec<StageRoot<'a>>,
    pub accepted_losses: Vec<ScopedAcceptedLoss<'a>>,
    pub stage_dir: &'a Path,
}

/// The in-memory plan and exact digest written to a newly published stage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StagedPublication {
    pub plan: PublicationPlan,
    pub plan_digest: PlanDigest,
    pub durability: StageDurability,
}

/// Whether the post-publication directory synchronization completed.
///
/// The stage directory is already atomically published when an uncertainty is
/// reported, so it is deliberately a successful staging result rather than a
/// failure that could imply the requested path was untouched.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StageDurability {
    Confirmed,
    Uncertain(String),
}

struct PreparedArtifact<'a> {
    entry: PlanEntry,
    bytes: &'a [u8],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StageState {
    Temporary,
    Published,
}

impl StageState {
    const fn cleans_owned_temporary(self) -> bool {
        matches!(self, Self::Temporary)
    }
}

/// Writes an isolated stage without modifying any mapped live root.
pub fn stage(request: StageRequest<'_>) -> Result<StagedPublication> {
    request.graph.validate()?;
    let lowerings = collect_scoped_lowerings(request.graph, &request.lowerings)?;
    let (stage_parent_path, stage_name) = stage_parent_and_name(request.stage_dir)?;
    let canonical_stage_parent = std::fs::canonicalize(&stage_parent_path).with_context(|| {
        format!(
            "could not canonicalize stage parent {}",
            stage_parent_path.display()
        )
    })?;
    let bindings = bind_live_roots(&request, &lowerings, &canonical_stage_parent)?;
    let (plan, artifacts) = build_plan(&request, &lowerings, bindings)?;
    let plan_bytes = canonical_plan_json(&plan)?;
    let plan_digest = PlanDigest::from_bytes(&plan_bytes);

    let stage_parent = open_root(&stage_parent_path)?;
    stage_parent.ensure_absent(&stage_name)?;
    let (temporary_name, temporary_root) = create_exclusive_temporary(&stage_parent)?;

    let state = StageState::Temporary;
    let pre_publish_result = write_stage_contents(&temporary_root, &artifacts, &plan_bytes)
        .and_then(|()| temporary_root.sync())
        .and_then(|()| stage_parent.sync())
        .and_then(|()| stage_parent.publish_new_directory(&temporary_name, &stage_name));
    if let Err(error) = pre_publish_result {
        debug_assert!(state.cleans_owned_temporary());
        if let Err(cleanup_error) =
            cleanup_temporary(&stage_parent, &temporary_root, &temporary_name, &artifacts)
        {
            return Err(error.context(format!(
                "could not clean up owned temporary stage: {cleanup_error:#}"
            )));
        }
        return Err(error);
    }

    let state = StageState::Published;
    debug_assert!(!state.cleans_owned_temporary());
    let durability = match stage_parent.sync() {
        Ok(()) => StageDurability::Confirmed,
        Err(error) => StageDurability::Uncertain(format!("{error:#}")),
    };

    Ok(StagedPublication {
        plan,
        plan_digest,
        durability,
    })
}

fn stage_parent_and_name(stage_dir: &Path) -> Result<(PathBuf, ResourcePath)> {
    let parent = stage_dir.parent().unwrap_or_else(|| Path::new("."));
    let parent = if parent.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        parent.to_owned()
    };
    let stage_name = stage_dir
        .file_name()
        .and_then(|name| name.to_str())
        .context("stage directory name must be valid UTF-8")?;
    Ok((parent, ResourcePath::parse(stage_name)?))
}

fn bind_live_roots(
    request: &StageRequest<'_>,
    lowerings: &BTreeMap<(NativeTarget, PublicationScope), &LoweringPlan>,
    canonical_stage_parent: &Path,
) -> Result<BTreeMap<(NativeTarget, PublicationScope), RootBinding>> {
    let mut bindings = BTreeMap::new();
    for root in &request.roots {
        let key = (root.target, root.scope);
        if !lowerings.contains_key(&key) {
            bail!("stage root has no matching target and scope lowering");
        }
        let canonical_root = std::fs::canonicalize(root.path)
            .with_context(|| format!("could not canonicalize live root {}", root.path.display()))?;
        if canonical_stage_parent.starts_with(&canonical_root) {
            bail!("stage directory must not be contained within a live publication root");
        }
        let opened = open_root(root.path)?;
        let binding = RootBinding {
            target: root.target,
            scope: root.scope,
            identity: opened.identity()?.clone(),
        };
        if bindings.insert(key, binding).is_some() {
            bail!("staging contains duplicate live root bindings");
        }
    }
    if bindings.len() != lowerings.len() {
        bail!("every scoped lowering requires exactly one live root binding");
    }
    Ok(bindings)
}

fn build_plan<'a>(
    request: &'a StageRequest<'a>,
    lowerings: &BTreeMap<(NativeTarget, PublicationScope), &'a LoweringPlan>,
    bindings: BTreeMap<(NativeTarget, PublicationScope), RootBinding>,
) -> Result<(PublicationPlan, Vec<PreparedArtifact<'a>>)> {
    let mut mappings = BTreeMap::new();
    let mut entries = Vec::new();
    let mut prepared = Vec::new();
    let mut mapped_paths = BTreeSet::new();
    let mut entry_lookup = BTreeMap::new();

    for (&(target, scope), lowering) in lowerings {
        let mapping = mapping_for(target, scope)?;
        mappings.insert((target, scope), mapping.version());
        for artifact in &lowering.artifacts {
            let descriptor = ArtifactDescriptor {
                class: artifact.class,
                native_path: artifact.path.clone(),
            };
            let mapped_path = mapping.map_artifact(&descriptor)?;
            if !mapped_paths.insert((target, scope, mapped_path)) {
                bail!("staging contains duplicate mapped artifact destinations");
            }
            let content_digest = PlanDigest::from_bytes(&artifact.bytes);
            let entry_id = entry_identifier(target, scope, artifact, &content_digest);
            let stage_artifact_path =
                ResourcePath::parse(format!("artifacts/{entry_id}/artifact"))?;
            let entry = PlanEntry {
                entry_id: entry_id.clone(),
                target,
                mapping_version: mapping.version(),
                scope,
                stage_artifact_path,
                artifact: descriptor.clone(),
                content_digest,
                byte_length: artifact
                    .bytes
                    .len()
                    .try_into()
                    .context("artifact is too large")?,
                executable: artifact.executable,
                source_package: artifact.source_package.clone(),
            };
            if entry_lookup
                .insert(
                    (target, scope, artifact.source_package.clone(), descriptor),
                    entry_id,
                )
                .is_some()
            {
                bail!("staging contains duplicate source artifact descriptors");
            }
            entries.push(entry.clone());
            prepared.push(PreparedArtifact {
                entry,
                bytes: &artifact.bytes,
            });
        }
    }

    let losses = collect_losses(request, lowerings, &entry_lookup)?;
    let canonical_graph = request.graph.to_canonical_json()?;
    let plan = PublicationPlan {
        plan_version: PLAN_VERSION,
        compiler_version: env!("CARGO_PKG_VERSION").to_owned(),
        graph_version: request.graph.graph_version.clone(),
        graph_digest: PlanDigest::from_bytes(canonical_graph.as_bytes()),
        mappings,
        roots: bindings.into_values().collect(),
        allow_lossy: !losses.is_empty(),
        losses,
        entries,
    };
    Ok((plan, prepared))
}

fn collect_losses(
    request: &StageRequest<'_>,
    lowerings: &BTreeMap<(NativeTarget, PublicationScope), &LoweringPlan>,
    entry_lookup: &BTreeMap<
        (
            NativeTarget,
            PublicationScope,
            PackageId,
            ArtifactDescriptor,
        ),
        String,
    >,
) -> Result<Vec<PlanLossFinding>> {
    let accepted = accepted_loss_findings(lowerings, &request.accepted_losses)?;

    let mut losses = Vec::new();
    for ((target, scope, _), finding) in accepted {
        let artifact = matching_artifact_descriptor(lowerings, target, scope, finding)?;
        let entry_id = artifact
            .as_ref()
            .and_then(|artifact| {
                entry_lookup.get(&(target, scope, finding.package_id.clone(), artifact.clone()))
            })
            .cloned();
        if artifact.is_some() && entry_id.is_none() {
            bail!("accepted artifact loss has no matching staged artifact entry");
        }
        let id = loss_identifier(target, scope, &finding.id, entry_id.as_deref());
        losses.push(PlanLossFinding {
            id,
            entry_id,
            package_id: finding.package_id.clone(),
            target,
            artifact,
            severity: finding.severity,
            reason_code: finding.reason_code,
            reason: finding.reason.clone(),
        });
    }
    losses.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(losses)
}

fn matching_artifact_descriptor(
    lowerings: &BTreeMap<(NativeTarget, PublicationScope), &LoweringPlan>,
    target: NativeTarget,
    scope: PublicationScope,
    finding: &CapabilityFinding,
) -> Result<Option<ArtifactDescriptor>> {
    let Some(path) = &finding.artifact_path else {
        return Ok(None);
    };
    let lowering = lowerings
        .get(&(target, scope))
        .expect("accepted loss has a matching scoped lowering");
    let mut artifacts = lowering
        .artifacts
        .iter()
        .filter(|artifact| artifact.source_package == finding.package_id && artifact.path == *path);
    let artifact = artifacts
        .next()
        .context("accepted artifact loss does not match a lowered artifact")?;
    if artifacts.next().is_some() {
        bail!("accepted artifact loss matches multiple lowered artifacts");
    }
    Ok(Some(ArtifactDescriptor {
        class: artifact.class,
        native_path: artifact.path.clone(),
    }))
}

fn loss_identifier(
    target: NativeTarget,
    scope: PublicationScope,
    finding_id: &str,
    entry_id: Option<&str>,
) -> String {
    digest_identifier(
        "loss",
        &[
            target.as_str().as_bytes(),
            scope_name(scope).as_bytes(),
            finding_id.as_bytes(),
            entry_id.unwrap_or_default().as_bytes(),
        ],
    )
}

fn digest_identifier(prefix: &str, fields: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for field in fields {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field);
    }
    format!("{prefix}_{}", hex::encode(hasher.finalize()))
}

fn scope_name(scope: PublicationScope) -> &'static str {
    match scope {
        PublicationScope::Project => "project",
        PublicationScope::User => "user",
    }
}

fn create_exclusive_temporary(parent: &PublicationRoot) -> Result<(ResourcePath, PublicationRoot)> {
    for _ in 0..32 {
        let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name =
            ResourcePath::parse(format!(".rulette-stage-{}-{sequence}", std::process::id()))?;
        match parent.create_new_directory(&name) {
            Ok(root) => return Ok((name, root)),
            Err(error) if error.to_string().contains("already exists") => continue,
            Err(error) => return Err(error),
        }
    }
    bail!("could not allocate an exclusive temporary stage directory")
}

fn write_stage_contents(
    temporary_root: &PublicationRoot,
    artifacts: &[PreparedArtifact<'_>],
    plan_bytes: &[u8],
) -> Result<()> {
    let artifacts_directory = ResourcePath::parse(ARTIFACTS_DIRECTORY)?;
    let _ = temporary_root.create_new_directory(&artifacts_directory)?;
    for artifact in artifacts {
        let entry_directory =
            ResourcePath::parse(format!("artifacts/{}", artifact.entry.entry_id))?;
        let _ = temporary_root.create_new_directory(&entry_directory)?;
        temporary_root.write_new_regular(
            &artifact.entry.stage_artifact_path,
            artifact.bytes,
            artifact.entry.executable,
        )?;
    }
    temporary_root.write_new_regular(&ResourcePath::parse(PLAN_FILE)?, plan_bytes, false)
}

fn cleanup_temporary(
    parent: &PublicationRoot,
    temporary: &PublicationRoot,
    temporary_name: &ResourcePath,
    artifacts: &[PreparedArtifact<'_>],
) -> Result<()> {
    temporary.remove_new_regular_if_exists(&ResourcePath::parse(PLAN_FILE)?)?;
    for artifact in artifacts.iter().rev() {
        temporary.remove_new_regular_if_exists(&artifact.entry.stage_artifact_path)?;
    }
    for artifact in artifacts.iter().rev() {
        temporary.remove_empty_directory_if_exists(&ResourcePath::parse(format!(
            "artifacts/{}",
            artifact.entry.entry_id
        ))?)?;
    }
    temporary.remove_empty_directory_if_exists(&ResourcePath::parse(ARTIFACTS_DIRECTORY)?)?;
    parent.remove_empty_directory_if_exists(temporary_name)
}

#[cfg(test)]
mod tests {
    use super::StageState;

    #[test]
    fn a_published_stage_is_never_cleaned_as_a_temporary_failure() {
        assert!(StageState::Temporary.cleans_owned_temporary());
        assert!(!StageState::Published.cleans_owned_temporary());
    }
}
