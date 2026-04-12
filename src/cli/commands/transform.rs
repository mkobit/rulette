use clap::Args;

#[derive(Args, Debug)]
pub struct TransformArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Keep only rules matching expression
    #[arg(long)]
    pub filter: Option<String>,

    /// Remove rules matching expression
    #[arg(long)]
    pub exclude: Option<String>,

    /// Rename a metadata field value (from=to)
    #[arg(long)]
    pub rename: Option<String>,

    /// Set a metadata field (field=value)
    #[arg(long)]
    pub set: Option<String>,

    /// Load transform pipeline from TOML file
    #[arg(long)]
    pub config: Option<String>,

    /// Pipe each rule body through a shell command
    #[arg(long)]
    pub shell: Option<String>,
}

impl TransformArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        println!("Executing 'transform' command:");
        println!("  Input: {:?}", self.input);
        println!("  Filter: {:?}", self.filter);
        println!("  Exclude: {:?}", self.exclude);
        println!("  Rename: {:?}", self.rename);
        println!("  Set: {:?}", self.set);
        println!("  Config: {:?}", self.config);
        println!("  Shell: {:?}", self.shell);
        Ok(())
    }
}
