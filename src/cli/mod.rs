pub mod commands;
pub mod formats;
pub mod globals;

use clap::{Parser, Subcommand};
use globals::GlobalFlags;

#[derive(Parser, Debug)]
#[command(
    name = "rulette",
    author,
    version,
    about = "Stateless CLI tool for transforming AI rules and skills across systems"
)]
pub struct Cli {
    #[command(flatten)]
    pub globals: GlobalFlags,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Pretty-print the IR for debugging
    Inspect(commands::inspect::InspectArgs),

    /// Evaluate incoming PreToolUse hook payloads against a permission policy
    #[command(name = "hook-eval")]
    HookEval(commands::hook_eval::HookEvalArgs),

    /// Emit polyglot PreToolUse hook manifests for Antigravity, Cursor, Claude, and Codex
    #[command(name = "hook-manifest")]
    HookManifest(commands::hook_manifest::HookManifestArgs),

    /// Output the JSON Schema for the compilation graph
    Schema(commands::schema::SchemaArgs),

    /// Compile graphs and stage or apply native publication plans
    Transform(Box<commands::transform::TransformArgs>),
}
