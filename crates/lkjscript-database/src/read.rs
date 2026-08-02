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

    pub(crate) fn bounded_range(
        &self,
        tenant: &TenantId,
        start_inclusive: Option<&[u8]>,
        end_exclusive: Option<&[u8]>,
        limit: usize,
        returned_byte_limit: usize,
    ) -> DatabaseResult<Option<Vec<(Key, Value)>>> {
        collect_bounded_range(
            &self.snapshot,
            tenant,
            start_inclusive,
            end_exclusive,
            limit,
            returned_byte_limit,
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
    let mut values = Vec::new();
    visit_range(index, tenant, start, end, limit, |key, value| {
        values.push((key.clone(), value.clone()));
        true
    })?;
    Ok(values)
}

pub(crate) fn collect_bounded_range(
    index: &BTreeMap<NamespacedKey, Value>,
    tenant: &TenantId,
    start: Option<&[u8]>,
    end: Option<&[u8]>,
    limit: usize,
    returned_byte_limit: usize,
) -> DatabaseResult<Option<Vec<(Key, Value)>>> {
    let mut returned_bytes = 0usize;
    let mut values = Vec::new();
    let complete = visit_range(index, tenant, start, end, limit, |key, value| {
        let Some(next_returned_bytes) = returned_bytes
            .checked_add(key.as_bytes().len())
            .and_then(|bytes| bytes.checked_add(value.as_bytes().len()))
        else {
            return false;
        };
        if next_returned_bytes > returned_byte_limit {
            return false;
        }
        returned_bytes = next_returned_bytes;
        values.push((key.clone(), value.clone()));
        true
    })?;
    Ok(complete.then_some(values))
}

fn visit_range(
    index: &BTreeMap<NamespacedKey, Value>,
    tenant: &TenantId,
    start: Option<&[u8]>,
    end: Option<&[u8]>,
    limit: usize,
    mut visitor: impl FnMut(&Key, &Value) -> bool,
) -> DatabaseResult<bool> {
    if limit > MAX_RANGE_RESULTS {
        return Err(crate::DatabaseError::RangeLimit {
            requested: limit,
            maximum: MAX_RANGE_RESULTS,
        });
    }
    if limit == 0 {
        return Ok(true);
    }
    let mut count = 0usize;
    for (name, value) in index {
        if &name.tenant != tenant {
            continue;
        }
        let bytes = name.key.as_bytes();
        if start.is_some_and(|bound| bytes < bound) || end.is_some_and(|bound| bytes >= bound) {
            continue;
        }
        if !visitor(&name.key, value) {
            return Ok(false);
        }
        count += 1;
        if count == limit {
            break;
        }
    }
    Ok(true)
}
