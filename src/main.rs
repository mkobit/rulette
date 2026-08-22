use anyhow::Result;
use clap::Parser;
use rulette::cli::{Cli, Commands};

use tracing_subscriber::EnvFilter;

fn init_tracing(log_level: &Option<String>) {
    let level = log_level.as_deref().unwrap_or("warn");

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

fn main() -> Result<()> {
    let args = Cli::parse();

    init_tracing(&args.globals.log_level);
    let quiet = args.globals.quiet;

    match args.command {
        Commands::Inspect(args) => args.execute(quiet),
        Commands::Schema(args) => args.execute(),
        Commands::Transform(args) => args.execute(quiet),
    }
}
