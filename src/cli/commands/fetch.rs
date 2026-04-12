use clap::Args;

#[derive(Args, Debug)]
pub struct FetchArgs {
    /// Source to fetch rules from
    pub source: String,

    /// Lockfile to verify against (default: rules.lock)
    #[arg(long, default_value = "rules.lock")]
    pub lockfile: String,

    /// Allow fetching without pinned version
    #[arg(long)]
    pub allow_mutable: bool,

    /// Skip integrity verification (requires --allow-mutable)
    #[arg(long)]
    pub no_verify: bool,

    /// Output path
    #[arg(short, long)]
    pub out: Option<String>,
}

impl FetchArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        println!("Executing 'fetch' command:");
        println!("  Source: {}", self.source);
        println!("  Lockfile: {}", self.lockfile);
        println!("  Allow Mutable: {}", self.allow_mutable);
        println!("  No Verify: {}", self.no_verify);
        println!("  Out: {:?}", self.out);
        Ok(())
    }
}
