use crate::inputs::ArtifactObservation;
use std::collections::BTreeSet;

/// Checks if the path is inside `.agents/skills/`.
pub(crate) fn is_agents_skills_path(path: &str) -> bool {
    let components: Vec<_> = path.split('/').collect();
    components
        .windows(2)
        .any(|window| window[0] == ".agents" && window[1] == "skills")
}

/// Checks if the path belongs to a shared skill layout (`.agents/skills/` or `skills/` or direct skill).
pub(crate) fn is_shared_skill_path(path: &str) -> bool {
    is_agents_skills_path(path)
        || path.starts_with("skills/")
        || path.split('/').any(|component| component == "skills")
        || path == "SKILL.md"
        || path.ends_with("/SKILL.md")
}

/// Checks whether an observation set contains any shared skills.
pub(crate) fn has_shared_skills(observations: &[ArtifactObservation]) -> bool {
    observations
        .iter()
        .any(|obs| is_shared_skill_path(obs.source_path.as_str()))
}

/// Paths reserved for harness rule or configuration files that should not be absorbed
/// as root skill companions when `SKILL.md` is at the root.
pub(crate) fn is_harness_path(path: &str) -> bool {
    let file_name = path.rsplit('/').next().unwrap_or(path);
    matches!(
        file_name,
        "AGENTS.md"
            | "CLAUDE.md"
            | ".mcp.json"
            | "opencode.json"
            | "opencode.jsonc"
            | "plugin.json"
    ) || path.starts_with(".codex/")
        || path.starts_with(".claude/")
        || path.starts_with(".cursor/")
        || path.starts_with(".opencode/")
        || path.starts_with(".antigravity/")
        || path.starts_with(".agent/")
        || path.starts_with(".agents/rules/")
        || path.ends_with(".mdc")
}

/// Direct skill roots discovered from observations.
#[derive(Clone, Debug, Default)]
pub(crate) struct DirectSkillRoots<'a> {
    roots: BTreeSet<(&'a str, &'a str)>,
    root_skills: BTreeSet<&'a str>,
}

impl<'a> DirectSkillRoots<'a> {
    pub(crate) fn from_observations(observations: &'a [ArtifactObservation]) -> Self {
        let mut roots = BTreeSet::new();
        let mut root_skills = BTreeSet::new();
        for obs in observations {
            let path = obs.source_path.as_str();
            let label = obs.provenance.input_label.as_str();
            if path == "SKILL.md" {
                root_skills.insert(label);
            } else if let Some((dir, "SKILL.md")) = path.split_once('/') {
                if !dir.is_empty() && !dir.starts_with('.') && dir != "skills" && !dir.contains('/')
                {
                    roots.insert((label, dir));
                }
            }
        }
        Self { roots, root_skills }
    }

    /// Matches an observation against harness skill prefixes, shared skill layouts,
    /// direct skill directories, and root skills.
    /// Returns `Some((skill_root, member_relative_path))`.
    pub(crate) fn match_skill(
        &self,
        harness_prefix: Option<&str>,
        observation: &'a ArtifactObservation,
    ) -> Option<(&'a str, &'a str)> {
        let path = observation.source_path.as_str();
        let label = observation.provenance.input_label.as_str();

        // 1. Harness prefix (e.g. ".codex/skills/", ".claude/skills/")
        if let Some(prefix) = harness_prefix {
            if let Some(suffix) = path.strip_prefix(prefix) {
                if let Some((name, member)) = suffix.split_once('/') {
                    if !name.is_empty() && !member.is_empty() {
                        let root_len = prefix.len() + name.len();
                        return Some((&path[..root_len], member));
                    }
                }
            }
        }

        // 2. Shared skill prefixes: .agents/skills/ and skills/
        for prefix in [".agents/skills/", "skills/"] {
            if let Some(suffix) = path.strip_prefix(prefix) {
                if let Some((name, member)) = suffix.split_once('/') {
                    if !name.is_empty() && !member.is_empty() {
                        let root_len = prefix.len() + name.len();
                        return Some((&path[..root_len], member));
                    }
                }
            }
        }

        // 3. Direct directory input: <name>/<member> where <name>/SKILL.md exists
        if let Some((dir, member)) = path.split_once('/') {
            if self.roots.contains(&(label, dir)) && !member.is_empty() {
                return Some((dir, member));
            }
        }

        // 4. Root skill: SKILL.md and non-harness companions at root "."
        if self.root_skills.contains(label) {
            if path == "SKILL.md" {
                return Some((".", "SKILL.md"));
            }
            if !is_harness_path(path) {
                return Some((".", path));
            }
        }

        None
    }
}
