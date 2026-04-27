use anyhow::Result;
use clap::Args;
use std::fs;
use std::path::Path;

#[derive(Args, Debug)]
pub struct InitArgs {
    /// Directory to initialize (defaults to current directory)
    #[arg(default_value = ".")]
    pub path: String,

    /// Force initialization even if directory is not empty
    #[arg(short, long)]
    pub force: bool,
}

impl InitArgs {
    pub fn execute(&self) -> Result<()> {
        let base_path = Path::new(&self.path);

        if !base_path.exists() {
            fs::create_dir_all(base_path)?;
        }

        let rules_dir = base_path.join("rules");
        if !rules_dir.exists() {
            fs::create_dir(&rules_dir)?;
            println!("Created directory: {}", rules_dir.display());
        }

        let example_rule = rules_dir.join("example.md");
        if !example_rule.exists() || self.force {
            let content = r#"---
name: example-rule
description: An example rule created by rulette init
---
# Example Rule
This is a sample rule. Rulette will parse the frontmatter and use it to generate tool-specific configurations.
"#;
            fs::write(&example_rule, content)?;
            println!("Created example rule: {}", example_rule.display());
        }

        let config_file = base_path.join("RULETTE.toml");
        if !config_file.exists() || self.force {
            let content = r#"# Rulette configuration file
# Use this to define common transform pipelines

[transform]
dedup = true
on-conflict = "error"

# Example: Define a default target format
# to = "claude"
# out = [ "claude:.claude" ]
"#;
            fs::write(&config_file, content)?;
            println!("Created config file: {}", config_file.display());
        }

        println!("\nRulette project initialized successfully!");
        println!("Try running: rulette transform rules/ -o claude:.claude/");

        Ok(())
    }
}
