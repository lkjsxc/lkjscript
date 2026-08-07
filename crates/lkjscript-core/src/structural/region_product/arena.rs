use std::num::NonZeroU32;

use super::model::RegionProductRecord;
use super::*;
use crate::RuntimeLayoutId;

pub struct RegionProductArena<T> {
    id: RegionProductArenaId,
    records: Vec<RegionProductRecord<T>>,
    fields: u64,
}

impl<T> RegionProductArena<T> {
    pub fn new() -> Result<Self, RegionProductError> {
        Ok(Self {
            id: RegionProductArenaId::fresh()?,
            records: Vec::new(),
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
        let next_records = self
            .records
            .len()
            .checked_add(1)
            .ok_or(RegionProductError::ArithmeticOverflow)?;
        let field_count =
            u64::try_from(fields.len()).map_err(|_| RegionProductError::ArithmeticOverflow)?;
        let next_fields = self
            .fields
            .checked_add(field_count)
            .ok_or(RegionProductError::ArithmeticOverflow)?;
        self.records
            .try_reserve(1)
            .map_err(|_| RegionProductError::HostAllocation)?;
        let record = NonZeroU32::new(
            u32::try_from(next_records).map_err(|_| RegionProductError::RepresentationExhausted)?,
        )
        .ok_or(RegionProductError::RepresentationExhausted)?;
        self.records.push(RegionProductRecord { identity, fields });
        self.fields = next_fields;
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
        fields
            .checked_add(record)
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
            .ok_or(RegionProductError::ArithmeticOverflow)
    }

    pub fn metrics(&self) -> RegionProductMetrics {
        let field_capacity = self.records.iter().fold(0_u64, |total, record| {
            total.saturating_add(record.fields.capacity() as u64)
        });
        let field_bytes = field_capacity.saturating_mul(std::mem::size_of::<T>() as u64);
        let record_bytes = (self.records.capacity() as u64)
            .saturating_mul(std::mem::size_of::<RegionProductRecord<T>>() as u64);
        RegionProductMetrics {
            records: self.records.len() as u64,
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
