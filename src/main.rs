use clap::{Parser, ValueEnum};
use anyhow::Result;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// input file path, or stdin if omitted
    input: Option<std::path::PathBuf>,

    /// output format
    #[arg(short, long, value_enum, default_value_t = Format::Gemini)]
    format: Format,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Format {
    Gemini,
    Claude,
    Cursor,
}

fn main() -> Result<()> {
    let _args = Args::parse();
    // cli logic will follow in implementation phase
    Ok(())
}
