use crate::cli::formats::OutputFormat;
use clap::Args;

#[derive(Args, Debug)]
pub struct EmitArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Target output format
    #[arg(short, long, value_enum)]
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

    /// Split into one file per rule (default for directory output)
    #[arg(long)]
    pub split: bool,
}

impl EmitArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        println!("Executing 'emit' command:");
        println!("  Input: {:?}", self.input);
        println!("  To: {:?}", self.to);
        println!("  Out: {:?}", self.out);
        println!("  Scope: {}", self.scope);
        println!("  Merge: {}", self.merge);
        println!("  Split: {}", self.split);
        Ok(())
    }
}
