use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use lkjscript_host::{
    DatabaseProvider, DatabaseTenantFactory, DatabaseTransactionId, HostError, HostResult,
};

use crate::{Database, Key, ReadTransaction, TenantId, Value, WriteTransaction};

const MAX_PROVIDER_TRANSACTIONS: usize = 4_096;
static NEXT_PROVIDER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub struct DatabaseTenantService {
    database: Database,
}

impl DatabaseTenantService {
    pub fn new(database: Database) -> Self {
        Self { database }
    }
}

impl DatabaseTenantFactory for DatabaseTenantService {
    fn attach(&self, tenant: &str, incarnation: u64) -> HostResult<Arc<dyn DatabaseProvider>> {
        let tenant = TenantId::new(tenant.as_bytes().to_vec()).map_err(host_error)?;
        if incarnation == 0 {
            return Err(HostError::InvalidName("zero database incarnation".into()));
        }
        let provider = NEXT_PROVIDER.fetch_add(1, Ordering::Relaxed);
        if provider == 0 {
            return Err(HostError::Io {
                operation: "attach database tenant".into(),
                message: "provider identity exhausted".into(),
            });
        }
        Ok(Arc::new(TenantDatabaseProvider {
            database: self.database.clone(),
            tenant,
            provider,
            incarnation,
            state: Mutex::new(ProviderState {
                next_slot: 1,
                active: BTreeMap::new(),
            }),
        }))
    }

    fn checkpoint(&self) -> HostResult<()> {
        self.database.checkpoint().map_err(host_error)
    }
}

pub struct TenantDatabaseProvider {
    database: Database,
    tenant: TenantId,
    provider: u64,
    incarnation: u64,
    state: Mutex<ProviderState>,
}

struct ProviderState {
    next_slot: u32,
    active: BTreeMap<u32, ActiveTransaction>,
}

enum ActiveTransaction {
    Read(ReadTransaction),
    Write(WriteTransaction),
}

impl TenantDatabaseProvider {
    fn lock(&self) -> HostResult<MutexGuard<'_, ProviderState>> {
        self.state.lock().map_err(|_| HostError::Io {
            operation: "lock database provider".into(),
            message: "provider state poisoned".into(),
        })
    }

    fn validate(&self, transaction: DatabaseTransactionId) -> HostResult<u32> {
        if transaction.provider() != self.provider || transaction.incarnation() != self.incarnation
        {
            return Err(HostError::PermissionDenied(
                "stale or foreign database transaction".into(),
            ));
        }
        Ok(transaction.slot())
    }

    fn insert(&self, transaction: ActiveTransaction) -> HostResult<DatabaseTransactionId> {
        let mut state = self.lock()?;
        if state.active.len() >= MAX_PROVIDER_TRANSACTIONS {
            return Err(HostError::Io {
                operation: "begin database transaction".into(),
                message: "provider transaction bound reached".into(),
            });
        }
        let slot = state.next_slot;
        state.next_slot = slot.checked_add(1).ok_or_else(|| HostError::Io {
            operation: "begin database transaction".into(),
            message: "transaction slot exhausted".into(),
        })?;
        state.active.insert(slot, transaction);
        DatabaseTransactionId::new(self.provider, slot, self.incarnation).ok_or_else(|| {
            HostError::Io {
                operation: "begin database transaction".into(),
                message: "invalid transaction identity".into(),
            }
        })
    }
}

include!("operations.rs");
include!("finish.rs");

fn host_error(error: crate::DatabaseError) -> HostError {
    HostError::Io {
        operation: "database provider".into(),
        message: error.to_string(),
    }
}
