//! Pure validation and identity helpers shared by staging and source checks.
//!
//! This module deliberately performs no filesystem I/O and does not depend on
//! either staging or apply execution.  It keeps source-mode candidate identity
//! aligned with the entries that staging would write.

use super::{mapping_for, PlanDigest, PublicationScope};
use crate::emitters::lowering::{
    CapabilityFinding, CapabilitySeverity, LoweringPlan, NativeArtifact, NativeArtifactClass,
    NativeTarget,
};
use crate::ir::graph::{CompilationGraph, ResourcePath};
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(super) type ScopedLoweringMap<'a> =
    BTreeMap<(NativeTarget, PublicationScope), &'a LoweringPlan>;

type ScopedFindingKey = (NativeTarget, PublicationScope, String);

/// A backend lowering selected for one explicit publication scope.
pub struct ScopedLowering<'a> {
    pub scope: PublicationScope,
    pub lowering: &'a LoweringPlan,
}

/// One capability finding whose loss the caller accepts for a specific scope.
///
/// Scope is required because lowering finding identifiers are target and
/// package/resource scoped, while publication can select a target twice.
pub struct ScopedAcceptedLoss<'a> {
    pub scope: PublicationScope,
    pub finding: &'a CapabilityFinding,
}

/// Validates the selected lowerings against the graph and mapping registry.
pub(super) fn collect_scoped_lowerings<'a>(
    graph: &CompilationGraph,
    scoped_lowerings: &[ScopedLowering<'a>],
) -> Result<ScopedLoweringMap<'a>> {
    if scoped_lowerings.is_empty() {
        bail!("publication requires at least one scoped lowering");
    }

    let mut lowerings = BTreeMap::new();
    for scoped in scoped_lowerings {
        mapping_for(scoped.lowering.target, scoped.scope)?;
        if lowerings
            .insert((scoped.lowering.target, scoped.scope), scoped.lowering)
            .is_some()
        {
            bail!("publication contains duplicate target and scope lowerings");
        }
        validate_lowering(graph, scoped.lowering)?;
    }
    Ok(lowerings)
}

/// Verifies that every non-supported lowering finding was explicitly accepted.
///
/// The returned map retains the caller's exact accepted findings for staging's
/// plan-loss records.  Source checks use this validation alone and retain no
/// loss data because they do not create a plan.
pub(super) fn accepted_loss_findings<'a>(
    lowerings: &ScopedLoweringMap<'a>,
    accepted_losses: &[ScopedAcceptedLoss<'a>],
) -> Result<BTreeMap<ScopedFindingKey, &'a CapabilityFinding>> {
    let mut accepted = BTreeMap::new();
    for scoped in accepted_losses {
        let key = (scoped.finding.target, scoped.scope);
        let lowering = lowerings
            .get(&key)
            .context("accepted capability loss has no matching scoped lowering")?;
        if scoped.finding.severity == CapabilitySeverity::Supported {
            bail!("supported capability finding cannot be accepted as a loss");
        }
        if !lowering
            .findings
            .iter()
            .any(|finding| finding == scoped.finding)
        {
            bail!("accepted capability loss does not belong to its scoped lowering");
        }
        if accepted
            .insert(scoped_finding_key(key, &scoped.finding.id), scoped.finding)
            .is_some()
        {
            bail!("accepted capability losses contain a duplicate scoped finding");
        }
    }

    for (&(target, scope), lowering) in lowerings {
        for finding in &lowering.findings {
            if finding.severity != CapabilitySeverity::Supported
                && !accepted.contains_key(&scoped_finding_key((target, scope), &finding.id))
            {
                bail!("unaccepted capability loss prevents publication");
            }
        }
    }
    Ok(accepted)
}

pub(super) fn scoped_finding_key(
    (target, scope): (NativeTarget, PublicationScope),
    finding_id: &str,
) -> ScopedFindingKey {
    (target, scope, finding_id.to_owned())
}

/// The stable, content-addressed entry identifier used by both staged plans
/// and source-mode checks.
pub(super) fn entry_identifier(
    target: NativeTarget,
    scope: PublicationScope,
    artifact: &NativeArtifact,
    content_digest: &PlanDigest,
) -> String {
    let class = artifact_class_name(artifact.class);
    digest_identifier(
        "entry",
        &[
            target.as_str().as_bytes(),
            scope_name(scope).as_bytes(),
            class.as_bytes(),
            artifact.path.as_str().as_bytes(),
            artifact.source_package.as_str().as_bytes(),
            content_digest.as_str().as_bytes(),
            &[u8::from(artifact.executable)],
        ],
    )
}

fn validate_lowering(graph: &CompilationGraph, lowering: &LoweringPlan) -> Result<()> {
    for artifact in &lowering.artifacts {
        if artifact.target != lowering.target {
            bail!("lowering artifact target does not match its lowering target");
        }
        if !graph.packages.contains_key(&artifact.source_package) {
            bail!("lowering artifact references a package outside the selected graph");
        }
        ResourcePath::parse(artifact.path.as_str())?;
    }
    for finding in &lowering.findings {
        if finding.target != lowering.target {
            bail!("capability finding target does not match its lowering target");
        }
        if !graph.packages.contains_key(&finding.package_id) {
            bail!("capability finding references a package outside the selected graph");
        }
        if let Some(path) = &finding.resource_path {
            ResourcePath::parse(path.as_str())?;
        }
        if let Some(path) = &finding.artifact_path {
            ResourcePath::parse(path.as_str())?;
        }
    }
    Ok(())
}

fn digest_identifier(prefix: &str, fields: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for field in fields {
        hasher.update((field.len() as u64).to_be_bytes());
        hasher.update(field);
    }
    format!("{prefix}_{}", hex::encode(hasher.finalize()))
}

fn artifact_class_name(class: NativeArtifactClass) -> &'static str {
    match class {
        NativeArtifactClass::Instruction => "instruction",
        NativeArtifactClass::Rule => "rule",
        NativeArtifactClass::SkillInstruction => "skill-instruction",
        NativeArtifactClass::SkillResource => "skill-resource",
    }
}

fn scope_name(scope: PublicationScope) -> &'static str {
    match scope {
        PublicationScope::Project => "project",
        PublicationScope::User => "user",
    }
}
