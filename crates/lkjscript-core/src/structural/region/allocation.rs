use std::marker::PhantomData;

use super::model::{ObjectLocation, RootRecord};
use super::{RegionOwner, RegionRef, RegionStore};
use crate::structural::{RootClass, RootKey, StructuralError, StructuralLimit};

impl<T: Copy, D: Copy> RegionStore<T, D> {
    pub fn allocate(
        &mut self,
        owner: &RegionOwner<T, D>,
        value: T,
    ) -> Result<RegionRef<T>, StructuralError> {
        let object_bytes = u64::try_from(std::mem::size_of::<T>())
            .map_err(|_| StructuralError::ArithmeticOverflow)?;
        let limits = self.limits;
        let (slot, epoch, chunk_created) = {
            let record = self.record_mut(owner.key)?;
            if record.roots.len() >= limits.max_objects_per_domain as usize {
                return Err(StructuralError::LimitExceeded(StructuralLimit::Objects));
            }
            let bytes = record
                .bytes
                .checked_add(object_bytes)
                .ok_or(StructuralError::ArithmeticOverflow)?;
            if bytes > limits.max_bytes_per_domain {
                return Err(StructuralError::LimitExceeded(StructuralLimit::Bytes));
            }
            record
                .roots
                .try_reserve(1)
                .map_err(|_| StructuralError::AllocationFailed)?;
            let chunks_before = record.chunks.len();
            let location = if object_bytes > u64::from(limits.large_object_bytes) {
                allocate_large(record, value, limits.max_objects_per_domain)?
            } else {
                allocate_chunked(record, value, limits)?
            };
            let chunk_created = record.chunks.len() != chunks_before;
            let slot = u32::try_from(record.roots.len())
                .map_err(|_| StructuralError::ArithmeticOverflow)?;
            let epoch = record.epoch;
            record.roots.push(RootRecord {
                generation: epoch,
                location,
            });
            record.bytes = bytes;
            (slot, epoch, chunk_created)
        };
        if chunk_created {
            self.metrics.chunks_created = self.metrics.chunks_created.saturating_add(1);
        }
        self.metrics.objects_allocated = self.metrics.objects_allocated.saturating_add(1);
        self.metrics.bytes_allocated = self.metrics.bytes_allocated.saturating_add(object_bytes);
        Ok(RegionRef {
            key: RootKey::from_parts(
                owner.key,
                RootClass::RegionInternal,
                slot,
                epoch,
                self.layout,
                self.semantic_type,
            ),
            marker: PhantomData,
        })
    }

    pub fn get(&self, reference: RegionRef<T>) -> Result<&T, StructuralError> {
        let key = reference.key;
        if key.layout() != self.layout {
            return Err(StructuralError::WrongLayout);
        }
        if key.semantic_type() != self.semantic_type {
            return Err(StructuralError::WrongSemanticType);
        }
        if key.class() != RootClass::RegionInternal {
            return Err(StructuralError::StaleRoot(key));
        }
        let index = self.record_index(key.domain())?;
        let record = &self.records[index].1;
        let root = record
            .roots
            .get(key.slot() as usize)
            .ok_or(StructuralError::StaleRoot(key))?;
        if root.generation != key.generation() || record.epoch != key.generation() {
            return Err(StructuralError::StaleRoot(key));
        }
        match root.location {
            ObjectLocation::Chunk { chunk, offset } => record
                .chunks
                .get(chunk as usize)
                .and_then(|values| values.get(offset as usize))
                .ok_or(StructuralError::StaleRoot(key)),
            ObjectLocation::Large { index } => record
                .large
                .get(index as usize)
                .and_then(|values| values.first())
                .ok_or(StructuralError::StaleRoot(key)),
        }
    }

    pub fn add_internal_edge(
        &mut self,
        owner: &RegionOwner<T, D>,
        from: RegionRef<T>,
        to: RegionRef<T>,
    ) -> Result<(), StructuralError> {
        if from.key.domain() != owner.key || to.key.domain() != owner.key {
            return Err(StructuralError::WrongRuntime);
        }
        self.get(from)?;
        self.get(to)?;
        let record = self.record_mut(owner.key)?;
        record.internal_edges.push(
            (from.key.slot(), to.key.slot()),
            StructuralLimit::Dependencies,
        )?;
        self.metrics.internal_edges = self.metrics.internal_edges.saturating_add(1);
        Ok(())
    }
}

fn allocate_chunked<T, D>(
    record: &mut super::model::RegionRecord<T, D>,
    value: T,
    limits: crate::structural::StructuralLimits,
) -> Result<ObjectLocation, StructuralError> {
    let needs_chunk = record
        .chunks
        .last()
        .is_none_or(|chunk| chunk.len() >= limits.chunk_objects as usize);
    if needs_chunk {
        if record.chunks.len() >= limits.max_chunks_per_domain as usize {
            return Err(StructuralError::LimitExceeded(StructuralLimit::Chunks));
        }
        record
            .chunks
            .try_reserve(1)
            .map_err(|_| StructuralError::AllocationFailed)?;
        let mut chunk = Vec::new();
        chunk
            .try_reserve_exact(limits.chunk_objects as usize)
            .map_err(|_| StructuralError::AllocationFailed)?;
        record.chunks.push(chunk);
    }
    let chunk = record.chunks.len() - 1;
    let values = &mut record.chunks[chunk];
    let offset = values.len();
    values.push(value);
    Ok(ObjectLocation::Chunk {
        chunk: u32::try_from(chunk).map_err(|_| StructuralError::ArithmeticOverflow)?,
        offset: u32::try_from(offset).map_err(|_| StructuralError::ArithmeticOverflow)?,
    })
}

fn allocate_large<T, D>(
    record: &mut super::model::RegionRecord<T, D>,
    value: T,
    limit: u32,
) -> Result<ObjectLocation, StructuralError> {
    if record.large.len() >= limit as usize {
        return Err(StructuralError::LimitExceeded(StructuralLimit::Objects));
    }
    record
        .large
        .try_reserve(1)
        .map_err(|_| StructuralError::AllocationFailed)?;
    let mut storage = Vec::new();
    storage
        .try_reserve_exact(1)
        .map_err(|_| StructuralError::AllocationFailed)?;
    storage.push(value);
    let index =
        u32::try_from(record.large.len()).map_err(|_| StructuralError::ArithmeticOverflow)?;
    record.large.push(storage);
    Ok(ObjectLocation::Large { index })
}
