use clap::Args;

#[derive(Args, Debug)]
pub struct UnarchiveArgs {
    /// Archive file to extract
    pub archive: String,

    /// Extraction directory
    #[arg(short, long)]
    pub out: Option<String>,
}

impl UnarchiveArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        println!("Executing 'unarchive' command:");
        println!("  Archive: {}", self.archive);
        println!("  Out: {:?}", self.out);
        Ok(())
    }
}
