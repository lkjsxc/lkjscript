use std::marker::PhantomData;

use super::model::{ObjectLocation, RootRecord};
use super::{SealedBuilder, SealedOwner, SealedRef, SealedRegionStore};
use crate::structural::{RootClass, RootKey, StructuralError};

const CHUNK_OBJECTS: usize = 256;
const LARGE_OBJECT_BYTES: usize = 16 * 1024;

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub fn allocate(
        &mut self,
        builder: &SealedBuilder<T, D>,
        value: T,
    ) -> Result<SealedRef<T>, StructuralError> {
        let object_bytes = u64::try_from(std::mem::size_of::<T>())
            .map_err(|_| StructuralError::ArithmeticOverflow)?;
        let record = self.record_mut(builder.key)?;
        let bytes = record
            .bytes
            .checked_add(object_bytes)
            .ok_or(StructuralError::ArithmeticOverflow)?;
        record
            .roots
            .try_reserve(1)
            .map_err(|_| StructuralError::AllocationFailed)?;
        let location = if std::mem::size_of::<T>() > LARGE_OBJECT_BYTES {
            allocate_large(record, value)?
        } else {
            allocate_chunked(record, value)?
        };
        let slot =
            u64::try_from(record.roots.len()).map_err(|_| StructuralError::ArithmeticOverflow)?;
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
        let from_slot =
            usize::try_from(from.key.slot()).map_err(|_| StructuralError::ArithmeticOverflow)?;
        let to_slot =
            usize::try_from(to.key.slot()).map_err(|_| StructuralError::ArithmeticOverflow)?;
        if from_slot >= root_count || to_slot >= root_count {
            return Err(StructuralError::StaleRoot(from.key));
        }
        record.internal_edges.push((from.key.slot(), to.key.slot()))
    }

    pub fn root(
        &self,
        owner: &SealedOwner<T, D>,
        slot: u64,
    ) -> Result<SealedRef<T>, StructuralError> {
        let index = self.record_index(owner.key)?;
        let record = &self.records[index].1;
        if usize::try_from(slot).map_err(|_| StructuralError::ArithmeticOverflow)?
            >= record.roots.len()
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
            .get(usize::try_from(key.slot()).map_err(|_| StructuralError::ArithmeticOverflow)?)
            .ok_or(StructuralError::StaleRoot(key))?;
        if root.generation != key.generation() || record.owners == 0 {
            return Err(StructuralError::StaleRoot(key));
        }
        match root.location {
            ObjectLocation::Chunk { chunk, offset } => record
                .chunks
                .get(usize::try_from(chunk).map_err(|_| StructuralError::ArithmeticOverflow)?)
                .and_then(|values| {
                    usize::try_from(offset)
                        .ok()
                        .and_then(|offset| values.get(offset))
                })
                .ok_or(StructuralError::StaleRoot(key)),
            ObjectLocation::Large { index } => record
                .large
                .get(usize::try_from(index).map_err(|_| StructuralError::ArithmeticOverflow)?)
                .and_then(|values| values.first())
                .ok_or(StructuralError::StaleRoot(key)),
        }
    }
}

fn allocate_chunked<T, D>(
    record: &mut super::model::SealedRecord<T, D>,
    value: T,
) -> Result<ObjectLocation, StructuralError> {
    let needs_chunk = record
        .chunks
        .last()
        .is_none_or(|chunk| chunk.len() >= CHUNK_OBJECTS);
    if needs_chunk {
        record
            .chunks
            .try_reserve(1)
            .map_err(|_| StructuralError::AllocationFailed)?;
        let mut chunk = Vec::new();
        chunk
            .try_reserve_exact(CHUNK_OBJECTS)
            .map_err(|_| StructuralError::AllocationFailed)?;
        record.chunks.push(chunk);
    }
    let chunk = record.chunks.len() - 1;
    let offset = record.chunks[chunk].len();
    record.chunks[chunk].push(value);
    Ok(ObjectLocation::Chunk {
        chunk: u64::try_from(chunk).map_err(|_| StructuralError::ArithmeticOverflow)?,
        offset: u64::try_from(offset).map_err(|_| StructuralError::ArithmeticOverflow)?,
    })
}

fn allocate_large<T, D>(
    record: &mut super::model::SealedRecord<T, D>,
    value: T,
) -> Result<ObjectLocation, StructuralError> {
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
        u64::try_from(record.large.len()).map_err(|_| StructuralError::ArithmeticOverflow)?;
    record.large.push(storage);
    Ok(ObjectLocation::Large { index })
}
