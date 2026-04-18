use anyhow::Result;
use clap::Parser;
use rulette::cli::{Cli, Commands};

fn main() -> Result<()> {
    let args = Cli::parse();

    let strict = args.globals.strict;

    match args.command {
        Commands::Parse(args) => args.execute(),
        Commands::Emit(args) => args.execute(strict),
        Commands::Convert(args) => args.execute(strict),
        Commands::Inspect(args) => args.execute(strict),
        Commands::Schema(args) => args.execute(),
        Commands::Transform(args) => args.execute(),
        Commands::Validate(args) => args.execute(),
        Commands::Archive(args) => args.execute(),
        Commands::Unarchive(args) => args.execute(),
    }
}
