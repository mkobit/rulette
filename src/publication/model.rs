use crate::emitters::lowering::{
    CapabilityReasonCode, CapabilitySeverity, NativeArtifactClass, NativeTarget,
};
use crate::ir::graph::{PackageId, ResourcePath};
use anyhow::{bail, Result};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// The only destination scopes that the v0.1 publication contract recognizes.
///
/// A scope is intent, not filesystem authority. Root authorization and every
/// filesystem operation are deliberately deferred to the publication apply
/// layer.
#[derive(
    Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "lowercase")]
pub enum PublicationScope {
    Project,
    User,
}

/// A version identifier for the compiled-in target mapping registry.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MappingVersion {
    V0_1,
}

impl MappingVersion {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::V0_1 => "0.1",
        }
    }

    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "0.1" => Ok(Self::V0_1),
            _ => bail!("unsupported target mapping version `{value}`"),
        }
    }
}

/// An opaque digest that binds a plan to a caller-supplied root without
/// retaining its path, credentials, or authorization capability.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RootIdentity(String);

impl RootIdentity {
    /// Hashes canonical root spelling, volume identity, and file identity
    /// with length prefixes to avoid ambiguous component concatenation.
    pub fn from_platform_components(
        canonical_root: &str,
        volume_identity: &[u8],
        file_identity: &[u8],
    ) -> Self {
        let mut hasher = Sha256::new();
        hash_component(&mut hasher, canonical_root.as_bytes());
        hash_component(&mut hasher, volume_identity);
        hash_component(&mut hasher, file_identity);
        Self(format!("root_{}", hex::encode(hasher.finalize())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_digest("root", &value)?;
        Ok(Self(value))
    }
}

/// SHA-256 digest retained in plans for exact staged artifact or plan bytes.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PlanDigest(String);

impl PlanDigest {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(format!("sha256_{}", hex::encode(Sha256::digest(bytes))))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_digest("sha256", &value)?;
        Ok(Self(value))
    }
}

/// A validated, target-relative artifact descriptor.
///
/// It deliberately carries no root path and cannot name an arbitrary
/// destination. The mapping registry is the only layer that can turn it
/// into a root-relative native path.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ArtifactDescriptor {
    pub class: NativeArtifactClass,
    pub native_path: ResourcePath,
}

/// The eventual plan entry shape, kept free of roots and authority inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanEntry {
    pub entry_id: String,
    pub target: NativeTarget,
    pub mapping_version: MappingVersion,
    pub scope: PublicationScope,
    pub stage_artifact_path: ResourcePath,
    pub artifact: ArtifactDescriptor,
    pub content_digest: PlanDigest,
    pub byte_length: u64,
    pub executable: bool,
    pub source_package: PackageId,
}

/// A structured loss finding retained when a caller explicitly accepts loss.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanLossFinding {
    pub id: String,
    /// The staged artifact entry affected by this loss, when the finding is
    /// associated with a concrete artifact.
    pub entry_id: Option<String>,
    pub package_id: PackageId,
    pub target: NativeTarget,
    pub artifact: Option<ArtifactDescriptor>,
    pub severity: CapabilitySeverity,
    pub reason_code: CapabilityReasonCode,
    pub reason: String,
}

/// A root binding records identity only; it is not an apply-time capability.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RootBinding {
    pub target: NativeTarget,
    pub scope: PublicationScope,
    pub identity: RootIdentity,
}

/// Library-owned publication-plan model.
///
/// Serialization, staging, apply authorization, and filesystem I/O are
/// intentionally outside this foundation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublicationPlan {
    pub plan_version: &'static str,
    pub compiler_version: String,
    pub graph_version: String,
    pub graph_digest: PlanDigest,
    pub mappings: BTreeMap<(NativeTarget, PublicationScope), MappingVersion>,
    pub roots: Vec<RootBinding>,
    pub allow_lossy: bool,
    pub losses: Vec<PlanLossFinding>,
    pub entries: Vec<PlanEntry>,
}

fn hash_component(hasher: &mut Sha256, component: &[u8]) {
    hasher.update((component.len() as u64).to_be_bytes());
    hasher.update(component);
}

fn validate_digest(prefix: &str, value: &str) -> Result<()> {
    let expected_prefix = format!("{prefix}_");
    if value.len() != expected_prefix.len() + 64
        || !value.starts_with(&expected_prefix)
        || !value[expected_prefix.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("{prefix} digest must use `{prefix}_` followed by a SHA-256 hex digest");
    }
    Ok(())
}
