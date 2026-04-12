use clap::Args;

#[derive(Args, Debug)]
pub struct ArchiveArgs {
    /// Input files or directories to archive
    pub input: Vec<String>,

    /// Output archive path
    #[arg(short, long)]
    pub out: Option<String>,

    /// Compression (none, gzip, zstd; default: gzip)
    #[arg(long, default_value = "gzip")]
    pub compress: String,
}

impl ArchiveArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        println!("Executing 'archive' command:");
        println!("  Input: {:?}", self.input);
        println!("  Out: {:?}", self.out);
        println!("  Compress: {}", self.compress);
        Ok(())
    }
}
