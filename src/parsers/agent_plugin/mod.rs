pub mod model;
pub mod wire;

pub use model::{McpConfiguration, McpServer, McpTransport, PluginAuthor, PluginManifest};
pub use wire::{
    McpConfigWireV1, McpServerWireV1, PluginAuthorWireV1, PluginManifestWireV1,
    MCP_CONFIG_SCHEMA_V1, PLUGIN_MANIFEST_SCHEMA_V1,
};

pub(crate) fn compile_native(
    observations: &[crate::inputs::ArtifactObservation],
) -> anyhow::Result<crate::parsers::frontend::NativeCompilation> {
    crate::parsers::frontend::NativeCompilation::new(
        crate::parsers::frontend::NativeFrontend::AgentPlugin,
        observations,
        Vec::new(),
        vec![
            crate::parsers::frontend::NativeObservationDisposition::UnrecognizedWarning;
            observations.len()
        ],
    )
}
