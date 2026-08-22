//! Same-directory atomic publication and observed-failure rollback.

use super::apply::{DestinationState, RootKey, VerifiedEntry, VerifiedOperation};
use super::fs::{PublicationRoot, RegularFileSnapshot};
use crate::ResourcePath;
use anyhow::{bail, Context, Result};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicU64, Ordering};

static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(super) struct TransactionReport {
    pub created: Vec<String>,
    pub replaced: Vec<String>,
}

enum MutationKind {
    Created,
    Replaced(RegularFileSnapshot),
}

struct MutationRecord {
    root_key: RootKey,
    destination: ResourcePath,
    written: RegularFileSnapshot,
    kind: MutationKind,
}

struct CreatedDirectory {
    root_key: RootKey,
    path: ResourcePath,
}

pub(super) fn apply_verified(operation: &VerifiedOperation) -> Result<TransactionReport> {
    apply_with_hook(operation, |_| Ok(()))
}

#[cfg(test)]
pub(super) fn apply_verified_with_late_failure_for_test(
    operation: &VerifiedOperation,
    failure_after_mutations: usize,
) -> Result<TransactionReport> {
    let mut observed_mutations = 0;
    apply_with_hook(operation, |_| {
        observed_mutations += 1;
        if observed_mutations >= failure_after_mutations {
            bail!("injected late publication failure")
        }
        Ok(())
    })
}

fn apply_with_hook(
    operation: &VerifiedOperation,
    mut after_mutation: impl FnMut(&VerifiedEntry) -> Result<()>,
) -> Result<TransactionReport> {
    let mut records = Vec::new();
    let mut created_directories = Vec::new();
    let mut created = Vec::new();
    let mut replaced = Vec::new();

    let mutation_result = (|| {
        for entry in operation
            .entries
            .iter()
            .filter(|entry| entry.state != DestinationState::Unchanged)
        {
            let root = &operation
                .authorities
                .get(&entry.root_key)
                .expect("verified entry has its opened authority")
                .root;
            for path in root.create_parent_directories_tracking(&entry.destination)? {
                created_directories.push(CreatedDirectory {
                    root_key: entry.root_key,
                    path,
                });
            }
            let temporary = write_same_directory_temporary(root, entry)?;
            let written = root
                .read_regular_snapshot(&temporary)?
                .context("new publication temporary file disappeared")?;
            let kind = match entry.state {
                DestinationState::Absent => {
                    if let Err(error) =
                        root.publish_new_regular_no_replace(&temporary, &entry.destination)
                    {
                        let _ = root.remove_new_regular_if_exists(&temporary);
                        return Err(error);
                    }
                    created.push(entry.candidate.entry_id.clone());
                    MutationKind::Created
                }
                DestinationState::Conflict => {
                    let original = entry
                        .existing
                        .clone()
                        .context("conflicting destination snapshot is absent")?;
                    if let Err(error) =
                        root.replace_regular_with_new(&temporary, &entry.destination)
                    {
                        let _ = root.remove_new_regular_if_exists(&temporary);
                        return Err(error);
                    }
                    replaced.push(entry.candidate.entry_id.clone());
                    MutationKind::Replaced(original)
                }
                DestinationState::Unchanged => unreachable!("unchanged entries are filtered"),
            };
            records.push(MutationRecord {
                root_key: entry.root_key,
                destination: entry.destination.clone(),
                written,
                kind,
            });
            root.sync()?;
            after_mutation(entry)?;
        }
        Ok(())
    })();

    if let Err(error) = mutation_result {
        if let Err(rollback_error) = rollback(operation, &records, &created_directories) {
            return Err(error.context(format!(
                "publication rollback also failed: {rollback_error:#}"
            )));
        }
        return Err(error);
    }
    Ok(TransactionReport { created, replaced })
}

fn write_same_directory_temporary(
    root: &PublicationRoot,
    entry: &VerifiedEntry,
) -> Result<ResourcePath> {
    for _ in 0..32 {
        let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = temporary_path(&entry.destination, sequence)?;
        match root.write_new_regular(
            &temporary,
            &entry.candidate.bytes,
            entry.candidate.executable,
        ) {
            Ok(()) => return Ok(temporary),
            Err(error) if error.to_string().contains("already exists") => continue,
            Err(error) => {
                let _ = root.remove_new_regular_if_exists(&temporary);
                return Err(error);
            }
        }
    }
    bail!("could not create an exclusive same-directory publication temporary")
}

fn temporary_path(destination: &ResourcePath, sequence: u64) -> Result<ResourcePath> {
    let (parent, _) = destination
        .as_str()
        .rsplit_once('/')
        .map_or(("", destination.as_str()), |(parent, leaf)| (parent, leaf));
    let leaf = format!(".rulette-apply-{}-{sequence}", std::process::id());
    ResourcePath::parse(if parent.is_empty() {
        leaf
    } else {
        format!("{parent}/{leaf}")
    })
}

fn rollback(
    operation: &VerifiedOperation,
    records: &[MutationRecord],
    created_directories: &[CreatedDirectory],
) -> Result<()> {
    let mut failures = Vec::new();
    for record in records.iter().rev() {
        let root = &operation
            .authorities
            .get(&record.root_key)
            .expect("rollback record has its opened authority")
            .root;
        let result = match &record.kind {
            MutationKind::Created => root
                .remove_regular_if_matches(&record.destination, &record.written)
                .and_then(|removed| {
                    if removed {
                        Ok(())
                    } else {
                        bail!("created destination changed before rollback")
                    }
                }),
            MutationKind::Replaced(original) => restore_replaced(root, record, original),
        };
        if let Err(error) = result {
            failures.push(format!("{}: {error:#}", record.destination.as_str()));
        }
    }
    for directory in reverse_created_directories(created_directories) {
        let root = &operation
            .authorities
            .get(&directory.root_key)
            .expect("created directory has its opened authority")
            .root;
        if let Err(error) = root.remove_empty_directory_if_exists(&directory.path) {
            failures.push(format!("{}: {error:#}", directory.path.as_str()));
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        bail!("{}", failures.join("; "))
    }
}

fn restore_replaced(
    root: &PublicationRoot,
    record: &MutationRecord,
    original: &RegularFileSnapshot,
) -> Result<()> {
    // This is an optimistic observed-state check, not a filesystem
    // compare-and-swap. A non-cooperating writer can still race the later
    // replacement, which remains outside the transaction guarantee.
    if !root.matches_regular_snapshot(&record.destination, &record.written)? {
        bail!("replaced destination changed before rollback")
    }
    let temporary = write_snapshot_temporary(root, &record.destination, original)?;
    if let Err(error) = root.replace_regular_with_new(&temporary, &record.destination) {
        let _ = root.remove_new_regular_if_exists(&temporary);
        return Err(error);
    }
    Ok(())
}

fn write_snapshot_temporary(
    root: &PublicationRoot,
    destination: &ResourcePath,
    snapshot: &RegularFileSnapshot,
) -> Result<ResourcePath> {
    for _ in 0..32 {
        let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = temporary_path(destination, sequence)?;
        match root.write_new_regular(&temporary, &snapshot.bytes, snapshot.metadata.executable) {
            Ok(()) => return Ok(temporary),
            Err(error) if error.to_string().contains("already exists") => continue,
            Err(error) => {
                let _ = root.remove_new_regular_if_exists(&temporary);
                return Err(error);
            }
        }
    }
    bail!("could not create an exclusive rollback temporary")
}

fn reverse_created_directories(directories: &[CreatedDirectory]) -> Vec<&CreatedDirectory> {
    let mut unique = BTreeSet::new();
    for directory in directories {
        unique.insert((directory.root_key, directory.path.clone()));
    }
    let mut paths = directories
        .iter()
        .filter(|directory| unique.remove(&(directory.root_key, directory.path.clone())))
        .collect::<Vec<_>>();
    paths.sort_by(|left, right| {
        right
            .path
            .as_str()
            .matches('/')
            .count()
            .cmp(&left.path.as_str().matches('/').count())
            .then_with(|| right.path.cmp(&left.path))
    });
    paths
}
