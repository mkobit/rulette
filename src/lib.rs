pub mod cli;
pub mod compilation;
pub mod emitters;
pub mod inputs;
pub mod ir;
pub mod parsers;
pub mod pipeline;
pub mod publication;

pub use compilation::{compile, lower_unique_targets, CompilationRequest};
pub use emitters::lowering::{
    lower, CapabilityFinding, CapabilityReasonCode, CapabilitySeverity, LoweringOptions,
    LoweringPlan, NativeArtifact, NativeArtifactClass, NativeTarget,
};
pub use ir::graph::{
    CompilationGraph, DiagnosticSeverity, FrontendPayload, GraphDiagnostic, Package, PackageId,
    PackageKind, PackageRoot, PortableActivation, Resource, ResourceContent, ResourcePath,
    ResourceRole, SemanticIdentity, SemanticItem, SourceProvenance, TargetActivation,
    TargetActivationOverrides, GRAPH_VERSION,
};
pub use ir::ActivationMode;
pub use parsers::{
    aggregate, compile_graph, AggregateCollisionCandidate, AggregateCollisionGroup,
    AggregateCollisionKey, AggregationCandidate, AggregationCollisionError, AggregationRequest,
    DecoderSelection, NativeFrontend, OuterInputIdentity,
};
