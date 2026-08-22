//! Verified plan-mode check and publication apply entry points.
//!
//! The plan and its artifacts are untrusted bytes until this module opens the
//! explicit stage root, verifies the raw plan digest, and completes preflight.

use super::candidate::{
    accepted_loss_findings, collect_scoped_lowerings, entry_identifier, ScopedLoweringMap,
};
use super::fs::{
    open_root, validate_distinct_paths, validate_relative_path, PublicationRoot,
    RegularFileSnapshot,
};
use super::{
    mapping_for, parse_plan_with_expected_digest, ArtifactDescriptor, MappingVersion, PlanDigest,
    PlanEntry, PublicationPlan, PublicationScope, RootIdentity, ScopedAcceptedLoss, ScopedLowering,
};
use crate::emitters::lowering::{NativeArtifact, NativeTarget};
use crate::{CompilationGraph, ResourcePath};
use anyhow::{bail, Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const PLAN_FILE: &str = "rulette.plan.json";

pub(super) type RootKey = (NativeTarget, PublicationScope);

/// One explicit caller-supplied root authorized for one target and scope.
pub struct AuthorizedRoot<'a> {
    pub target: NativeTarget,
    pub scope: PublicationScope,
    pub path: &'a Path,
}

/// Untrusted plan input and all explicit root authorities for plan-mode work.
pub struct PlanOperationRequest<'a> {
    pub stage_dir: &'a Path,
    pub expected_plan_digest: PlanDigest,
    pub roots: Vec<AuthorizedRoot<'a>>,
}

/// In-memory lowerings and explicit root authority for a source-mode check.
///
/// This operation is strictly read-only: it does not create a stage, a
/// temporary file, parent directories, or mapped destinations.
pub struct SourceCheckRequest<'a> {
    pub graph: &'a CompilationGraph,
    pub lowerings: Vec<ScopedLowering<'a>>,
    pub roots: Vec<AuthorizedRoot<'a>>,
    pub accepted_losses: Vec<ScopedAcceptedLoss<'a>>,
}

/// The live state of one verified destination.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DestinationState {
    Absent,
    Unchanged,
    Conflict,
}

/// The non-mutating status of one plan entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DestinationCheck {
    pub entry_id: String,
    pub state: DestinationState,
}

/// Result of a fully verified plan-mode check.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanCheckReport {
    pub entries: Vec<DestinationCheck>,
}

impl PlanCheckReport {
    pub fn is_clean(&self) -> bool {
        self.entries
            .iter()
            .all(|entry| entry.state == DestinationState::Unchanged)
    }
}

/// Explicit mutation policy for a verified apply operation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ApplyOptions {
    pub replace: bool,
}

/// Entries changed by a successful apply operation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ApplyReport {
    pub created: Vec<String>,
    pub replaced: Vec<String>,
    pub unchanged: Vec<String>,
}

pub(super) struct OpenedAuthority {
    pub identity: RootIdentity,
    pub root: PublicationRoot,
}

pub(super) struct VerifiedEntry {
    pub candidate: CandidateEntry,
    pub destination: ResourcePath,
    pub existing: Option<RegularFileSnapshot>,
    pub state: DestinationState,
    pub root_key: RootKey,
}

/// A verified-or-in-memory artifact before destination classification.
///
/// It carries no path authority: `artifact` remains target-relative and the
/// compiled mapping registry supplies the root-relative destination.
pub(super) struct CandidateEntry {
    pub entry_id: String,
    pub target: NativeTarget,
    pub mapping_version: MappingVersion,
    pub scope: PublicationScope,
    pub artifact: ArtifactDescriptor,
    pub bytes: Vec<u8>,
    pub executable: bool,
}

pub(super) struct VerifiedOperation {
    pub entries: Vec<VerifiedEntry>,
    pub authorities: BTreeMap<RootKey, OpenedAuthority>,
}

/// Verifies one staged plan and reports destination drift without mutation.
pub fn check_plan(request: PlanOperationRequest<'_>) -> Result<PlanCheckReport> {
    let verified = verify_operation(request)?;
    Ok(check_report(&verified))
}

/// Classifies lowerings against explicitly authorized roots without staging.
pub fn check_sources(request: SourceCheckRequest<'_>) -> Result<PlanCheckReport> {
    request.graph.validate()?;
    let lowerings = collect_scoped_lowerings(request.graph, &request.lowerings)?;
    accepted_loss_findings(&lowerings, &request.accepted_losses)?;
    let expected = lowerings.keys().copied().collect::<BTreeSet<_>>();
    let authorities = open_requested_authorities(&expected, request.roots)?;
    let candidates = source_candidates(&lowerings)?;
    let verified = classify_candidates(candidates, authorities)?;
    Ok(check_report(&verified))
}

/// Verifies and publishes one staged plan using explicit root authority.
pub fn apply_plan(request: PlanOperationRequest<'_>, options: ApplyOptions) -> Result<ApplyReport> {
    let verified = verify_operation(request)?;
    let conflicts = verified
        .entries
        .iter()
        .filter(|entry| entry.state == DestinationState::Conflict)
        .map(|entry| entry.candidate.entry_id.as_str())
        .collect::<Vec<_>>();
    if !options.replace && !conflicts.is_empty() {
        bail!(
            "publication conflicts require explicit replacement permission: {}",
            conflicts.join(", ")
        );
    }

    let unchanged = verified
        .entries
        .iter()
        .filter(|entry| entry.state == DestinationState::Unchanged)
        .map(|entry| entry.candidate.entry_id.clone())
        .collect();
    let transaction = super::transaction::apply_verified(&verified)?;
    Ok(ApplyReport {
        created: transaction.created,
        replaced: transaction.replaced,
        unchanged,
    })
}

pub(super) fn verify_operation(request: PlanOperationRequest<'_>) -> Result<VerifiedOperation> {
    let stage_root = open_root(request.stage_dir)?;
    let plan_path = ResourcePath::parse(PLAN_FILE)?;
    let plan_bytes = stage_root
        .read_regular_snapshot(&plan_path)?
        .context("staged publication plan is absent")?
        .bytes;
    let plan = parse_plan_with_expected_digest(&plan_bytes, &request.expected_plan_digest)?;
    let authorities = open_plan_authorities(&plan, request.roots)?;

    let mut candidates = Vec::with_capacity(plan.entries.len());
    for entry in &plan.entries {
        let staged = stage_root
            .read_regular_snapshot(&entry.stage_artifact_path)?
            .with_context(|| format!("staged artifact `{}` is absent", entry.entry_id))?;
        verify_staged_artifact(entry, &staged)?;
        candidates.push(CandidateEntry {
            entry_id: entry.entry_id.clone(),
            target: entry.target,
            mapping_version: entry.mapping_version,
            scope: entry.scope,
            artifact: entry.artifact.clone(),
            bytes: staged.bytes,
            executable: staged.metadata.executable,
        });
    }
    classify_candidates(candidates, authorities)
}

fn open_plan_authorities(
    plan: &PublicationPlan,
    requested_roots: Vec<AuthorizedRoot<'_>>,
) -> Result<BTreeMap<RootKey, OpenedAuthority>> {
    let expected = plan
        .roots
        .iter()
        .map(|root| (root.target, root.scope))
        .collect::<BTreeSet<_>>();
    let bindings = open_requested_authorities(&expected, requested_roots)?;
    for binding in &plan.roots {
        let key = (binding.target, binding.scope);
        let authority = bindings
            .get(&key)
            .expect("validated explicit authority root exists");
        if authority.identity != binding.identity {
            bail!("explicit authority root does not match the plan root identity");
        }
    }
    Ok(bindings)
}

fn open_requested_authorities(
    expected: &BTreeSet<RootKey>,
    requested_roots: Vec<AuthorizedRoot<'_>>,
) -> Result<BTreeMap<RootKey, OpenedAuthority>> {
    let mut requested = BTreeMap::new();
    for root in requested_roots {
        let key = (root.target, root.scope);
        if requested.insert(key, root.path).is_some() {
            bail!("duplicate explicit authority root");
        }
    }
    let requested_keys = requested.keys().copied().collect::<BTreeSet<_>>();
    let missing = expected.difference(&requested_keys).collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!("missing explicit authority root for a plan binding");
    }
    let surplus = requested_keys.difference(expected).collect::<Vec<_>>();
    if !surplus.is_empty() {
        bail!("surplus explicit authority root is not referenced by the plan");
    }

    let mut bindings = BTreeMap::new();
    for key in expected {
        let path = requested
            .get(key)
            .expect("validated explicit authority root exists");
        let root = open_root(path)?;
        let identity = root.identity()?.clone();
        bindings.insert(*key, OpenedAuthority { identity, root });
    }
    Ok(bindings)
}

fn source_candidates(lowerings: &ScopedLoweringMap<'_>) -> Result<Vec<CandidateEntry>> {
    let mut candidates = Vec::new();
    for (&(target, scope), lowering) in lowerings {
        let mapping = mapping_for(target, scope)?;
        for artifact in &lowering.artifacts {
            candidates.push(source_candidate(
                target,
                scope,
                mapping.version(),
                artifact,
            )?);
        }
    }
    Ok(candidates)
}

fn source_candidate(
    target: NativeTarget,
    scope: PublicationScope,
    mapping_version: MappingVersion,
    artifact: &NativeArtifact,
) -> Result<CandidateEntry> {
    let content_digest = PlanDigest::from_bytes(&artifact.bytes);
    let _: u64 = artifact
        .bytes
        .len()
        .try_into()
        .context("lowered artifact is too large")?;
    Ok(CandidateEntry {
        entry_id: entry_identifier(target, scope, artifact, &content_digest),
        target,
        mapping_version,
        scope,
        artifact: ArtifactDescriptor {
            class: artifact.class,
            native_path: artifact.path.clone(),
        },
        bytes: artifact.bytes.clone(),
        executable: artifact.executable,
    })
}

fn classify_candidates(
    candidates: Vec<CandidateEntry>,
    authorities: BTreeMap<RootKey, OpenedAuthority>,
) -> Result<VerifiedOperation> {
    let mut entries = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let root_key = (candidate.target, candidate.scope);
        if !authorities.contains_key(&root_key) {
            bail!("publication candidate has no explicit authorized root");
        }
        let mapping = mapping_for(candidate.target, candidate.scope)?;
        if mapping.version() != candidate.mapping_version {
            bail!("publication candidate has an unsupported target mapping version");
        }
        let destination = mapping.map_artifact(&candidate.artifact)?;
        validate_relative_path(&destination)?;
        entries.push(VerifiedEntry {
            candidate,
            destination,
            existing: None,
            state: DestinationState::Absent,
            root_key,
        });
    }
    reject_destination_collisions(&entries, &authorities)?;
    for entry in &mut entries {
        let authority = authorities
            .get(&entry.root_key)
            .expect("verified entry has an opened authority");
        authority
            .root
            .validate_parent_directories(&entry.destination)?;
        entry.existing = authority.root.read_regular_snapshot(&entry.destination)?;
        entry.state = match &entry.existing {
            None => DestinationState::Absent,
            Some(existing)
                if existing.bytes == entry.candidate.bytes
                    && existing.metadata.executable == entry.candidate.executable =>
            {
                DestinationState::Unchanged
            }
            Some(_) => DestinationState::Conflict,
        };
    }
    Ok(VerifiedOperation {
        entries,
        authorities,
    })
}

fn check_report(verified: &VerifiedOperation) -> PlanCheckReport {
    PlanCheckReport {
        entries: verified
            .entries
            .iter()
            .map(|entry| DestinationCheck {
                entry_id: entry.candidate.entry_id.clone(),
                state: entry.state,
            })
            .collect(),
    }
}

fn verify_staged_artifact(entry: &PlanEntry, staged: &RegularFileSnapshot) -> Result<()> {
    let byte_length: u64 = staged
        .bytes
        .len()
        .try_into()
        .context("staged artifact is too large")?;
    if byte_length != entry.byte_length || staged.metadata.byte_length != entry.byte_length {
        bail!("staged artifact byte length does not match the plan");
    }
    if PlanDigest::from_bytes(&staged.bytes) != entry.content_digest {
        bail!("staged artifact digest does not match the plan");
    }
    if staged.metadata.executable != entry.executable {
        bail!("staged artifact executable metadata does not match the plan");
    }
    Ok(())
}

fn reject_destination_collisions(
    entries: &[VerifiedEntry],
    authorities: &BTreeMap<RootKey, OpenedAuthority>,
) -> Result<()> {
    let mut grouped = BTreeMap::<RootIdentity, Vec<&VerifiedEntry>>::new();
    for entry in entries {
        let identity = authorities
            .get(&entry.root_key)
            .expect("verified entry has an opened authority")
            .identity
            .clone();
        grouped.entry(identity).or_default().push(entry);
    }
    for entries in grouped.values() {
        validate_distinct_paths(entries.iter().map(|entry| &entry.destination))?;
        for (index, left) in entries.iter().enumerate() {
            for right in &entries[index + 1..] {
                let left = left.destination.as_str().to_ascii_lowercase();
                let right = right.destination.as_str().to_ascii_lowercase();
                if left.starts_with(&(right.clone() + "/"))
                    || right.starts_with(&(left.clone() + "/"))
                {
                    bail!("plan contains ancestor and descendant destinations under one root");
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn apply_plan_with_late_failure_for_test(
    request: PlanOperationRequest<'_>,
    options: ApplyOptions,
    failure_after_mutations: usize,
) -> Result<ApplyReport> {
    let verified = verify_operation(request)?;
    if !options.replace
        && verified
            .entries
            .iter()
            .any(|entry| entry.state == DestinationState::Conflict)
    {
        bail!("publication conflicts require explicit replacement permission");
    }
    let unchanged = verified
        .entries
        .iter()
        .filter(|entry| entry.state == DestinationState::Unchanged)
        .map(|entry| entry.candidate.entry_id.clone())
        .collect();
    let transaction = super::transaction::apply_verified_with_late_failure_for_test(
        &verified,
        failure_after_mutations,
    )?;
    Ok(ApplyReport {
        created: transaction.created,
        replaced: transaction.replaced,
        unchanged,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        apply_plan_with_late_failure_for_test, mapping_for, ApplyOptions, AuthorizedRoot,
        PlanOperationRequest,
    };
    use crate::publication::{
        canonical_plan_json, stage, PlanDigest, PublicationScope, ScopedLowering, StageRequest,
        StageRoot,
    };
    use crate::{
        lower, CompilationGraph, LoweringOptions, NativeTarget, Package, Resource, ResourceContent,
        ResourcePath, SemanticIdentity, SourceProvenance,
    };
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn rule(name: &str) -> Package {
        Package::rule(
            SemanticIdentity::parse(format!("rule:{name}")).unwrap(),
            SourceProvenance::new("opencode", format!("rules/{name}.md")).unwrap(),
            Resource::primary_instruction(
                ResourcePath::parse("RULE.md").unwrap(),
                ResourceContent::Text(format!("# {name}\n")),
                false,
            ),
        )
        .unwrap()
    }

    #[cfg(any(target_os = "linux", target_os = "android"))]
    #[test]
    fn late_failure_restores_replacements_and_removes_created_files_and_directories() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("project");
        let stage_dir = temporary.path().join("stage");
        fs::create_dir(&root).unwrap();
        let graph = CompilationGraph::new([rule("alpha")]).unwrap();
        let codex = lower(&graph, NativeTarget::Codex, LoweringOptions::strict()).unwrap();
        let opencode = lower(&graph, NativeTarget::OpenCode, LoweringOptions::strict()).unwrap();
        let staged = stage(StageRequest {
            graph: &graph,
            lowerings: vec![
                ScopedLowering {
                    scope: PublicationScope::Project,
                    lowering: &opencode,
                },
                ScopedLowering {
                    scope: PublicationScope::Project,
                    lowering: &codex,
                },
            ],
            roots: vec![
                StageRoot {
                    target: NativeTarget::OpenCode,
                    scope: PublicationScope::Project,
                    path: &root,
                },
                StageRoot {
                    target: NativeTarget::Codex,
                    scope: PublicationScope::Project,
                    path: &root,
                },
            ],
            accepted_losses: vec![],
            stage_dir: &stage_dir,
        })
        .unwrap();
        let mut plan = staged.plan.clone();
        let open_entry = plan
            .entries
            .iter_mut()
            .find(|entry| entry.target == NativeTarget::OpenCode)
            .unwrap();
        open_entry.entry_id = "entry_a_created".to_owned();
        let codex_entry = plan
            .entries
            .iter_mut()
            .find(|entry| entry.target == NativeTarget::Codex)
            .unwrap();
        codex_entry.entry_id = "entry_b_replaced".to_owned();
        let plan_bytes = canonical_plan_json(&plan).unwrap();
        let plan_digest = PlanDigest::from_bytes(&plan_bytes);
        fs::write(stage_dir.join("rulette.plan.json"), plan_bytes).unwrap();
        let open_entry = plan
            .entries
            .iter()
            .find(|entry| entry.target == NativeTarget::OpenCode)
            .unwrap();
        let codex_entry = plan
            .entries
            .iter()
            .find(|entry| entry.target == NativeTarget::Codex)
            .unwrap();
        let created_destination = root.join(
            mapping_for(open_entry.target, open_entry.scope)
                .unwrap()
                .map_artifact(&open_entry.artifact)
                .unwrap()
                .as_str(),
        );
        let replaced_destination = root.join(
            mapping_for(codex_entry.target, codex_entry.scope)
                .unwrap()
                .map_artifact(&codex_entry.artifact)
                .unwrap()
                .as_str(),
        );
        fs::write(&replaced_destination, b"original codex destination").unwrap();
        fs::set_permissions(&replaced_destination, fs::Permissions::from_mode(0o700)).unwrap();

        let error = apply_plan_with_late_failure_for_test(
            PlanOperationRequest {
                stage_dir: &stage_dir,
                expected_plan_digest: plan_digest,
                roots: vec![
                    AuthorizedRoot {
                        target: NativeTarget::OpenCode,
                        scope: PublicationScope::Project,
                        path: &root,
                    },
                    AuthorizedRoot {
                        target: NativeTarget::Codex,
                        scope: PublicationScope::Project,
                        path: &root,
                    },
                ],
            },
            ApplyOptions { replace: true },
            2,
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("injected late publication failure"));
        assert!(!created_destination.exists());
        assert!(!root.join(".opencode").exists());
        assert_eq!(
            fs::read(&replaced_destination).unwrap(),
            b"original codex destination"
        );
        assert_ne!(
            fs::metadata(&replaced_destination)
                .unwrap()
                .permissions()
                .mode()
                & 0o111,
            0
        );
    }
}
