use clap::CommandFactory;
use clap_markdown::help_markdown_command;
use rulette::cli::Cli;

fn main() {
    let markdown = help_markdown_command(&Cli::command());
    println!("{}", markdown);
}
