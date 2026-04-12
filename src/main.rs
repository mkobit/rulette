use anyhow::Result;
use clap::Parser;
use rulette::cli::{Cli, Commands};

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Parse(args) => args.execute(),
        Commands::Emit(args) => args.execute(),
        Commands::Convert(args) => args.execute(),
        Commands::Inspect(args) => args.execute(),
        Commands::Schema(args) => args.execute(),
        Commands::Transform(args) => args.execute(),
        Commands::Validate(args) => args.execute(),
        Commands::Fetch(args) => args.execute(),
        Commands::Lock(args) => args.execute(),
        Commands::Verify(args) => args.execute(),
        Commands::Archive(args) => args.execute(),
        Commands::Unarchive(args) => args.execute(),
    }
}
