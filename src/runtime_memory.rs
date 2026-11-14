//! Independent raw-address model for the sealed source allocator.
//! Mappings are disjoint, independently released byte regions.
use crate::runtime::{ALLOCATOR_STATE_BYTES, Primitive};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MemoryFault {
    Allocation,
    Bounds,
}
impl MemoryFault {
    pub fn message(self) -> &'static str {
        match self {
            Self::Allocation => "runtime allocation failed",
            Self::Bounds => "runtime raw address does not name live mapped storage",
        }
    }
}
struct Region {
    base: u64,
    bytes: Vec<u8>,
    live: bool,
}
pub struct Memory {
    regions: Vec<Region>,
    next_address: u64,
    refuse_next: bool,
}
impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
impl Memory {
    pub fn new() -> Self {
        Self {
            regions: vec![Region {
                base: 4096,
                bytes: vec![0; ALLOCATOR_STATE_BYTES as usize],
                live: true,
            }],
            next_address: 8192,
            refuse_next: false,
        }
    }
    /// Inject a kernel mapping refusal for allocation-failure tests.
    pub fn refuse_next_mapping(&mut self) {
        self.refuse_next = true;
    }
    pub fn mapped_bytes(&self) -> usize {
        let mut total = 0;
        for region in &self.regions {
            if region.live {
                total += region.bytes.len();
            }
        }
        total
    }
    pub fn mapping_count(&self) -> usize {
        let mut total = 0;
        for region in &self.regions {
            if region.live {
                total += 1;
            }
        }
        total
    }
    fn range(&self, address: u64, length: u64) -> Result<(usize, usize, usize), MemoryFault> {
        let end = address.checked_add(length).ok_or(MemoryFault::Bounds)?;
        for index in 0usize..self.regions.len() {
            let region = &self.regions[index];
            if region.live
                && address >= region.base
                && end >= address
                && end - region.base <= region.bytes.len() as u64
            {
                return Ok((
                    index,
                    (address - region.base) as usize,
                    (end - region.base) as usize,
                ));
            }
        }
        Err(MemoryFault::Bounds)
    }
    pub fn read(&self, address: u64, length: u64) -> Result<&[u8], MemoryFault> {
        if length == 0 {
            return Ok(&[]);
        }
        let (index, start, end) = self.range(address, length)?;
        Ok(&self.regions[index].bytes[start..end])
    }
    pub fn write(&mut self, address: u64, bytes: &[u8]) -> Result<(), MemoryFault> {
        if bytes.is_empty() {
            return Ok(());
        }
        let (index, start, end) = self.range(address, bytes.len() as u64)?;
        self.regions[index].bytes[start..end].copy_from_slice(bytes);
        Ok(())
    }
    fn load(&self, address: u64, length: u64) -> Result<u64, MemoryFault> {
        let mut result = 0;
        for (index, byte) in self.read(address, length)?.iter().enumerate() {
            result |= u64::from(*byte) << (index * 8);
        }
        Ok(result)
    }
    fn map(&mut self, bytes: u64) -> u64 {
        if self.refuse_next {
            self.refuse_next = false;
            return 0;
        }
        if bytes == 0 {
            return 0;
        }
        let Some(rounded) = bytes.checked_add(4095).map(|size| size & !4095) else {
            return 0;
        };
        let Ok(length) = usize::try_from(rounded) else {
            return 0;
        };
        let Some(next) = self
            .next_address
            .checked_add(rounded)
            .and_then(|end| end.checked_add(4096))
        else {
            return 0;
        };
        let mut storage = Vec::new();
        if storage.try_reserve_exact(length).is_err() {
            return 0;
        }
        storage.resize(length, 0);
        let base = self.next_address;
        let region = Region {
            base,
            bytes: storage,
            live: true,
        };
        let mut unused = None;
        for index in 1usize..self.regions.len() {
            if !self.regions[index].live {
                unused = Some(index);
                break;
            }
        }
        if let Some(index) = unused {
            self.regions[index] = region;
        } else {
            if self.regions.try_reserve(1).is_err() {
                return 0;
            }
            self.regions.push(region);
        }
        self.next_address = next;
        base
    }
    fn unmap(&mut self, address: u64, bytes: u64) -> Result<u64, MemoryFault> {
        let rounded = bytes
            .checked_add(4095)
            .map(|size| size & !4095)
            .ok_or(MemoryFault::Bounds)?;
        for index in 1usize..self.regions.len() {
            if self.regions[index].live
                && self.regions[index].base == address
                && self.regions[index].bytes.len() as u64 == rounded
            {
                self.regions[index].live = false;
                self.regions[index].bytes = Vec::new();
                return Ok(0);
            }
        }
        Err(MemoryFault::Bounds)
    }
    pub(crate) fn invoke(
        &mut self,
        primitive: Primitive,
        args: &[u64],
    ) -> Result<u64, MemoryFault> {
        let value = match primitive {
            Primitive::Load8 => self.load(args[0], 1)?,
            Primitive::Load64 => self.load(args[0], 8)?,
            Primitive::Store8 => {
                self.write(args[0], &[args[1] as u8])?;
                0
            }
            Primitive::Store64 => {
                self.write(args[0], &args[1].to_le_bytes())?;
                0
            }
            Primitive::MemoryCopy => {
                let copied = self.read(args[1], args[2])?.to_vec();
                self.write(args[0], &copied)?;
                0
            }
            Primitive::Map => self.map(args[0]),
            Primitive::Unmap => self.unmap(args[0], args[1])?,
            Primitive::AllocatorState => self.regions[0].base,
            Primitive::Fail => return Err(MemoryFault::Allocation),
        };
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mappings_are_disjoint_and_release_invalidates_raw_addresses() {
        let mut memory = Memory::new();
        let left = memory.invoke(Primitive::Map, &[4096]).unwrap();
        let right = memory.invoke(Primitive::Map, &[4096]).unwrap();
        assert!(right > left + 4096);
        memory.write(left, &[1, 2, 3, 4, 5, 6]).unwrap();
        memory
            .invoke(Primitive::MemoryCopy, &[left + 2, left, 4])
            .unwrap();
        assert_eq!(memory.read(left, 6).unwrap(), &[1, 2, 1, 2, 3, 4]);
        memory
            .invoke(Primitive::MemoryCopy, &[right, left + 2, 4])
            .unwrap();
        assert_eq!(memory.read(right, 4).unwrap(), &[1, 2, 3, 4]);
        assert_eq!(memory.read(left + 4095, 2), Err(MemoryFault::Bounds));
        memory.invoke(Primitive::Unmap, &[left, 4096]).unwrap();
        assert_eq!(memory.read(left, 1), Err(MemoryFault::Bounds));
        assert_eq!(memory.read(right, 4).unwrap(), &[1, 2, 3, 4]);
    }
}
