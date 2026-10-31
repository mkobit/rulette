pub mod agent_plugin;
pub mod aggregation;
pub mod antigravity;
pub mod claude;
pub mod codex;
pub mod cursor;
pub mod frontend;
pub mod opencode;
pub mod skills;

pub use aggregation::{
    aggregate, AggregateCollisionCandidate, AggregateCollisionGroup, AggregateCollisionKey,
    AggregationCandidate, AggregationCollisionError, AggregationRequest, OuterInputIdentity,
};
pub use frontend::{
    compile_graph, compile_native_frontend, DecoderSelection, NativeCompilation, NativeFrontend,
    NativeObservationDisposition,
};

/// Splits a comma-separated glob string while preserving commas inside `{...}`
/// brace expansions and `[...]` character classes.
pub(crate) fn split_glob_list(raw: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut start = 0usize;

    for (idx, ch) in raw.char_indices() {
        match ch {
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            ',' if brace_depth == 0 && bracket_depth == 0 => {
                let segment = raw[start..idx].trim();
                if !segment.is_empty() {
                    result.push(segment.to_owned());
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    let tail = raw[start..].trim();
    if !tail.is_empty() {
        result.push(tail.to_owned());
    }
    result
}

/// Splits a comma- or whitespace-separated tool allowlist string while
/// preserving spaces and commas inside `(...)` argument patterns such as
/// `Bash(git *)`.
pub(crate) fn split_tool_list(raw: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut paren_depth = 0usize;
    let mut start = 0usize;

    for (idx, ch) in raw.char_indices() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if paren_depth == 0 => {
                let segment = raw[start..idx].trim();
                if !segment.is_empty() {
                    result.push(segment.to_owned());
                }
                start = idx + ch.len_utf8();
            }
            c if c.is_whitespace() && paren_depth == 0 => {
                let segment = raw[start..idx].trim();
                if !segment.is_empty() {
                    result.push(segment.to_owned());
                }
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    let tail = raw[start..].trim();
    if !tail.is_empty() {
        result.push(tail.to_owned());
    }
    result
}

/// Validates every glob pattern in a slice using `validate_safe_relative_glob`.
pub(crate) fn validate_glob_list(globs: &[String]) -> anyhow::Result<()> {
    for glob in globs {
        crate::ir::validate_safe_relative_glob(glob)?;
    }
    Ok(())
}

/// Splits optional YAML frontmatter delimited by `---` lines from a Markdown document.
pub(crate) fn split_optional_frontmatter(input: &str) -> (Option<&str>, &str) {
    let body_start = if input.starts_with("---\r\n") {
        5
    } else if input.starts_with("---\n") {
        4
    } else {
        return (None, input);
    };
    let rest = &input[body_start..];
    let mut frontmatter_len = 0usize;
    for line in rest.split_inclusive('\n') {
        let line_without_ending = line.trim_end_matches(['\r', '\n']);
        if line_without_ending == "---" {
            let fm = input[body_start..body_start + frontmatter_len].trim();
            let body = &rest[frontmatter_len + line.len()..];
            return (Some(fm), body);
        }
        frontmatter_len += line.len();
    }
    (None, input)
}
