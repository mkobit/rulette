use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "rulette",
    author,
    version,
    about = "Stateless CLI tool for transforming AI rules and skills across systems"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add/Fetch a skill package and output it
    Add {
        /// The skill package to fetch (e.g., vercel-labs/agent-skills)
        package: String,

        /// Output file path (writes to stdout if not provided)
        #[arg(short, long)]
        out: Option<PathBuf>,

        /// Transform to target format
        #[arg(short, long, value_enum)]
        format: Option<Format>,
    },

    /// List available skills
    List {
        /// Directory or remote package to list skills from
        source: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Initialize a new skill
    Init {
        /// Name of the skill to create
        name: Option<String>,

        /// Output file path (writes to stdout if not provided)
        #[arg(short, long)]
        out: Option<PathBuf>,
    },

    /// Transform an existing skill
    Transform {
        /// Input file path (reads from stdin if not provided)
        input: Option<PathBuf>,

        /// Output file path (writes to stdout if not provided)
        #[arg(short, long)]
        out: Option<PathBuf>,

        /// Target output format
        #[arg(short, long, value_enum, default_value_t = Format::Ir)]
        format: Format,
    },

    /// Inspect a skill and output its internal representation (IR)
    Inspect {
        /// Input file path (reads from stdin if not provided)
        input: Option<PathBuf>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum Format {
    Gemini,
    Claude,
    Cursor,
    Codex,
    Ir,
}
