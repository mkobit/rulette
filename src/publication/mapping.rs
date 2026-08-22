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
        (NativeTarget::Codex, PublicationScope::User) => Ok(&CODEX_USER),
        (NativeTarget::OpenCode, PublicationScope::User) => Ok(&OPENCODE_USER),
        (NativeTarget::Claude, PublicationScope::User) => Ok(&CLAUDE_USER),
        (NativeTarget::Antigravity, PublicationScope::User) => Ok(&ANTIGRAVITY_USER),
        (NativeTarget::Cursor, PublicationScope::User) => {
            bail!("user mapping is unavailable for target `cursor`")
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
    }
}

fn scope_name(scope: PublicationScope) -> &'static str {
    match scope {
        PublicationScope::Project => "project",
        PublicationScope::User => "user",
    }
}
