//! Raw-memory model for exercising the same runtime source on IR. This byte arena
//! is separate from the interpreter's independently typed owner/loan heap.
use crate::runtime::Primitive;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MemoryFault {
    Allocation,
    Bounds,
    DropTarget,
}
impl MemoryFault {
    pub fn message(self) -> &'static str {
        match self {
            Self::Allocation => "runtime allocation failed",
            Self::Bounds => "runtime raw memory access is out of bounds",
            Self::DropTarget => "runtime cleanup target is missing",
        }
    }
}
pub(crate) enum Action {
    Value(u64),
    Call { function: usize, argument: u64 },
}
pub struct Memory {
    bytes: Vec<u8>,
    heap_tip: u64,
    classes: u64,
    drop_steps: Vec<usize>,
}
impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}
impl Memory {
    pub fn new() -> Self {
        Self {
            bytes: vec![0; 65536],
            heap_tip: 8,
            classes: 0,
            drop_steps: Vec::new(),
        }
    }
    pub fn pages(&self) -> u64 {
        self.bytes.len() as u64 / 65536
    }
    pub fn heap_tip(&self) -> u64 {
        self.heap_tip
    }
    pub fn set_drop_steps(&mut self, functions: Vec<usize>) {
        self.drop_steps = functions;
    }
    fn range(&self, address: u64, length: u64) -> Result<std::ops::Range<usize>, MemoryFault> {
        let end = address.checked_add(length).ok_or(MemoryFault::Bounds)?;
        if end > self.bytes.len() as u64 {
            return Err(MemoryFault::Bounds);
        }
        Ok(address as usize..end as usize)
    }
    pub fn read(&self, address: u64, length: u64) -> Result<&[u8], MemoryFault> {
        Ok(&self.bytes[self.range(address, length)?])
    }
    pub fn write(&mut self, address: u64, bytes: &[u8]) -> Result<(), MemoryFault> {
        let range = self.range(address, bytes.len() as u64)?;
        self.bytes[range].copy_from_slice(bytes);
        Ok(())
    }
    fn load(&self, address: u64, length: u64) -> Result<u64, MemoryFault> {
        let mut value = 0;
        for (index, byte) in self.read(address, length)?.iter().enumerate() {
            value |= u64::from(*byte) << (index * 8);
        }
        Ok(value)
    }
    fn grow(&mut self, pages: u64) -> u64 {
        let old = self.pages();
        let Some(end) = old.checked_add(pages).and_then(|n| n.checked_mul(65536)) else {
            return u64::MAX;
        };
        let Ok(end) = usize::try_from(end) else {
            return u64::MAX;
        };
        let extra = end - self.bytes.len();
        if self.bytes.try_reserve(extra).is_err() {
            return u64::MAX;
        }
        self.bytes.resize(end, 0);
        old
    }
    pub(crate) fn invoke(
        &mut self,
        primitive: Primitive,
        args: &[u64],
    ) -> Result<Action, MemoryFault> {
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
            Primitive::MemorySize => self.pages(),
            Primitive::MemoryGrow => self.grow(args[0]),
            Primitive::MemoryCopy => {
                let source = self.range(args[1], args[2])?;
                let destination = self.range(args[0], args[2])?;
                self.bytes.copy_within(source, destination.start);
                0
            }
            Primitive::HeapTip => self.heap_tip,
            Primitive::SetHeapTip => {
                self.heap_tip = args[0];
                0
            }
            Primitive::Classes => self.classes,
            Primitive::SetClasses => {
                self.classes = args[0];
                0
            }
            Primitive::Fail => return Err(MemoryFault::Allocation),
            Primitive::DropStep => {
                let index = usize::try_from(args[1]).map_err(|_| MemoryFault::DropTarget)?;
                let function = *self.drop_steps.get(index).ok_or(MemoryFault::DropTarget)?;
                return Ok(Action::Call {
                    function,
                    argument: args[0],
                });
            }
        };
        Ok(Action::Value(value))
    }
}
