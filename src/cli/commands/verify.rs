use clap::Args;

#[derive(Args, Debug)]
pub struct VerifyArgs {
    /// Lockfile to verify
    #[arg(default_value = "rules.lock")]
    pub lockfile: String,

    /// Vendor directory to verify
    #[arg(long, default_value = "vendor/rules/")]
    pub vendor: String,
}

impl VerifyArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        println!("Executing 'verify' command:");
        println!("  Lockfile: {}", self.lockfile);
        println!("  Vendor: {}", self.vendor);
        Ok(())
    }
}
