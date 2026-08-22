//! Handle-oriented filesystem primitives for staged publication.
//!
//! This module opens an authority root once and performs every later
//! operation through descriptor-relative, no-follow traversal.
//! It deliberately does not stage, apply, replace, or authorize a plan.

use super::model::RootIdentity;
use crate::ResourcePath;
use anyhow::{bail, Result};
use std::collections::BTreeSet;
use std::path::Path;

/// Metadata retained for a verified regular file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegularFileMetadata {
    pub byte_length: u64,
    pub executable: bool,
}

/// The identity of one observed regular file.
///
/// It is valid only as an optimistic ownership check within the current
/// process; callers must not treat it as a persistent authority token.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegularFileIdentity {
    pub device: u64,
    pub inode: u64,
}

/// Bytes, metadata, and identity observed from one opened regular file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegularFileSnapshot {
    pub bytes: Vec<u8>,
    pub metadata: RegularFileMetadata,
    pub identity: RegularFileIdentity,
}

/// An opaque handle to an authorized publication root.
///
/// The caller path is used only by [`open_root`].
/// All subsequent operations accept a validated [`ResourcePath`] and are
/// performed relative to this already-open root.
pub struct PublicationRoot {
    inner: platform::Root,
    identity: Option<RootIdentity>,
}

/// Opens a root directory for descriptor-relative publication operations.
///
/// Platforms that cannot uphold the no-follow descriptor contract return an
/// explicit unsupported-publication error instead of falling back to paths.
pub fn open_root(path: impl AsRef<Path>) -> Result<PublicationRoot> {
    let path = path.as_ref();
    let inner = platform::Root::open(path)?;
    Ok(PublicationRoot {
        identity: Some(inner.identity(path)?),
        inner,
    })
}

/// Validates the extra platform-independent publication restrictions imposed
/// on top of the graph's slash-separated relative-path grammar.
pub fn validate_relative_path(path: &ResourcePath) -> Result<()> {
    ResourcePath::parse(path.as_str())?;

    for component in path.as_str().split('/') {
        if !component.is_ascii() {
            bail!("publication path component `{component}` is not portable ASCII");
        }
        if component.ends_with([' ', '.']) {
            bail!("publication path component `{component}` has a reserved trailing character");
        }
        if component.eq_ignore_ascii_case(".git") {
            bail!("publication path must not enter repository-control namespace `.git`");
        }
        if component.bytes().any(|byte| {
            matches!(
                byte,
                b'<' | b'>' | b':' | b'"' | b'/' | b'\\' | b'|' | b'?' | b'*'
            )
        }) {
            bail!(
                "publication path component `{component}` contains a reserved platform character"
            );
        }
        if is_reserved_platform_name(component) {
            bail!("publication path component `{component}` is a reserved platform name");
        }
    }

    Ok(())
}

/// Rejects paths which a case-insensitive platform would normalize to the
/// same destination.
pub fn validate_distinct_paths<'a>(
    paths: impl IntoIterator<Item = &'a ResourcePath>,
) -> Result<()> {
    let mut normalized = BTreeSet::new();
    for path in paths {
        validate_relative_path(path)?;
        let normalized_path = path.as_str().to_ascii_lowercase();
        if !normalized.insert(normalized_path) {
            bail!(
                "publication paths contain a platform-normalization collision at `{}`",
                path.as_str()
            );
        }
    }
    Ok(())
}

impl PublicationRoot {
    /// Returns the opaque identity bound to this explicitly opened root.
    ///
    /// Child handles created through this API are intentionally not root
    /// bindings and therefore have no independently serializable identity.
    pub fn identity(&self) -> Result<&RootIdentity> {
        self.identity
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("publication child handle has no root identity"))
    }

    /// Refuses a present path without following a leaf or parent link.
    pub fn ensure_absent(&self, path: &ResourcePath) -> Result<()> {
        let components = validated_components(path)?;
        self.inner.ensure_absent(&components)
    }

    /// Creates a new directory and returns a descriptor-relative child handle.
    pub fn create_new_directory(&self, path: &ResourcePath) -> Result<PublicationRoot> {
        let components = validated_components(path)?;
        Ok(PublicationRoot {
            inner: self.inner.create_new_directory(&components)?,
            identity: None,
        })
    }

    /// Flushes this directory handle before publication.
    pub fn sync(&self) -> Result<()> {
        self.inner.sync()
    }

    /// Atomically publishes one new directory without replacing a destination.
    pub fn publish_new_directory(
        &self,
        source: &ResourcePath,
        destination: &ResourcePath,
    ) -> Result<()> {
        let source = validated_components(source)?;
        let destination = validated_components(destination)?;
        self.inner.publish_new_directory(&source, &destination)
    }

    /// Removes a known newly-created regular file if it is still regular.
    pub fn remove_new_regular_if_exists(&self, path: &ResourcePath) -> Result<()> {
        let components = validated_components(path)?;
        self.inner.remove_new_regular_if_exists(&components)
    }

    /// Removes a known newly-created empty directory if it is still a directory.
    pub fn remove_empty_directory_if_exists(&self, path: &ResourcePath) -> Result<()> {
        let components = validated_components(path)?;
        self.inner.remove_empty_directory_if_exists(&components)
    }

    /// Validates existing parent components without creating any directory.
    ///
    /// Missing parents are valid for a later create operation; all existing
    /// components must be safe directories below the opened root.
    pub fn validate_parent_directories(&self, path: &ResourcePath) -> Result<()> {
        let components = validated_components(path)?;
        self.inner.validate_parent_directories(&components)
    }

    /// Creates missing parent directories and returns exactly those created
    /// by this call as root-relative paths.
    pub fn create_parent_directories_tracking(
        &self,
        path: &ResourcePath,
    ) -> Result<Vec<ResourcePath>> {
        let components = validated_components(path)?;
        let created_depths = self.inner.create_parent_directories_tracking(&components)?;
        created_depths
            .into_iter()
            .map(|depth| ResourcePath::parse(components[..depth].join("/")))
            .collect()
    }

    /// Creates every missing parent directory of `path` through no-follow
    /// descriptor traversal.
    pub fn create_parent_directories(&self, path: &ResourcePath) -> Result<()> {
        self.create_parent_directories_tracking(path).map(|_| ())
    }

    /// Reads a regular file without following a leaf or parent link.
    ///
    /// `Ok(None)` indicates that the path or one of its parent directories is
    /// absent. Any other file kind is rejected.
    pub fn read_regular(&self, path: &ResourcePath) -> Result<Option<Vec<u8>>> {
        Ok(self
            .read_regular_snapshot(path)?
            .map(|snapshot| snapshot.bytes))
    }

    /// Reads a regular file through one descriptor and retains the exact
    /// metadata and identity observed from that descriptor.
    pub fn read_regular_snapshot(
        &self,
        path: &ResourcePath,
    ) -> Result<Option<RegularFileSnapshot>> {
        let components = validated_components(path)?;
        self.inner.read_regular_snapshot(&components)
    }

    /// Returns metadata for a regular file without following a leaf or parent
    /// link. `Ok(None)` indicates an absent path.
    pub fn regular_metadata(&self, path: &ResourcePath) -> Result<Option<RegularFileMetadata>> {
        let components = validated_components(path)?;
        self.inner.regular_metadata(&components)
    }

    /// Creates a new regular file through the opened root.
    ///
    /// Existing files are never replaced by this primitive. Callers must
    /// create its parents first; atomic replacement is intentionally part of
    /// the later apply transaction boundary rather than this foundation.
    pub fn write_new_regular(
        &self,
        path: &ResourcePath,
        bytes: &[u8],
        executable: bool,
    ) -> Result<()> {
        let components = validated_components(path)?;
        self.inner.write_new_regular(&components, bytes, executable)
    }

    /// Atomically publishes a newly-created regular file without replacing an
    /// existing destination. Source and destination must share a parent.
    pub fn publish_new_regular_no_replace(
        &self,
        source: &ResourcePath,
        destination: &ResourcePath,
    ) -> Result<()> {
        let source = validated_components(source)?;
        let destination = validated_components(destination)?;
        self.inner
            .publish_new_regular_no_replace(&source, &destination)
    }

    /// Atomically replaces an existing regular destination with a newly
    /// created same-directory regular file.
    pub fn replace_regular_with_new(
        &self,
        source: &ResourcePath,
        destination: &ResourcePath,
    ) -> Result<()> {
        let source = validated_components(source)?;
        let destination = validated_components(destination)?;
        self.inner.replace_regular_with_new(&source, &destination)
    }

    /// Returns whether the current regular file still exactly matches one
    /// earlier snapshot. A missing or changed file returns `false`.
    pub fn matches_regular_snapshot(
        &self,
        path: &ResourcePath,
        expected: &RegularFileSnapshot,
    ) -> Result<bool> {
        Ok(self
            .read_regular_snapshot(path)?
            .is_some_and(|current| current == *expected))
    }

    /// Removes a regular file only after it matches the supplied snapshot.
    ///
    /// This is an optimistic ownership guard for transaction rollback rather
    /// than a persistent lock against a non-cooperating concurrent writer.
    /// The snapshot check and later unlink cannot form a filesystem-level
    /// compare-and-swap, so callers must report a concurrent-writer rollback
    /// uncertainty rather than treating this as a lock.
    pub fn remove_regular_if_matches(
        &self,
        path: &ResourcePath,
        expected: &RegularFileSnapshot,
    ) -> Result<bool> {
        let components = validated_components(path)?;
        self.inner.remove_regular_if_matches(&components, expected)
    }
}

fn validated_components(path: &ResourcePath) -> Result<Vec<&str>> {
    validate_relative_path(path)?;
    Ok(path.as_str().split('/').collect())
}

fn is_reserved_platform_name(component: &str) -> bool {
    let stem = component.split('.').next().unwrap_or_default();
    let upper = stem.to_ascii_uppercase();
    matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && matches!(&upper[..3], "COM" | "LPT")
            && matches!(upper.as_bytes()[3], b'1'..=b'9'))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
mod platform {
    use super::{RegularFileIdentity, RegularFileMetadata, RegularFileSnapshot, RootIdentity};
    use anyhow::{bail, Context, Result};
    use rustix::fd::OwnedFd;
    use rustix::fs::{
        fchmod, fstat, fsync, mkdirat, openat, openat2, renameat, renameat_with, statat, unlinkat,
        AtFlags, FileType, Mode, OFlags, RenameFlags, ResolveFlags, CWD,
    };
    use rustix::io::Errno;
    use std::fs;
    use std::io::{Read, Write};
    use std::path::Path;

    pub(super) struct Root {
        descriptor: OwnedFd,
        device: u64,
        inode: u64,
    }

    impl Root {
        pub(super) fn open(path: &Path) -> Result<Self> {
            let metadata = fs::symlink_metadata(path).with_context(|| {
                format!("could not inspect publication root {}", path.display())
            })?;
            if metadata.file_type().is_symlink() {
                bail!("publication root {} is a symlink", path.display());
            }

            let descriptor = openat(CWD, path, directory_flags(), Mode::empty())
                .with_context(|| format!("could not open publication root {}", path.display()))?;
            let stat = fstat(&descriptor).context("could not inspect opened publication root")?;
            if !FileType::from_raw_mode(stat.st_mode).is_dir() {
                bail!("publication root {} is not a directory", path.display());
            }

            Ok(Self {
                descriptor,
                device: stat.st_dev,
                inode: stat.st_ino,
            })
        }

        pub(super) fn identity(&self, path: &Path) -> Result<RootIdentity> {
            let canonical = fs::canonicalize(path).with_context(|| {
                format!("could not canonicalize publication root {}", path.display())
            })?;
            let canonical = canonical
                .to_str()
                .context("publication root canonical spelling is not valid UTF-8")?;
            Ok(RootIdentity::from_platform_components(
                canonical,
                &self.device.to_be_bytes(),
                &self.inode.to_be_bytes(),
            ))
        }

        pub(super) fn ensure_absent(&self, components: &[&str]) -> Result<()> {
            let Some(parent) = self.open_existing_parent(components)? else {
                return Ok(());
            };
            let leaf = leaf(components)?;
            match statat(&parent, leaf, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(stat) => {
                    if FileType::from_raw_mode(stat.st_mode).is_symlink() {
                        bail!("publication destination `{leaf}` is a symlink");
                    }
                    bail!("publication destination `{leaf}` already exists")
                }
                Err(Errno::NOENT) => Ok(()),
                Err(error) => Err(error).with_context(|| {
                    format!("could not inspect publication destination `{leaf}` without following links")
                }),
            }
        }

        pub(super) fn create_new_directory(&self, components: &[&str]) -> Result<Self> {
            let Some(parent) = self.open_existing_parent(components)? else {
                bail!("publication parent directory does not exist");
            };
            let leaf = leaf(components)?;
            match mkdirat(&parent, leaf, Mode::from_raw_mode(0o700)) {
                Ok(()) => {}
                Err(Errno::EXIST) => bail!("publication directory `{leaf}` already exists"),
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("could not create publication directory `{leaf}`")
                    })
                }
            }
            let descriptor = self
                .open_existing_directory(&parent, leaf)?
                .ok_or_else(|| {
                    anyhow::anyhow!("publication directory `{leaf}` disappeared after creation")
                })?;
            let stat = fstat(&descriptor).context("could not inspect new publication directory")?;
            Ok(Self {
                descriptor,
                device: self.device,
                inode: stat.st_ino,
            })
        }

        pub(super) fn sync(&self) -> Result<()> {
            fsync(&self.descriptor).context("could not synchronize publication directory")
        }

        pub(super) fn publish_new_directory(
            &self,
            source: &[&str],
            destination: &[&str],
        ) -> Result<()> {
            let Some(source_parent) = self.open_existing_parent(source)? else {
                bail!("publication source parent directory does not exist");
            };
            let Some(destination_parent) = self.open_existing_parent(destination)? else {
                bail!("publication destination parent directory does not exist");
            };
            let source_leaf = leaf(source)?;
            let destination_leaf = leaf(destination)?;
            let source_stat = statat(&source_parent, source_leaf, AtFlags::SYMLINK_NOFOLLOW)
                .with_context(|| format!("could not inspect publication source `{source_leaf}`"))?;
            let source_type = FileType::from_raw_mode(source_stat.st_mode);
            if source_type.is_symlink() || !source_type.is_dir() {
                bail!("publication source `{source_leaf}` is not a directory");
            }
            self.ensure_absent(destination)?;
            match renameat_with(
                &source_parent,
                source_leaf,
                &destination_parent,
                destination_leaf,
                RenameFlags::NOREPLACE,
            ) {
                Ok(()) => Ok(()),
                Err(Errno::EXIST) => bail!("publication destination `{destination_leaf}` already exists"),
                Err(Errno::NOSYS) | Err(Errno::INVAL) => unsupported_platform(),
                Err(error) => Err(error).with_context(|| {
                    format!(
                        "could not publish publication directory `{source_leaf}` without replacement"
                    )
                }),
            }
        }

        pub(super) fn remove_new_regular_if_exists(&self, components: &[&str]) -> Result<()> {
            let Some(parent) = self.open_existing_parent(components)? else {
                return Ok(());
            };
            let leaf = leaf(components)?;
            match self.stat_regular(&parent, leaf)? {
                Some(_) => unlinkat(&parent, leaf, AtFlags::empty())
                    .with_context(|| format!("could not remove publication file `{leaf}`")),
                None => Ok(()),
            }
        }

        pub(super) fn remove_empty_directory_if_exists(&self, components: &[&str]) -> Result<()> {
            let Some(parent) = self.open_existing_parent(components)? else {
                return Ok(());
            };
            let leaf = leaf(components)?;
            match statat(&parent, leaf, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(stat) => {
                    let file_type = FileType::from_raw_mode(stat.st_mode);
                    if file_type.is_symlink() || !file_type.is_dir() {
                        bail!("publication directory `{leaf}` is not an empty directory");
                    }
                    unlinkat(&parent, leaf, AtFlags::REMOVEDIR)
                        .with_context(|| format!("could not remove publication directory `{leaf}`"))
                }
                Err(Errno::NOENT) => Ok(()),
                Err(error) => Err(error).with_context(|| {
                    format!(
                        "could not inspect publication directory `{leaf}` without following links"
                    )
                }),
            }
        }

        pub(super) fn validate_parent_directories(&self, components: &[&str]) -> Result<()> {
            let _ = self.open_existing_parent(components)?;
            Ok(())
        }

        pub(super) fn create_parent_directories_tracking(
            &self,
            components: &[&str],
        ) -> Result<Vec<usize>> {
            let mut created = Vec::<(usize, OwnedFd, &str)>::new();
            let result = (|| {
                let mut directory = self.duplicate_root()?;
                for (index, component) in components[..components.len().saturating_sub(1)]
                    .iter()
                    .enumerate()
                {
                    match self.open_existing_directory(&directory, component)? {
                        Some(next) => directory = next,
                        None => {
                            let cleanup_parent =
                                open_beneath(&directory, ".", directory_flags(), Mode::empty())?;
                            mkdirat(&directory, *component, Mode::from_raw_mode(0o700))
                                .with_context(|| {
                                    format!("could not create publication directory `{component}`")
                                })?;
                            created.push((index + 1, cleanup_parent, component));
                            directory = self
                                .open_existing_directory(&directory, component)?
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                        "publication directory `{component}` disappeared after creation"
                                    )
                                })?;
                        }
                    }
                }
                Ok(())
            })();
            match result {
                Ok(()) => Ok(created.into_iter().map(|(depth, _, _)| depth).collect()),
                Err(error) => {
                    let mut cleanup_failures = Vec::new();
                    for (_, parent, component) in created.into_iter().rev() {
                        if let Err(cleanup_error) = unlinkat(&parent, component, AtFlags::REMOVEDIR)
                        {
                            cleanup_failures.push(cleanup_error.to_string());
                        }
                    }
                    if cleanup_failures.is_empty() {
                        Err(error)
                    } else {
                        Err(error.context(format!(
                            "could not remove partially created publication parents: {}",
                            cleanup_failures.join("; ")
                        )))
                    }
                }
            }
        }

        pub(super) fn read_regular_snapshot(
            &self,
            components: &[&str],
        ) -> Result<Option<RegularFileSnapshot>> {
            let Some(parent) = self.open_existing_parent(components)? else {
                return Ok(None);
            };
            let leaf = leaf(components)?;
            let Some(_) = self.stat_regular(&parent, leaf)? else {
                return Ok(None);
            };

            let descriptor = open_beneath(
                &parent,
                leaf,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )?;
            self.ensure_regular(&descriptor, "publication file")?;
            let stat = fstat(&descriptor)
                .with_context(|| format!("could not inspect publication file `{leaf}`"))?;
            let mut bytes = Vec::with_capacity(stat.st_size.try_into().unwrap_or(0));
            fs::File::from(descriptor)
                .read_to_end(&mut bytes)
                .with_context(|| format!("could not read publication file `{leaf}`"))?;
            Ok(Some(RegularFileSnapshot {
                bytes,
                metadata: RegularFileMetadata {
                    byte_length: stat
                        .st_size
                        .try_into()
                        .context("publication file has a negative size")?,
                    executable: stat.st_mode & 0o111 != 0,
                },
                identity: RegularFileIdentity {
                    device: stat.st_dev,
                    inode: stat.st_ino,
                },
            }))
        }

        pub(super) fn regular_metadata(
            &self,
            components: &[&str],
        ) -> Result<Option<RegularFileMetadata>> {
            let Some(parent) = self.open_existing_parent(components)? else {
                return Ok(None);
            };
            let leaf = leaf(components)?;
            let Some(_) = self.stat_regular(&parent, leaf)? else {
                return Ok(None);
            };
            let descriptor = open_beneath(
                &parent,
                leaf,
                OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            )?;
            let stat = fstat(&descriptor)
                .with_context(|| format!("could not inspect publication file `{leaf}`"))?;
            self.ensure_regular(&descriptor, "publication file")?;
            Ok(Some(RegularFileMetadata {
                byte_length: stat
                    .st_size
                    .try_into()
                    .context("publication file has a negative size")?,
                executable: stat.st_mode & 0o111 != 0,
            }))
        }

        pub(super) fn write_new_regular(
            &self,
            components: &[&str],
            bytes: &[u8],
            executable: bool,
        ) -> Result<()> {
            let Some(parent) = self.open_existing_parent(components)? else {
                bail!("publication parent directory does not exist");
            };
            let leaf = leaf(components)?;
            if self.stat_regular(&parent, leaf)?.is_some() {
                bail!("publication destination `{leaf}` already exists");
            }

            let descriptor = match openat2(
                &parent,
                leaf,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o600),
                resolve_flags(),
            ) {
                Ok(descriptor) => descriptor,
                Err(Errno::EXIST) => bail!("publication destination `{leaf}` already exists"),
                Err(Errno::NOSYS) => return unsupported_platform(),
                Err(Errno::XDEV) => bail!("publication path crosses a mount boundary"),
                Err(Errno::LOOP) => bail!("publication file `{leaf}` is a symlink"),
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!(
                            "could not create publication file `{leaf}` without following links"
                        )
                    })
                }
            };
            self.ensure_regular(&descriptor, "new publication file")?;
            let mode = if executable { 0o700 } else { 0o600 };
            fchmod(&descriptor, Mode::from_raw_mode(mode))
                .with_context(|| format!("could not set publication file mode for `{leaf}`"))?;
            let mut file = fs::File::from(descriptor);
            file.write_all(bytes)
                .with_context(|| format!("could not write publication file `{leaf}`"))?;
            file.sync_all()
                .with_context(|| format!("could not synchronize publication file `{leaf}`"))?;
            Ok(())
        }

        pub(super) fn publish_new_regular_no_replace(
            &self,
            source: &[&str],
            destination: &[&str],
        ) -> Result<()> {
            let parent = self.same_parent(source, destination)?;
            let source_leaf = leaf(source)?;
            let destination_leaf = leaf(destination)?;
            self.stat_regular(&parent, source_leaf)?
                .context("publication temporary file is absent")?;
            self.ensure_absent(destination)?;
            match renameat_with(
                &parent,
                source_leaf,
                &parent,
                destination_leaf,
                RenameFlags::NOREPLACE,
            ) {
                Ok(()) => Ok(()),
                Err(Errno::EXIST) => {
                    bail!("publication destination `{destination_leaf}` already exists")
                }
                Err(Errno::NOSYS) | Err(Errno::INVAL) => unsupported_platform(),
                Err(error) => Err(error).with_context(|| {
                    format!("could not atomically create publication file `{destination_leaf}`")
                }),
            }
        }

        pub(super) fn replace_regular_with_new(
            &self,
            source: &[&str],
            destination: &[&str],
        ) -> Result<()> {
            let parent = self.same_parent(source, destination)?;
            let source_leaf = leaf(source)?;
            let destination_leaf = leaf(destination)?;
            self.stat_regular(&parent, source_leaf)?
                .context("publication temporary file is absent")?;
            self.stat_regular(&parent, destination_leaf)?
                .context("publication replacement destination is absent")?;
            renameat(&parent, source_leaf, &parent, destination_leaf).with_context(|| {
                format!("could not atomically replace publication file `{destination_leaf}`")
            })
        }

        pub(super) fn remove_regular_if_matches(
            &self,
            components: &[&str],
            expected: &RegularFileSnapshot,
        ) -> Result<bool> {
            let Some(parent) = self.open_existing_parent(components)? else {
                return Ok(false);
            };
            let leaf = leaf(components)?;
            let Some(current) = self.read_regular_snapshot(components)? else {
                return Ok(false);
            };
            if current != *expected {
                return Ok(false);
            }
            self.stat_regular(&parent, leaf)?
                .context("publication rollback destination disappeared")?;
            unlinkat(&parent, leaf, AtFlags::empty())
                .with_context(|| format!("could not remove publication file `{leaf}`"))?;
            Ok(true)
        }

        fn duplicate_root(&self) -> Result<OwnedFd> {
            open_beneath(&self.descriptor, ".", directory_flags(), Mode::empty())
        }

        fn open_existing_parent(&self, components: &[&str]) -> Result<Option<OwnedFd>> {
            let mut directory = self.duplicate_root()?;
            for component in &components[..components.len().saturating_sub(1)] {
                match self.open_existing_directory(&directory, component)? {
                    Some(next) => directory = next,
                    None => return Ok(None),
                }
            }
            Ok(Some(directory))
        }

        fn same_parent(&self, source: &[&str], destination: &[&str]) -> Result<OwnedFd> {
            if source.len() != destination.len()
                || source[..source.len().saturating_sub(1)]
                    != destination[..destination.len().saturating_sub(1)]
            {
                bail!("publication temporary and destination files must share one directory");
            }
            self.open_existing_parent(source)?
                .context("publication temporary parent directory does not exist")
        }

        fn open_existing_directory(
            &self,
            parent: &OwnedFd,
            component: &str,
        ) -> Result<Option<OwnedFd>> {
            match statat(parent, component, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(stat) => {
                    let file_type = FileType::from_raw_mode(stat.st_mode);
                    if file_type.is_symlink() {
                        bail!("publication path component `{component}` is a symlink");
                    }
                    if !file_type.is_dir() {
                        bail!("publication path component `{component}` is not a directory");
                    }
                }
                Err(Errno::NOENT) => return Ok(None),
                Err(error) => {
                    return Err(error).with_context(|| {
                        format!("could not inspect publication path component `{component}`")
                    })
                }
            }

            let directory = open_beneath(parent, component, directory_flags(), Mode::empty())?;
            self.ensure_directory_on_root_device(&directory, component)?;
            Ok(Some(directory))
        }

        fn stat_regular(&self, parent: &OwnedFd, leaf: &str) -> Result<Option<rustix::fs::Stat>> {
            match statat(parent, leaf, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(stat) => {
                    let file_type = FileType::from_raw_mode(stat.st_mode);
                    if file_type.is_symlink() {
                        bail!("publication file `{leaf}` is a symlink");
                    }
                    if !file_type.is_file() {
                        bail!("publication file `{leaf}` is not a regular file");
                    }
                    Ok(Some(stat))
                }
                Err(Errno::NOENT) => Ok(None),
                Err(error) => Err(error).with_context(|| {
                    format!("could not inspect publication file `{leaf}` without following links")
                }),
            }
        }

        fn ensure_directory_on_root_device(
            &self,
            descriptor: &OwnedFd,
            component: &str,
        ) -> Result<()> {
            let stat = fstat(descriptor).with_context(|| {
                format!("could not inspect publication directory `{component}`")
            })?;
            if !FileType::from_raw_mode(stat.st_mode).is_dir() {
                bail!("publication path component `{component}` is not a directory");
            }
            if stat.st_dev != self.device {
                bail!("publication path component `{component}` crosses a mount boundary");
            }
            Ok(())
        }

        fn ensure_regular(&self, descriptor: &OwnedFd, label: &str) -> Result<()> {
            let stat = fstat(descriptor).with_context(|| format!("could not inspect {label}"))?;
            if !FileType::from_raw_mode(stat.st_mode).is_file() {
                bail!("{label} is not a regular file");
            }
            Ok(())
        }
    }

    fn directory_flags() -> OFlags {
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
    }

    fn resolve_flags() -> ResolveFlags {
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_XDEV
    }

    fn open_beneath(
        parent: &OwnedFd,
        component: &str,
        flags: OFlags,
        mode: Mode,
    ) -> Result<OwnedFd> {
        match openat2(parent, component, flags, mode, resolve_flags()) {
            Ok(descriptor) => Ok(descriptor),
            Err(Errno::NOSYS) => unsupported_platform(),
            Err(Errno::XDEV) => bail!("publication path crosses a mount boundary"),
            Err(Errno::LOOP) => bail!("publication path contains a symlink"),
            Err(error) => Err(error).with_context(|| {
                format!("could not open publication path component `{component}` without following links")
            }),
        }
    }

    fn unsupported_platform<T>() -> Result<T> {
        bail!(
            "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
        )
    }

    fn leaf<'a>(components: &'a [&'a str]) -> Result<&'a str> {
        components
            .last()
            .copied()
            .ok_or_else(|| anyhow::anyhow!("publication path must not be empty"))
    }
}

#[cfg(not(any(target_os = "linux", target_os = "android")))]
mod platform {
    use super::{RegularFileMetadata, RegularFileSnapshot, RootIdentity};
    use anyhow::{bail, Result};
    use std::path::Path;

    pub(super) struct Root;

    impl Root {
        pub(super) fn open(_path: &Path) -> Result<Self> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn identity(&self, _path: &Path) -> Result<RootIdentity> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn ensure_absent(&self, _components: &[&str]) -> Result<()> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn validate_parent_directories(&self, _components: &[&str]) -> Result<()> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn create_parent_directories_tracking(
            &self,
            _components: &[&str],
        ) -> Result<Vec<usize>> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn create_new_directory(&self, _components: &[&str]) -> Result<Self> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn sync(&self) -> Result<()> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn publish_new_directory(
            &self,
            _source: &[&str],
            _destination: &[&str],
        ) -> Result<()> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn remove_new_regular_if_exists(&self, _components: &[&str]) -> Result<()> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn remove_empty_directory_if_exists(&self, _components: &[&str]) -> Result<()> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn read_regular_snapshot(
            &self,
            _components: &[&str],
        ) -> Result<Option<RegularFileSnapshot>> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn regular_metadata(
            &self,
            _components: &[&str],
        ) -> Result<Option<RegularFileMetadata>> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn write_new_regular(
            &self,
            _components: &[&str],
            _bytes: &[u8],
            _executable: bool,
        ) -> Result<()> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn publish_new_regular_no_replace(
            &self,
            _source: &[&str],
            _destination: &[&str],
        ) -> Result<()> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn replace_regular_with_new(
            &self,
            _source: &[&str],
            _destination: &[&str],
        ) -> Result<()> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }

        pub(super) fn remove_regular_if_matches(
            &self,
            _components: &[&str],
            _expected: &RegularFileSnapshot,
        ) -> Result<bool> {
            bail!(
                "publication is unsupported on this platform: descriptor-relative no-follow operations cannot be guaranteed"
            )
        }
    }
}
