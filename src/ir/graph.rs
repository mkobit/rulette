use super::ActivationMode;
use anyhow::{bail, Result};
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::de::Deserializer;
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const GRAPH_VERSION: &str = "0.1";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct PackageId(String);

impl PackageId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn from_digest(digest: [u8; 32]) -> Self {
        Self(format!("pkg_{}", hex::encode(digest)))
    }

    fn validate(&self) -> Result<()> {
        if self.0.len() != 68
            || !self.0.starts_with("pkg_")
            || !self.0[4..].bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            bail!("package ID must be `pkg_` followed by a SHA-256 hex digest");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct SemanticIdentity(String);

impl SemanticIdentity {
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        let Some((kind, logical_name)) = value.split_once(':') else {
            bail!("semantic identity must have the form `<kind>:<logical-name>`");
        };
        if !matches!(kind, "rule" | "skill" | "unsupported") {
            bail!("semantic identity kind must be `rule`, `skill`, or `unsupported`");
        }
        if logical_name.is_empty()
            || logical_name.chars().any(char::is_control)
            || logical_name.contains(':')
        {
            bail!("semantic identity logical name must be non-empty and control-character free");
        }
        if kind == "skill" {
            validate_skill_name(logical_name)?;
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn kind(&self) -> &str {
        self.0
            .split_once(':')
            .map(|(kind, _)| kind)
            .expect("validated semantic identity has a kind")
    }

    fn validate(&self) -> Result<()> {
        Self::parse(self.0.clone()).map(|_| ())
    }
}

fn validate_skill_name(name: &str) -> Result<()> {
    let length = name.chars().count();
    if !(1..=64).contains(&length)
        || !name.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        || name.starts_with('-')
        || name.ends_with('-')
        || name.contains("--")
    {
        bail!("skill semantic identity must use the Agent Skills name grammar");
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct ResourcePath(String);

impl ResourcePath {
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty()
            || value.starts_with('/')
            || value.starts_with('\\')
            || value.contains('\\')
            || value.chars().any(char::is_control)
            || has_platform_prefix(&value)
        {
            bail!("resource path must be a safe slash-separated relative path");
        }

        for component in value.split('/') {
            if component.is_empty() || matches!(component, "." | "..") {
                bail!("resource path must not contain empty, dot, or parent components");
            }
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(&self) -> Result<()> {
        Self::parse(self.0.clone()).map(|_| ())
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct PackageRoot(String);

impl PackageRoot {
    pub fn parse(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value == "." {
            return Ok(Self(value));
        }
        ResourcePath::parse(value.clone())?;
        Ok(Self(value))
    }

    pub fn root() -> Self {
        Self(".".to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn validate(&self) -> Result<()> {
        Self::parse(self.0.clone()).map(|_| ())
    }
}

fn has_platform_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PackageKind {
    Rule,
    Skill,
    Unsupported,
}

impl PackageKind {
    fn semantic_kind(&self) -> Option<&'static str> {
        match self {
            Self::Rule => Some("rule"),
            Self::Skill => Some("skill"),
            Self::Unsupported => Some("unsupported"),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ResourceRole {
    PrimaryInstruction,
    Opaque,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResourceContent {
    Text(String),
    Bytes(Vec<u8>),
}

impl ResourceContent {
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Text(text) => text.as_bytes(),
            Self::Bytes(bytes) => bytes,
        }
    }

    fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }
}

impl Serialize for ResourceContent {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ResourceContent", 2)?;
        match self {
            Self::Text(text) => {
                state.serialize_field("encoding", "utf-8")?;
                state.serialize_field("content", text)?;
            }
            Self::Bytes(bytes) => {
                state.serialize_field("encoding", "base64")?;
                state.serialize_field("content", &base64_encode(bytes))?;
            }
        }
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(tag = "encoding", content = "content", deny_unknown_fields)]
enum ResourceContentWire {
    #[serde(rename = "utf-8")]
    Text(String),
    #[serde(rename = "base64")]
    Bytes(String),
}

impl<'de> Deserialize<'de> for ResourceContent {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match ResourceContentWire::deserialize(deserializer)? {
            ResourceContentWire::Text(text) => Ok(Self::Text(text)),
            ResourceContentWire::Bytes(encoded) => {
                let bytes = base64_decode(&encoded).map_err(serde::de::Error::custom)?;
                if base64_encode(&bytes) != encoded {
                    return Err(serde::de::Error::custom(
                        "base64 resource content must use canonical unwrapped RFC 4648 encoding",
                    ));
                }
                Ok(Self::Bytes(bytes))
            }
        }
    }
}

impl JsonSchema for ResourceContent {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "ResourceContent".into()
    }

    fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "oneOf": [
                {
                    "type": "object",
                    "properties": {
                        "encoding": { "const": "utf-8" },
                        "content": { "type": "string" }
                    },
                    "required": ["encoding", "content"],
                    "additionalProperties": false
                },
                {
                    "type": "object",
                    "properties": {
                        "encoding": { "const": "base64" },
                        "content": { "type": "string" }
                    },
                    "required": ["encoding", "content"],
                    "additionalProperties": false
                }
            ]
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    pub path: ResourcePath,
    pub role: ResourceRole,
    pub content: ResourceContent,
    pub executable: bool,
}

impl Resource {
    pub fn primary_instruction(
        path: ResourcePath,
        content: ResourceContent,
        executable: bool,
    ) -> Self {
        Self {
            path,
            role: ResourceRole::PrimaryInstruction,
            content,
            executable,
        }
    }

    pub fn opaque(path: ResourcePath, content: ResourceContent, executable: bool) -> Self {
        Self {
            path,
            role: ResourceRole::Opaque,
            content,
            executable,
        }
    }

    fn content_digest(&self) -> [u8; 32] {
        Sha256::digest(self.content.bytes()).into()
    }

    fn validate(&self) -> Result<()> {
        self.path.validate()?;
        if matches!(self.role, ResourceRole::PrimaryInstruction) && !self.content.is_text() {
            bail!("a primary instruction must contain UTF-8 text");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceProvenance {
    pub frontend: String,
    pub input_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive_member: Option<ResourcePath>,
}

impl SourceProvenance {
    pub fn new(frontend: impl Into<String>, input_label: impl Into<String>) -> Result<Self> {
        let provenance = Self {
            frontend: frontend.into(),
            input_label: input_label.into(),
            archive_member: None,
        };
        provenance.validate()?;
        Ok(provenance)
    }

    fn validate(&self) -> Result<()> {
        if self.frontend.is_empty()
            || self.frontend.chars().any(char::is_control)
            || self.input_label.is_empty()
            || self.input_label.chars().any(char::is_control)
        {
            bail!("source provenance frontend and input label must be non-empty and control-character free");
        }
        if self.input_label != "stdin" {
            ResourcePath::parse(self.input_label.clone())?;
        }
        if let Some(member) = &self.archive_member {
            member.validate()?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FrontendPayload {
    pub namespace: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, serde_json::Value>,
}

impl FrontendPayload {
    fn validate(&self) -> Result<()> {
        if self.namespace.is_empty() || self.namespace.chars().any(char::is_control) {
            bail!("frontend payload namespace must be non-empty and control-character free");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", tag = "kind", deny_unknown_fields)]
pub enum SemanticItem {
    Rule {
        primary_instruction: ResourcePath,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        activation: Option<TargetActivation>,
        #[serde(skip_serializing_if = "Option::is_none")]
        frontend_payload: Option<FrontendPayload>,
    },
    Skill {
        primary_instruction: ResourcePath,
        description: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        frontend_payload: Option<FrontendPayload>,
    },
    Unsupported {
        native_kind: String,
    },
}

impl SemanticItem {
    fn primary_instruction(&self) -> Option<&ResourcePath> {
        match self {
            Self::Rule {
                primary_instruction,
                ..
            }
            | Self::Skill {
                primary_instruction,
                ..
            } => Some(primary_instruction),
            Self::Unsupported { .. } => None,
        }
    }

    fn validate(&self) -> Result<()> {
        match self {
            Self::Rule {
                description,
                activation,
                ..
            } => {
                if description
                    .as_deref()
                    .is_some_and(|value| value.chars().any(char::is_control))
                {
                    bail!("rule description must be control-character free");
                }
                if let Some(activation) = activation {
                    validate_activation(activation)?;
                }
            }
            Self::Skill { description, .. } => {
                let length = description.chars().count();
                if !(1..=1024).contains(&length) {
                    bail!("skill discovery description must contain 1 to 1024 characters");
                }
            }
            Self::Unsupported { .. } => {}
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PortableActivation {
    pub mode: Vec<ActivationMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub globs: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetActivationOverrides {
    pub default: PortableActivation,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub overrides: BTreeMap<String, PortableActivation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum TargetActivation {
    Wrapped(TargetActivationOverrides),
    Bare(PortableActivation),
}

impl TargetActivation {
    pub fn resolve(&self, target: &str) -> &PortableActivation {
        match self {
            Self::Bare(activation) => activation,
            Self::Wrapped(overrides) => {
                let target = target.trim().to_ascii_lowercase();
                if let Some(activation) = overrides.overrides.iter().find_map(|(key, value)| {
                    (key.trim().eq_ignore_ascii_case(&target)).then_some(value)
                }) {
                    return activation;
                }
                if let Some(alias) = target
                    .split(['-', '_'])
                    .next()
                    .filter(|alias| *alias != target)
                {
                    if let Some(activation) = overrides.overrides.iter().find_map(|(key, value)| {
                        (key.trim().eq_ignore_ascii_case(alias)).then_some(value)
                    }) {
                        return activation;
                    }
                }
                &overrides.default
            }
        }
    }
}

fn validate_activation(activation: &TargetActivation) -> Result<()> {
    if let TargetActivation::Wrapped(overrides) = activation {
        let mut normalized = BTreeSet::new();
        for key in overrides.overrides.keys() {
            let key = key.trim().to_ascii_lowercase();
            if key.is_empty() || !normalized.insert(key) {
                bail!("activation override keys must be unique after normalized comparison");
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Package {
    pub id: PackageId,
    pub kind: PackageKind,
    pub semantic_identity: SemanticIdentity,
    pub provenance: SourceProvenance,
    pub package_root: PackageRoot,
    pub semantic_item: SemanticItem,
    pub resources: BTreeMap<ResourcePath, Resource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frontend_payload: Option<FrontendPayload>,
}

impl Package {
    pub fn rule(
        semantic_identity: SemanticIdentity,
        provenance: SourceProvenance,
        primary_instruction: Resource,
    ) -> Result<Self> {
        if !matches!(primary_instruction.role, ResourceRole::PrimaryInstruction) {
            bail!("a rule package requires a primary-instruction resource");
        }
        let primary_path = primary_instruction.path.clone();
        let mut resources = BTreeMap::new();
        resources.insert(primary_path.clone(), primary_instruction);
        Self::new(
            PackageKind::Rule,
            semantic_identity,
            provenance,
            PackageRoot::root(),
            SemanticItem::Rule {
                primary_instruction: primary_path,
                description: None,
                activation: None,
                frontend_payload: None,
            },
            resources,
            None,
        )
    }

    pub fn unsupported(
        semantic_identity: SemanticIdentity,
        provenance: SourceProvenance,
        native_kind: impl Into<String>,
        resource: Resource,
    ) -> Result<Self> {
        let resource_path = resource.path.clone();
        let mut resources = BTreeMap::new();
        resources.insert(resource_path, resource);
        Self::new(
            PackageKind::Unsupported,
            semantic_identity,
            provenance,
            PackageRoot::root(),
            SemanticItem::Unsupported {
                native_kind: native_kind.into(),
            },
            resources,
            None,
        )
    }

    pub fn new(
        kind: PackageKind,
        semantic_identity: SemanticIdentity,
        provenance: SourceProvenance,
        package_root: PackageRoot,
        semantic_item: SemanticItem,
        resources: BTreeMap<ResourcePath, Resource>,
        frontend_payload: Option<FrontendPayload>,
    ) -> Result<Self> {
        let id = package_id(&kind, &semantic_identity, &resources);
        let package = Self {
            id,
            kind,
            semantic_identity,
            provenance,
            package_root,
            semantic_item,
            resources,
            frontend_payload,
        };
        package.validate()?;
        Ok(package)
    }

    pub fn id(&self) -> &PackageId {
        &self.id
    }

    pub fn package_root(&self) -> &PackageRoot {
        &self.package_root
    }

    pub fn validate(&self) -> Result<()> {
        self.id.validate()?;
        self.semantic_identity.validate()?;
        self.provenance.validate()?;
        self.package_root.validate()?;
        self.semantic_item.validate()?;
        if let Some(payload) = &self.frontend_payload {
            payload.validate()?;
        }

        if self.kind.semantic_kind() != Some(self.semantic_identity.kind()) {
            bail!("package kind and semantic identity kind must match");
        }

        let mut primary_paths = Vec::new();
        for (path, resource) in &self.resources {
            path.validate()?;
            resource.validate()?;
            if path != &resource.path {
                bail!("resource map key must equal the resource path");
            }
            if matches!(resource.role, ResourceRole::PrimaryInstruction) {
                primary_paths.push(path);
            }
        }

        match &self.semantic_item {
            SemanticItem::Rule {
                frontend_payload, ..
            }
            | SemanticItem::Skill {
                frontend_payload, ..
            } => {
                if let Some(payload) = frontend_payload {
                    payload.validate()?;
                }
                if primary_paths.len() != 1 {
                    bail!("a supported package must contain exactly one primary instruction");
                }
                if self.semantic_item.primary_instruction() != Some(primary_paths[0]) {
                    bail!("semantic item must reference the package primary instruction");
                }
            }
            SemanticItem::Unsupported { native_kind } => {
                if native_kind.is_empty() || native_kind.chars().any(char::is_control) {
                    bail!("unsupported native kind must be non-empty and control-character free");
                }
                if self.resources.is_empty() || !primary_paths.is_empty() {
                    bail!("unsupported packages must retain opaque resources without a primary instruction");
                }
            }
        }

        if self.id != package_id(&self.kind, &self.semantic_identity, &self.resources) {
            bail!("package ID does not match canonical package content");
        }

        Ok(())
    }
}

fn package_id(
    kind: &PackageKind,
    semantic_identity: &SemanticIdentity,
    resources: &BTreeMap<ResourcePath, Resource>,
) -> PackageId {
    let mut hasher = Sha256::new();
    hash_component(&mut hasher, GRAPH_VERSION.as_bytes());
    hash_component(&mut hasher, package_kind_name(kind).as_bytes());
    hash_component(&mut hasher, semantic_identity.as_str().as_bytes());
    for (path, resource) in resources {
        hash_component(&mut hasher, path.as_str().as_bytes());
        hash_component(&mut hasher, &[u8::from(resource.executable)]);
        hash_component(&mut hasher, &resource.content_digest());
    }
    PackageId::from_digest(hasher.finalize().into())
}

fn hash_component(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn package_kind_name(kind: &PackageKind) -> &'static str {
    match kind {
        PackageKind::Rule => "rule",
        PackageKind::Skill => "skill",
        PackageKind::Unsupported => "unsupported",
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticSeverity {
    Warning,
    UnsupportedSemantic,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GraphDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_id: Option<PackageId>,
}

impl GraphDiagnostic {
    fn sort_key(&self) -> (&DiagnosticSeverity, &str, Option<&PackageId>, &str) {
        (
            &self.severity,
            &self.code,
            self.package_id.as_ref(),
            &self.message,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompilationGraph {
    pub graph_version: String,
    pub packages: BTreeMap<PackageId, Package>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<GraphDiagnostic>,
}

impl CompilationGraph {
    pub fn new(packages: impl IntoIterator<Item = Package>) -> Result<Self> {
        let packages = packages.into_iter().collect::<Vec<_>>();
        let diagnostics = packages
            .iter()
            .filter_map(|package| match &package.semantic_item {
                SemanticItem::Unsupported { native_kind } => Some(GraphDiagnostic {
                    severity: DiagnosticSeverity::UnsupportedSemantic,
                    code: "unsupported-semantic".to_owned(),
                    message: format!(
                        "{} frontend recognized unsupported native {}",
                        package.provenance.frontend, native_kind
                    ),
                    package_id: Some(package.id.clone()),
                }),
                _ => None,
            })
            .collect();
        Self::from_parts(packages, diagnostics)
    }

    /// Constructs a graph from packages and diagnostics decoded from graph interchange.
    ///
    /// Diagnostics are retained and placed in their canonical stable order.
    pub(crate) fn from_parts(
        packages: impl IntoIterator<Item = Package>,
        mut diagnostics: Vec<GraphDiagnostic>,
    ) -> Result<Self> {
        let mut ordered = BTreeMap::new();
        for package in packages {
            let id = package.id.clone();
            if ordered.insert(id.clone(), package).is_some() {
                bail!("duplicate package ID `{}`", id.as_str());
            }
        }
        diagnostics.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
        let graph = Self {
            graph_version: GRAPH_VERSION.to_owned(),
            packages: ordered,
            diagnostics,
        };
        graph.validate()?;
        Ok(graph)
    }

    pub fn validate(&self) -> Result<()> {
        if self.graph_version != GRAPH_VERSION {
            bail!("unsupported graph version `{}`", self.graph_version);
        }
        let mut identities = BTreeSet::new();
        for (id, package) in &self.packages {
            if id != &package.id {
                bail!("package map key must equal the package ID");
            }
            package.validate()?;
            if !identities.insert(package.semantic_identity.clone()) {
                bail!(
                    "duplicate semantic identity `{}`",
                    package.semantic_identity.as_str()
                );
            }
        }
        if self
            .diagnostics
            .windows(2)
            .any(|pair| pair[0].sort_key() > pair[1].sort_key())
        {
            bail!("graph diagnostics must be in deterministic order");
        }
        for diagnostic in &self.diagnostics {
            if let Some(package_id) = &diagnostic.package_id {
                if !self.packages.contains_key(package_id) {
                    bail!(
                        "graph diagnostic references missing package ID `{}`",
                        package_id.as_str()
                    );
                }
            }
        }
        for package in self.packages.values() {
            if matches!(package.semantic_item, SemanticItem::Unsupported { .. })
                && !self.diagnostics.iter().any(|diagnostic| {
                    diagnostic.code == "unsupported-semantic"
                        && diagnostic.package_id.as_ref() == Some(&package.id)
                })
            {
                bail!("unsupported package must have an unsupported-semantic diagnostic");
            }
        }
        Ok(())
    }

    pub fn to_canonical_json(&self) -> Result<String> {
        self.validate()?;
        let mut output = serde_json::to_string_pretty(self)?;
        output.push('\n');
        Ok(output)
    }

    pub fn from_json(input: &str) -> Result<Self> {
        reject_duplicate_json_object_keys(input)?;
        let graph: Self = serde_json::from_str(input)?;
        graph.validate()?;
        Ok(graph)
    }

    pub fn to_toml(&self) -> Result<String> {
        self.validate()?;
        Ok(toml::to_string(self)?)
    }

    pub fn from_toml(input: &str) -> Result<Self> {
        let graph: Self = toml::from_str(input)?;
        graph.validate()?;
        Ok(graph)
    }
}

fn reject_duplicate_json_object_keys(input: &str) -> Result<()> {
    let mut validator = JsonKeyValidator {
        input: input.as_bytes(),
        index: 0,
    };
    validator.value()?;
    validator.whitespace();
    if validator.index != validator.input.len() {
        bail!("invalid trailing content in graph JSON");
    }
    Ok(())
}

struct JsonKeyValidator<'a> {
    input: &'a [u8],
    index: usize,
}

impl JsonKeyValidator<'_> {
    fn value(&mut self) -> Result<()> {
        self.whitespace();
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => self.string().map(|_| ()),
            Some(b't') => self.literal(b"true"),
            Some(b'f') => self.literal(b"false"),
            Some(b'n') => self.literal(b"null"),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => bail!("invalid JSON value while checking duplicate object keys"),
        }
    }

    fn object(&mut self) -> Result<()> {
        self.expect(b'{')?;
        self.whitespace();
        if self.consume(b'}') {
            return Ok(());
        }

        let mut keys = BTreeSet::new();
        loop {
            self.whitespace();
            let key = self.string()?;
            if !keys.insert(key.clone()) {
                bail!("duplicate JSON object key `{key}` in graph interchange");
            }
            self.whitespace();
            self.expect(b':')?;
            self.value()?;
            self.whitespace();
            if self.consume(b'}') {
                return Ok(());
            }
            self.expect(b',')?;
        }
    }

    fn array(&mut self) -> Result<()> {
        self.expect(b'[')?;
        self.whitespace();
        if self.consume(b']') {
            return Ok(());
        }
        loop {
            self.value()?;
            self.whitespace();
            if self.consume(b']') {
                return Ok(());
            }
            self.expect(b',')?;
        }
    }

    fn string(&mut self) -> Result<String> {
        self.whitespace();
        let start = self.index;
        self.expect(b'"')?;
        while let Some(byte) = self.peek() {
            match byte {
                b'"' => {
                    self.index += 1;
                    let raw = std::str::from_utf8(&self.input[start..self.index])?;
                    return Ok(serde_json::from_str(raw)?);
                }
                b'\\' => {
                    self.index += 1;
                    let escape = self
                        .peek()
                        .ok_or_else(|| anyhow::anyhow!("unterminated JSON escape"))?;
                    self.index += 1;
                    if escape == b'u' {
                        for _ in 0..4 {
                            if self.peek().is_none() {
                                bail!("unterminated JSON unicode escape");
                            }
                            self.index += 1;
                        }
                    }
                }
                _ => self.index += 1,
            }
        }
        bail!("unterminated JSON string");
    }

    fn number(&mut self) -> Result<()> {
        let start = self.index;
        while matches!(
            self.peek(),
            Some(b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E')
        ) {
            self.index += 1;
        }
        if self.index == start {
            bail!("expected JSON number");
        }
        Ok(())
    }

    fn literal(&mut self, literal: &[u8]) -> Result<()> {
        if self.input.get(self.index..self.index + literal.len()) != Some(literal) {
            bail!("invalid JSON literal");
        }
        self.index += literal.len();
        Ok(())
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.index += 1;
        }
    }

    fn expect(&mut self, expected: u8) -> Result<()> {
        if self.consume(expected) {
            Ok(())
        } else {
            bail!("expected `{}` in graph JSON", expected as char)
        }
    }

    fn consume(&mut self, expected: u8) -> bool {
        if self.peek() == Some(expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.index).copied()
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    use base64::Engine as _;

    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn base64_decode(encoded: &str) -> Result<Vec<u8>> {
    use base64::Engine as _;

    Ok(base64::engine::general_purpose::STANDARD.decode(encoded)?)
}

#[cfg(test)]
mod tests {
    use super::{
        CompilationGraph, DiagnosticSeverity, GraphDiagnostic, Package, PackageId, Resource,
        ResourceContent, ResourcePath, SemanticIdentity, SourceProvenance,
    };

    fn supported_rule_package() -> Package {
        Package::rule(
            SemanticIdentity::parse("rule:repository-guidance")
                .expect("rule semantic identity is valid"),
            SourceProvenance::new("codex", "fixtures/AGENTS.md")
                .expect("source provenance is valid"),
            Resource::primary_instruction(
                ResourcePath::parse("AGENTS.md").expect("primary instruction path is valid"),
                ResourceContent::Text("# Repository guidance\n".to_owned()),
                false,
            ),
        )
        .expect("supported rule package is valid")
    }

    #[test]
    fn supported_rule_package_id_is_deterministic() {
        let first = supported_rule_package();
        let second = supported_rule_package();

        assert_eq!(first.id(), second.id());
        assert!(first.id().as_str().starts_with("pkg_"));
        assert_eq!(first.id().as_str().len(), "pkg_".len() + 64);
    }

    #[test]
    fn resource_path_rejects_unsafe_values() {
        for path in [
            "",
            "/AGENTS.md",
            "C:/AGENTS.md",
            "./AGENTS.md",
            "../AGENTS.md",
            "rules/../AGENTS.md",
            "rules//AGENTS.md",
            "rules\\AGENTS.md",
            "rules/\0AGENTS.md",
        ] {
            assert!(
                ResourcePath::parse(path).is_err(),
                "{path:?} must be rejected"
            );
        }
    }

    #[test]
    fn valid_graph_canonical_json_is_byte_identical() {
        let graph = CompilationGraph::new([supported_rule_package()])
            .expect("one supported rule package is a valid graph");

        let first = graph
            .to_canonical_json()
            .expect("graph serializes to canonical JSON");
        let second = graph
            .to_canonical_json()
            .expect("graph serializes to canonical JSON again");

        assert_eq!(first, second);
        assert!(first.ends_with('\n'));
    }

    #[test]
    fn supported_package_records_a_relative_package_root() {
        let package = supported_rule_package();

        assert_eq!(package.package_root().as_str(), ".");
    }

    #[test]
    fn unsupported_native_package_is_retained_with_a_diagnostic() {
        let package = Package::unsupported(
            SemanticIdentity::parse("unsupported:opencode-agent-reviewer")
                .expect("unsupported semantic identity is valid"),
            SourceProvenance::new("opencode", ".opencode/agents/reviewer.md")
                .expect("source provenance is valid"),
            "agent",
            Resource::opaque(
                ResourcePath::parse("reviewer.md").expect("resource path is valid"),
                ResourceContent::Text("---\nmode: subagent\n---\nReview changes.".to_owned()),
                false,
            ),
        )
        .expect("unsupported package is valid");

        let graph = CompilationGraph::new([package]).expect("graph retains unsupported package");

        assert_eq!(graph.packages.len(), 1);
        assert!(graph
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "unsupported-semantic"));
    }

    #[test]
    fn skill_identity_uses_the_agent_skills_name_grammar() {
        for identity in [
            "skill:UPPER CASE",
            "skill:-leading",
            "skill:trailing-",
            "skill:a--b",
        ] {
            assert!(
                SemanticIdentity::parse(identity).is_err(),
                "{identity:?} must be rejected"
            );
        }
    }

    #[test]
    fn graph_interchange_rejects_unknown_provenance_fields() {
        let graph = CompilationGraph::new([supported_rule_package()]).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(&graph.to_canonical_json().unwrap()).unwrap();
        let package = value["packages"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap();
        package["provenance"]["unexpected"] = serde_json::json!(true);

        assert!(CompilationGraph::from_json(&value.to_string()).is_err());
    }

    #[test]
    fn graph_interchange_rejects_duplicate_package_keys() {
        let package = supported_rule_package();
        let package_json = serde_json::to_string(&package).unwrap();
        let input = format!(
            r#"{{"graph_version":"0.1","packages":{{"{id}":{package},"{id}":{package}}},"diagnostics":[]}}"#,
            id = package.id().as_str(),
            package = package_json,
        );

        assert!(CompilationGraph::from_json(&input).is_err());
    }

    #[test]
    fn graph_interchange_rejects_duplicate_resource_keys_and_invalid_base64() {
        let duplicate_resource_key = r#"{
          "graph_version":"0.1",
          "packages":{
            "pkg_placeholder":{
              "resources":{"AGENTS.md":{},"AGENTS.md":{}}
            }
          }
        }"#;

        assert!(CompilationGraph::from_json(duplicate_resource_key).is_err());

        let graph = CompilationGraph::new([supported_rule_package()]).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(&graph.to_canonical_json().unwrap()).unwrap();
        let resource = value["packages"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap()["resources"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap();
        resource["content"] = serde_json::json!({"encoding":"base64","content":"!!!"});

        assert!(CompilationGraph::from_json(&value.to_string()).is_err());
    }

    #[test]
    fn graph_interchange_rejects_an_unsupported_version_and_legacy_document() {
        let graph = CompilationGraph::new([supported_rule_package()]).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(&graph.to_canonical_json().unwrap()).unwrap();
        value["graph_version"] = serde_json::json!("1.0");

        assert!(CompilationGraph::from_json(&value.to_string()).is_err());
        assert!(CompilationGraph::from_json(r#"{"ir_version":"0.1","entities":[]}"#).is_err());
    }

    #[test]
    fn graph_interchange_rejects_diagnostics_for_missing_packages() {
        let mut graph = CompilationGraph::new([supported_rule_package()]).unwrap();
        graph.diagnostics.push(GraphDiagnostic {
            severity: DiagnosticSeverity::Warning,
            code: "missing-package".to_owned(),
            message: "diagnostic package is absent".to_owned(),
            package_id: Some(PackageId(format!("pkg_{}", "0".repeat(64)))),
        });

        assert!(CompilationGraph::from_json(&serde_json::to_string(&graph).unwrap()).is_err());
        assert!(CompilationGraph::from_toml(&toml::to_string(&graph).unwrap()).is_err());
    }

    #[test]
    fn unsupported_diagnostics_are_deterministically_ordered() {
        let packages = ["alpha", "bravo", "charlie", "delta"].map(|name| {
            Package::unsupported(
                SemanticIdentity::parse(format!("unsupported:opencode-agent-{name}")).unwrap(),
                SourceProvenance::new("opencode", format!(".opencode/agents/{name}.md")).unwrap(),
                "agent",
                Resource::opaque(
                    ResourcePath::parse(format!("{name}.md")).unwrap(),
                    ResourceContent::Text(name.to_owned()),
                    false,
                ),
            )
            .unwrap()
        });

        let graph = CompilationGraph::new(packages).expect("unsupported diagnostics are sorted");
        assert!(graph
            .diagnostics
            .windows(2)
            .all(|pair| pair[0].sort_key() <= pair[1].sort_key()));
    }

    #[test]
    fn graph_interchange_rejects_unknown_activation_fields() {
        let graph = CompilationGraph::new([supported_rule_package()]).unwrap();
        let mut value: serde_json::Value =
            serde_json::from_str(&graph.to_canonical_json().unwrap()).unwrap();
        let package = value["packages"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap();
        package["semantic_item"]["activation"] = serde_json::json!({
            "mode": ["always"],
            "unexpected": true
        });

        assert!(CompilationGraph::from_json(&value.to_string()).is_err());
    }

    #[test]
    fn graph_interchange_round_trips_canonical_json_and_toml() {
        let graph = CompilationGraph::new([supported_rule_package()]).unwrap();
        let json = graph.to_canonical_json().unwrap();
        let toml = graph.to_toml().unwrap();

        assert_eq!(
            CompilationGraph::from_json(&json)
                .unwrap()
                .to_canonical_json()
                .unwrap(),
            json
        );
        assert_eq!(
            CompilationGraph::from_toml(&toml)
                .unwrap()
                .to_canonical_json()
                .unwrap(),
            json
        );
    }

    #[test]
    fn graph_type_exposes_a_json_schema() {
        let schema = schemars::schema_for!(CompilationGraph);
        let value = serde_json::to_value(schema).unwrap();

        assert!(value["properties"].get("graph_version").is_some());
        assert!(value["properties"].get("packages").is_some());
    }
}
