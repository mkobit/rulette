use clap::Args;
use anyhow::Result;
use ureq;
use std::fs;
#[derive(Args, Debug)]
pub struct FetchArgs {
    /// Remote source URL to fetch rules or skills from
    pub source: String,

    /// Allow mutable fetched content without verification
    #[arg(long)]
    pub allow_mutable: bool,

    /// Disable integrity verification (must be used with --allow-mutable)
    #[arg(long)]
    pub no_verify: bool,

    /// Lockfile to verify against (default: rules.lock)
    #[arg(long, default_value = "rules.lock")]
    pub lockfile: String,

    /// Output path (file) to save the fetched content (or "-" for stdout)
    #[arg(short, long, default_value = "-")]
    pub out: String,
}

impl FetchArgs {
    pub fn execute(&self) -> Result<()> {
        if !self.allow_mutable && self.no_verify {
            anyhow::bail!("--no-verify must be used with --allow-mutable");
        }

        let mut response = ureq::get(&self.source).call()?;
        let content = response.body_mut().read_to_string()?;

        if self.out == "-" {
            print!("{}", content);
        } else {
            fs::write(&self.out, content)?;
        }

        Ok(())
    }
}
