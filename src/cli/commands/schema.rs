use clap::Args;
use schemars::schema_for;

#[derive(Args, Debug)]
pub struct SchemaArgs {
    /// Schema contract to output (only `graph` is supported).
    #[arg(short, long, default_value = "graph")]
    pub to: String,
}

impl SchemaArgs {
    pub fn execute(&self) -> anyhow::Result<()> {
        let schema = match self.to.as_str() {
            "graph" => schema_for!(crate::CompilationGraph),
            _ => anyhow::bail!("unsupported schema contract `{}`; expected graph", self.to),
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
}
