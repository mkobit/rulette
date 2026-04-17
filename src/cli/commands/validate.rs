use crate::cli::formats::InputFormat;
use crate::frontend::parse;
use crate::Entity;
use clap::Args;
use std::fs;
use std::io::{self, Read};

#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Input files or directories (or "-" for stdin)
    #[arg(default_value = "-")]
    pub input: Vec<String>,

    /// Policy file (TOML) defining additional constraints
    #[arg(long)]
    pub policy: Option<String>,

    /// Treat warnings as errors
    #[arg(long)]
    pub strict: bool,
}

impl ValidateArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        if self.policy.is_some() {
            anyhow::bail!("Policy validation is not yet implemented");
        }

        let mut combined_entities = vec![];

        for input_path in &self.input {
            let content = if input_path == "-" {
                let mut buffer = String::new();
                io::stdin().read_to_string(&mut buffer)?;
                buffer
            } else {
                fs::read_to_string(input_path)?
            };

            let filename = if input_path == "-" {
                None
            } else {
                Some(input_path.as_str())
            };
            let doc = parse(&content, InputFormat::Auto, filename)?;
            combined_entities.extend(doc.entities);
        }

        let mut has_errors = false;

        for entity in combined_entities {
            if let Entity::Skill(skill) = entity {
                if let Err(e) = skill.metadata.validate() {
                    eprintln!(
                        "Validation error for skill '{}': {}",
                        skill.metadata.name, e
                    );
                    has_errors = true;
                }
            }
        }

        if has_errors {
            anyhow::bail!("Validation failed");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_validate_valid_skill() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "---\nname: valid-skill\ndescription: valid description\n---\nbody"
        )
        .unwrap();

        let args = ValidateArgs {
            input: vec![file.path().to_str().unwrap().to_string()],
            policy: None,
            strict: false,
        };

        assert!(args.execute().is_ok());
    }

    #[test]
    fn test_validate_invalid_skill() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            "---\nname: invalid--skill\ndescription: valid description\n---\nbody"
        )
        .unwrap();

        let args = ValidateArgs {
            input: vec![file.path().to_str().unwrap().to_string()],
            policy: None,
            strict: false,
        };

        let result = args.execute();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Validation failed");
    }

    #[test]
    fn test_validate_unimplemented_policy() {
        let args = ValidateArgs {
            input: vec!["-".to_string()],
            policy: Some("policy.toml".to_string()),
            strict: false,
        };

        let result = args.execute();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Policy validation is not yet implemented"
        );
    }
}
