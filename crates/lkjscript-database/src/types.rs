use std::sync::atomic::{AtomicU64, Ordering};

use crate::{DatabaseError, DatabaseResult};

pub const MAX_TENANT_BYTES: usize = 128;
pub const MAX_KEY_BYTES: usize = 4 * 1024;
pub const MAX_VALUE_BYTES: usize = 1024 * 1024;
pub const MAX_RANGE_RESULTS: usize = 4096;
const DEFAULT_LOGICAL_BUFFER_BYTES: usize = 8 * 1024 * 1024;
static NEXT_DATABASE_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct DatabaseId {
    slot: u64,
    generation: u64,
}

impl DatabaseId {
    pub fn slot(self) -> u64 {
        self.slot
    }
    pub fn generation(self) -> u64 {
        self.generation
    }
}

pub(crate) fn fresh_database_id() -> DatabaseId {
    DatabaseId {
        slot: NEXT_DATABASE_ID.fetch_add(1, Ordering::Relaxed),
        generation: 1,
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TransactionId {
    database: DatabaseId,
    generation: u64,
}

impl TransactionId {
    pub fn database(self) -> DatabaseId {
        self.database
    }
    pub fn generation(self) -> u64 {
        self.generation
    }
}

pub(crate) fn transaction_id(database: DatabaseId, generation: u64) -> TransactionId {
    TransactionId {
        database,
        generation,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DatabaseLimits {
    pub logical_write_buffer_bytes: usize,
}

impl Default for DatabaseLimits {
    fn default() -> Self {
        Self {
            logical_write_buffer_bytes: DEFAULT_LOGICAL_BUFFER_BYTES,
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct TenantId(Vec<u8>);

impl TenantId {
    pub fn new(bytes: impl Into<Vec<u8>>) -> DatabaseResult<Self> {
        let bytes = bytes.into();
        if bytes.is_empty() || bytes.len() > MAX_TENANT_BYTES {
            return Err(DatabaseError::InvalidTenantLength {
                length: bytes.len(),
            });
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Key(Vec<u8>);

impl Key {
    pub fn new(bytes: impl Into<Vec<u8>>) -> DatabaseResult<Self> {
        let bytes = bytes.into();
        if bytes.is_empty() || bytes.len() > MAX_KEY_BYTES {
            return Err(DatabaseError::InvalidKeyLength {
                length: bytes.len(),
            });
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Value(Vec<u8>);

impl Value {
    pub fn new(bytes: impl Into<Vec<u8>>) -> DatabaseResult<Self> {
        let bytes = bytes.into();
        if bytes.len() > MAX_VALUE_BYTES {
            return Err(DatabaseError::InvalidValueLength {
                length: bytes.len(),
            });
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct NamespacedKey {
    pub tenant: TenantId,
    pub key: Key,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Operation {
    Put(NamespacedKey, Value),
    Delete(NamespacedKey),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RecoveryReport {
    pub damaged_tail_discarded: bool,
    pub uncommitted_transactions_discarded: usize,
}
