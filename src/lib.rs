#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]
// Public typed diagnostics intentionally remain rich values at every deterministic boundary.
#![allow(clippy::result_large_err)]

//! A programming system for coding agents that validates, saves, compiles, and runs typed program revisions.

pub mod artifact;
mod codec;
mod compile;
mod core_ir;
pub mod daemon;
pub mod diff;
pub mod error;
pub mod graph;
pub mod ids;
pub mod interpret;
pub mod machine;
mod machine_contract;
mod managed;
mod ownership;
mod persistence;
pub mod protocol;
pub mod query;
pub mod schema;
pub mod transaction;
pub mod transport;
pub mod type_layout;
mod validate;
pub mod workbench;

#[cfg(test)]
mod campaign_tests;

pub use error::{ErrorCode, LkError, Result};
pub use ids::{
    ChangeDigest, DraftSymbol, IdempotencyKey, NodeId, QueryId, RequestId, Revision, SnapshotHash,
    WorkspaceId,
};
pub use interpret::{RunPolicy, RuntimeFieldValue, RuntimeValue};
pub use machine::{
    DescribeSchemaRequest, DescribeSchemaResult, MachineSchemaDigest, SchemaProjection, SchemaRoot,
};
pub use protocol::{Request, RequestCode, Response, ResponseCode};
pub use schema::{
    BlockArgumentDescriptor, BlockArgumentRole, ByteString, DirectReference, LiteralField,
    MatchArm, MatchArmOperationDraft, Node, NodeKind, OperandArity, OperandDescriptor, OperandUse,
    OperationCode, OperationDescriptor, OperationDraft, OperationKind, ProductFieldValue,
    ProductFieldValueDraft, RegionArity, RegionDescriptor, RegionRole, SemanticType, TypeDraft,
    TypeReferenceSlot, TypeRule, ValueDraft, ValueRef,
};
pub use transaction::{
    ApplyTransactionRequest, ExpressionDraft, ExpressionKindDraft, FunctionBodyDraft,
    FunctionParameterDraft, MAX_RETURNED_BINDINGS, MAX_STRUCTURED_DRAFT_DEPTH,
    MAX_STRUCTURED_DRAFT_ITEMS, MatchArmDraft, NodeTarget, ProductFieldDraft, SumVariantDraft,
    Transaction, TransactionMode, TransactionOp, TransactionOpCode, TransactionReceipt,
    TransactionResponseSpec, YieldingBodyDraft,
};
pub use transport::Client;
