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
    /// Parse one or more input files (or stdin) into the Rulette IR
    Parse(commands::parse::ParseArgs),

    /// Emit IR (from stdin or files) to a target format
    Emit(commands::emit::EmitArgs),

    /// Parse input and emit to a target format in one step
    Convert(commands::convert::ConvertArgs),

    /// Pretty-print the IR for debugging
    Inspect(commands::inspect::InspectArgs),

    /// Output JSON Schema for the IR or a specific target format
    Schema(commands::schema::SchemaArgs),

    /// Apply transformations to IR (v0.1.1)
    Transform(commands::transform::TransformArgs),

    /// Fetch rules from a remote source (v0.2)
    Fetch(commands::fetch::FetchArgs),

    /// Generate or update a lockfile from a manifest (v0.2)
    Lock(commands::lock::LockArgs),

    /// Verify that fetched content matches the lockfile (v0.2)
    Verify(commands::verify::VerifyArgs),

    /// Bundle rules into a content-addressed tar archive (v0.2)
    Archive(commands::archive::ArchiveArgs),

    /// Extract and verify a content-addressed archive (v0.2)
    Unarchive(commands::unarchive::UnarchiveArgs),
}
