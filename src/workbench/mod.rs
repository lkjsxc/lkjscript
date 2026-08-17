//! Agent-oriented derived context, review, and proposal projections.
//!
//! These types never own semantic state. Editable documents normalize into the existing typed
//! transaction request, and packets are disposable observations over an immutable revision.

mod context;
mod document;
mod help;
mod view;

pub use context::{
    ContextAlias, ContextBuildRequest, ContextObservation, ContextObservationRole,
    ContextOmissions, ContextPacket, ContextPacketDigest, ContextPacketPayload, ContextPurpose,
    build_context_packet, decode_context_packet, encode_context_packet,
};
pub use document::{
    DocumentError, DocumentErrorCode, EDIT_DOCUMENT_VERSION, EditScope, ParsedEditDocument,
    ParsedRunDocument, parse_edit_document, parse_run_document, render_function_document,
};
pub use help::authoring_help_cards;
pub use view::{render_context_packet, render_semantic_diff};

pub const WORKBENCH_VERSION: u16 = 2;
pub const MAX_WORKBENCH_INPUT_BYTES: usize = crate::machine::MAX_JSON_INPUT_BYTES;
pub const MAX_CONTEXT_PACKET_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_REVIEW_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
