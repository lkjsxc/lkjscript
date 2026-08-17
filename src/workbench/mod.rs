//! Agent-oriented derived context, review, and proposal projections.
//!
//! These types never own semantic state. Plans normalize into the existing typed transaction
//! request, and packets are disposable observations over an immutable revision.

mod context;
mod help;
mod plan;
mod view;

pub use context::{
    ContextAlias, ContextBuildRequest, ContextObservation, ContextObservationRole,
    ContextOmissions, ContextPacket, ContextPacketDigest, ContextPacketPayload, ContextPurpose,
    build_context_packet, decode_context_packet, encode_context_packet,
};
pub use help::authoring_help_cards;
pub use plan::{
    ParsedEditPlan, ParsedRunPlan, PlanError, PlanErrorCode, parse_edit_plan, parse_run_plan,
};
pub use view::{render_context_packet, render_semantic_diff};

pub const WORKBENCH_VERSION: u16 = 1;
pub const MAX_WORKBENCH_INPUT_BYTES: usize = crate::machine::MAX_JSON_INPUT_BYTES;
pub const MAX_CONTEXT_PACKET_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_REVIEW_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
