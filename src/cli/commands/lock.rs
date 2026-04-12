use clap::Args;

#[derive(Args, Debug)]
pub struct LockArgs {
    /// Manifest file (rulette.toml)
    pub manifest: Option<String>,

    /// Lockfile output path (default: rules.lock)
    #[arg(short, long, default_value = "rules.lock")]
    pub out: String,

    /// Update only the named package
    #[arg(long)]
    pub update: Option<String>,
}

impl LockArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        println!("Executing 'lock' command:");
        println!("  Manifest: {:?}", self.manifest);
        println!("  Out: {}", self.out);
        println!("  Update: {:?}", self.update);
        Ok(())
    }
}
