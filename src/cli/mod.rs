pub mod commands;
pub mod formats;
pub mod globals;
pub mod io;

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

    /// Output JSON Schema for the IR or a specific target format
    Schema(commands::schema::SchemaArgs),

    /// Parse, transform, and emit rules across formats
    Transform(Box<commands::transform::TransformArgs>),
}
