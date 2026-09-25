use crate::emitters::lowering::{NativeArtifactClass, NativeTarget};
use crate::ir::graph::ResourcePath;
use crate::publication::model::{ArtifactDescriptor, MappingVersion, PublicationScope};
use anyhow::{bail, Result};

/// A compiled-in, versioned mapping from one target/scope pair to allowed
/// target-relative artifacts.
#[derive(Debug)]
pub struct TargetMapping {
    target: NativeTarget,
    scope: PublicationScope,
    version: MappingVersion,
    rules: &'static [ArtifactPathRule],
}

#[derive(Clone, Copy, Debug)]
enum ArtifactPathRule {
    Exact {
        class: NativeArtifactClass,
        native: &'static str,
        mapped: &'static str,
    },
    Prefix {
        class: NativeArtifactClass,
        native_prefix: &'static str,
        mapped_prefix: &'static str,
        required_suffix: Option<&'static str>,
    },
    ReverseDomainExtension {
        class: NativeArtifactClass,
    },
}

impl TargetMapping {
    pub const fn target(&self) -> NativeTarget {
        self.target
    }

    pub const fn scope(&self) -> PublicationScope {
        self.scope
    }

    pub const fn version(&self) -> MappingVersion {
        self.version
    }

    /// Validates one backend artifact against this mapping and returns only a
    /// normalized path relative to the caller-authorized root.
    ///
    /// No filesystem path, root identity, or authority is accepted here.
    pub fn map_artifact(&self, artifact: &ArtifactDescriptor) -> Result<ResourcePath> {
        for rule in self.rules {
            match *rule {
                ArtifactPathRule::Exact {
                    class,
                    native,
                    mapped,
                } if artifact.class == class && artifact.native_path.as_str() == native => {
                    return ResourcePath::parse(mapped);
                }
                ArtifactPathRule::Prefix {
                    class,
                    native_prefix,
                    mapped_prefix,
                    required_suffix,
                } if artifact.class == class => {
                    if let Some(tail) = artifact.native_path.as_str().strip_prefix(native_prefix) {
                        if !tail.is_empty()
                            && required_suffix.is_none_or(|suffix| tail.ends_with(suffix))
                        {
                            return ResourcePath::parse(format!("{mapped_prefix}{tail}"));
                        }
                    }
                }
                ArtifactPathRule::ReverseDomainExtension { class } if artifact.class == class => {
                    let path = artifact.native_path.as_str();
                    if let Some((dir, tail)) = path.split_once('/') {
                        if !dir.is_empty()
                            && !tail.is_empty()
                            && crate::parsers::agent_plugin::is_reverse_domain(dir)
                            && !path.split('/').any(|comp| comp.starts_with('.'))
                        {
                            return ResourcePath::parse(path);
                        }
                    }
                }
                _ => {}
            }
        }

        bail!(
            "artifact class/path `{}` at `{}` is not permitted by {}@{} mapping version {}",
            artifact_class_name(artifact.class),
            artifact.native_path.as_str(),
            self.target.as_str(),
            scope_name(self.scope),
            self.version.as_str(),
        );
    }
}

/// Looks up a mapping from the fixed v0.1 allow-list.
///
/// The registry purposefully has no representation for local, enterprise,
/// managed, system, or caller-provided arbitrary destinations.
pub fn mapping_for(
    target: NativeTarget,
    scope: PublicationScope,
) -> Result<&'static TargetMapping> {
    match (target, scope) {
        (NativeTarget::Codex, PublicationScope::Project) => Ok(&CODEX_PROJECT),
        (NativeTarget::OpenCode, PublicationScope::Project) => Ok(&OPENCODE_PROJECT),
        (NativeTarget::Claude, PublicationScope::Project) => Ok(&CLAUDE_PROJECT),
        (NativeTarget::Cursor, PublicationScope::Project) => Ok(&CURSOR_PROJECT),
        (NativeTarget::Antigravity, PublicationScope::Project) => Ok(&ANTIGRAVITY_PROJECT),
        (NativeTarget::AgentPlugin, PublicationScope::Project) => Ok(&AGENT_PLUGIN_PROJECT),
        (NativeTarget::Codex, PublicationScope::User) => Ok(&CODEX_USER),
        (NativeTarget::OpenCode, PublicationScope::User) => Ok(&OPENCODE_USER),
        (NativeTarget::Claude, PublicationScope::User) => Ok(&CLAUDE_USER),
        (NativeTarget::Antigravity, PublicationScope::User) => Ok(&ANTIGRAVITY_USER),
        (NativeTarget::Cursor, PublicationScope::User) => {
            bail!("user mapping is unavailable for target `cursor`")
        }
        (NativeTarget::AgentPlugin, PublicationScope::User) => {
            bail!("user mapping is unavailable for target `agent-plugin`")
        }
    }
}

const CODEX_PROJECT_RULES: &[ArtifactPathRule] = &[
    ArtifactPathRule::Exact {
        class: NativeArtifactClass::Instruction,
        native: "AGENTS.md",
        mapped: "AGENTS.md",
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillInstruction,
        native_prefix: "skills/",
        mapped_prefix: ".codex/skills/",
        required_suffix: Some("SKILL.md"),
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillResource,
        native_prefix: "skills/",
        mapped_prefix: ".codex/skills/",
        required_suffix: None,
    },
];

const OPENCODE_PROJECT_RULES: &[ArtifactPathRule] = &[
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::Rule,
        native_prefix: "rules/",
        mapped_prefix: ".opencode/rules/",
        required_suffix: Some(".md"),
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillInstruction,
        native_prefix: "skills/",
        mapped_prefix: ".opencode/skills/",
        required_suffix: Some("SKILL.md"),
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillResource,
        native_prefix: "skills/",
        mapped_prefix: ".opencode/skills/",
        required_suffix: None,
    },
];

const CLAUDE_PROJECT_RULES: &[ArtifactPathRule] = &[
    ArtifactPathRule::Exact {
        class: NativeArtifactClass::Instruction,
        native: "CLAUDE.md",
        mapped: "CLAUDE.md",
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillInstruction,
        native_prefix: "skills/",
        mapped_prefix: ".claude/skills/",
        required_suffix: Some("SKILL.md"),
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillResource,
        native_prefix: "skills/",
        mapped_prefix: ".claude/skills/",
        required_suffix: None,
    },
];

const CURSOR_PROJECT_RULES: &[ArtifactPathRule] = &[ArtifactPathRule::Prefix {
    class: NativeArtifactClass::Rule,
    native_prefix: "rules/",
    mapped_prefix: ".cursor/rules/",
    required_suffix: Some(".mdc"),
}];

const ANTIGRAVITY_PROJECT_RULES: &[ArtifactPathRule] = &[
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::Rule,
        native_prefix: "rules/",
        mapped_prefix: ".agents/rules/",
        required_suffix: Some(".md"),
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillInstruction,
        native_prefix: "skills/",
        mapped_prefix: ".agents/skills/",
        required_suffix: Some("SKILL.md"),
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillResource,
        native_prefix: "skills/",
        mapped_prefix: ".agents/skills/",
        required_suffix: None,
    },
];

const AGENT_PLUGIN_PROJECT_RULES: &[ArtifactPathRule] = &[
    ArtifactPathRule::Exact {
        class: NativeArtifactClass::PluginManifest,
        native: "plugin.json",
        mapped: "plugin.json",
    },
    ArtifactPathRule::Exact {
        class: NativeArtifactClass::McpConfig,
        native: "mcp.json",
        mapped: "mcp.json",
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillInstruction,
        native_prefix: "skills/",
        mapped_prefix: "skills/",
        required_suffix: Some("SKILL.md"),
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillResource,
        native_prefix: "skills/",
        mapped_prefix: "skills/",
        required_suffix: None,
    },
    ArtifactPathRule::ReverseDomainExtension {
        class: NativeArtifactClass::ClientExtension,
    },
];

const CODEX_USER_RULES: &[ArtifactPathRule] = &[
    ArtifactPathRule::Exact {
        class: NativeArtifactClass::Instruction,
        native: "AGENTS.md",
        mapped: "AGENTS.md",
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillInstruction,
        native_prefix: "skills/",
        mapped_prefix: "skills/",
        required_suffix: Some("SKILL.md"),
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillResource,
        native_prefix: "skills/",
        mapped_prefix: "skills/",
        required_suffix: None,
    },
];

const OPENCODE_USER_RULES: &[ArtifactPathRule] = &[
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::Rule,
        native_prefix: "rules/",
        mapped_prefix: "rules/",
        required_suffix: Some(".md"),
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillInstruction,
        native_prefix: "skills/",
        mapped_prefix: "skills/",
        required_suffix: Some("SKILL.md"),
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillResource,
        native_prefix: "skills/",
        mapped_prefix: "skills/",
        required_suffix: None,
    },
];

const CLAUDE_USER_RULES: &[ArtifactPathRule] = &[
    ArtifactPathRule::Exact {
        class: NativeArtifactClass::Instruction,
        native: "CLAUDE.md",
        mapped: "CLAUDE.md",
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillInstruction,
        native_prefix: "skills/",
        mapped_prefix: "skills/",
        required_suffix: Some("SKILL.md"),
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillResource,
        native_prefix: "skills/",
        mapped_prefix: "skills/",
        required_suffix: None,
    },
];

const ANTIGRAVITY_USER_RULES: &[ArtifactPathRule] = &[
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::Rule,
        native_prefix: "rules/",
        mapped_prefix: "rules/",
        required_suffix: Some(".md"),
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillInstruction,
        native_prefix: "skills/",
        mapped_prefix: "skills/",
        required_suffix: Some("SKILL.md"),
    },
    ArtifactPathRule::Prefix {
        class: NativeArtifactClass::SkillResource,
        native_prefix: "skills/",
        mapped_prefix: "skills/",
        required_suffix: None,
    },
];

static CODEX_PROJECT: TargetMapping = TargetMapping {
    target: NativeTarget::Codex,
    scope: PublicationScope::Project,
    version: MappingVersion::V0_1,
    rules: CODEX_PROJECT_RULES,
};
static OPENCODE_PROJECT: TargetMapping = TargetMapping {
    target: NativeTarget::OpenCode,
    scope: PublicationScope::Project,
    version: MappingVersion::V0_1,
    rules: OPENCODE_PROJECT_RULES,
};
static CLAUDE_PROJECT: TargetMapping = TargetMapping {
    target: NativeTarget::Claude,
    scope: PublicationScope::Project,
    version: MappingVersion::V0_1,
    rules: CLAUDE_PROJECT_RULES,
};
static CURSOR_PROJECT: TargetMapping = TargetMapping {
    target: NativeTarget::Cursor,
    scope: PublicationScope::Project,
    version: MappingVersion::V0_1,
    rules: CURSOR_PROJECT_RULES,
};
static ANTIGRAVITY_PROJECT: TargetMapping = TargetMapping {
    target: NativeTarget::Antigravity,
    scope: PublicationScope::Project,
    version: MappingVersion::V0_1,
    rules: ANTIGRAVITY_PROJECT_RULES,
};
static AGENT_PLUGIN_PROJECT: TargetMapping = TargetMapping {
    target: NativeTarget::AgentPlugin,
    scope: PublicationScope::Project,
    version: MappingVersion::V0_1,
    rules: AGENT_PLUGIN_PROJECT_RULES,
};
static CODEX_USER: TargetMapping = TargetMapping {
    target: NativeTarget::Codex,
    scope: PublicationScope::User,
    version: MappingVersion::V0_1,
    rules: CODEX_USER_RULES,
};
static OPENCODE_USER: TargetMapping = TargetMapping {
    target: NativeTarget::OpenCode,
    scope: PublicationScope::User,
    version: MappingVersion::V0_1,
    rules: OPENCODE_USER_RULES,
};
static CLAUDE_USER: TargetMapping = TargetMapping {
    target: NativeTarget::Claude,
    scope: PublicationScope::User,
    version: MappingVersion::V0_1,
    rules: CLAUDE_USER_RULES,
};
static ANTIGRAVITY_USER: TargetMapping = TargetMapping {
    target: NativeTarget::Antigravity,
    scope: PublicationScope::User,
    version: MappingVersion::V0_1,
    rules: ANTIGRAVITY_USER_RULES,
};

fn artifact_class_name(class: NativeArtifactClass) -> &'static str {
    match class {
        NativeArtifactClass::Instruction => "instruction",
        NativeArtifactClass::Rule => "rule",
        NativeArtifactClass::SkillInstruction => "skill-instruction",
        NativeArtifactClass::SkillResource => "skill-resource",
        NativeArtifactClass::PluginManifest => "plugin-manifest",
        NativeArtifactClass::McpConfig => "mcp-config",
        NativeArtifactClass::ClientExtension => "client-extension",
    }
}

fn scope_name(scope: PublicationScope) -> &'static str {
    match scope {
        PublicationScope::Project => "project",
        PublicationScope::User => "user",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(class: NativeArtifactClass, path: &str) -> ArtifactDescriptor {
        ArtifactDescriptor {
            class,
            native_path: ResourcePath::parse(path).expect("valid resource path"),
        }
    }

    #[test]
    fn agent_plugin_project_mapping_maps_all_valid_artifact_classes() {
        let mapping = mapping_for(NativeTarget::AgentPlugin, PublicationScope::Project)
            .expect("agent-plugin project mapping is available");
        assert_eq!(mapping.version(), MappingVersion::V0_1);

        let cases = [
            (
                descriptor(NativeArtifactClass::PluginManifest, "plugin.json"),
                "plugin.json",
            ),
            (
                descriptor(NativeArtifactClass::McpConfig, "mcp.json"),
                "mcp.json",
            ),
            (
                descriptor(
                    NativeArtifactClass::SkillInstruction,
                    "skills/my-skill/SKILL.md",
                ),
                "skills/my-skill/SKILL.md",
            ),
            (
                descriptor(
                    NativeArtifactClass::SkillResource,
                    "skills/my-skill/scripts/run.sh",
                ),
                "skills/my-skill/scripts/run.sh",
            ),
            (
                descriptor(
                    NativeArtifactClass::ClientExtension,
                    "com.example.my-ext/config.json",
                ),
                "com.example.my-ext/config.json",
            ),
            (
                descriptor(
                    NativeArtifactClass::ClientExtension,
                    "org.acme.tools/sub/deep/data.yaml",
                ),
                "org.acme.tools/sub/deep/data.yaml",
            ),
        ];

        for (desc, expected) in cases {
            let mapped = mapping
                .map_artifact(&desc)
                .unwrap_or_else(|e| panic!("failed to map {}: {e}", desc.native_path.as_str()));
            assert_eq!(mapped.as_str(), expected);
        }
    }

    #[test]
    fn agent_plugin_project_mapping_rejects_invalid_or_unsafe_client_extensions() {
        let mapping = mapping_for(NativeTarget::AgentPlugin, PublicationScope::Project).unwrap();

        // Not reverse domain (only 1 segment)
        assert!(mapping
            .map_artifact(&descriptor(
                NativeArtifactClass::ClientExtension,
                "single/config.json",
            ))
            .is_err());

        // Hidden directory collision in path
        assert!(mapping
            .map_artifact(&descriptor(
                NativeArtifactClass::ClientExtension,
                "com.example.ext/.git/config",
            ))
            .is_err());

        // Hidden segment inside domain
        assert!(mapping
            .map_artifact(&descriptor(
                NativeArtifactClass::ClientExtension,
                ".com.example/file.json",
            ))
            .is_err());

        // Invalid domain character (uppercase)
        assert!(mapping
            .map_artifact(&descriptor(
                NativeArtifactClass::ClientExtension,
                "Com.Example/file.json",
            ))
            .is_err());
    }

    #[test]
    fn agent_plugin_project_mapping_rejects_unpermitted_classes() {
        let mapping = mapping_for(NativeTarget::AgentPlugin, PublicationScope::Project).unwrap();

        assert!(mapping
            .map_artifact(&descriptor(NativeArtifactClass::Rule, "rules/foo.md"))
            .is_err());
        assert!(mapping
            .map_artifact(&descriptor(NativeArtifactClass::Instruction, "AGENTS.md"))
            .is_err());
    }

    #[test]
    fn agent_plugin_user_mapping_is_unavailable() {
        assert!(mapping_for(NativeTarget::AgentPlugin, PublicationScope::User).is_err());
    }
}
