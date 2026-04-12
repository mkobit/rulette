use crate::cli::formats::InputFormat;
use clap::Args;

#[derive(Args, Debug)]
pub struct ParseArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Force input format detection
    #[arg(long, value_enum, default_value_t = InputFormat::Auto)]
    pub from: InputFormat,

    /// Write output to file instead of stdout
    #[arg(short, long)]
    pub out: Option<String>,

    /// Fail on parse warnings
    #[arg(long)]
    pub strict: bool,
}

impl ParseArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        println!("Executing 'parse' command:");
        println!("  Input: {:?}", self.input);
        println!("  From: {:?}", self.from);
        println!("  Out: {:?}", self.out);
        println!("  Strict: {}", self.strict);
        Ok(())
    }
}
