use std::marker::PhantomData;

use super::model::{ObjectLocation, RootRecord};
use super::{RegionOwner, RegionRef, RegionStore};
use crate::structural::{RootClass, RootKey, StructuralError};

// Private allocation geometry only. Crossing either value starts another
// checked chunk; it never changes whether a structural program is valid.
const CHUNK_OBJECTS: usize = 256;
const LARGE_OBJECT_BYTES: usize = 16 * 1024;

impl<T: Copy, D: Copy> RegionStore<T, D> {
    pub fn allocate(
        &mut self,
        owner: &RegionOwner<T, D>,
        value: T,
    ) -> Result<RegionRef<T>, StructuralError> {
        let object_bytes = u64::try_from(std::mem::size_of::<T>())
            .map_err(|_| StructuralError::ArithmeticOverflow)?;
        let (slot, epoch, chunk_created) = {
            let record = self.record_mut(owner.key)?;
            let bytes = record
                .bytes
                .checked_add(object_bytes)
                .ok_or(StructuralError::ArithmeticOverflow)?;
            record
                .roots
                .try_reserve(1)
                .map_err(|_| StructuralError::AllocationFailed)?;
            let chunks_before = record.chunks.len();
            let location = if std::mem::size_of::<T>() > LARGE_OBJECT_BYTES {
                allocate_large(record, value)?
            } else {
                allocate_chunked(record, value)?
            };
            let chunk_created = record.chunks.len() != chunks_before;
            let slot = u64::try_from(record.roots.len())
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
            .get(usize::try_from(key.slot()).map_err(|_| StructuralError::ArithmeticOverflow)?)
            .ok_or(StructuralError::StaleRoot(key))?;
        if root.generation != key.generation() || record.epoch != key.generation() {
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
        record
            .internal_edges
            .push((from.key.slot(), to.key.slot()))?;
        self.metrics.internal_edges = self.metrics.internal_edges.saturating_add(1);
        Ok(())
    }
}

fn allocate_chunked<T, D>(
    record: &mut super::model::RegionRecord<T, D>,
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
    let values = &mut record.chunks[chunk];
    let offset = values.len();
    values.push(value);
    Ok(ObjectLocation::Chunk {
        chunk: u64::try_from(chunk).map_err(|_| StructuralError::ArithmeticOverflow)?,
        offset: u64::try_from(offset).map_err(|_| StructuralError::ArithmeticOverflow)?,
    })
}

fn allocate_large<T, D>(
    record: &mut super::model::RegionRecord<T, D>,
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
