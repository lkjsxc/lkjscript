use std::collections::HashMap;

use super::model::{next_region_product_token, RegionProductRecord};
use super::*;
use crate::RuntimeLayoutId;

pub struct RegionProductArena<T> {
    id: RegionProductArenaId,
    records: Vec<RegionProductRecord<T>>,
    tokens: HashMap<u64, u64>,
    fields: u64,
}

impl<T> RegionProductArena<T> {
    pub fn new() -> Result<Self, RegionProductError> {
        Ok(Self {
            id: RegionProductArenaId::fresh()?,
            records: Vec::new(),
            tokens: HashMap::new(),
            fields: 0,
        })
    }

    pub const fn id(&self) -> RegionProductArenaId {
        self.id
    }

    pub fn publish(
        &mut self,
        identity: crate::RuntimeLayoutId,
        fields: Vec<T>,
    ) -> Result<RegionProductKey, RegionProductError> {
        let record = u64::try_from(self.records.len())
            .map_err(|_| RegionProductError::ArithmeticOverflow)?;
        let field_count =
            u64::try_from(fields.len()).map_err(|_| RegionProductError::ArithmeticOverflow)?;
        let next_fields = self
            .fields
            .checked_add(field_count)
            .ok_or(RegionProductError::ArithmeticOverflow)?;
        self.records
            .try_reserve(1)
            .map_err(|_| RegionProductError::HostAllocation)?;
        self.tokens
            .try_reserve(1)
            .map_err(|_| RegionProductError::HostAllocation)?;
        let token = next_region_product_token()?;
        self.records.push(RegionProductRecord { identity, fields });
        if self.tokens.insert(token.get(), record).is_some() {
            self.records.pop();
            return Err(RegionProductError::ArithmeticOverflow);
        }
        self.fields = next_fields;
        Ok(RegionProductKey::new(self.id, token))
    }

    pub fn fields(
        &self,
        key: RegionProductKey,
        identity: crate::RuntimeLayoutId,
    ) -> Result<&[T], RegionProductError> {
        let record = self.record(key)?;
        if record.identity != identity {
            return Err(RegionProductError::WrongType);
        }
        Ok(&record.fields)
    }

    pub fn validate_identity(
        &self,
        key: RegionProductKey,
        identity: RuntimeLayoutId,
    ) -> Result<(), RegionProductError> {
        let record = self.record(key)?;
        if record.identity == identity {
            Ok(())
        } else {
            Err(RegionProductError::WrongType)
        }
    }

    pub fn field(
        &self,
        key: RegionProductKey,
        identity: crate::RuntimeLayoutId,
        field: usize,
    ) -> Result<&T, RegionProductError> {
        self.fields(key, identity)?
            .get(field)
            .ok_or(RegionProductError::FieldOutOfRange)
    }

    pub fn publish_storage_increase(
        &self,
        field_capacity: usize,
    ) -> Result<u64, RegionProductError> {
        let fields = storage_bytes::<T>(field_capacity)?;
        let record = if self.records.len() == self.records.capacity() {
            storage_bytes::<RegionProductRecord<T>>(1)?
        } else {
            0
        };
        let token = if self.tokens.len() == self.tokens.capacity() {
            storage_bytes::<(u64, u64)>(1)?
        } else {
            0
        };
        fields
            .checked_add(record)
            .and_then(|bytes| bytes.checked_add(token))
            .ok_or(RegionProductError::ArithmeticOverflow)
    }

    pub fn reserved_bytes_estimate(&self) -> Result<u64, RegionProductError> {
        let mut fields = 0_u64;
        for record in &self.records {
            fields = fields
                .checked_add(storage_bytes::<T>(record.fields.capacity())?)
                .ok_or(RegionProductError::ArithmeticOverflow)?;
        }
        fields
            .checked_add(storage_bytes::<RegionProductRecord<T>>(
                self.records.capacity(),
            )?)
            .and_then(|bytes| {
                storage_bytes::<(u64, u64)>(self.tokens.capacity())
                    .ok()
                    .and_then(|tokens| bytes.checked_add(tokens))
            })
            .ok_or(RegionProductError::ArithmeticOverflow)
    }

    pub fn metrics(&self) -> Result<RegionProductMetrics, RegionProductError> {
        Ok(RegionProductMetrics {
            records: u64::try_from(self.records.len())
                .map_err(|_| RegionProductError::ArithmeticOverflow)?,
            fields: self.fields,
            retained_bytes: self.reserved_bytes_estimate()?,
        })
    }

    fn record(&self, key: RegionProductKey) -> Result<&RegionProductRecord<T>, RegionProductError> {
        if key.arena != self.id {
            return Err(RegionProductError::InvalidKey);
        }
        let record = self
            .tokens
            .get(&key.token.get())
            .copied()
            .ok_or(RegionProductError::InvalidKey)?;
        let index = usize::try_from(record).map_err(|_| RegionProductError::InvalidKey)?;
        self.records
            .get(index)
            .ok_or(RegionProductError::InvalidKey)
    }
}

fn storage_bytes<T>(count: usize) -> Result<u64, RegionProductError> {
    u64::try_from(count)
        .ok()
        .zip(u64::try_from(std::mem::size_of::<T>()).ok())
        .and_then(|(count, item)| count.checked_mul(item))
        .ok_or(RegionProductError::ArithmeticOverflow)
}

impl<T: Copy> RegionProductArena<T> {
    pub fn update(
        &mut self,
        key: RegionProductKey,
        identity: crate::RuntimeLayoutId,
        field: usize,
        replacement: T,
    ) -> Result<RegionProductKey, RegionProductError> {
        let current = self.fields(key, identity)?;
        if field >= current.len() {
            return Err(RegionProductError::FieldOutOfRange);
        }
        let mut fields = Vec::new();
        fields
            .try_reserve_exact(current.len())
            .map_err(|_| RegionProductError::HostAllocation)?;
        fields.extend_from_slice(current);
        fields[field] = replacement;
        self.publish(identity, fields)
    }
}
