use std::collections::BTreeMap;
use std::sync::Arc;

use crate::types::NamespacedKey;
use crate::{DatabaseResult, Key, TenantId, TransactionId, Value, MAX_RANGE_RESULTS};

pub struct ReadTransaction {
    id: TransactionId,
    snapshot: Arc<BTreeMap<NamespacedKey, Value>>,
}

impl ReadTransaction {
    pub(crate) fn new(id: TransactionId, snapshot: Arc<BTreeMap<NamespacedKey, Value>>) -> Self {
        Self { id, snapshot }
    }

    pub fn id(&self) -> TransactionId {
        self.id
    }

    pub fn get(&self, tenant: &TenantId, key: &Key) -> Option<Value> {
        self.snapshot
            .get(&NamespacedKey {
                tenant: tenant.clone(),
                key: key.clone(),
            })
            .cloned()
    }

    pub fn range(
        &self,
        tenant: &TenantId,
        start_inclusive: Option<&[u8]>,
        end_exclusive: Option<&[u8]>,
        limit: usize,
    ) -> DatabaseResult<Vec<(Key, Value)>> {
        collect_range(
            &self.snapshot,
            tenant,
            start_inclusive,
            end_exclusive,
            limit,
        )
    }

    pub fn commit(self) {}

    pub fn abort(self) {}
}

pub(crate) fn collect_range(
    index: &BTreeMap<NamespacedKey, Value>,
    tenant: &TenantId,
    start: Option<&[u8]>,
    end: Option<&[u8]>,
    limit: usize,
) -> DatabaseResult<Vec<(Key, Value)>> {
    if limit > MAX_RANGE_RESULTS {
        return Err(crate::DatabaseError::RangeLimit {
            requested: limit,
            maximum: MAX_RANGE_RESULTS,
        });
    }
    let mut values = Vec::new();
    for (name, value) in index {
        if &name.tenant != tenant {
            continue;
        }
        let bytes = name.key.as_bytes();
        if start.is_some_and(|bound| bytes < bound) || end.is_some_and(|bound| bytes >= bound) {
            continue;
        }
        values.push((name.key.clone(), value.clone()));
        if values.len() == limit {
            break;
        }
    }
    Ok(values)
}
