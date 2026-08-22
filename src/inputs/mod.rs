use crate::ResourcePath;
use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File, Metadata};
use std::io::{BufRead, BufReader, Read};
use std::path::{Component, Path};
use tar::Archive;

pub const MAX_OBSERVATIONS: usize = 10_000;
pub const MAX_RESOURCE_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_TOTAL_RESOURCE_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObservationLimits {
    pub max_observations: usize,
    pub max_resource_bytes: usize,
    pub max_total_bytes: usize,
}

impl Default for ObservationLimits {
    fn default() -> Self {
        Self {
            max_observations: MAX_OBSERVATIONS,
            max_resource_bytes: MAX_RESOURCE_BYTES,
            max_total_bytes: MAX_TOTAL_RESOURCE_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputOrigin {
    Filesystem,
    Stdin,
    Tar,
    GzipTar,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationProvenance {
    pub input_label: String,
    pub archive_member: Option<ResourcePath>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactObservation {
    pub bytes: Vec<u8>,
    pub source_path: ResourcePath,
    pub executable: bool,
    pub provenance: ObservationProvenance,
    pub origin: InputOrigin,
}

impl ArtifactObservation {
    pub fn new(
        bytes: Vec<u8>,
        source_path: impl Into<String>,
        executable: bool,
        origin: InputOrigin,
        input_label: impl Into<String>,
        archive_member: Option<ResourcePath>,
    ) -> Result<Self> {
        let source_path = ResourcePath::parse(source_path)?;
        let input_label = input_label.into();
        validate_input_label(&input_label)?;

        match origin {
            InputOrigin::Tar | InputOrigin::GzipTar if archive_member.is_none() => {
                bail!("archive observations must retain their archive member path")
            }
            InputOrigin::Filesystem | InputOrigin::Stdin if archive_member.is_some() => {
                bail!("non-archive observations must not retain an archive member path")
            }
            _ => {}
        }

        Ok(Self {
            bytes,
            source_path,
            executable,
            provenance: ObservationProvenance {
                input_label,
                archive_member,
            },
            origin,
        })
    }
}

pub fn observe_path(path: impl AsRef<Path>) -> Result<Vec<ArtifactObservation>> {
    observe_path_with_limits(path, ObservationLimits::default())
}

pub fn observe_path_with_limits(
    path: impl AsRef<Path>,
    limits: ObservationLimits,
) -> Result<Vec<ArtifactObservation>> {
    let path = path.as_ref();
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("could not inspect input path {}", path.display()))?;
    if metadata.file_type().is_symlink() {
        bail!("input path {} is a symlink", path.display());
    }

    let input_label = input_label(path)?;
    let mut collector = ObservationCollector::new(limits);
    if metadata.is_dir() {
        observe_directory(path, path, &input_label, &mut collector)?;
    } else if metadata.is_file() {
        match archive_origin(path)? {
            Some(InputOrigin::GzipTar) => {
                let file = File::open(path)
                    .with_context(|| format!("could not open gzip tar input {}", path.display()))?;
                collector.extend(observe_gzip_tar_with_limits(file, &input_label, limits)?)?;
            }
            Some(InputOrigin::Tar) => {
                let file = File::open(path)
                    .with_context(|| format!("could not open tar input {}", path.display()))?;
                collector.extend(observe_tar_with_limits(file, &input_label, limits)?)?;
            }
            None => {
                let source_path = file_name_path(path)?;
                let bytes = read_resource(
                    File::open(path)
                        .with_context(|| format!("could not open input file {}", path.display()))?,
                    limits.max_resource_bytes,
                )?;
                collector.push(ArtifactObservation::new(
                    bytes,
                    source_path.as_str(),
                    executable(&metadata),
                    InputOrigin::Filesystem,
                    input_label,
                    None,
                )?)?;
            }
            Some(InputOrigin::Filesystem | InputOrigin::Stdin) => {
                unreachable!("path inspection only recognizes tar archive origins")
            }
        }
    } else {
        bail!(
            "input path {} is not a regular file or directory",
            path.display()
        );
    }

    Ok(collector.finish())
}

pub fn observe_stdin<R: Read>(reader: R) -> Result<Vec<ArtifactObservation>> {
    observe_stdin_with_limits(reader, ObservationLimits::default())
}

pub fn observe_stdin_with_limits<R: Read>(
    reader: R,
    limits: ObservationLimits,
) -> Result<Vec<ArtifactObservation>> {
    let mut reader = BufReader::with_capacity(512, reader);
    let origin = {
        let prefix = reader
            .fill_buf()
            .context("could not inspect standard input")?;
        if prefix.starts_with(&[0x1f, 0x8b]) {
            InputOrigin::GzipTar
        } else if is_ustar(prefix) {
            InputOrigin::Tar
        } else {
            InputOrigin::Stdin
        }
    };

    match origin {
        InputOrigin::GzipTar => observe_gzip_tar_with_limits(reader, "stdin", limits),
        InputOrigin::Tar => observe_tar_with_limits(reader, "stdin", limits),
        InputOrigin::Stdin => {
            let bytes = read_resource(reader, limits.max_resource_bytes)?;
            let mut collector = ObservationCollector::new(limits);
            collector.push(ArtifactObservation::new(
                bytes,
                "stdin",
                false,
                InputOrigin::Stdin,
                "stdin",
                None,
            )?)?;
            Ok(collector.finish())
        }
        InputOrigin::Filesystem => unreachable!("stdin has no filesystem observation origin"),
    }
}

fn observe_directory(
    root: &Path,
    directory: &Path,
    input_label: &str,
    collector: &mut ObservationCollector,
) -> Result<()> {
    let mut entries = fs::read_dir(directory)
        .with_context(|| format!("could not read input directory {}", directory.display()))?
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("could not inspect input path {}", path.display()))?;
        if metadata.file_type().is_symlink() {
            bail!("input path {} is a symlink", path.display());
        }
        if metadata.is_dir() {
            observe_directory(root, &path, input_label, collector)?;
            continue;
        }
        if !metadata.is_file() {
            bail!("input path {} is not a regular file", path.display());
        }

        let source_path = relative_path(root, &path)?;
        let bytes = read_resource(
            File::open(&path)
                .with_context(|| format!("could not open input file {}", path.display()))?,
            collector.limits.max_resource_bytes,
        )?;
        collector.push(ArtifactObservation::new(
            bytes,
            source_path.as_str(),
            executable(&metadata),
            InputOrigin::Filesystem,
            input_label,
            None,
        )?)?;
    }

    Ok(())
}

#[cfg(test)]
fn observe_tar<R: Read>(reader: R, input_label: &str) -> Result<Vec<ArtifactObservation>> {
    observe_tar_with_limits(reader, input_label, ObservationLimits::default())
}

fn observe_tar_with_limits<R: Read>(
    reader: R,
    input_label: &str,
    limits: ObservationLimits,
) -> Result<Vec<ArtifactObservation>> {
    observe_archive(reader, input_label, InputOrigin::Tar, limits)
}

fn observe_gzip_tar_with_limits<R: Read>(
    reader: R,
    input_label: &str,
    limits: ObservationLimits,
) -> Result<Vec<ArtifactObservation>> {
    observe_archive(
        GzDecoder::new(reader),
        input_label,
        InputOrigin::GzipTar,
        limits,
    )
}

fn observe_archive<R: Read>(
    reader: R,
    input_label: &str,
    origin: InputOrigin,
    limits: ObservationLimits,
) -> Result<Vec<ArtifactObservation>> {
    let mut archive = Archive::new(reader);
    let mut collector = ObservationCollector::new(limits);
    let mut member_paths = BTreeSet::new();

    for entry in archive
        .entries()
        .context("could not read archive entries")?
    {
        let mut entry = entry.context("could not read archive entry")?;
        if !entry.header().entry_type().is_file() {
            bail!("archive contains a non-regular entry");
        }
        if entry.size() > limits.max_resource_bytes as u64 {
            bail!(
                "resource byte limit exceeded: archive member is larger than {} bytes",
                limits.max_resource_bytes
            );
        }

        let source_path = archive_member_path(&entry)?;
        if !member_paths.insert(source_path.clone()) {
            bail!(
                "archive contains duplicate member path {}",
                source_path.as_str()
            );
        }
        let bytes = read_resource(&mut entry, limits.max_resource_bytes)?;
        let source_path_string = source_path.as_str().to_owned();
        collector.push(ArtifactObservation::new(
            bytes,
            source_path_string,
            false,
            origin,
            input_label,
            Some(source_path),
        )?)?;
    }

    Ok(collector.finish())
}

fn read_resource<R: Read>(reader: R, max_resource_bytes: usize) -> Result<Vec<u8>> {
    let mut reader = reader.take(max_resource_bytes.saturating_add(1) as u64);
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .context("could not read input resource")?;
    if bytes.len() > max_resource_bytes {
        bail!(
            "resource byte limit exceeded: resource is larger than {} bytes",
            max_resource_bytes
        );
    }
    Ok(bytes)
}

fn input_label(path: &Path) -> Result<String> {
    if path.as_os_str().is_empty() {
        bail!("input path must not be empty");
    }
    if path.is_absolute() {
        return hashed_input_label(path);
    }

    if let Some(label) = normalized_relative_input_label(path)? {
        return Ok(label);
    }

    hashed_input_label(path)
}

fn hashed_input_label(path: &Path) -> Result<String> {
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("could not canonicalize input path {}", path.display()))?;
    let canonical = canonical
        .to_str()
        .context("input path cannot be represented as UTF-8")?;
    Ok(format!(
        "input_{}",
        hex::encode(Sha256::digest(canonical.as_bytes()))
    ))
}

fn normalized_relative_input_label(path: &Path) -> Result<Option<String>> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(component) => components.push(
                component
                    .to_str()
                    .context("input path cannot be represented as UTF-8")?,
            ),
            Component::ParentDir => bail!("input path must not escape its declared input root"),
            Component::RootDir | Component::Prefix(_) => {
                bail!("input path must be relative or an absolute root")
            }
        }
    }

    if components.is_empty() {
        return Ok(None);
    }
    let label = components.join("/");
    ResourcePath::parse(label.clone()).context("input path must be a safe relative path")?;
    Ok(Some(label))
}

fn validate_input_label(label: &str) -> Result<()> {
    if label == "stdin" {
        return Ok(());
    }
    ResourcePath::parse(label).context("input label must be stdin or a safe relative path")?;
    Ok(())
}

fn relative_path(root: &Path, path: &Path) -> Result<ResourcePath> {
    let relative = path
        .strip_prefix(root)
        .context("input path escapes its declared input root")?;
    path_as_resource_path(relative, "input path")
}

fn file_name_path(path: &Path) -> Result<ResourcePath> {
    let name = path
        .file_name()
        .context("input file path must have a file name")?;
    path_as_resource_path(Path::new(name), "input file name")
}

fn archive_member_path<R: Read>(entry: &tar::Entry<'_, R>) -> Result<ResourcePath> {
    let path = entry
        .path()
        .context("archive member path must be a safe relative path")?;
    path_as_resource_path(&path, "archive member path")
}

fn path_as_resource_path(path: &Path, context: &str) -> Result<ResourcePath> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(c) => {
                let s = c
                    .to_str()
                    .with_context(|| format!("{context} cannot be represented as UTF-8"))?;
                components.push(s);
            }
            Component::CurDir => {}
            _ => bail!("{context} must be a safe relative path"),
        }
    }
    let joined = components.join("/");
    ResourcePath::parse(joined).with_context(|| format!("{context} must be a safe relative path"))
}

fn archive_origin(path: &Path) -> Result<Option<InputOrigin>> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("input file name cannot be represented as UTF-8")?
        .to_ascii_lowercase();
    if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        return Ok(Some(InputOrigin::GzipTar));
    }
    if name.ends_with(".tar") {
        return Ok(Some(InputOrigin::Tar));
    }

    let mut reader = BufReader::with_capacity(512, File::open(path)?);
    let prefix = reader.fill_buf().context("could not inspect input file")?;
    if prefix.starts_with(&[0x1f, 0x8b]) {
        Ok(Some(InputOrigin::GzipTar))
    } else if is_ustar(prefix) {
        Ok(Some(InputOrigin::Tar))
    } else {
        Ok(None)
    }
}

fn is_ustar(prefix: &[u8]) -> bool {
    prefix.len() >= 262 && &prefix[257..262] == b"ustar"
}

#[cfg(unix)]
fn executable(metadata: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn executable(_metadata: &Metadata) -> bool {
    false
}

struct ObservationCollector {
    limits: ObservationLimits,
    total_bytes: usize,
    observations: Vec<ArtifactObservation>,
}

impl ObservationCollector {
    fn new(limits: ObservationLimits) -> Self {
        Self {
            limits,
            total_bytes: 0,
            observations: Vec::new(),
        }
    }

    fn push(&mut self, observation: ArtifactObservation) -> Result<()> {
        if self.observations.len() >= self.limits.max_observations {
            bail!(
                "observation limit exceeded: at most {} observations are allowed",
                self.limits.max_observations
            );
        }
        if observation.bytes.len() > self.limits.max_resource_bytes {
            bail!(
                "resource byte limit exceeded: resource is larger than {} bytes",
                self.limits.max_resource_bytes
            );
        }
        let total_bytes = self
            .total_bytes
            .checked_add(observation.bytes.len())
            .context("total input byte count overflowed")?;
        if total_bytes > self.limits.max_total_bytes {
            bail!(
                "total byte limit exceeded: at most {} bytes are allowed",
                self.limits.max_total_bytes
            );
        }
        self.total_bytes = total_bytes;
        self.observations.push(observation);
        Ok(())
    }

    fn extend(&mut self, observations: Vec<ArtifactObservation>) -> Result<()> {
        for observation in observations {
            self.push(observation)?;
        }
        Ok(())
    }

    fn finish(mut self) -> Vec<ArtifactObservation> {
        self.observations.sort_by(|left, right| {
            (
                &left.provenance.input_label,
                &left.provenance.archive_member,
                &left.source_path,
            )
                .cmp(&(
                    &right.provenance.input_label,
                    &right.provenance.archive_member,
                    &right.source_path,
                ))
        });
        self.observations
    }
}

#[cfg(test)]
mod tests;
