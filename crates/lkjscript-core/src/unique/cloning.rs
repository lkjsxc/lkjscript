use super::object::{Payload, SlotState};
use super::{
    ByteVectorKey, BytesKey, PathKey, StaticBytes, UniqueLayout, UniqueStore, UniqueStoreError,
};

impl UniqueStore {
    pub fn clone_bytes(&mut self, key: BytesKey) -> Result<BytesKey, UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::Bytes)?;
        let copy = match &self.slots[index].state {
            SlotState::Occupied(Payload::Bytes(bytes)) => self.copy_bytes(bytes)?,
            _ => return Err(UniqueStoreError::ArithmeticOverflow),
        };
        self.allocate(Payload::Bytes(copy)).map(BytesKey::from_raw)
    }

    pub fn clone_path(&mut self, key: PathKey) -> Result<PathKey, UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::Path)?;
        let copy = match &self.slots[index].state {
            SlotState::Occupied(Payload::Path(bytes)) => self.copy_path(bytes)?,
            _ => return Err(UniqueStoreError::ArithmeticOverflow),
        };
        self.allocate(Payload::Path(copy)).map(PathKey::from_raw)
    }

    pub fn clone_path_slice(&mut self, bytes: &[u8]) -> Result<PathKey, UniqueStoreError> {
        let copy = self.copy_path(bytes)?;
        self.allocate(Payload::Path(copy)).map(PathKey::from_raw)
    }

    pub fn paths_equal(&mut self, left: PathKey, right: PathKey) -> Result<bool, UniqueStoreError> {
        let left = self.locate(left.raw(), UniqueLayout::Path)?;
        let right = self.locate(right.raw(), UniqueLayout::Path)?;
        let left = match &self.slots[left].state {
            SlotState::Occupied(Payload::Path(bytes)) => bytes.as_ref(),
            _ => return Err(UniqueStoreError::ArithmeticOverflow),
        };
        let right = match &self.slots[right].state {
            SlotState::Occupied(Payload::Path(bytes)) => bytes.as_ref(),
            _ => return Err(UniqueStoreError::ArithmeticOverflow),
        };
        Ok(left == right)
    }

    pub fn clone_bytes_range(
        &mut self,
        key: BytesKey,
        start: usize,
        len: usize,
    ) -> Result<BytesKey, UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::Bytes)?;
        let copy = match &self.slots[index].state {
            SlotState::Occupied(Payload::Bytes(bytes)) => {
                let range = super::access::checked_range(start, len, bytes.len())?;
                self.copy_bytes(&bytes[range])?
            }
            _ => return Err(UniqueStoreError::ArithmeticOverflow),
        };
        self.allocate(Payload::Bytes(copy)).map(BytesKey::from_raw)
    }

    pub fn clone_static_bytes(&mut self, bytes: &[u8]) -> Result<BytesKey, UniqueStoreError> {
        let copy = self.copy_bytes(bytes)?;
        self.allocate(Payload::Bytes(copy)).map(BytesKey::from_raw)
    }

    pub fn clone_static_bytes_range(
        &mut self,
        bytes: &[u8],
        start: usize,
        len: usize,
    ) -> Result<BytesKey, UniqueStoreError> {
        let range = super::access::checked_range(start, len, bytes.len())?;
        self.clone_static_bytes(&bytes[range])
    }

    pub fn clone_bytes_to_byte_vector(
        &mut self,
        key: BytesKey,
    ) -> Result<ByteVectorKey, UniqueStoreError> {
        let index = self.locate(key.raw(), UniqueLayout::Bytes)?;
        let copy = match &self.slots[index].state {
            SlotState::Occupied(Payload::Bytes(bytes)) => self.copy_bytes(bytes)?,
            _ => return Err(UniqueStoreError::ArithmeticOverflow),
        };
        self.allocate(Payload::ByteVector(copy))
            .map(ByteVectorKey::from_raw)
    }

    pub fn thaw_static_bytes(
        &mut self,
        bytes: StaticBytes,
    ) -> Result<ByteVectorKey, UniqueStoreError> {
        self.thaw_bytes_slice(bytes.as_slice())
    }

    pub fn thaw_bytes_slice(&mut self, bytes: &[u8]) -> Result<ByteVectorKey, UniqueStoreError> {
        let copy = self.copy_bytes(bytes)?;
        self.allocate(Payload::ByteVector(copy))
            .map(ByteVectorKey::from_raw)
    }

    fn copy_path(&self, bytes: &[u8]) -> Result<Box<[u8]>, UniqueStoreError> {
        let requested =
            u64::try_from(bytes.len()).map_err(|_| UniqueStoreError::ArithmeticOverflow)?;
        self.check_allocation(requested)?;
        let mut copy = Vec::new();
        copy.try_reserve_exact(bytes.len())
            .map_err(|_| UniqueStoreError::StorageCapacity)?;
        copy.extend_from_slice(bytes);
        Ok(copy.into_boxed_slice())
    }

    fn copy_bytes(&self, bytes: &[u8]) -> Result<Vec<u8>, UniqueStoreError> {
        let requested =
            u64::try_from(bytes.len()).map_err(|_| UniqueStoreError::ArithmeticOverflow)?;
        self.check_allocation(requested)?;
        let mut copy = Vec::new();
        copy.try_reserve_exact(bytes.len())
            .map_err(|_| UniqueStoreError::StorageCapacity)?;
        let retained =
            u64::try_from(copy.capacity()).map_err(|_| UniqueStoreError::ArithmeticOverflow)?;
        self.check_allocation(retained)?;
        copy.extend_from_slice(bytes);
        Ok(copy)
    }
}
