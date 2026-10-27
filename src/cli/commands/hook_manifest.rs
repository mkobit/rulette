use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use serde_json::json;

use crate::emitters::hooks::{
    build_hook_command, emit_polyglot_hook_manifests, generate_hook_manifest, write_hook_manifest,
    write_polyglot_hook_manifests, HookManifestTarget,
};

/// Arguments for `rulette hook-manifest`.
#[derive(Args, Debug, Clone)]
pub struct HookManifestArgs {
    /// Path to the permission policy JSON file
    #[arg(long, value_name = "PATH", default_value = ".rulette/policy.json")]
    pub policy: PathBuf,

    /// Target harness format (antigravity, cursor, claude, codex).
    /// Emits all manifests if omitted.
    #[arg(long, value_name = "TARGET")]
    pub target: Option<String>,

    /// Destination root directory to emit manifests to
    #[arg(long, value_name = "DIR", default_value = ".")]
    pub output_dir: PathBuf,

    /// Force Windows invocation normalization (rulette.exe)
    #[arg(long)]
    pub windows: bool,

    /// Print manifests to standard output instead of writing files
    #[arg(long)]
    pub dry_run: bool,
}

impl HookManifestArgs {
    pub fn execute(&self) -> Result<()> {
        let is_windows = self.windows || cfg!(windows);
        let policy_str = self.policy.to_string_lossy();

        if let Some(target_str) = &self.target {
            let target = HookManifestTarget::parse(target_str).with_context(|| {
                format!(
                    "Invalid target `{target_str}`. Must be one of: antigravity, cursor, claude, codex"
                )
            })?;

            if self.dry_run {
                let cmd = build_hook_command(&policy_str, is_windows);
                let manifest = generate_hook_manifest(target, &cmd);
                let output = json!({
                    "target": target.as_str(),
                    "path": target.manifest_path(),
                    "manifest": manifest,
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                let written =
                    write_hook_manifest(&self.output_dir, target, &policy_str, is_windows)?;
                println!(
                    "Emitted hook manifest for {} at {:?}",
                    target.as_str(),
                    written
                );
            }
        } else if self.dry_run {
            let manifests = emit_polyglot_hook_manifests(&policy_str, is_windows);
            let items: Vec<_> = manifests
                .into_iter()
                .map(|m| {
                    json!({
                        "target": m.target.as_str(),
                        "path": m.path,
                        "manifest": m.content,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&items)?);
        } else {
            let written = write_polyglot_hook_manifests(&self.output_dir, &policy_str, is_windows)?;
            for path in written {
                println!("Emitted hook manifest at {:?}", path);
            }
        }

        Ok(())
    }
}
