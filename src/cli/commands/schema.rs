use clap::Args;
use schemars::schema_for;

#[derive(Args, Debug)]
pub struct SchemaArgs {
    /// Schema contract to output (`graph` or `hook`).
    #[arg(short, long, default_value = "graph")]
    pub to: String,
}

impl SchemaArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        let schema = match self.to.as_str() {
            "graph" => schema_for!(crate::CompilationGraph),
            "hook" | "hooks" => schema_for!(crate::TargetHookPolicy),
            _ => anyhow::bail!(
                "unsupported schema contract `{}`; expected graph or hook",
                self.to
            ),
        };

        let schema_json = serde_json::to_string_pretty(&schema)?;
        println!("{}", schema_json);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::SchemaArgs;

    #[test]
    fn rejects_legacy_document_and_extension_schemas() {
        assert!(SchemaArgs {
            to: "ir".to_owned(),
        }
        .execute()
        .is_err());
    }

    #[test]
    fn outputs_graph_and_hook_schemas() {
        assert!(SchemaArgs {
            to: "graph".to_owned(),
        }
        .execute()
        .is_ok());

        assert!(SchemaArgs {
            to: "hook".to_owned(),
        }
        .execute()
        .is_ok());
    }
}
