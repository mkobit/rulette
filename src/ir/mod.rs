use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod graph;

/// A portable rule-activation mode retained by the compilation graph.
#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ActivationMode {
    Always,
    Glob,
    Pattern,
    Manual,
    Model,
}
