use clap::Args;

#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Policy file (TOML) defining additional constraints
    #[arg(long)]
    pub policy: Option<String>,

    /// Treat warnings as errors
    #[arg(long)]
    pub strict: bool,
}

impl ValidateArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        println!("Executing 'validate' command:");
        println!("  Input: {:?}", self.input);
        println!("  Policy: {:?}", self.policy);
        println!("  Strict: {}", self.strict);
        Ok(())
    }
}
