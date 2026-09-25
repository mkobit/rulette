use crate::inputs::ArtifactObservation;
use crate::ir::graph::{
    validate_skill_name, DiagnosticSeverity, FrontendPayload, GraphDiagnostic, Package,
    PackageKind, PackageRoot, Resource, ResourceContent, ResourcePath, SemanticIdentity,
    SemanticItem, SourceProvenance,
};
use crate::parsers::frontend::{NativeCompilation, NativeFrontend, NativeObservationDisposition};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

pub mod model;
pub mod wire;

pub use model::{McpConfiguration, McpServer, McpTransport, PluginAuthor, PluginManifest};
pub use wire::{
    McpConfigWireV1, McpServerWireV1, PluginAuthorWireV1, PluginManifestWireV1,
    SkillFrontmatterWire, MCP_CONFIG_SCHEMA_V1, PLUGIN_MANIFEST_SCHEMA_V1,
};

pub(crate) fn compile_native(observations: &[ArtifactObservation]) -> Result<NativeCompilation> {
    let mut ordered_observations: Vec<_> = observations.iter().enumerate().collect();
    ordered_observations
        .sort_by(|left, right| observation_key(left.1).cmp(&observation_key(right.1)));

    let mut packages = Vec::new();
    let mut dispositions = vec![None; observations.len()];
    let mut extra_diagnostics = Vec::new();

    let mut skill_members: BTreeMap<SkillGroupKey, Vec<(usize, &ArtifactObservation)>> =
        BTreeMap::new();
    let mut extension_members: BTreeMap<ExtensionGroupKey, Vec<(usize, &ArtifactObservation)>> =
        BTreeMap::new();

    for (index, observation) in ordered_observations {
        let path = observation.source_path.as_str();
        if path == "plugin.json" {
            let manifest_text = std::str::from_utf8(&observation.bytes)
                .context("Agent Plugin manifest plugin.json must be valid UTF-8")?;
            let wire: PluginManifestWireV1 = serde_json::from_str(manifest_text)
                .context("Agent Plugin manifest plugin.json must match PluginManifestWireV1")?;
            let domain_manifest = PluginManifest::try_from(wire.clone())
                .context("Agent Plugin manifest validation failed")?;

            let resource_path = ResourcePath::parse("plugin.json")?;
            let mut resources = BTreeMap::new();
            resources.insert(
                resource_path.clone(),
                Resource::opaque(
                    resource_path,
                    ResourceContent::Text(manifest_text.to_owned()),
                    observation.executable,
                ),
            );

            let mut payload_fields = BTreeMap::new();
            if let serde_json::Value::Object(map) = serde_json::to_value(&domain_manifest)? {
                payload_fields = map.into_iter().collect();
            }

            let package = Package::new(
                PackageKind::Unsupported,
                SemanticIdentity::parse("unsupported:agent-plugin-manifest/plugin.json")?,
                provenance(observation)?,
                PackageRoot::root(),
                SemanticItem::Unsupported {
                    native_kind: "manifest".to_owned(),
                },
                resources,
                Some(FrontendPayload {
                    namespace: "agent-plugin.manifest".to_owned(),
                    fields: payload_fields,
                }),
            )?;

            for extra_key in wire.extra.keys() {
                extra_diagnostics.push(GraphDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    code: "unknown-manifest-field".to_owned(),
                    message: format!("plugin manifest contains unknown field `{extra_key}`"),
                    package_id: Some(package.id.clone()),
                });
            }

            packages.push(package);
            dispositions[index] = Some(NativeObservationDisposition::RetainedUnsupportedContent);
        } else if path == "mcp.json" {
            let mcp_text = std::str::from_utf8(&observation.bytes)
                .context("Agent Plugin mcp.json must be valid UTF-8")?;
            let wire: McpConfigWireV1 = serde_json::from_str(mcp_text)
                .context("Agent Plugin mcp.json must match McpConfigWireV1")?;

            for (name, server_wire) in wire.mcp_servers {
                match McpServer::try_from(server_wire.clone()) {
                    Ok(_domain_server) => {
                        let semantic_identity = SemanticIdentity::parse(format!(
                            "unsupported:agent-plugin-mcp-server/{name}"
                        ))?;
                        let resource_path =
                            ResourcePath::parse(format!("mcp-servers/{name}.json"))?;
                        let resource_content = ResourceContent::Text(
                            serde_json::to_string_pretty(&server_wire)? + "\n",
                        );
                        let mut resources = BTreeMap::new();
                        resources.insert(
                            resource_path.clone(),
                            Resource::opaque(resource_path, resource_content, false),
                        );

                        let mut payload_fields = BTreeMap::new();
                        payload_fields
                            .insert("name".to_string(), serde_json::Value::String(name.clone()));
                        let wire_val = serde_json::to_value(&server_wire)?;
                        payload_fields.insert("server".to_string(), wire_val.clone());
                        payload_fields.insert("config".to_string(), wire_val);

                        let package = Package::new(
                            PackageKind::Unsupported,
                            semantic_identity,
                            provenance(observation)?,
                            PackageRoot::root(),
                            SemanticItem::Unsupported {
                                native_kind: "mcp-server".to_owned(),
                            },
                            resources,
                            Some(FrontendPayload {
                                namespace: "agent-plugin.mcp-server".to_owned(),
                                fields: payload_fields,
                            }),
                        )?;
                        packages.push(package);
                    }
                    Err(err) => {
                        extra_diagnostics.push(GraphDiagnostic {
                            severity: DiagnosticSeverity::Warning,
                            code: "invalid-mcp-server".to_owned(),
                            message: format!("MCP server `{name}` is invalid: {err:#}"),
                            package_id: None,
                        });
                    }
                }
            }
            dispositions[index] = Some(NativeObservationDisposition::RetainedUnsupportedContent);
        } else if let Some((skill_dir, _)) = skill_dir_and_member(path) {
            skill_members
                .entry(SkillGroupKey::from_observation(skill_dir, observation))
                .or_default()
                .push((index, observation));
        } else if let Some((extension_dir, _)) = extension_dir_and_member(path) {
            extension_members
                .entry(ExtensionGroupKey::from_observation(
                    extension_dir,
                    observation,
                ))
                .or_default()
                .push((index, observation));
        } else {
            dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
        }
    }

    for (key, members) in skill_members {
        let skill_dir = &key.skill_dir;
        let root = format!("skills/{skill_dir}");

        if let Err(err) = validate_skill_name(skill_dir) {
            extra_diagnostics.push(GraphDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "invalid-skill".to_owned(),
                message: format!("skill `{skill_dir}` has invalid name: {err:#}"),
                package_id: None,
            });
            for (index, _) in members {
                dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
            }
            continue;
        }

        let primary_pos = members.iter().position(|(_, obs)| {
            skill_dir_and_member(obs.source_path.as_str()).map(|(_, member)| member)
                == Some("SKILL.md")
        });

        let Some(primary_pos) = primary_pos else {
            extra_diagnostics.push(GraphDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "invalid-skill".to_owned(),
                message: format!("skill `{skill_dir}` is missing SKILL.md"),
                package_id: None,
            });
            for (index, _) in members {
                dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
            }
            continue;
        };

        let (_, primary_obs) = members[primary_pos];
        let primary_text = match std::str::from_utf8(&primary_obs.bytes) {
            Ok(text) => text,
            Err(_) => {
                extra_diagnostics.push(GraphDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    code: "invalid-skill".to_owned(),
                    message: format!("skill `{skill_dir}` SKILL.md is not valid UTF-8"),
                    package_id: None,
                });
                for (index, _) in members {
                    dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
                }
                continue;
            }
        };

        let (frontmatter_str, body) = split_frontmatter(primary_text);
        let Some(frontmatter_str) = frontmatter_str else {
            extra_diagnostics.push(GraphDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "invalid-skill".to_owned(),
                message: format!("skill `{skill_dir}` SKILL.md is missing YAML frontmatter"),
                package_id: None,
            });
            for (index, _) in members {
                dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
            }
            continue;
        };

        let frontmatter_wire: SkillFrontmatterWire = match serde_yaml::from_str(frontmatter_str) {
            Ok(wire) => wire,
            Err(err) => {
                extra_diagnostics.push(GraphDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    code: "invalid-skill".to_owned(),
                    message: format!("skill `{skill_dir}` frontmatter YAML error: {err}"),
                    package_id: None,
                });
                for (index, _) in members {
                    dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
                }
                continue;
            }
        };

        let Some(description) = frontmatter_wire.description.filter(|d| !d.is_empty()) else {
            extra_diagnostics.push(GraphDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "invalid-skill".to_owned(),
                message: format!("skill `{skill_dir}` SKILL.md is missing a non-empty description"),
                package_id: None,
            });
            for (index, _) in members {
                dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
            }
            continue;
        };

        let desc_len = description.chars().count();
        if !(1..=1024).contains(&desc_len) {
            extra_diagnostics.push(GraphDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "invalid-skill".to_owned(),
                message: format!(
                    "skill `{skill_dir}` description length must be 1 to 1024 characters"
                ),
                package_id: None,
            });
            for (index, _) in members {
                dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
            }
            continue;
        }

        if let Some(ref name) = frontmatter_wire.name {
            if name != skill_dir {
                extra_diagnostics.push(GraphDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    code: "invalid-skill".to_owned(),
                    message: format!(
                        "skill `{skill_dir}` name in frontmatter `{name}` does not match directory"
                    ),
                    package_id: None,
                });
                for (index, _) in members {
                    dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
                }
                continue;
            }
        }

        let primary_path = ResourcePath::parse("SKILL.md")?;
        let mut resources = BTreeMap::new();
        let mut duplicate_found = false;

        for (_, obs) in &members {
            let (_, member_path) = skill_dir_and_member(obs.source_path.as_str()).unwrap();
            let path = match ResourcePath::parse(member_path) {
                Ok(path) => path,
                Err(_) => {
                    duplicate_found = true;
                    break;
                }
            };
            let resource = if obs.source_path == primary_obs.source_path {
                Resource::primary_instruction(
                    path.clone(),
                    ResourceContent::Text(body.to_owned()),
                    obs.executable,
                )
            } else {
                let content = match std::str::from_utf8(&obs.bytes) {
                    Ok(text) => ResourceContent::Text(text.to_owned()),
                    Err(_) => ResourceContent::Bytes(obs.bytes.clone()),
                };
                Resource::opaque(path.clone(), content, obs.executable)
            };
            if resources.insert(path, resource).is_some() {
                duplicate_found = true;
                break;
            }
        }

        if duplicate_found {
            extra_diagnostics.push(GraphDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "invalid-skill".to_owned(),
                message: format!("skill `{skill_dir}` contains duplicate or invalid resources"),
                package_id: None,
            });
            for (index, _) in members {
                dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
            }
            continue;
        }

        let payload = (!frontmatter_wire.extra.is_empty()).then(|| FrontendPayload {
            namespace: "agent-plugin.skill-frontmatter".to_owned(),
            fields: frontmatter_wire.extra,
        });

        let package = Package::new(
            PackageKind::Skill,
            SemanticIdentity::parse(format!("skill:{skill_dir}"))?,
            provenance(primary_obs)?,
            PackageRoot::parse(root)?,
            SemanticItem::Skill {
                primary_instruction: primary_path,
                description,
                frontend_payload: payload,
            },
            resources,
            None,
        )?;

        packages.push(package);
        for (index, _) in members {
            dispositions[index] = Some(NativeObservationDisposition::PackageContent);
        }
    }

    for (key, members) in extension_members {
        let extension_dir = &key.extension_dir;
        let (_, primary_obs) = members[0];
        let mut resources = BTreeMap::new();
        let mut duplicate_found = false;

        for (_, obs) in &members {
            let (_, member_path) = extension_dir_and_member(obs.source_path.as_str()).unwrap();
            let path = match ResourcePath::parse(member_path) {
                Ok(p) => p,
                Err(_) => {
                    duplicate_found = true;
                    break;
                }
            };
            let content = match std::str::from_utf8(&obs.bytes) {
                Ok(text) => ResourceContent::Text(text.to_owned()),
                Err(_) => ResourceContent::Bytes(obs.bytes.clone()),
            };
            if resources
                .insert(
                    path.clone(),
                    Resource::opaque(path, content, obs.executable),
                )
                .is_some()
            {
                duplicate_found = true;
                break;
            }
        }

        if duplicate_found {
            for (index, _) in members {
                dispositions[index] = Some(NativeObservationDisposition::UnrecognizedWarning);
            }
            continue;
        }

        let package = Package::new(
            PackageKind::Unsupported,
            SemanticIdentity::parse(format!(
                "unsupported:agent-plugin-extension/{extension_dir}"
            ))?,
            provenance(primary_obs)?,
            PackageRoot::parse(extension_dir)?,
            SemanticItem::Unsupported {
                native_kind: "extension".to_owned(),
            },
            resources,
            None,
        )?;

        packages.push(package);
        for (index, _) in members {
            dispositions[index] = Some(NativeObservationDisposition::RetainedUnsupportedContent);
        }
    }

    NativeCompilation::new_with_diagnostics(
        NativeFrontend::AgentPlugin,
        observations,
        packages,
        dispositions
            .into_iter()
            .collect::<Option<Vec<NativeObservationDisposition>>>()
            .expect("AgentPlugin parser classifies every observation"),
        extra_diagnostics,
    )
}

fn observation_key(observation: &ArtifactObservation) -> (&str, &str) {
    (
        observation.source_path.as_str(),
        observation.provenance.input_label.as_str(),
    )
}

fn provenance(observation: &ArtifactObservation) -> Result<SourceProvenance> {
    let mut provenance =
        SourceProvenance::new("agent-plugin", &observation.provenance.input_label)?;
    provenance.archive_member = observation.provenance.archive_member.clone();
    Ok(provenance)
}

fn split_frontmatter(input: &str) -> (Option<&str>, &str) {
    let Some(remainder) = input
        .strip_prefix("---\n")
        .or_else(|| input.strip_prefix("---\r\n"))
    else {
        return (None, input);
    };
    let Some(end) = remainder.find("---") else {
        return (None, input);
    };
    let body = &remainder[end + 3..];
    let body = body
        .strip_prefix("\r\n")
        .or_else(|| body.strip_prefix('\n'))
        .unwrap_or(body);
    (Some(remainder[..end].trim()), body)
}

pub(crate) fn is_reverse_domain(dir: &str) -> bool {
    let mut segments = dir.split('.');
    let Some(first) = segments.next() else {
        return false;
    };
    if first.is_empty() || !is_domain_segment(first) {
        return false;
    }
    let mut count = 1;
    for segment in segments {
        if segment.is_empty() || !is_domain_segment(segment) {
            return false;
        }
        count += 1;
    }
    count >= 2
}

fn is_domain_segment(segment: &str) -> bool {
    segment
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn extension_dir_and_member(path: &str) -> Option<(&str, &str)> {
    let (dir, member) = path.split_once('/')?;
    if !dir.is_empty() && !member.is_empty() && is_reverse_domain(dir) {
        Some((dir, member))
    } else {
        None
    }
}

fn skill_dir_and_member(path: &str) -> Option<(&str, &str)> {
    let rest = path.strip_prefix("skills/")?;
    let (dir, member) = rest.split_once('/')?;
    if !dir.is_empty() && !member.is_empty() {
        Some((dir, member))
    } else {
        None
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct SkillGroupKey {
    skill_dir: String,
    input_label: String,
}

impl SkillGroupKey {
    fn from_observation(skill_dir: &str, observation: &ArtifactObservation) -> Self {
        Self {
            skill_dir: skill_dir.to_owned(),
            input_label: observation.provenance.input_label.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ExtensionGroupKey {
    extension_dir: String,
    input_label: String,
}

impl ExtensionGroupKey {
    fn from_observation(extension_dir: &str, observation: &ArtifactObservation) -> Self {
        Self {
            extension_dir: extension_dir.to_owned(),
            input_label: observation.provenance.input_label.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inputs::{ArtifactObservation, InputOrigin};
    use crate::{CompilationGraph, ResourceRole};

    fn observation(path: &str, bytes: impl Into<Vec<u8>>, executable: bool) -> ArtifactObservation {
        observation_with_label(path, bytes, executable, "fixtures/agent-plugin")
    }

    fn observation_with_label(
        path: &str,
        bytes: impl Into<Vec<u8>>,
        executable: bool,
        input_label: &str,
    ) -> ArtifactObservation {
        ArtifactObservation::new(
            bytes.into(),
            path,
            executable,
            InputOrigin::Filesystem,
            input_label,
            None,
        )
        .expect("test observation is valid")
    }

    fn compile_agent_plugin_graph(inputs: &[ArtifactObservation]) -> Result<CompilationGraph> {
        compile_native(inputs)?.into_graph()
    }

    #[test]
    fn parses_valid_agent_plugin_package() {
        let manifest = serde_json::json!({
            "$schema": PLUGIN_MANIFEST_SCHEMA_V1,
            "name": "sample-plugin",
            "version": "1.0.0",
            "description": "A sample plugin"
        });
        let mcp = serde_json::json!({
            "$schema": MCP_CONFIG_SCHEMA_V1,
            "mcpServers": {
                "fetch": {
                    "type": "stdio",
                    "command": "./bin/fetch-server"
                }
            }
        });
        let skill = "---\nname: my-skill\ndescription: Test skill description.\n---\n# My Skill\n";

        let graph = compile_agent_plugin_graph(&[
            observation("plugin.json", serde_json::to_vec(&manifest).unwrap(), false),
            observation("mcp.json", serde_json::to_vec(&mcp).unwrap(), false),
            observation("skills/my-skill/SKILL.md", skill.as_bytes(), false),
            observation("skills/my-skill/scripts/run.sh", b"echo hi", true),
            observation("com.example.ext/config.json", b"{}", false),
            observation("notes.txt", b"random notes", false),
        ])
        .unwrap();

        assert_eq!(graph.packages.len(), 4);

        let manifest_pkg = graph
            .packages
            .values()
            .find(|p| {
                p.semantic_identity.as_str() == "unsupported:agent-plugin-manifest/plugin.json"
            })
            .unwrap();
        assert_eq!(manifest_pkg.kind, PackageKind::Unsupported);

        let mcp_pkg = graph
            .packages
            .values()
            .find(|p| p.semantic_identity.as_str() == "unsupported:agent-plugin-mcp-server/fetch")
            .unwrap();
        assert_eq!(mcp_pkg.kind, PackageKind::Unsupported);

        let skill_pkg = graph
            .packages
            .values()
            .find(|p| p.semantic_identity.as_str() == "skill:my-skill")
            .unwrap();
        assert_eq!(skill_pkg.kind, PackageKind::Skill);
        assert_eq!(skill_pkg.resources.len(), 2);
        let primary = skill_pkg
            .resources
            .get(&ResourcePath::parse("SKILL.md").unwrap())
            .unwrap();
        assert_eq!(primary.role, ResourceRole::PrimaryInstruction);

        let ext_pkg = graph
            .packages
            .values()
            .find(|p| {
                p.semantic_identity.as_str() == "unsupported:agent-plugin-extension/com.example.ext"
            })
            .unwrap();
        assert_eq!(ext_pkg.kind, PackageKind::Unsupported);
    }

    #[test]
    fn emits_warning_for_unknown_manifest_fields() {
        let manifest = serde_json::json!({
            "$schema": PLUGIN_MANIFEST_SCHEMA_V1,
            "name": "sample-plugin",
            "unknown_field": 123
        });
        let graph = compile_agent_plugin_graph(&[observation(
            "plugin.json",
            serde_json::to_vec(&manifest).unwrap(),
            false,
        )])
        .unwrap();

        assert_eq!(graph.packages.len(), 1);
        let warnings: Vec<_> = graph
            .diagnostics
            .iter()
            .filter(|d| d.code == "unknown-manifest-field")
            .collect();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("unknown_field"));
    }

    #[test]
    fn emits_warning_for_invalid_mcp_server_entry_without_failing_valid_sibling() {
        let mcp = serde_json::json!({
            "$schema": MCP_CONFIG_SCHEMA_V1,
            "mcpServers": {
                "valid-server": {
                    "type": "stdio",
                    "command": "./bin/server"
                },
                "escaping-server": {
                    "type": "stdio",
                    "command": "../escape"
                }
            }
        });
        let manifest = serde_json::json!({
            "$schema": PLUGIN_MANIFEST_SCHEMA_V1,
            "name": "test-plugin"
        });

        let graph = compile_agent_plugin_graph(&[
            observation("plugin.json", serde_json::to_vec(&manifest).unwrap(), false),
            observation("mcp.json", serde_json::to_vec(&mcp).unwrap(), false),
        ])
        .unwrap();

        assert!(graph
            .packages
            .values()
            .any(|p| p.semantic_identity.as_str()
                == "unsupported:agent-plugin-mcp-server/valid-server"));
        assert!(
            !graph.packages.values().any(|p| p.semantic_identity.as_str()
                == "unsupported:agent-plugin-mcp-server/escaping-server")
        );

        let warnings: Vec<_> = graph
            .diagnostics
            .iter()
            .filter(|d| d.code == "invalid-mcp-server")
            .collect();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("escaping-server"));
    }

    #[test]
    fn skips_non_conforming_skills_with_warning() {
        let manifest = serde_json::json!({
            "$schema": PLUGIN_MANIFEST_SCHEMA_V1,
            "name": "test-plugin"
        });
        let valid_skill = "---\nname: valid-skill\ndescription: A valid skill.\n---\n# Valid\n";
        let invalid_skill = "---\nname: invalid-skill\n---\n# Missing description\n";

        let graph = compile_agent_plugin_graph(&[
            observation("plugin.json", serde_json::to_vec(&manifest).unwrap(), false),
            observation("skills/valid-skill/SKILL.md", valid_skill.as_bytes(), false),
            observation(
                "skills/invalid-skill/SKILL.md",
                invalid_skill.as_bytes(),
                false,
            ),
        ])
        .unwrap();

        assert!(graph
            .packages
            .values()
            .any(|p| p.semantic_identity.as_str() == "skill:valid-skill"));
        assert!(!graph
            .packages
            .values()
            .any(|p| p.semantic_identity.as_str() == "skill:invalid-skill"));

        let warnings: Vec<_> = graph
            .diagnostics
            .iter()
            .filter(|d| d.code == "invalid-skill")
            .collect();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("invalid-skill"));
    }
}
