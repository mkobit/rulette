use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

pub const FORBIDDEN_KEYS: &[&str] = &[
    "secrets",
    "bindings",
    "registries",
    "additionalWorkspaces",
    "localWorkspaces",
];

pub const REQUIRED_PORTS: &[u16] = &[];

pub fn is_valid_kit_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    let chars: Vec<char> = name.chars().collect();
    if !chars[0].is_ascii_lowercase() && !chars[0].is_ascii_digit() {
        return false;
    }
    if !chars.last().copied().unwrap_or(' ').is_ascii_lowercase()
        && !chars.last().copied().unwrap_or(' ').is_ascii_digit()
    {
        return false;
    }
    chars
        .iter()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
}

pub fn is_valid_env_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn is_valid_dotted_path(path: &str) -> bool {
    if path.is_empty() {
        return false;
    }
    for segment in path.split('.') {
        let mut chars = segment.chars();
        match chars.next() {
            Some(c) if c.is_ascii_lowercase() => {}
            _ => return false,
        }
        if !chars.all(|c| c.is_ascii_alphanumeric()) {
            return false;
        }
    }
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SchemaVersion {
    Number(u64),
    String(String),
}

impl SchemaVersion {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Number(n) => match n {
                1 => "1",
                2 => "2",
                _ => "unknown",
            },
            Self::String(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StringOrVec {
    Single(String),
    Multiple(Vec<String>),
}

impl StringOrVec {
    pub fn contains_item(&self, item: &str) -> bool {
        match self {
            Self::Single(s) => s == item,
            Self::Multiple(vec) => vec.iter().any(|s| s == item),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SetupInstallItem {
    pub command: String,
    pub user: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SetupStartupItem {
    pub command: StringOrVec,
    pub user: Option<String>,
    pub background: Option<bool>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SetupFileItem {
    pub path: String,
    pub content: String,
    pub mode: Option<String>,
    #[serde(rename = "onlyIfMissing")]
    pub only_if_missing: Option<bool>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Setup {
    pub install: Option<Vec<SetupInstallItem>>,
    pub startup: Option<Vec<SetupStartupItem>>,
    pub files: Option<Vec<SetupFileItem>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionsNetwork {
    pub allow: Option<Vec<String>>,
    pub deny: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Permissions {
    pub network: Option<PermissionsNetwork>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Environment {
    pub variables: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KitPort {
    pub container: u16,
    pub protocol: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentInstructions {
    pub filename: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApiKeyInject {
    pub domain: String,
    pub header: Option<String>,
    pub format: Option<String>,
    pub scheme: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialApiKey {
    pub name: String,
    #[serde(rename = "proxyManaged")]
    pub proxy_managed: Option<bool>,
    pub inject: Option<Vec<ApiKeyInject>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OAuthTokenEndpoint {
    pub host: String,
    pub path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OAuthSentinels {
    #[serde(rename = "accessToken")]
    pub access_token: Option<String>,
    #[serde(rename = "refreshToken")]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OAuthCredentialFile {
    pub path: String,
    pub structure: Option<serde_json::Value>,
    pub template: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialOAuth {
    #[serde(rename = "tokenEndpoint")]
    pub token_endpoint: Option<OAuthTokenEndpoint>,
    pub sentinels: Option<OAuthSentinels>,
    #[serde(rename = "credentialFile")]
    pub credential_file: Option<OAuthCredentialFile>,
    #[serde(rename = "resourceHosts")]
    pub resource_hosts: Option<Vec<String>>,
    #[serde(rename = "skipIfEnv")]
    pub skip_if_env: Option<bool>,
    #[serde(rename = "responseFields")]
    pub response_fields: Option<BTreeMap<String, String>>,
    pub passthrough: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CredentialItem {
    pub service: String,
    pub description: Option<String>,
    pub required: Option<bool>,
    pub provider: Option<String>,
    #[serde(rename = "apiKey")]
    pub api_key: Option<CredentialApiKey>,
    pub oauth: Option<CredentialOAuth>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeItem {
    pub path: String,
    #[serde(rename = "type")]
    pub volume_type: Option<String>,
    pub source: Option<String>,
    #[serde(rename = "readOnly")]
    pub read_only: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Security {
    pub privileged: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Requires {
    pub agent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SandboxCommand {
    List(Vec<String>),
    Structured {
        default: Option<Vec<String>>,
        interactive: Option<Vec<String>>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SandboxResources {
    pub cpu: Option<f64>,
    pub memory: Option<String>,
    pub gpu: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SandboxBlock {
    pub image: Option<String>,
    pub entrypoint: Option<Vec<String>>,
    pub command: Option<SandboxCommand>,
    pub resources: Option<SandboxResources>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KitMixinSpecV2 {
    #[serde(rename = "schemaVersion")]
    pub schema_version: SchemaVersion,
    pub name: String,
    pub version: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "sourceURL")]
    pub source_url: Option<String>,
    pub licenses: Option<Vec<String>>,
    pub locked: Option<Vec<String>>,
    pub security: Option<Security>,
    pub permissions: Option<Permissions>,
    pub ports: Option<Vec<KitPort>>,
    pub credentials: Option<Vec<CredentialItem>>,
    pub environment: Option<Environment>,
    pub setup: Option<Setup>,
    pub volumes: Option<Vec<VolumeItem>>,
    #[serde(rename = "agentInstructions")]
    pub agent_instructions: Option<AgentInstructions>,
    pub requires: Option<Requires>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct KitSandboxSpecV2 {
    #[serde(rename = "schemaVersion")]
    pub schema_version: SchemaVersion,
    pub name: String,
    pub version: Option<String>,
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "sourceURL")]
    pub source_url: Option<String>,
    pub licenses: Option<Vec<String>>,
    pub locked: Option<Vec<String>>,
    pub security: Option<Security>,
    pub permissions: Option<Permissions>,
    pub ports: Option<Vec<KitPort>>,
    pub credentials: Option<Vec<CredentialItem>>,
    pub environment: Option<Environment>,
    pub setup: Option<Setup>,
    pub volumes: Option<Vec<VolumeItem>>,
    #[serde(rename = "agentInstructions")]
    pub agent_instructions: Option<AgentInstructions>,
    pub sandbox: SandboxBlock,
    pub extends: Option<String>,
    pub mixins: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind")]
pub enum KitSpecV2 {
    #[serde(rename = "mixin")]
    Mixin(KitMixinSpecV2),
    #[serde(rename = "sandbox")]
    Sandbox(KitSandboxSpecV2),
}

impl KitSpecV2 {
    pub fn name(&self) -> &str {
        match self {
            Self::Mixin(m) => &m.name,
            Self::Sandbox(s) => &s.name,
        }
    }

    pub fn schema_version(&self) -> &SchemaVersion {
        match self {
            Self::Mixin(m) => &m.schema_version,
            Self::Sandbox(s) => &s.schema_version,
        }
    }

    pub fn environment(&self) -> Option<&Environment> {
        match self {
            Self::Mixin(m) => m.environment.as_ref(),
            Self::Sandbox(s) => s.environment.as_ref(),
        }
    }

    pub fn locked(&self) -> Option<&[String]> {
        match self {
            Self::Mixin(m) => m.locked.as_deref(),
            Self::Sandbox(s) => s.locked.as_deref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct KitSpecV1 {
    #[serde(alias = "schema_version", rename = "schemaVersion")]
    pub schema_version: SchemaVersion,
    pub kind: Option<String>,
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub network: Option<serde_yaml::Value>,
    pub environment: Option<BTreeMap<String, String>>,
    pub commands: Option<serde_yaml::Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum KitSpec {
    V2(Box<KitSpecV2>),
    V1(Box<KitSpecV1>),
}

impl KitSpec {
    pub fn name(&self) -> &str {
        match self {
            Self::V2(v2) => v2.name(),
            Self::V1(v1) => &v1.name,
        }
    }

    pub fn schema_version(&self) -> &SchemaVersion {
        match self {
            Self::V2(v2) => v2.schema_version(),
            Self::V1(v1) => &v1.schema_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SbxEnvPort {
    pub sandbox: u16,
    pub host: Option<u16>,
    pub protocol: Option<String>,
    #[serde(rename = "hostIP")]
    pub host_ip: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SbxEnvWorkspaceObject {
    pub path: String,
    pub clone: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum SbxEnvWorkspace {
    Object(SbxEnvWorkspaceObject),
    Path(String),
}

impl SbxEnvWorkspace {
    pub fn is_clone(&self) -> bool {
        match self {
            Self::Object(obj) => obj.clone,
            Self::Path(_) => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SbxEnvV1 {
    #[serde(alias = "schema_version", rename = "schemaVersion")]
    pub schema_version: Option<SchemaVersion>,
    pub name: Option<String>,
    pub agent: String,
    pub workspace: SbxEnvWorkspace,
    pub kits: Option<Vec<String>>,
    pub kit: Option<StringOrVec>,
    pub ports: Option<Vec<SbxEnvPort>>,
    pub env: Option<BTreeMap<String, String>>,
    pub setup_commands: Option<Vec<String>>,
}

impl SbxEnvV1 {
    pub fn has_kit(&self, kit_path: &str) -> bool {
        if let Some(kits) = &self.kits {
            if kits.iter().any(|k| k == kit_path) {
                return true;
            }
        }
        if let Some(kit) = &self.kit {
            if kit.contains_item(kit_path) {
                return true;
            }
        }
        false
    }
}

pub fn check_kit_spec(file: &Path) -> Result<KitSpec> {
    if !file.exists() {
        bail!("Missing expected kit spec file: {}", file.display());
    }

    let raw = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read file {}", file.display()))?;

    let spec: KitSpec = serde_yaml::from_str(&raw)
        .with_context(|| format!("Validation failed for {}", file.display()))?;

    match &spec {
        KitSpec::V2(v2)
            if matches!(v2.schema_version(), SchemaVersion::String(version) if version == "2")
                && matches!(**v2, KitSpecV2::Mixin(_)) => {}
        KitSpec::V2(v2) if !matches!(v2.schema_version(), SchemaVersion::String(version) if version == "2") =>
        {
            bail!("File {} must have schemaVersion: \"2\"", file.display());
        }
        KitSpec::V2(_) => bail!("File {} must have kind: mixin", file.display()),
        KitSpec::V1(_) => bail!("File {} must have schemaVersion: \"2\"", file.display()),
    }

    if !is_valid_kit_name(spec.name()) {
        bail!(
            "Invalid kit name '{}' in {}: must match ^[a-z0-9]([a-z0-9-]{{0,62}}[a-z0-9])?$",
            spec.name(),
            file.display()
        );
    }

    if let KitSpec::V2(v2) = &spec {
        if let Some(env) = v2.environment() {
            if let Some(vars) = &env.variables {
                for key in vars.keys() {
                    if !is_valid_env_var_name(key) {
                        bail!(
                            "Invalid environment variable name '{}' in {}",
                            key,
                            file.display()
                        );
                    }
                }
            }
        }
        if let Some(locked) = v2.locked() {
            for path in locked {
                if !is_valid_dotted_path(path) {
                    bail!(
                        "Invalid dotted path '{}' in locked list of {}",
                        path,
                        file.display()
                    );
                }
            }
        }
    }

    println!(
        "✓ {} is valid ({}, schemaVersion: {})",
        file.display(),
        spec.name(),
        spec.schema_version().as_str()
    );

    Ok(spec)
}

pub fn check_sbx_env(file: &Path, required_ports: &[u16]) -> Result<SbxEnvV1> {
    if !file.exists() {
        bail!("Missing expected environment file: {}", file.display());
    }

    let raw = std::fs::read_to_string(file)
        .with_context(|| format!("Failed to read file {}", file.display()))?;

    let parsed_value: serde_yaml::Value = serde_yaml::from_str(&raw)
        .with_context(|| format!("Failed to parse YAML in {}", file.display()))?;

    let mapping = parsed_value
        .as_mapping()
        .ok_or_else(|| anyhow::anyhow!("File {} does not contain a YAML object", file.display()))?;

    for &forbidden in FORBIDDEN_KEYS {
        let key = serde_yaml::Value::String(forbidden.to_string());
        if mapping.contains_key(&key) {
            bail!(
                "File {} contains forbidden tracked property: {}",
                file.display(),
                forbidden
            );
        }
    }

    let env: SbxEnvV1 = serde_yaml::from_value(parsed_value)
        .with_context(|| format!("Validation failed for {}", file.display()))?;

    if !matches!(&env.schema_version, Some(SchemaVersion::String(version)) if version == "1") {
        bail!("File {} must have schemaVersion: \"1\"", file.display());
    }

    if !matches!(&env.workspace, SbxEnvWorkspace::Object(workspace) if workspace.path == "..") {
        bail!("File {} must have workspace.path: ..", file.display());
    }

    if !env.workspace.is_clone() {
        bail!("File {} must have workspace.clone: true", file.display());
    }

    if !env.has_kit("./kit") {
        bail!("File {} must include \"./kit\" in kits", file.display());
    }

    let declared_ports: std::collections::BTreeSet<u16> = env
        .ports
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|p| p.sandbox)
        .collect();

    for &required_port in required_ports {
        if !declared_ports.contains(&required_port) {
            bail!(
                "File {} must forward required sandbox port {}",
                file.display(),
                required_port
            );
        }
    }

    println!(
        "✓ {} is valid ({}, agent: {})",
        file.display(),
        env.name.as_deref().unwrap_or("<unnamed>"),
        env.agent
    );

    Ok(env)
}

pub fn check_kit_with_sbx_if_available(kit_dir: &Path) -> Result<()> {
    let sbx_version = Command::new("sbx").arg("version").output();

    if let Ok(output) = sbx_version {
        if !output.status.success() {
            bail!("Host 'sbx version' failed");
        }
        let version_output = String::from_utf8(output.stdout)
            .context("Host 'sbx version' returned non-UTF-8 output")?;
        check_sbx_version(&version_output)?;

        println!("Running host 'sbx kit validate {}'...", kit_dir.display());
        let status = Command::new("sbx")
            .arg("kit")
            .arg("validate")
            .arg(kit_dir)
            .status()
            .with_context(|| format!("Failed to run 'sbx kit validate {}'", kit_dir.display()))?;

        if !status.success() {
            bail!("Host 'sbx kit validate' failed on {}", kit_dir.display());
        }
    }

    Ok(())
}

fn check_sbx_version(output: &str) -> Result<()> {
    let version = output
        .split_whitespace()
        .find_map(|word| {
            let candidate = word.trim_start_matches('v');
            let mut parts = candidate.split('.');
            let major = parts.next()?.parse::<u64>().ok()?;
            let minor = parts.next()?.parse::<u64>().ok()?;
            let patch = parts
                .next()?
                .split(|character: char| !character.is_ascii_digit())
                .next()?
                .parse::<u64>()
                .ok()?;
            Some((major, minor, patch))
        })
        .ok_or_else(|| anyhow::anyhow!("Could not parse version from 'sbx version': {output:?}"))?;

    if version < (0, 39, 0) {
        bail!(
            "Host kit validation requires sbx 0.39.0 or newer; found {}.{}.{}",
            version.0,
            version.1,
            version.2
        );
    }

    Ok(())
}

fn shell_command_segments(command: &str) -> Vec<Vec<String>> {
    let mut segments = Vec::new();
    let mut segment = Vec::new();
    let mut token = String::new();
    let mut quote = None;
    let mut characters = command.chars().peekable();

    while let Some(character) = characters.next() {
        if let Some(delimiter) = quote {
            if character == delimiter {
                quote = None;
            } else if character == '\\' && delimiter == '"' {
                if let Some(escaped) = characters.next() {
                    token.push(escaped);
                }
            } else {
                token.push(character);
            }
            continue;
        }

        match character {
            '\'' | '"' => quote = Some(character),
            '\\' => {
                if let Some(escaped) = characters.next() {
                    token.push(escaped);
                }
            }
            '#' => {
                for rest in characters.by_ref() {
                    if rest == '\n' {
                        break;
                    }
                }
                if !token.is_empty() {
                    segment.push(std::mem::take(&mut token));
                }
                if !segment.is_empty() {
                    segments.push(std::mem::take(&mut segment));
                }
            }
            '\n' | ';' | '&' | '|' => {
                if !token.is_empty() {
                    segment.push(std::mem::take(&mut token));
                }
                if !segment.is_empty() {
                    segments.push(std::mem::take(&mut segment));
                }
            }
            whitespace if whitespace.is_whitespace() => {
                if !token.is_empty() {
                    segment.push(std::mem::take(&mut token));
                }
            }
            _ => token.push(character),
        }
    }

    if !token.is_empty() {
        segment.push(token);
    }
    if !segment.is_empty() {
        segments.push(segment);
    }

    segments
}

fn has_mise_use_argument(segments: &[Vec<String>], expected: &str) -> bool {
    segments.iter().any(|segment| {
        segment
            .windows(2)
            .position(|pair| pair == ["mise", "use"])
            .is_some_and(|use_index| segment[use_index + 2..].iter().any(|arg| arg == expected))
    })
}

pub fn check_toolchain_parity(repo_root: &Path) -> Result<()> {
    // 1. Read mise.toml
    let mise_path = repo_root.join("mise.toml");
    let mise_raw = std::fs::read_to_string(&mise_path)
        .with_context(|| format!("Failed to read {}", mise_path.display()))?;

    let mise_doc: toml::Table = toml::from_str(&mise_raw)
        .with_context(|| format!("Failed to parse {}", mise_path.display()))?;
    let tools = mise_doc
        .get("tools")
        .and_then(|v| v.as_table())
        .ok_or_else(|| anyhow::anyhow!("Missing [tools] in {}", mise_path.display()))?;

    let bun_version = tools.get("bun").and_then(|v| v.as_str()).ok_or_else(|| {
        anyhow::anyhow!("Missing bun version in [tools] in {}", mise_path.display())
    })?;

    let beads_version = tools
        .get("github:gastownhall/beads")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Missing github:gastownhall/beads in [tools] in {}",
                mise_path.display()
            )
        })?;

    let rust_version = match tools.get("rust") {
        Some(toml::Value::String(s)) => s.as_str(),
        Some(toml::Value::Table(t)) => t
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing rust.version in {}", mise_path.display()))?,
        _ => bail!("Missing rust in [tools] in {}", mise_path.display()),
    };

    // 2. Read rust-toolchain.toml
    let rust_toolchain_path = repo_root.join("rust-toolchain.toml");
    let rust_toolchain_raw = std::fs::read_to_string(&rust_toolchain_path)
        .with_context(|| format!("Failed to read {}", rust_toolchain_path.display()))?;
    let rust_toolchain_doc: toml::Table = toml::from_str(&rust_toolchain_raw)
        .with_context(|| format!("Failed to parse {}", rust_toolchain_path.display()))?;
    let rust_toolchain_channel = rust_toolchain_doc
        .get("toolchain")
        .and_then(|v| v.as_table())
        .and_then(|t| t.get("channel"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Missing toolchain.channel in {}",
                rust_toolchain_path.display()
            )
        })?;

    if rust_toolchain_channel != rust_version {
        bail!(
            "Rust toolchain mismatch: mise.toml specifies '{}' but rust-toolchain.toml specifies '{}'",
            rust_version,
            rust_toolchain_channel
        );
    }

    let cargo_path = repo_root.join("Cargo.toml");
    let cargo_raw = std::fs::read_to_string(&cargo_path)
        .with_context(|| format!("Failed to read {}", cargo_path.display()))?;
    let cargo_doc: toml::Table = toml::from_str(&cargo_raw)
        .with_context(|| format!("Failed to parse {}", cargo_path.display()))?;
    let cargo_rust_version = cargo_doc
        .get("package")
        .and_then(|value| value.as_table())
        .and_then(|package| package.get("rust-version"))
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!("Missing package.rust-version in {}", cargo_path.display())
        })?;

    if cargo_rust_version != rust_version {
        bail!(
            "Rust toolchain mismatch: mise.toml specifies '{}' but Cargo.toml specifies '{}'",
            rust_version,
            cargo_rust_version
        );
    }

    // 3. Read GitHub Actions workflows and extract MISE_VERSION
    let workflows_dir = repo_root.join(".github").join("workflows");
    let mut workflow_mise_versions: BTreeMap<String, String> = BTreeMap::new();
    if workflows_dir.exists() {
        for entry in std::fs::read_dir(&workflows_dir)
            .with_context(|| format!("Failed to read {}", workflows_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yml")
                || path.extension().and_then(|s| s.to_str()) == Some("yaml")
            {
                let content = std::fs::read_to_string(&path)?;
                for line in content.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("MISE_VERSION:") {
                        let val = trimmed
                            .trim_start_matches("MISE_VERSION:")
                            .trim()
                            .trim_matches('"')
                            .trim_matches('\'');
                        workflow_mise_versions.insert(
                            path.file_name().unwrap().to_string_lossy().to_string(),
                            val.to_string(),
                        );
                        break;
                    }
                }
            }
        }
    }

    let expected_mise_version =
        if let Some((first_file, first_ver)) = workflow_mise_versions.iter().next() {
            for (file, ver) in &workflow_mise_versions {
                if ver != first_ver {
                    bail!(
                        "Workflow MISE_VERSION mismatch: {} has '{}' but {} has '{}'",
                        first_file,
                        first_ver,
                        file,
                        ver
                    );
                }
            }
            first_ver.clone()
        } else {
            bail!(
                "No workflow files found with MISE_VERSION in {}",
                workflows_dir.display()
            );
        };

    // 4. Verify .sbx/kit/spec.yaml
    let kit_spec_path = repo_root.join(".sbx").join("kit").join("spec.yaml");
    let kit_spec = check_kit_spec(&kit_spec_path)?;
    let KitSpec::V2(v2) = kit_spec else {
        unreachable!("check_kit_spec only accepts schema version 2 mixins");
    };
    let KitSpecV2::Mixin(mixin) = *v2 else {
        unreachable!("check_kit_spec only accepts mixins");
    };
    let install_commands = mixin
        .setup
        .and_then(|setup| setup.install)
        .unwrap_or_default()
        .into_iter()
        .map(|item| item.command)
        .collect::<Vec<_>>()
        .join("\n");

    let command_segments = shell_command_segments(&install_commands);
    let executable_tokens = command_segments.iter().flatten().collect::<Vec<_>>();
    let expected_mise = format!("MISE_VERSION={expected_mise_version}");
    if !executable_tokens.contains(&&expected_mise) {
        bail!(
            ".sbx/kit/spec.yaml setup.install commands do not contain expected '{}'",
            expected_mise
        );
    }

    for expected in [
        format!("bun@{bun_version}"),
        format!("github:gastownhall/beads@{beads_version}"),
        format!("rust@{rust_version}"),
    ] {
        if !has_mise_use_argument(&command_segments, &expected) {
            bail!(
                ".sbx/kit/spec.yaml setup.install commands do not contain expected '{}'",
                expected
            );
        }
    }

    println!(
        "✓ Toolchain versions aligned (rust: {}, bun: {}, beads: {}, mise: {})",
        rust_version, bun_version, beads_version, expected_mise_version
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_toolchain_fixture(root: &Path, cargo_rust: &str, kit_yaml: &str) {
        std::fs::create_dir_all(root.join(".github/workflows")).unwrap();
        std::fs::create_dir_all(root.join(".sbx/kit")).unwrap();
        std::fs::write(
            root.join("mise.toml"),
            "[tools]\nbun = \"1.4.2\"\n\"github:gastownhall/beads\" = \"1.2.2\"\nrust = \"1.98.1\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("rust-toolchain.toml"),
            "[toolchain]\nchannel = \"1.98.1\"\n",
        )
        .unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            format!("[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nrust-version = \"{cargo_rust}\"\n"),
        )
        .unwrap();
        std::fs::write(
            root.join(".github/workflows/ci.yml"),
            "env:\n  MISE_VERSION: v2026.9.1\n",
        )
        .unwrap();
        std::fs::write(root.join(".sbx/kit/spec.yaml"), kit_yaml).unwrap();
    }

    #[test]
    fn test_toolchain_parity() {
        let repo_root = Path::new(".");
        check_toolchain_parity(repo_root)
            .expect("toolchain versions should be aligned across repo files");
    }

    #[test]
    fn test_cargo_rust_version_mismatch_rejected() {
        let dir = tempdir().unwrap();
        write_toolchain_fixture(
            dir.path(),
            "1.97.0",
            "schemaVersion: \"2\"\nkind: mixin\nname: fixture\nsetup:\n  install:\n    - command: mise use -g bun@1.4.2 github:gastownhall/beads@1.2.2 rust@1.98.1\n    - command: curl https://mise.run | MISE_VERSION=v2026.9.1 sh\n",
        );

        let err = check_toolchain_parity(dir.path()).unwrap_err();
        assert!(err.to_string().contains("Cargo.toml specifies '1.97.0'"));
    }

    #[test]
    fn test_tool_versions_outside_setup_commands_do_not_satisfy_parity() {
        let dir = tempdir().unwrap();
        write_toolchain_fixture(
            dir.path(),
            "1.98.1",
            "schemaVersion: \"2\"\nkind: mixin\nname: fixture\ndescription: MISE_VERSION=v2026.9.1 bun@1.4.2 github:gastownhall/beads@1.2.2 rust@1.98.1\nsetup:\n  install:\n    - command: mise use -g bun@0.1.0\n",
        );

        let err = check_toolchain_parity(dir.path()).unwrap_err();
        assert!(err
            .to_string()
            .contains("setup.install commands do not contain expected"));
    }

    #[test]
    fn test_valid_kit_name() {
        assert!(is_valid_kit_name("rulette-toolchain"));
        assert!(is_valid_kit_name("a"));
        assert!(is_valid_kit_name("a-b-c-1"));
        assert!(!is_valid_kit_name(""));
        assert!(!is_valid_kit_name("-toolchain"));
        assert!(!is_valid_kit_name("toolchain-"));
        assert!(!is_valid_kit_name("Rulette"));
        assert!(!is_valid_kit_name("rulette_toolchain"));
    }

    #[test]
    fn test_valid_env_var_name() {
        assert!(is_valid_env_var_name("PATH"));
        assert!(is_valid_env_var_name("MISE_YES"));
        assert!(is_valid_env_var_name("_FOO_123"));
        assert!(!is_valid_env_var_name("123FOO"));
        assert!(!is_valid_env_var_name("FOO-BAR"));
    }

    #[test]
    fn test_valid_dotted_path() {
        assert!(is_valid_dotted_path("sandbox.image"));
        assert!(is_valid_dotted_path("foo.bar.baz"));
        assert!(!is_valid_dotted_path(""));
        assert!(!is_valid_dotted_path(".foo"));
        assert!(!is_valid_dotted_path("Foo.bar"));
        assert!(!is_valid_dotted_path("foo..bar"));
    }

    #[test]
    fn test_repo_kit_spec_and_env_files() {
        let kit_spec = Path::new(".sbx/kit/spec.yaml");
        let env_file = Path::new(".sbx/sbxenv.yaml");
        let agy_file = Path::new(".sbx/sbxenv.agy.yaml");

        let spec = check_kit_spec(kit_spec).expect(".sbx/kit/spec.yaml should be valid");
        assert_eq!(spec.name(), "rulette-toolchain");

        let env =
            check_sbx_env(env_file, REQUIRED_PORTS).expect(".sbx/sbxenv.yaml should be valid");
        assert_eq!(env.agent, "codex");

        let agy =
            check_sbx_env(agy_file, REQUIRED_PORTS).expect(".sbx/sbxenv.agy.yaml should be valid");
        assert_eq!(agy.agent, "agy");
    }

    #[test]
    fn test_forbidden_keys_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("sbxenv.yaml");
        for forbidden in FORBIDDEN_KEYS {
            let yaml = format!(
                "schemaVersion: \"1\"\nagent: codex\nworkspace:\n  path: ..\n  clone: true\nkits:\n  - ./kit\nports:\n  - sandbox: 5173\n{}: {{}}\n",
                forbidden
            );
            std::fs::write(&file, yaml).unwrap();
            let err = check_sbx_env(&file, REQUIRED_PORTS).unwrap_err();
            assert!(err.to_string().contains("forbidden tracked property"));
        }
    }

    #[test]
    fn test_missing_required_port_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("sbxenv.yaml");
        let yaml = "schemaVersion: \"1\"\nagent: codex\nworkspace:\n  path: ..\n  clone: true\nkits:\n  - ./kit\nports:\n  - sandbox: 8080\n";
        std::fs::write(&file, yaml).unwrap();
        let err = check_sbx_env(&file, &[5173]).unwrap_err();
        assert!(err
            .to_string()
            .contains("must forward required sandbox port 5173"));
    }

    #[test]
    fn test_missing_clone_mode_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("sbxenv.yaml");
        let yaml = "schemaVersion: \"1\"\nagent: codex\nworkspace:\n  path: ..\n  clone: false\nkits:\n  - ./kit\nports:\n  - sandbox: 5173\n";
        std::fs::write(&file, yaml).unwrap();
        let err = check_sbx_env(&file, &[5173]).unwrap_err();
        assert!(err.to_string().contains("must have workspace.clone: true"));
    }

    #[test]
    fn test_missing_local_kit_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("sbxenv.yaml");
        let yaml = "schemaVersion: \"1\"\nagent: codex\nworkspace:\n  path: ..\n  clone: true\nkits:\n  - some-other-kit\nports:\n  - sandbox: 5173\n";
        std::fs::write(&file, yaml).unwrap();
        let err = check_sbx_env(&file, &[5173]).unwrap_err();
        assert!(err.to_string().contains("must include \"./kit\" in kits"));
    }

    #[test]
    fn test_legacy_v1_kit_spec_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("spec.yaml");
        let yaml = "schemaVersion: \"1\"\nname: legacy-kit\nversion: \"0.1.0\"\n";
        std::fs::write(&file, yaml).unwrap();
        let err = check_kit_spec(&file).unwrap_err();
        assert!(err.to_string().contains("schemaVersion: \"2\""));
    }

    #[test]
    fn test_sandbox_kit_spec_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("spec.yaml");
        let yaml = "schemaVersion: \"2\"\nkind: sandbox\nname: sandbox-kit\nsandbox: {}\n";
        std::fs::write(&file, yaml).unwrap();
        let err = check_kit_spec(&file).unwrap_err();
        assert!(err.to_string().contains("kind: mixin"));
    }

    #[test]
    fn test_mixin_kit_with_wrong_schema_version_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("spec.yaml");
        let yaml = "schemaVersion: \"3\"\nkind: mixin\nname: future-kit\n";
        std::fs::write(&file, yaml).unwrap();
        let err = check_kit_spec(&file).unwrap_err();
        assert!(err.to_string().contains("schemaVersion: \"2\""));
    }

    #[test]
    fn test_mixin_kit_with_numeric_schema_version_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("spec.yaml");
        let yaml = "schemaVersion: 2\nkind: mixin\nname: numeric-schema-kit\n";
        std::fs::write(&file, yaml).unwrap();
        let err = check_kit_spec(&file).unwrap_err();
        assert!(err.to_string().contains("schemaVersion: \"2\""));
    }

    #[test]
    fn test_environment_without_schema_version_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("sbxenv.yaml");
        let yaml = "agent: codex\nworkspace:\n  path: ..\n  clone: true\nkits:\n  - ./kit\n";
        std::fs::write(&file, yaml).unwrap();
        let err = check_sbx_env(&file, &[]).unwrap_err();
        assert!(err.to_string().contains("schemaVersion: \"1\""));
    }

    #[test]
    fn test_environment_with_wrong_schema_version_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("sbxenv.yaml");
        let yaml = "schemaVersion: \"2\"\nagent: codex\nworkspace:\n  path: ..\n  clone: true\nkits:\n  - ./kit\n";
        std::fs::write(&file, yaml).unwrap();
        let err = check_sbx_env(&file, &[]).unwrap_err();
        assert!(err.to_string().contains("schemaVersion: \"1\""));
    }

    #[test]
    fn test_environment_with_numeric_schema_version_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("sbxenv.yaml");
        let yaml = "schemaVersion: 1\nagent: codex\nworkspace:\n  path: ..\n  clone: true\nkits:\n  - ./kit\n";
        std::fs::write(&file, yaml).unwrap();
        let err = check_sbx_env(&file, &[]).unwrap_err();
        assert!(err.to_string().contains("schemaVersion: \"1\""));
    }

    #[test]
    fn test_tool_version_prefix_collisions_rejected() {
        let dir = tempdir().unwrap();
        write_toolchain_fixture(
            dir.path(),
            "1.98.1",
            "schemaVersion: \"2\"\nkind: mixin\nname: fixture\nsetup:\n  install:\n    - command: curl https://mise.run | MISE_VERSION=v2026.9.1 sh\n    - command: mise use -g bun@1.4.20 github:gastownhall/beads@1.2.2 rust@1.98.10\n",
        );

        let err = check_toolchain_parity(dir.path()).unwrap_err();
        assert!(err
            .to_string()
            .contains("setup.install commands do not contain expected 'bun@1.4.2'"));
    }

    #[test]
    fn test_tool_versions_in_shell_comments_do_not_satisfy_parity() {
        let dir = tempdir().unwrap();
        write_toolchain_fixture(
            dir.path(),
            "1.98.1",
            "schemaVersion: \"2\"\nkind: mixin\nname: fixture\nsetup:\n  install:\n    - command: |\n        curl https://mise.run | sh # MISE_VERSION=v2026.9.1\n        mise use -g bun@0.1.0 # bun@1.4.2 github:gastownhall/beads@1.2.2 rust@1.98.1\n",
        );

        let err = check_toolchain_parity(dir.path()).unwrap_err();
        assert!(err
            .to_string()
            .contains("setup.install commands do not contain expected 'MISE_VERSION=v2026.9.1'"));
    }

    #[test]
    fn test_environment_with_wrong_workspace_path_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("sbxenv.yaml");
        let yaml = "schemaVersion: \"1\"\nagent: codex\nworkspace:\n  path: .\n  clone: true\nkits:\n  - ./kit\n";
        std::fs::write(&file, yaml).unwrap();
        let err = check_sbx_env(&file, &[]).unwrap_err();
        assert!(err.to_string().contains("workspace.path: .."));
    }

    #[test]
    fn test_sbx_version_below_minimum_rejected() {
        let err = check_sbx_version("sbx 0.38.9").unwrap_err();
        assert!(err.to_string().contains("requires sbx 0.39.0 or newer"));
        check_sbx_version("sbx 0.39.0").unwrap();
        check_sbx_version("sbx 1.0.0").unwrap();
    }
}
