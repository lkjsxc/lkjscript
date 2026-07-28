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

#[derive(Clone, Debug)]
pub struct GenerationTable<I> {
    generations: Vec<u32>,
    live: Vec<bool>,
    limit: usize,
    marker: PhantomData<I>,
}

impl<I: GenerationId> GenerationTable<I> {
    pub fn new(limit: usize) -> Self {
        Self {
            generations: Vec::new(),
            live: Vec::new(),
            limit,
            marker: PhantomData,
        }
    }

    pub fn allocate(&mut self) -> ResourceResult<I> {
        if let Some(slot) = self.live.iter().position(|live| !live) {
            self.live[slot] = true;
            return Ok(I::from_parts(slot as u32, self.generations[slot]));
        }
        if self.live.len() >= self.limit || self.live.len() > u32::MAX as usize {
            return Err(ResourceError::new(
                "id-capacity",
                "generation table is full",
            ));
        }
        let slot = self.live.len();
        self.live.push(true);
        self.generations.push(1);
        Ok(I::from_parts(slot as u32, 1))
    }

    pub fn contains(&self, id: I) -> bool {
        let (slot, generation) = id.parts();
        self.live.get(slot as usize) == Some(&true)
            && self.generations.get(slot as usize) == Some(&generation)
    }

    pub fn release(&mut self, id: I) -> ResourceResult<()> {
        if !self.contains(id) {
            return Err(ResourceError::new("stale-id", "identifier is not live"));
        }
        let slot = id.parts().0 as usize;
        self.live[slot] = false;
        self.generations[slot] = self.generations[slot].checked_add(1).ok_or_else(|| {
            ResourceError::new("generation-overflow", "identifier generation exhausted")
        })?;
        Ok(())
    }
}
