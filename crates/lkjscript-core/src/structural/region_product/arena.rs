use std::num::NonZeroU32;

use super::model::RegionProductRecord;
use super::*;
use crate::RuntimeLayoutId;

pub struct RegionProductArena<T> {
    id: RegionProductArenaId,
    limits: RegionProductLimits,
    records: Vec<RegionProductRecord<T>>,
    fields: u32,
    reserved_fields: u32,
}

impl<T> RegionProductArena<T> {
    pub fn new(limits: RegionProductLimits) -> Result<Self, RegionProductError> {
        Ok(Self {
            id: RegionProductArenaId::fresh()?,
            limits,
            records: Vec::new(),
            fields: 0,
            reserved_fields: 0,
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
        let next_records = self
            .records
            .len()
            .checked_add(1)
            .ok_or(RegionProductError::Records)?;
        if next_records > self.limits.max_records.get() as usize {
            return Err(RegionProductError::Records);
        }
        let field_count = u32::try_from(fields.len()).map_err(|_| RegionProductError::Fields)?;
        let next_fields = self
            .fields
            .checked_add(field_count)
            .ok_or(RegionProductError::Fields)?;
        if next_fields > self.limits.max_fields.get() {
            return Err(RegionProductError::Fields);
        }
        let reserved = u32::try_from(fields.capacity()).map_err(|_| RegionProductError::Fields)?;
        let next_reserved = self
            .reserved_fields
            .checked_add(reserved)
            .ok_or(RegionProductError::Fields)?;
        if next_reserved > self.limits.max_fields.get() {
            return Err(RegionProductError::Fields);
        }
        self.records
            .try_reserve(1)
            .map_err(|_| RegionProductError::HostAllocation)?;
        let record =
            NonZeroU32::new(u32::try_from(next_records).map_err(|_| RegionProductError::Records)?)
                .ok_or(RegionProductError::Records)?;
        self.records.push(RegionProductRecord { identity, fields });
        self.fields = next_fields;
        self.reserved_fields = next_reserved;
        Ok(RegionProductKey::new(self.id, record))
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

    pub fn publish_storage_increase(&self, field_capacity: usize) -> u64 {
        let fields = u64::try_from(field_capacity)
            .unwrap_or(u64::MAX)
            .saturating_mul(u64::try_from(std::mem::size_of::<T>()).unwrap_or(u64::MAX));
        let record = if self.records.len() == self.records.capacity() {
            u64::try_from(std::mem::size_of::<RegionProductRecord<T>>()).unwrap_or(u64::MAX)
        } else {
            0
        };
        fields.saturating_add(record)
    }

    pub fn metrics(&self) -> RegionProductMetrics {
        let field_capacity = self.records.iter().fold(0_u64, |total, record| {
            total.saturating_add(u64::try_from(record.fields.capacity()).unwrap_or(u64::MAX))
        });
        let field_bytes = field_capacity
            .saturating_mul(u64::try_from(std::mem::size_of::<T>()).unwrap_or(u64::MAX));
        let record_bytes = u64::try_from(self.records.capacity())
            .unwrap_or(u64::MAX)
            .saturating_mul(
                u64::try_from(std::mem::size_of::<RegionProductRecord<T>>()).unwrap_or(u64::MAX),
            );
        RegionProductMetrics {
            records: u32::try_from(self.records.len()).unwrap_or(u32::MAX),
            fields: self.fields,
            reserved_bytes_estimate: field_bytes.saturating_add(record_bytes),
        }
    }

    fn record(&self, key: RegionProductKey) -> Result<&RegionProductRecord<T>, RegionProductError> {
        if key.arena != self.id {
            return Err(RegionProductError::InvalidKey);
        }
        key.index()
            .and_then(|index| self.records.get(index))
            .ok_or(RegionProductError::InvalidKey)
    }
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
