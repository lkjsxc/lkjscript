//! Durable, ordered, multi-tenant key/value database.
//! Isolation is exactly single-writer serializable with reader snapshots.

mod checkpoint;
mod database;
mod error;
mod persistence;
mod read;
mod types;
mod wal;
mod write;

pub use database::Database;
pub use error::{DatabaseError, DatabaseResult};
pub use read::ReadTransaction;
pub use types::{
    DatabaseId, DatabaseLimits, Key, RecoveryReport, TenantId, TransactionId, Value, MAX_KEY_BYTES,
    MAX_RANGE_RESULTS, MAX_TENANT_BYTES, MAX_VALUE_BYTES,
};
pub use write::WriteTransaction;
