use anyhow::Result;
use clap::Parser;
use rulette::cli::{Cli, Commands};

fn main() -> Result<()> {
    let args = Cli::parse();

    match args.command {
        Commands::Add {
            package,
            out,
            format,
        } => {
            println!("Adding package: {}", package);
            if let Some(o) = out {
                println!("Output path: {:?}", o);
            }
            if let Some(f) = format {
                println!("Format: {:?}", f);
            }
        }
        Commands::List { source, json } => {
            if let Some(s) = source {
                println!("Listing skills from: {}", s);
            } else {
                println!("Listing local skills");
            }
            if json {
                println!("Outputting as JSON");
            }
        }
        Commands::Init { name, out } => {
            if let Some(n) = name {
                println!("Initializing skill: {}", n);
            } else {
                println!("Initializing skill");
            }
            if let Some(o) = out {
                println!("Output path: {:?}", o);
            }
        }
        Commands::Transform { input, out, format } => {
            if let Some(i) = input {
                println!("Transforming input file: {:?}", i);
            } else {
                println!("Transforming from stdin");
            }
            if let Some(o) = out {
                println!("Output path: {:?}", o);
            }
            println!("Target format: {:?}", format);
        }
        Commands::Inspect { input } => {
            if let Some(i) = input {
                println!("Inspecting file: {:?}", i);
            } else {
                println!("Inspecting from stdin");
            }
        }
    }

    Ok(())
}
