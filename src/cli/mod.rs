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

    /// Output the JSON Schema for the compilation graph
    Schema(commands::schema::SchemaArgs),

    /// Compile graphs and stage or apply native publication plans
    Transform(Box<commands::transform::TransformArgs>),
}
