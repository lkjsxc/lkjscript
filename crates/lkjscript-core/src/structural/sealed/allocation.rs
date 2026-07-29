use std::marker::PhantomData;

use super::model::{ObjectLocation, RootRecord};
use super::{SealedBuilder, SealedOwner, SealedRef, SealedRegionStore};
use crate::structural::{RootClass, RootKey, StructuralError, StructuralLimit};

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub fn allocate(
        &mut self,
        builder: &SealedBuilder<T, D>,
        value: T,
    ) -> Result<SealedRef<T>, StructuralError> {
        let object_bytes = u64::try_from(std::mem::size_of::<T>())
            .map_err(|_| StructuralError::ArithmeticOverflow)?;
        let limits = self.limits;
        let record = self.record_mut(builder.key)?;
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
        let location = if object_bytes > u64::from(limits.large_object_bytes) {
            allocate_large(record, value, limits.max_objects_per_domain)?
        } else {
            allocate_chunked(record, value, limits)?
        };
        let slot =
            u32::try_from(record.roots.len()).map_err(|_| StructuralError::ArithmeticOverflow)?;
        let generation = Self::epoch(builder.key);
        record.roots.push(RootRecord {
            generation,
            location,
        });
        record.bytes = bytes;
        self.metrics.bytes_allocated = self.metrics.bytes_allocated.saturating_add(object_bytes);
        Ok(SealedRef {
            key: RootKey::from_parts(
                builder.key,
                RootClass::RegionInternal,
                slot,
                generation,
                self.layout,
                self.semantic_type,
            ),
            marker: PhantomData,
        })
    }

    pub fn add_internal_edge(
        &mut self,
        builder: &SealedBuilder<T, D>,
        from: SealedRef<T>,
        to: SealedRef<T>,
    ) -> Result<(), StructuralError> {
        if from.key.domain() != builder.key || to.key.domain() != builder.key {
            return Err(StructuralError::WrongRuntime);
        }
        let record = self.record_mut(builder.key)?;
        let root_count = record.roots.len();
        if from.key.slot() as usize >= root_count || to.key.slot() as usize >= root_count {
            return Err(StructuralError::StaleRoot(from.key));
        }
        record.internal_edges.push(
            (from.key.slot(), to.key.slot()),
            StructuralLimit::Dependencies,
        )
    }

    pub fn root(
        &self,
        owner: &SealedOwner<T, D>,
        slot: u32,
    ) -> Result<SealedRef<T>, StructuralError> {
        let index = self.record_index(owner.key)?;
        let record = &self.records[index].1;
        if slot as usize >= record.roots.len()
            || owner.key.class() != crate::structural::DomainClass::RegionSealed
        {
            return Err(StructuralError::StaleDomain(owner.key));
        }
        Ok(SealedRef {
            key: RootKey::from_parts(
                owner.key,
                RootClass::SealedPublic,
                slot,
                Self::epoch(owner.key),
                self.layout,
                self.semantic_type,
            ),
            marker: PhantomData,
        })
    }

    pub fn get(&self, reference: SealedRef<T>) -> Result<&T, StructuralError> {
        let key = reference.key;
        if key.layout() != self.layout {
            return Err(StructuralError::WrongLayout);
        }
        if key.semantic_type() != self.semantic_type {
            return Err(StructuralError::WrongSemanticType);
        }
        if key.class() != RootClass::SealedPublic {
            return Err(StructuralError::StaleRoot(key));
        }
        let index = self.record_index(key.domain())?;
        let record = &self.records[index].1;
        let root = record
            .roots
            .get(key.slot() as usize)
            .ok_or(StructuralError::StaleRoot(key))?;
        if root.generation != key.generation() || record.owners == 0 {
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
}

fn allocate_chunked<T, D>(
    record: &mut super::model::SealedRecord<T, D>,
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
    let offset = record.chunks[chunk].len();
    record.chunks[chunk].push(value);
    Ok(ObjectLocation::Chunk {
        chunk: u32::try_from(chunk).map_err(|_| StructuralError::ArithmeticOverflow)?,
        offset: u32::try_from(offset).map_err(|_| StructuralError::ArithmeticOverflow)?,
    })
}

fn allocate_large<T, D>(
    record: &mut super::model::SealedRecord<T, D>,
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
