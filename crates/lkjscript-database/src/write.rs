use std::collections::BTreeMap;
use std::sync::Arc;

use crate::database::Inner;
use crate::read::{collect_bounded_range, collect_range};
use crate::types::{NamespacedKey, Operation};
use crate::{DatabaseError, DatabaseLimits, DatabaseResult, Key, TenantId, TransactionId, Value};

pub struct WriteTransaction {
    inner: Arc<Inner>,
    id: TransactionId,
    working: BTreeMap<NamespacedKey, Value>,
    changes: BTreeMap<NamespacedKey, Option<Value>>,
    buffered_bytes: usize,
    limit: usize,
    active: bool,
}

impl WriteTransaction {
    pub(crate) fn new(
        inner: Arc<Inner>,
        id: TransactionId,
        working: BTreeMap<NamespacedKey, Value>,
        limits: DatabaseLimits,
    ) -> Self {
        Self {
            inner,
            id,
            working,
            changes: BTreeMap::new(),
            buffered_bytes: 0,
            limit: limits.logical_write_buffer_bytes,
            active: true,
        }
    }

    pub fn id(&self) -> TransactionId {
        self.id
    }

    pub fn buffered_logical_bytes(&self) -> usize {
        self.buffered_bytes
    }

    pub fn get(&self, tenant: &TenantId, key: &Key) -> Option<Value> {
        self.working
            .get(&NamespacedKey {
                tenant: tenant.clone(),
                key: key.clone(),
            })
            .cloned()
    }

    pub fn put(&mut self, tenant: TenantId, key: Key, value: Value) -> DatabaseResult<()> {
        self.ensure_active()?;
        let name = NamespacedKey { tenant, key };
        let old = self
            .changes
            .get(&name)
            .map(|item| operation_size(&name, item.as_ref()));
        let requested =
            self.buffered_bytes - old.unwrap_or(0) + operation_size(&name, Some(&value));
        if requested > self.limit {
            return Err(DatabaseError::LogicalBufferLimit {
                requested,
                maximum: self.limit,
            });
        }
        self.working.insert(name.clone(), value.clone());
        self.changes.insert(name, Some(value));
        self.buffered_bytes = requested;
        Ok(())
    }

    pub fn delete(&mut self, tenant: TenantId, key: Key) -> DatabaseResult<()> {
        self.ensure_active()?;
        let name = NamespacedKey { tenant, key };
        let old = self
            .changes
            .get(&name)
            .map(|item| operation_size(&name, item.as_ref()));
        let requested = self.buffered_bytes - old.unwrap_or(0) + operation_size(&name, None);
        if requested > self.limit {
            return Err(DatabaseError::LogicalBufferLimit {
                requested,
                maximum: self.limit,
            });
        }
        self.working.remove(&name);
        self.changes.insert(name, None);
        self.buffered_bytes = requested;
        Ok(())
    }

    pub fn range(
        &self,
        tenant: &TenantId,
        start_inclusive: Option<&[u8]>,
        end_exclusive: Option<&[u8]>,
        limit: usize,
    ) -> DatabaseResult<Vec<(Key, Value)>> {
        self.ensure_active()?;
        collect_range(&self.working, tenant, start_inclusive, end_exclusive, limit)
    }

    pub(crate) fn bounded_range(
        &self,
        tenant: &TenantId,
        start_inclusive: Option<&[u8]>,
        end_exclusive: Option<&[u8]>,
        limit: usize,
        returned_byte_limit: usize,
    ) -> DatabaseResult<Option<Vec<(Key, Value)>>> {
        self.ensure_active()?;
        collect_bounded_range(
            &self.working,
            tenant,
            start_inclusive,
            end_exclusive,
            limit,
            returned_byte_limit,
        )
    }

    pub fn commit(mut self) -> DatabaseResult<()> {
        self.ensure_active()?;
        let operations = std::mem::take(&mut self.changes)
            .into_iter()
            .map(|(name, value)| match value {
                Some(value) => Operation::Put(name, value),
                None => Operation::Delete(name),
            })
            .collect();
        let working = std::mem::take(&mut self.working);
        self.active = false;
        self.inner.commit(operations, working)
    }

    pub fn abort(mut self) -> DatabaseResult<()> {
        self.ensure_active()?;
        self.active = false;
        self.inner.lock().writer_active = false;
        Ok(())
    }

    fn ensure_active(&self) -> DatabaseResult<()> {
        if self.active {
            Ok(())
        } else {
            Err(DatabaseError::TransactionClosed)
        }
    }
}

impl Drop for WriteTransaction {
    fn drop(&mut self) {
        if self.active {
            self.inner.lock().writer_active = false;
            self.active = false;
        }
    }
}

fn operation_size(name: &NamespacedKey, value: Option<&Value>) -> usize {
    32usize
        .saturating_add(name.tenant.as_bytes().len())
        .saturating_add(name.key.as_bytes().len())
        .saturating_add(value.map_or(0, |item| item.as_bytes().len()))
}
