use std::cmp::Reverse;
use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Canonical enforcement action for a tool access rule.
/// Ordered by restrictiveness: Allow (1) < Ask (2) < Deny (3).
#[derive(
    Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum PermissionAction {
    Allow,
    Ask,
    Deny,
}

impl PermissionAction {
    pub const fn restrictiveness(self) -> u8 {
        match self {
            Self::Allow => 1,
            Self::Ask => 2,
            Self::Deny => 3,
        }
    }

    /// Commutative, associative, idempotent fail-closed join (`Deny > Ask > Allow`).
    pub const fn join_restrictive(self, other: Self) -> Self {
        if self.restrictiveness() >= other.restrictiveness() {
            self
        } else {
            other
        }
    }
}

/// Configuration authority tier for a permission rule or policy.
/// Ordered from lowest authority to highest authority: User (1) < Project (2) < Local (3) < Enterprise (4).
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum PermissionScope {
    User,
    #[default]
    Project,
    Local,
    Enterprise,
}

impl PermissionScope {
    pub const fn rank(self) -> u8 {
        match self {
            Self::User => 1,
            Self::Project => 2,
            Self::Local => 3,
            Self::Enterprise => 4,
        }
    }
}

/// Canonical tool classification.
#[derive(
    Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum CanonicalTool {
    All,
    Bash,
    Read,
    Edit,
    Write,
    WebFetch,
    WebSearch,
    Glob,
    Grep,
    List,
    Agent,
    Skill,
    ExternalDirectory,
    Mcp,
    Custom(String),
}

impl CanonicalTool {
    pub fn as_str(&self) -> &str {
        match self {
            Self::All => "*",
            Self::Bash => "bash",
            Self::Read => "read",
            Self::Edit => "edit",
            Self::Write => "write",
            Self::WebFetch => "webfetch",
            Self::WebSearch => "websearch",
            Self::Glob => "glob",
            Self::Grep => "grep",
            Self::List => "list",
            Self::Agent => "agent",
            Self::Skill => "skill",
            Self::ExternalDirectory => "external_directory",
            Self::Mcp => "mcp",
            Self::Custom(name) => name.as_str(),
        }
    }

    pub fn from_raw(raw: &str) -> Self {
        let normalized = raw.trim().to_lowercase();
        match normalized.as_str() {
            "*" | "all" => Self::All,
            "bash" | "sh" | "shell" | "run_command" | "command" => Self::Bash,
            "read" | "read_file" => Self::Read,
            "edit" => Self::Edit,
            "write" | "write_file" => Self::Write,
            "webfetch" | "read_url" | "fetch" => Self::WebFetch,
            "websearch" | "search" => Self::WebSearch,
            "glob" => Self::Glob,
            "grep" => Self::Grep,
            "list" | "ls" => Self::List,
            "agent" => Self::Agent,
            "skill" => Self::Skill,
            "external_directory" | "externaldirectory" => Self::ExternalDirectory,
            "mcp" => Self::Mcp,
            other => Self::Custom(other.to_string()),
        }
    }
}

/// Normalized pattern representation.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum NormalizedPattern {
    Universal,
    UnanchoredGlob(String),
    AnchoredPrefix(String),
    Exact(String),
}

impl NormalizedPattern {
    pub fn parse(raw: Option<&str>) -> Result<Self> {
        let trimmed = raw.map(str::trim).unwrap_or("");
        if trimmed.is_empty() || trimmed == "*" || trimmed == "**" {
            return Ok(Self::Universal);
        }

        if trimmed.chars().any(|c| c.is_control()) {
            bail!("Pattern cannot contain control characters");
        }

        let has_wildcard = trimmed.chars().any(|c| c == '*' || c == '?' || c == '[');
        if !has_wildcard {
            return Ok(Self::Exact(trimmed.to_string()));
        }

        let starts_with_wildcard = trimmed.starts_with('*') || trimmed.starts_with('?');
        if starts_with_wildcard {
            Ok(Self::UnanchoredGlob(trimmed.to_string()))
        } else {
            Ok(Self::AnchoredPrefix(trimmed.to_string()))
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Universal => "*",
            Self::UnanchoredGlob(s) | Self::AnchoredPrefix(s) | Self::Exact(s) => s.as_str(),
        }
    }

    pub fn specificity_key(&self, tool_is_exact: bool) -> SpecificityKey {
        let pattern_str = self.as_str();
        let class = match self {
            Self::Universal => SpecificityClass::UniversalWildcard,
            Self::UnanchoredGlob(_) => SpecificityClass::UnanchoredGlob,
            Self::AnchoredPrefix(_) => SpecificityClass::AnchoredPrefix,
            Self::Exact(_) => SpecificityClass::Exact,
        };

        let leading_literal_len = match self {
            Self::Universal => 0,
            Self::UnanchoredGlob(_) => 0,
            Self::AnchoredPrefix(s) => s
                .chars()
                .take_while(|&c| c != '*' && c != '?' && c != '[')
                .map(|c| c.len_utf8())
                .sum(),
            Self::Exact(s) => s.len(),
        };

        let literal_segment_count = pattern_str
            .split(|c: char| c == '/' || c.is_whitespace())
            .filter(|segment| {
                !segment.is_empty()
                    && segment
                        .chars()
                        .any(|c| c != '*' && c != '?' && c != '[' && c != ']')
            })
            .count();

        let total_literal_len = pattern_str
            .chars()
            .filter(|&c| c != '*' && c != '?' && c != '[' && c != ']')
            .map(|c| c.len_utf8())
            .sum();

        let wildcard_count = pattern_str
            .chars()
            .filter(|&c| c == '*' || c == '?' || c == '[')
            .count();

        SpecificityKey {
            tool_is_exact,
            class,
            leading_literal_len,
            literal_segment_count,
            total_literal_len,
            negated_wildcard_count: Reverse(wildcard_count),
        }
    }

    /// Check if self structurally subsumes (covers all inputs of) narrower.
    pub fn subsumes(&self, narrower: &Self) -> bool {
        if self == narrower {
            return true;
        }

        match self {
            Self::Universal => true,
            Self::AnchoredPrefix(prefix_pat) => {
                let leading_prefix = prefix_pat
                    .split_once('*')
                    .map(|(literal, _)| literal)
                    .unwrap_or(prefix_pat.as_str());

                match narrower {
                    Self::Universal => false,
                    Self::Exact(s) => s.starts_with(leading_prefix),
                    Self::AnchoredPrefix(p2) => p2.starts_with(leading_prefix),
                    Self::UnanchoredGlob(_) => false,
                }
            }
            Self::Exact(_) | Self::UnanchoredGlob(_) => false,
        }
    }

    /// Check if self and other can match common inputs.
    pub fn overlaps(&self, other: &Self) -> bool {
        if self.subsumes(other) || other.subsumes(self) {
            return true;
        }

        match (self, other) {
            (Self::Exact(a), Self::Exact(b)) => a == b,
            (Self::AnchoredPrefix(p), Self::Exact(s))
            | (Self::Exact(s), Self::AnchoredPrefix(p)) => {
                let literal_prefix = p.split_once('*').map(|(l, _)| l).unwrap_or(p.as_str());
                s.starts_with(literal_prefix)
            }
            (Self::AnchoredPrefix(p1), Self::AnchoredPrefix(p2)) => {
                let l1 = p1.split_once('*').map(|(l, _)| l).unwrap_or(p1.as_str());
                let l2 = p2.split_once('*').map(|(l, _)| l).unwrap_or(p2.as_str());
                l1.starts_with(l2) || l2.starts_with(l1)
            }
            _ => true,
        }
    }

    /// Match candidate string against glob.
    pub fn matches_invocation(&self, candidate: &str) -> bool {
        match self {
            Self::Universal => true,
            Self::Exact(s) => s == candidate,
            Self::UnanchoredGlob(pat) | Self::AnchoredPrefix(pat) => {
                glob_match(pat.as_str(), candidate)
            }
        }
    }
}

/// Simple deterministic glob matching supporting `*` and `?`.
pub fn glob_match(pattern: &str, candidate: &str) -> bool {
    let p_bytes = pattern.as_bytes();
    let c_bytes = candidate.as_bytes();
    let mut p_idx = 0;
    let mut c_idx = 0;
    let mut star_idx = None;
    let mut match_idx = 0;

    while c_idx < c_bytes.len() {
        if p_idx < p_bytes.len() && (p_bytes[p_idx] == b'?' || p_bytes[p_idx] == c_bytes[c_idx]) {
            p_idx += 1;
            c_idx += 1;
        } else if p_idx < p_bytes.len() && p_bytes[p_idx] == b'*' {
            star_idx = Some(p_idx);
            match_idx = c_idx;
            p_idx += 1;
        } else if let Some(s_idx) = star_idx {
            p_idx = s_idx + 1;
            match_idx += 1;
            c_idx = match_idx;
        } else {
            return false;
        }
    }

    while p_idx < p_bytes.len() && p_bytes[p_idx] == b'*' {
        p_idx += 1;
    }

    p_idx == p_bytes.len()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SpecificityClass {
    UniversalWildcard = 1,
    UnanchoredGlob = 2,
    AnchoredPrefix = 3,
    Exact = 4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SpecificityKey {
    pub tool_is_exact: bool,
    pub class: SpecificityClass,
    pub leading_literal_len: usize,
    pub literal_segment_count: usize,
    pub total_literal_len: usize,
    pub negated_wildcard_count: Reverse<usize>,
}

/// Canonical tool access rule.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ToolAccessRule {
    pub tool: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    pub action: PermissionAction,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<PermissionScope>,
}

impl ToolAccessRule {
    pub fn new(tool: impl Into<String>, pattern: Option<String>, action: PermissionAction) -> Self {
        Self {
            tool: tool.into(),
            pattern,
            action,
            scope: None,
        }
    }

    pub fn with_scope(mut self, scope: PermissionScope) -> Self {
        self.scope = Some(scope);
        self
    }
}

/// Canonical permission policy containing default mode and rules.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PermissionPolicy {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_mode: Option<PermissionAction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<ToolAccessRule>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TargetPermissionOverrides {
    pub default: PermissionPolicy,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub overrides: BTreeMap<String, PermissionPolicy>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum TargetPermissionPolicy {
    Wrapped(TargetPermissionOverrides),
    Bare(PermissionPolicy),
}

impl TargetPermissionPolicy {
    pub fn resolve(&self, target: &str) -> &PermissionPolicy {
        match self {
            Self::Bare(policy) => policy,
            Self::Wrapped(wrapped) => {
                if let Some(p) = wrapped.overrides.get(target) {
                    return p;
                }
                if let Some((prefix, _)) = target.split_once('-') {
                    if let Some(p) = wrapped.overrides.get(prefix) {
                        return p;
                    }
                }
                &wrapped.default
            }
        }
    }
}

/// Precedence tier for 4-tier total ordering.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum PrecedenceTier {
    UniversalBaseline = 1,
    SpecificAllow = 2,
    SpecificAsk = 3,
    HardDeny = 4,
}

/// Normalized rule after extraction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPermissionRule {
    pub tier: PrecedenceTier,
    pub specificity_key: SpecificityKey,
    pub canonical_tool: CanonicalTool,
    pub normalized_pattern: NormalizedPattern,
    pub action: PermissionAction,
    pub scope: PermissionScope,
    pub source_identity: String,
    pub source_package_id: String,
    pub original_index: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShadowedPermissionRule {
    pub shadowed_rule: ResolvedPermissionRule,
    pub shadowed_by: ResolvedPermissionRule,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedPermissionPolicy {
    pub default_mode: Option<PermissionAction>,
    pub active_rules: Vec<ResolvedPermissionRule>,
    pub shadowed_rules: Vec<ShadowedPermissionRule>,
}

impl ResolvedPermissionPolicy {
    /// Evaluates the policy against an invocation: highest matching tier wins.
    pub fn evaluate(&self, tool: &str, arg: Option<&str>) -> PermissionAction {
        let inv_tool = CanonicalTool::from_raw(tool);
        let candidate_arg = arg.unwrap_or("").trim();

        for tier in [
            PrecedenceTier::HardDeny,
            PrecedenceTier::SpecificAsk,
            PrecedenceTier::SpecificAllow,
            PrecedenceTier::UniversalBaseline,
        ] {
            for rule in self.active_rules.iter().filter(|r| r.tier == tier) {
                let tool_matches = match &rule.canonical_tool {
                    CanonicalTool::All => true,
                    ct => ct == &inv_tool,
                };
                if !tool_matches {
                    continue;
                }

                if rule.normalized_pattern.matches_invocation(candidate_arg) {
                    return rule.action;
                }
            }
        }

        self.default_mode.unwrap_or(PermissionAction::Ask)
    }
}

/// Normalize tool and pattern specifiers.
pub fn normalize_tool_specifier(
    raw_tool: &str,
    raw_pattern: Option<&str>,
) -> Result<(CanonicalTool, NormalizedPattern)> {
    let raw_tool = raw_tool.trim();
    if raw_tool.is_empty() {
        bail!("Tool name cannot be empty");
    }
    if raw_tool.chars().any(|c| c.is_control()) {
        bail!("Tool name cannot contain control characters");
    }

    if let Some((tool_prefix, inner)) = raw_tool.split_once('(') {
        if !inner.ends_with(')') {
            bail!("Malformed tool specifier missing closing parenthesis: {raw_tool}");
        }
        let specifier = &inner[..inner.len() - 1].trim();
        if specifier.is_empty() {
            bail!("Empty parenthesized specifier in tool name: {raw_tool}");
        }

        let canonical_tool = CanonicalTool::from_raw(tool_prefix);
        let normalized_pattern = match canonical_tool {
            CanonicalTool::Bash => {
                if let Some(stripped) = specifier.strip_prefix("cmd:") {
                    NormalizedPattern::parse(Some(stripped))?
                } else if let Some((cmd, rest)) = specifier.split_once(':') {
                    NormalizedPattern::parse(Some(&format!("{cmd} {rest}")))?
                } else {
                    NormalizedPattern::parse(Some(specifier))?
                }
            }
            CanonicalTool::WebFetch => {
                if let Some(domain) = specifier.strip_prefix("domain:") {
                    NormalizedPattern::parse(Some(domain))?
                } else {
                    NormalizedPattern::parse(Some(specifier))?
                }
            }
            _ => NormalizedPattern::parse(Some(specifier))?,
        };
        return Ok((canonical_tool, normalized_pattern));
    }

    if let Some(mcp_spec) = raw_tool.strip_prefix("mcp__") {
        if let Some((server, tool)) = mcp_spec.split_once("__") {
            return Ok((
                CanonicalTool::Mcp,
                NormalizedPattern::parse(Some(&format!("{server}/{tool}")))?,
            ));
        }
    }
    if let Some((server, tool)) = raw_tool.split_once(':') {
        if !server.is_empty() && !tool.is_empty() && !server.contains('/') {
            return Ok((
                CanonicalTool::Mcp,
                NormalizedPattern::parse(Some(&format!("{server}/{tool}")))?,
            ));
        }
    }

    let canonical_tool = CanonicalTool::from_raw(raw_tool);
    let normalized_pattern = NormalizedPattern::parse(raw_pattern)?;
    Ok((canonical_tool, normalized_pattern))
}

/// Resolve a permission policy into canonical 4-tier resolved policy.
pub fn resolve_permission_policy(
    policy: &PermissionPolicy,
    source_identity: &str,
    source_package_id: &str,
) -> Result<ResolvedPermissionPolicy> {
    let mut normalized_rules = Vec::new();

    for (idx, rule) in policy.rules.iter().enumerate() {
        let (canonical_tool, normalized_pattern) =
            normalize_tool_specifier(&rule.tool, rule.pattern.as_deref())
                .with_context(|| format!("Failed to normalize rule at index {idx}"))?;

        let tool_is_exact = canonical_tool != CanonicalTool::All;
        let specificity_key = normalized_pattern.specificity_key(tool_is_exact);

        let tier = match rule.action {
            PermissionAction::Deny => PrecedenceTier::HardDeny,
            PermissionAction::Ask => {
                if normalized_pattern == NormalizedPattern::Universal {
                    PrecedenceTier::UniversalBaseline
                } else {
                    PrecedenceTier::SpecificAsk
                }
            }
            PermissionAction::Allow => {
                if normalized_pattern == NormalizedPattern::Universal {
                    PrecedenceTier::UniversalBaseline
                } else {
                    PrecedenceTier::SpecificAllow
                }
            }
        };

        normalized_rules.push(ResolvedPermissionRule {
            tier,
            specificity_key,
            canonical_tool,
            normalized_pattern,
            action: rule.action,
            scope: rule.scope.unwrap_or_default(),
            source_identity: source_identity.to_string(),
            source_package_id: source_package_id.to_string(),
            original_index: idx,
        });
    }

    // Deduplicate exact (CanonicalTool, NormalizedPattern) collisions
    let mut deduplicated: BTreeMap<(CanonicalTool, NormalizedPattern), ResolvedPermissionRule> =
        BTreeMap::new();

    for rule in normalized_rules {
        let key = (rule.canonical_tool.clone(), rule.normalized_pattern.clone());
        match deduplicated.get_mut(&key) {
            Some(existing) => {
                // Fail-Closed Deny dominance
                if existing.action == PermissionAction::Deny
                    || rule.action == PermissionAction::Deny
                {
                    existing.action = PermissionAction::Deny;
                    existing.tier = PrecedenceTier::HardDeny;
                } else if rule.scope.rank() > existing.scope.rank() {
                    *existing = rule;
                } else if rule.scope.rank() == existing.scope.rank() {
                    existing.action = existing.action.join_restrictive(rule.action);
                    existing.tier = match existing.action {
                        PermissionAction::Deny => PrecedenceTier::HardDeny,
                        PermissionAction::Ask => {
                            if existing.normalized_pattern == NormalizedPattern::Universal {
                                PrecedenceTier::UniversalBaseline
                            } else {
                                PrecedenceTier::SpecificAsk
                            }
                        }
                        PermissionAction::Allow => {
                            if existing.normalized_pattern == NormalizedPattern::Universal {
                                PrecedenceTier::UniversalBaseline
                            } else {
                                PrecedenceTier::SpecificAllow
                            }
                        }
                    };
                }
            }
            None => {
                deduplicated.insert(key, rule);
            }
        }
    }

    let mut rules: Vec<ResolvedPermissionRule> = deduplicated.into_values().collect();

    // Sort active_rules in strict ascending order of (tier, SpecificityKey, canonical_tool, normalized_pattern)
    rules.sort_by(|a, b| {
        a.tier
            .cmp(&b.tier)
            .then_with(|| a.specificity_key.cmp(&b.specificity_key))
            .then_with(|| a.canonical_tool.cmp(&b.canonical_tool))
            .then_with(|| a.normalized_pattern.cmp(&b.normalized_pattern))
            .then_with(|| a.source_identity.cmp(&b.source_identity))
            .then_with(|| a.source_package_id.cmp(&b.source_package_id))
            .then_with(|| a.original_index.cmp(&b.original_index))
    });

    // Shadowing detection:
    // Any rule R_low in Tier 1-3 subsumed by R_high in Tier 3-4 with tier(R_high) > tier(R_low) is shadowed.
    let mut active_rules = Vec::new();
    let mut shadowed_rules = Vec::new();

    for r_low in rules.iter() {
        let is_low_tier = r_low.tier == PrecedenceTier::UniversalBaseline
            || r_low.tier == PrecedenceTier::SpecificAllow
            || r_low.tier == PrecedenceTier::SpecificAsk;

        let mut shadowed_by = None;
        if is_low_tier {
            for r_high in rules.iter() {
                let is_high_tier = r_high.tier == PrecedenceTier::SpecificAsk
                    || r_high.tier == PrecedenceTier::HardDeny;

                if is_high_tier && (r_high.tier as u8) > (r_low.tier as u8) {
                    let tool_covers = r_high.canonical_tool == CanonicalTool::All
                        || r_high.canonical_tool == r_low.canonical_tool;
                    if tool_covers
                        && r_high
                            .normalized_pattern
                            .subsumes(&r_low.normalized_pattern)
                    {
                        shadowed_by = Some(r_high.clone());
                        break;
                    }
                }
            }
        }

        if let Some(higher) = shadowed_by {
            shadowed_rules.push(ShadowedPermissionRule {
                shadowed_rule: r_low.clone(),
                shadowed_by: higher,
            });
        } else {
            active_rules.push(r_low.clone());
        }
    }

    Ok(ResolvedPermissionPolicy {
        default_mode: policy.default_mode,
        active_rules,
        shadowed_rules,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_1_deserialization_fail_closed() {
        let missing_action = r#"{"tool": "bash"}"#;
        assert!(serde_json::from_str::<ToolAccessRule>(missing_action).is_err());

        let unknown_action = r#"{"tool": "bash", "action": "permit"}"#;
        assert!(serde_json::from_str::<ToolAccessRule>(unknown_action).is_err());
    }

    #[test]
    fn fixture_2_specificity_key_total_order() {
        let git_status = NormalizedPattern::parse(Some("git status"))
            .unwrap()
            .specificity_key(true);
        let git_push_force = NormalizedPattern::parse(Some("git push --force *"))
            .unwrap()
            .specificity_key(true);
        let git_push = NormalizedPattern::parse(Some("git push *"))
            .unwrap()
            .specificity_key(true);
        let git_wildcard = NormalizedPattern::parse(Some("git *"))
            .unwrap()
            .specificity_key(true);
        let version_glob = NormalizedPattern::parse(Some("* --version"))
            .unwrap()
            .specificity_key(true);
        let universal = NormalizedPattern::parse(Some("*"))
            .unwrap()
            .specificity_key(true);

        assert!(git_status > git_push_force);
        assert!(git_push_force > git_push);
        assert!(git_push > git_wildcard);
        assert!(git_wildcard > version_glob);
        assert!(version_glob > universal);
    }

    #[test]
    fn fixture_3_exact_collision_fail_closed_and_permutation_invariance() {
        let r_allow =
            ToolAccessRule::new("bash", Some("git push *".into()), PermissionAction::Allow);
        let r_ask = ToolAccessRule::new("bash", Some("git push *".into()), PermissionAction::Ask);
        let r_deny = ToolAccessRule::new("bash", Some("git push *".into()), PermissionAction::Deny);

        let permutations = vec![
            vec![r_allow.clone(), r_ask.clone(), r_deny.clone()],
            vec![r_allow.clone(), r_deny.clone(), r_ask.clone()],
            vec![r_ask.clone(), r_allow.clone(), r_deny.clone()],
            vec![r_ask.clone(), r_deny.clone(), r_allow.clone()],
            vec![r_deny.clone(), r_allow.clone(), r_ask.clone()],
            vec![r_deny.clone(), r_ask.clone(), r_allow.clone()],
        ];

        for perm in permutations {
            let policy = PermissionPolicy {
                default_mode: None,
                rules: perm,
            };
            let resolved = resolve_permission_policy(&policy, "id", "pkg").unwrap();
            assert_eq!(resolved.active_rules.len(), 1);
            assert_eq!(resolved.active_rules[0].action, PermissionAction::Deny);
        }
    }

    #[test]
    fn fixture_4_shadowing_detection() {
        let policy = PermissionPolicy {
            default_mode: None,
            rules: vec![
                ToolAccessRule::new("bash", Some("git *".into()), PermissionAction::Deny),
                ToolAccessRule::new("bash", Some("git status".into()), PermissionAction::Allow),
            ],
        };
        let resolved = resolve_permission_policy(&policy, "id", "pkg").unwrap();
        assert_eq!(resolved.active_rules.len(), 1);
        assert_eq!(resolved.active_rules[0].action, PermissionAction::Deny);
        assert_eq!(resolved.shadowed_rules.len(), 1);
        assert_eq!(
            resolved.shadowed_rules[0].shadowed_rule.action,
            PermissionAction::Allow
        );
        assert_eq!(
            resolved.evaluate("bash", Some("git status")),
            PermissionAction::Deny
        );
    }

    #[test]
    fn fixture_5_partial_glob_overlap_fail_closed() {
        let policy = PermissionPolicy {
            default_mode: None,
            rules: vec![
                ToolAccessRule::new("bash", Some("* --help".into()), PermissionAction::Allow),
                ToolAccessRule::new("bash", Some("rm *".into()), PermissionAction::Deny),
            ],
        };
        let resolved = resolve_permission_policy(&policy, "id", "pkg").unwrap();
        assert_eq!(
            resolved.evaluate("bash", Some("rm -rf / --help")),
            PermissionAction::Deny
        );
    }

    #[test]
    fn fixture_6_sequential_vs_set_based_equivalence() {
        let policy = PermissionPolicy {
            default_mode: Some(PermissionAction::Ask),
            rules: vec![
                ToolAccessRule::new("*", Some("*".into()), PermissionAction::Ask),
                ToolAccessRule::new("bash", Some("git *".into()), PermissionAction::Allow),
                ToolAccessRule::new("bash", Some("git push *".into()), PermissionAction::Ask),
                ToolAccessRule::new(
                    "bash",
                    Some("git push --force *".into()),
                    PermissionAction::Deny,
                ),
            ],
        };
        let resolved = resolve_permission_policy(&policy, "id", "pkg").unwrap();

        let test_matrix = [
            ("bash", "git status"),
            ("bash", "git log"),
            ("bash", "git push origin main"),
            ("bash", "git push --force origin main"),
            ("bash", "rm -rf /"),
            ("read", "src/main.rs"),
        ];

        for (tool, arg) in test_matrix {
            let set_decision = resolved.evaluate(tool, Some(arg));

            // Sequential last-match evaluation over active_rules (OpenCode semantics)
            let mut seq_decision = resolved.default_mode.unwrap_or(PermissionAction::Ask);
            let inv_tool = CanonicalTool::from_raw(tool);
            for r in &resolved.active_rules {
                let tool_match =
                    r.canonical_tool == CanonicalTool::All || r.canonical_tool == inv_tool;
                if tool_match && r.normalized_pattern.matches_invocation(arg) {
                    seq_decision = r.action;
                }
            }

            assert_eq!(
                set_decision, seq_decision,
                "Mismatch for tool {tool} with arg {arg}: set={set_decision:?}, seq={seq_decision:?}"
            );
        }
    }
}
