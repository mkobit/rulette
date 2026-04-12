use clap::Args;

#[derive(Args, Debug)]
pub struct InspectArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,
}

impl InspectArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        println!("Executing 'inspect' command:");
        println!("  Input: {:?}", self.input);
        Ok(())
    }
}
