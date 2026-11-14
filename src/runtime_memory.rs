//! Independent raw-address model for the sealed source allocator.
//! Mappings are disjoint, independently released byte regions.
// The raw interface is modelled here independently of its native seal/emitter.
const ALLOCATOR_STATE_BYTES: usize = (64 + 3) * 8;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Primitive {
    Load8,
    Load64,
    Store8,
    Store64,
    MemoryCopy,
    Map,
    Unmap,
    Remap,
    AllocatorState,
    Fail,
}
impl Primitive {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "raw_load8" => Some(Self::Load8),
            "raw_load64" => Some(Self::Load64),
            "raw_store8" => Some(Self::Store8),
            "raw_store64" => Some(Self::Store64),
            "raw_memory_copy" => Some(Self::MemoryCopy),
            "raw_map" => Some(Self::Map),
            "raw_unmap" => Some(Self::Unmap),
            "raw_remap" => Some(Self::Remap),
            "raw_allocator_state" => Some(Self::AllocatorState),
            "raw_fail" => Some(Self::Fail),
            _ => None,
        }
    }
    pub(crate) fn parameters(self) -> usize {
        match self {
            Self::Load8 | Self::Load64 | Self::Map => 1,
            Self::Store8 | Self::Store64 | Self::Unmap => 2,
            Self::MemoryCopy | Self::Remap => 3,
            Self::AllocatorState | Self::Fail => 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
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
    refuse_all: bool,
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
            refuse_all: false,
        }
    }
    /// Inject a kernel mapping refusal for allocation-failure tests.
    pub fn refuse_next_mapping(&mut self) {
        self.refuse_next = true;
    }
    /// Model a kernel that cannot supply another mapping, including retry attempts.
    pub fn refuse_mappings(&mut self) {
        self.refuse_all = true;
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
            return Ok(&self.regions[0].bytes[0..0]);
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
            result |= (*byte as u64) << (index * 8);
        }
        Ok(result)
    }
    fn map(&mut self, bytes: u64) -> u64 {
        if self.refuse_next || self.refuse_all {
            self.refuse_next = false;
            return 0;
        }
        if bytes == 0 {
            return 0;
        }
        let Some(rounded) = bytes.checked_add(4095).map(|size| size & !4095) else {
            return 0;
        };
        let length = rounded as usize;
        let Some(next) = self
            .next_address
            .checked_add(rounded)
            .and_then(|end| end.checked_add(4096))
        else {
            return 0;
        };
        let mut storage = Vec::new();
        for _ in 0..length {
            storage.push(0u8);
        }
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
    fn remap(&mut self, address: u64, old_bytes: u64, new_bytes: u64) -> Result<u64, MemoryFault> {
        let old_rounded = old_bytes
            .checked_add(4095)
            .map(|size| size & !4095)
            .ok_or(MemoryFault::Bounds)?;
        let (index, start, end) = self.range(address, old_rounded)?;
        if index == 0 || start != 0 || end != self.regions[index].bytes.len() {
            return Err(MemoryFault::Bounds);
        }
        let Some(new_rounded) = new_bytes.checked_add(4095).map(|size| size & !4095) else {
            return Ok(0);
        };
        let replacement = self.map(new_bytes);
        if replacement == 0 {
            return Ok(0);
        }
        let copied = old_rounded.min(new_rounded);
        let bytes = self.read(address, copied)?.to_vec();
        self.write(replacement, &bytes)?;
        self.unmap(address, old_bytes)?;
        Ok(replacement)
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
                let mut bytes = Vec::new();
                let mut index = 0usize;
                while index < 8 {
                    bytes.push((args[1] >> (index * 8)) as u8);
                    index += 1;
                }
                self.write(args[0], &bytes)?;
                0
            }
            Primitive::MemoryCopy => {
                let copied = self.read(args[1], args[2])?.to_vec();
                self.write(args[0], &copied)?;
                0
            }
            Primitive::Map => self.map(args[0]),
            Primitive::Unmap => self.unmap(args[0], args[1])?,
            Primitive::Remap => self.remap(args[0], args[1], args[2])?,
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

    #[test]
    fn remapping_preserves_contents_and_refusal_keeps_the_old_mapping_live() {
        let mut memory = Memory::new();
        let original = memory.invoke(Primitive::Map, &[17]).unwrap();
        memory.write(original, &[1, 2, 3, 4]).unwrap();
        memory.write(original + 4095, &[173]).unwrap();
        memory.refuse_next_mapping();
        assert_eq!(
            memory.invoke(Primitive::Remap, &[original, 17, 8192]),
            Ok(0)
        );
        assert_eq!(memory.read(original, 4).unwrap(), &[1, 2, 3, 4]);
        let grown = memory
            .invoke(Primitive::Remap, &[original, 4096, 8192])
            .unwrap();
        assert_ne!(grown, 0);
        assert_eq!(memory.read(original, 1), Err(MemoryFault::Bounds));
        assert_eq!(memory.read(grown, 4).unwrap(), &[1, 2, 3, 4]);
        assert_eq!(memory.read(grown + 4095, 2).unwrap(), &[173, 0]);
        assert_eq!(memory.read(grown + 8191, 1).unwrap(), &[0]);
        assert_eq!(memory.mapping_count(), 2);
        assert_eq!(
            memory.invoke(Primitive::Remap, &[grown + 1, 4096, 8192]),
            Err(MemoryFault::Bounds)
        );
    }
}
