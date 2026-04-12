use clap::Args;

#[derive(Args, Debug)]
pub struct GlobalFlags {
    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Fail on warnings (including lossy conversion warnings)
    #[arg(long, global = true)]
    pub strict: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Log verbosity (error, warn, info, debug, trace)
    #[arg(long, global = true)]
    pub log_level: Option<String>,
}
