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

pub const REQUIRED_PORTS: &[u16] = &[5173];

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
    let sbx_found = Command::new("sbx")
        .arg("--help")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);

    if sbx_found {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
        let env_file = Path::new(".sbx/.sbxenv.yaml");
        let agy_file = Path::new(".sbx/.sbxenv.agy.yaml");

        let spec = check_kit_spec(kit_spec).expect(".sbx/kit/spec.yaml should be valid");
        assert_eq!(spec.name(), "rulette-toolchain");

        let env =
            check_sbx_env(env_file, REQUIRED_PORTS).expect(".sbx/.sbxenv.yaml should be valid");
        assert_eq!(env.agent, "codex");

        let agy =
            check_sbx_env(agy_file, REQUIRED_PORTS).expect(".sbx/.sbxenv.agy.yaml should be valid");
        assert_eq!(agy.agent, "agy");
    }

    #[test]
    fn test_forbidden_keys_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join(".sbxenv.yaml");
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
        let file = dir.path().join(".sbxenv.yaml");
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
        let file = dir.path().join(".sbxenv.yaml");
        let yaml = "schemaVersion: \"1\"\nagent: codex\nworkspace:\n  path: ..\n  clone: false\nkits:\n  - ./kit\nports:\n  - sandbox: 5173\n";
        std::fs::write(&file, yaml).unwrap();
        let err = check_sbx_env(&file, &[5173]).unwrap_err();
        assert!(err.to_string().contains("must have workspace.clone: true"));
    }

    #[test]
    fn test_missing_local_kit_rejected() {
        let dir = tempdir().unwrap();
        let file = dir.path().join(".sbxenv.yaml");
        let yaml = "schemaVersion: \"1\"\nagent: codex\nworkspace:\n  path: ..\n  clone: true\nkits:\n  - some-other-kit\nports:\n  - sandbox: 5173\n";
        std::fs::write(&file, yaml).unwrap();
        let err = check_sbx_env(&file, &[5173]).unwrap_err();
        assert!(err.to_string().contains("must include \"./kit\" in kits"));
    }

    #[test]
    fn test_legacy_v1_kit_spec_compatible() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("spec.yaml");
        let yaml = "schemaVersion: \"1\"\nname: legacy-kit\nversion: \"0.1.0\"\n";
        std::fs::write(&file, yaml).unwrap();
        let spec = check_kit_spec(&file).expect("legacy v1 spec should validate");
        assert_eq!(spec.name(), "legacy-kit");
    }
}
