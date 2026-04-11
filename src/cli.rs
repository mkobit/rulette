use clap::{Parser, Subcommand, ValueEnum};

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
    /// Transform skills from one format to another
    Transform {
        /// Input source (file, directory, URL, archive, or "-" for stdin)
        #[arg(default_value = "-")]
        input: String,

        /// Output destination (file path, directory, or "-" for stdout)
        #[arg(short, long, default_value = "-")]
        out: String,

        /// Input format (auto-detected if not specified)
        #[arg(short, long, value_enum, default_value_t = InputFormat::Auto)]
        from: InputFormat,

        /// Target output format
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Ir)]
        to: OutputFormat,

        /// Apply a jq filter to the internal representation before outputting
        #[arg(long)]
        jq: Option<String>,
    },

    /// Inspect a skill source and output its internal representation (IR)
    Inspect {
        /// Input source (file, directory, URL, archive, or "-" for stdin)
        #[arg(default_value = "-")]
        input: String,

        /// Input format (auto-detected if not specified)
        #[arg(short, long, value_enum, default_value_t = InputFormat::Auto)]
        from: InputFormat,

        /// Apply a jq filter to the internal representation before outputting
        #[arg(long)]
        jq: Option<String>,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum InputFormat {
    Auto,
    Gemini,
    Claude,
    Cursor,
    Codex,
    AgentSkills,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
pub enum OutputFormat {
    Gemini,
    Claude,
    Cursor,
    Codex,
    AgentSkills,
    Ir,
}
