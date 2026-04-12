use crate::cli::formats::{InputFormat, OutputFormat};
use clap::Args;

#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Source format (auto-detected if omitted)
    #[arg(long, value_enum, default_value_t = InputFormat::Auto)]
    pub from: InputFormat,

    /// Target output format
    #[arg(long, value_enum)]
    pub to: OutputFormat,

    /// Output path (file or directory)
    #[arg(short, long)]
    pub out: Option<String>,

    /// Output scope: project (default) or user
    #[arg(long, default_value = "project")]
    pub scope: String,

    /// Merge multiple rules into a single output file
    #[arg(long)]
    pub merge: bool,
}

impl ConvertArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        println!("Executing 'convert' command:");
        println!("  Input: {:?}", self.input);
        println!("  From: {:?}", self.from);
        println!("  To: {:?}", self.to);
        println!("  Out: {:?}", self.out);
        println!("  Scope: {}", self.scope);
        println!("  Merge: {}", self.merge);
        Ok(())
    }
}
