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
mod persistence;
pub mod protocol;
pub mod query;
pub mod schema;
pub mod transaction;
mod validate;

pub use error::{ErrorCode, LkError, Result};
pub use ids::{
    IdempotencyKey, LocalHandle, NodeId, RequestId, Revision, SnapshotHash, WorkspaceId,
};
pub use interpret::RuntimeValue;
pub use protocol::{Client, Request, Response};
pub use schema::{
    Node, NodeKind, OperationDraft, OperationKind, SemanticType, ValueDraft, ValueRef,
};
pub use transaction::{NodeTarget, Transaction, TransactionOp, TransactionResult};
