use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod graph;
pub mod hooks;
pub mod permissions;

pub use graph::{validate_safe_relative_glob, SkillMetadata};
pub use hooks::{
    HookAction, HookDefinition, HookEventKind, HookMatcher, HookScope, TargetHookPolicy,
};

pub use permissions::{
    CanonicalTool, NormalizedPattern, PermissionAction, PermissionPolicy, PermissionScope,
    PrecedenceTier, ResolvedPermissionPolicy, ResolvedPermissionRule, SpecificityClass,
    SpecificityKey, TargetPermissionOverrides, TargetPermissionPolicy, ToolAccessRule,
};

/// A portable rule-activation mode retained by the compilation graph.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivationMode {
    Always,
    Glob,
    Pattern,
    Manual,
    Model,
}
