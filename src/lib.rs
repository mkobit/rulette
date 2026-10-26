pub mod cli;
pub mod compilation;
pub mod emitters;
pub mod inputs;
pub mod ir;
pub mod parsers;
pub mod pipeline;
pub mod publication;

pub use compilation::{
    compile, lower_unique_targets, lower_unique_targets_with_options, CompilationRequest,
};
pub use emitters::lowering::{
    lower, CapabilityFinding, CapabilityReasonCode, CapabilitySeverity, LoweringOptions,
    LoweringPlan, NativeArtifact, NativeArtifactClass, NativeTarget,
};
pub use ir::graph::{
    validate_plugin_name, CompilationGraph, DiagnosticSeverity, FrontendPayload, GraphDiagnostic,
    Package, PackageId, PackageKind, PackageRoot, PortableActivation, Resource, ResourceContent,
    ResourcePath, ResourceRole, SemanticIdentity, SemanticItem, SourceProvenance, TargetActivation,
    TargetActivationOverrides, GRAPH_VERSION,
};
pub use ir::permissions::{
    CanonicalTool, NormalizedPattern, PermissionAction, PermissionPolicy, PermissionScope,
    PrecedenceTier, ResolvedPermissionPolicy, ResolvedPermissionRule, SpecificityClass,
    SpecificityKey, TargetPermissionOverrides, TargetPermissionPolicy, ToolAccessRule,
};
pub use ir::ActivationMode;
pub use parsers::{
    aggregate, compile_graph, AggregateCollisionCandidate, AggregateCollisionGroup,
    AggregateCollisionKey, AggregationCandidate, AggregationCollisionError, AggregationRequest,
    DecoderSelection, NativeFrontend, OuterInputIdentity,
};
pub use pipeline::{apply_transforms, select_packages, Transform};
