//! Typed publication-plan values and target mapping validation.
//!
//! Plan and mapping values deliberately have no staging, apply, check, or
//! filesystem I/O. The `fs` submodule exposes only handle-oriented primitives.

mod apply;
mod candidate;
mod mapping;
mod model;
mod plan;
mod stage;
mod transaction;

pub mod fs;

pub use apply::{
    apply_plan, check_plan, check_sources, ApplyOptions, ApplyReport, AuthorizedRoot,
    DestinationCheck, DestinationState, PlanCheckReport, PlanOperationRequest, SourceCheckRequest,
};
pub use mapping::{mapping_for, TargetMapping};
pub use model::{
    ArtifactDescriptor, MappingVersion, PlanDigest, PlanEntry, PlanLossFinding, PublicationPlan,
    PublicationScope, RootBinding, RootIdentity,
};
pub use plan::{canonical_plan_json, parse_plan_with_expected_digest, PLAN_VERSION};
pub use stage::{
    stage, ScopedAcceptedLoss, ScopedLowering, StageDurability, StageRequest, StageRoot,
    StagedPublication,
};
