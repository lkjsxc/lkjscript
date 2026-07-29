use std::marker::PhantomData;

use lkjscript_contracts::{sha256, ContractDigest};

use crate::{ResourceError, ResourceResult};

macro_rules! generation_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name {
            pub slot: u32,
            pub generation: u32,
        }
        impl $name {
            pub const fn new(slot: u32, generation: u32) -> Self {
                Self { slot, generation }
            }
        }
        impl GenerationId for $name {
            fn from_parts(slot: u32, generation: u32) -> Self {
                Self::new(slot, generation)
            }
            fn parts(self) -> (u32, u32) {
                (self.slot, self.generation)
            }
        }
    };
}

pub trait GenerationId: Copy {
    fn from_parts(slot: u32, generation: u32) -> Self;
    fn parts(self) -> (u32, u32);
}

generation_id!(TaskId);
generation_id!(TaskScopeId);
generation_id!(TaskResultId);
generation_id!(DataOwnerId);
generation_id!(AccessRecordId);
generation_id!(WorkerId);
generation_id!(WorkerGroupId);
generation_id!(ExecutionDomainId);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ResourcePlaneId(pub ContractDigest);

impl ResourcePlaneId {
    pub fn from_content(content: &[u8]) -> Self {
        Self(ContractDigest::from_bytes(sha256(content)))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TaskClassId(pub u64);

impl TaskClassId {
    pub fn from_name(name: &str) -> Self {
        let digest = sha256(name.as_bytes());
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(&digest[..8]);
        Self(u64::from_be_bytes(bytes))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SlotState {
    Vacant,
    Live,
    Retired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GenerationSlot {
    generation: u32,
    state: SlotState,
}

#[derive(Clone, Debug)]
pub struct GenerationTable<I> {
    slots: Vec<GenerationSlot>,
    limit: usize,
    max_generation: u32,
    marker: PhantomData<I>,
}

impl<I: GenerationId> GenerationTable<I> {
    pub fn new(limit: usize) -> Self {
        Self {
            slots: Vec::new(),
            limit,
            max_generation: u32::MAX,
            marker: PhantomData,
        }
    }

    pub fn with_max_generation(limit: usize, max_generation: u32) -> ResourceResult<Self> {
        if max_generation == 0 {
            return Err(ResourceError::new(
                "generation-limit",
                "maximum generation must be nonzero",
            ));
        }
        Ok(Self {
            slots: Vec::new(),
            limit,
            max_generation,
            marker: PhantomData,
        })
    }

    pub fn allocate(&mut self) -> ResourceResult<I> {
        if let Some((slot, entry)) = self
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, entry)| entry.state == SlotState::Vacant)
        {
            entry.state = SlotState::Live;
            return Ok(I::from_parts(slot as u32, entry.generation));
        }
        if self.slots.len() >= self.limit || self.slots.len() > u32::MAX as usize {
            return Err(ResourceError::new(
                "id-capacity",
                "generation table is full",
            ));
        }
        self.slots
            .try_reserve(1)
            .map_err(|_| ResourceError::new("id-allocation", "identifier storage failed"))?;
        let slot = self.slots.len();
        self.slots.push(GenerationSlot {
            generation: 1,
            state: SlotState::Live,
        });
        Ok(I::from_parts(slot as u32, 1))
    }

    pub fn contains(&self, id: I) -> bool {
        let (slot, generation) = id.parts();
        self.slots
            .get(slot as usize)
            .is_some_and(|entry| entry.state == SlotState::Live && entry.generation == generation)
    }

    pub fn release(&mut self, id: I) -> ResourceResult<()> {
        if !self.contains(id) {
            return Err(ResourceError::new("stale-id", "identifier is not live"));
        }
        let slot = id.parts().0 as usize;
        let entry = &mut self.slots[slot];
        if entry.generation >= self.max_generation {
            entry.state = SlotState::Retired;
        } else {
            entry.generation = entry.generation.checked_add(1).ok_or_else(|| {
                ResourceError::new("generation-overflow", "identifier generation exhausted")
            })?;
            entry.state = SlotState::Vacant;
        }
        Ok(())
    }
}
