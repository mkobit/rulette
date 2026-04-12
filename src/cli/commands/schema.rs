use clap::Args;

#[derive(Args, Debug)]
pub struct SchemaArgs {
    /// Format to output schema for (ir, claude, cursor-mdc, etc.)
    #[arg(default_value = "ir")]
    pub format: String,
}

impl SchemaArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        println!("Executing 'schema' command:");
        println!("  Format: {}", self.format);
        Ok(())
    }
}
