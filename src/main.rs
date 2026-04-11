use anyhow::Result;
use clap::Parser;
use rulette::cli::{Cli, Commands};

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Transform {
            input,
            out,
            from,
            to,
            jq,
        } => {
            println!("Transforming input: {}", input);
            println!("Output path: {}", out);
            println!("From format: {:?}", from);
            println!("To format: {:?}", to);
            if let Some(j) = jq {
                println!("JQ filter: {}", j);
            }
        }
        Commands::Inspect { input, from, jq } => {
            println!("Inspecting input: {}", input);
            println!("From format: {:?}", from);
            if let Some(j) = jq {
                println!("JQ filter: {}", j);
            }
        }
    }

    Ok(())
}
