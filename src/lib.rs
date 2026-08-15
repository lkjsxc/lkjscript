#![cfg_attr(test, allow(clippy::expect_used, clippy::panic, clippy::unwrap_used))]

//! Source-free semantic graph kernel, durable daemon, compiler, and interpreter.

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
mod persistence;
pub mod protocol;
pub mod query;
pub mod schema;
pub mod transaction;
mod validate;

#[cfg(test)]
mod campaign_tests;

pub use error::{ErrorCode, LkError, Result};
pub use ids::{
    ChangeDigest, IdempotencyKey, LocalHandle, NodeId, QueryId, RequestId, Revision, SnapshotHash,
    WorkspaceId,
};
pub use interpret::{RunPolicy, RuntimeValue};
pub use protocol::{Client, Request, RequestCode, Response, ResponseCode};
pub use schema::{
    BlockArgumentDescriptor, BlockArgumentRole, DirectReference, LiteralField, Node, NodeKind,
    OperandArity, OperandDescriptor, OperandUse, OperationCode, OperationDescriptor,
    OperationDraft, OperationKind, RegionDescriptor, RegionRole, SemanticType, TypeRule,
    ValueDraft, ValueRef,
};
pub use transaction::{
    ApplyTransactionRequest, ExpressionDraft, ExpressionKindDraft, FunctionBodyDraft,
    FunctionParameterDraft, MAX_RETURNED_BINDINGS, MAX_STRUCTURED_DRAFT_DEPTH,
    MAX_STRUCTURED_DRAFT_ITEMS, NodeTarget, Transaction, TransactionMode, TransactionOp,
    TransactionOpCode, TransactionReceipt, TransactionResponseSpec, YieldingBodyDraft,
};
