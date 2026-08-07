use std::num::NonZeroU64;

use super::super::{DomainKey, RootClass, RootKey};
use super::{StaticStructuralArtifact, StructuralImage, StructuralValueError, TreeFacts};

#[derive(Debug)]
pub(super) enum StructuralObject {
    Owned {
        image: StructuralImage,
        facts: TreeFacts,
    },
    Sealed {
        image: StructuralImage,
        facts: TreeFacts,
        owners: u64,
    },
    Static(StaticStructuralArtifact),
}

impl StructuralObject {
    pub(super) fn value_type(&self) -> super::StructuralType {
        match self {
            Self::Owned { image, .. } | Self::Sealed { image, .. } => image.root().value_type(),
            Self::Static(artifact) => artifact.value_type,
        }
    }
}

#[derive(Debug)]
pub(super) enum ObjectSlot {
    Vacant(NonZeroU64),
    Live {
        generation: NonZeroU64,
        domain: DomainKey,
        object: StructuralObject,
    },
    Retired,
}

#[derive(Debug)]
pub(super) struct ObjectSlab {
    pub(super) slots: Vec<ObjectSlot>,
    pub(super) free: Vec<u64>,
    pub live: u64,
}

impl ObjectSlab {
    pub(super) const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            live: 0,
        }
    }

    pub(super) fn insert(
        &mut self,
        domain: DomainKey,
        class: RootClass,
        object: StructuralObject,
    ) -> Result<(RootKey, bool), Box<(StructuralValueError, StructuralObject)>> {
        if let Err(error) = self.free.try_reserve(1) {
            return Err(Box::new((error.into(), object)));
        }
        let (slot, generation, reused) = if let Some(slot) = self.free.pop() {
            let index = match usize::try_from(slot) {
                Ok(index) => index,
                Err(_) => {
                    return Err(Box::new((StructuralValueError::ArithmeticOverflow, object)));
                }
            };
            let ObjectSlot::Vacant(generation) = self.slots[index] else {
                return Err(Box::new((StructuralValueError::InvariantViolation, object)));
            };
            (slot, generation, true)
        } else {
            let slot = match u64::try_from(self.slots.len()) {
                Ok(slot) => slot,
                Err(_) => {
                    return Err(Box::new((StructuralValueError::ArithmeticOverflow, object)));
                }
            };
            if let Err(error) = self.slots.try_reserve(1) {
                return Err(Box::new((error.into(), object)));
            }
            self.slots.push(ObjectSlot::Vacant(NonZeroU64::MIN));
            (slot, NonZeroU64::MIN, false)
        };
        let value_type = object.value_type();
        let index = match usize::try_from(slot) {
            Ok(index) => index,
            Err(_) => {
                return Err(Box::new((StructuralValueError::ArithmeticOverflow, object)));
            }
        };
        self.slots[index] = ObjectSlot::Live {
            generation,
            domain,
            object,
        };
        self.live += 1;
        Ok((
            RootKey::from_parts(
                domain,
                class,
                slot,
                generation,
                value_type.layout,
                value_type.semantic_type,
            ),
            reused,
        ))
    }

    pub(super) fn get(&self, root: RootKey) -> Result<&StructuralObject, StructuralValueError> {
        let ObjectSlot::Live {
            generation,
            domain,
            object,
        } = self
            .slots
            .get(
                usize::try_from(root.slot())
                    .map_err(|_| StructuralValueError::ArithmeticOverflow)?,
            )
            .ok_or(StructuralValueError::StaleObject)?
        else {
            return Err(StructuralValueError::StaleObject);
        };
        if *generation != root.generation() || *domain != root.domain() {
            return Err(StructuralValueError::StaleObject);
        }
        Ok(object)
    }

    pub(super) fn get_mut(
        &mut self,
        root: RootKey,
    ) -> Result<&mut StructuralObject, StructuralValueError> {
        let ObjectSlot::Live {
            generation,
            domain,
            object,
        } = self
            .slots
            .get_mut(
                usize::try_from(root.slot())
                    .map_err(|_| StructuralValueError::ArithmeticOverflow)?,
            )
            .ok_or(StructuralValueError::StaleObject)?
        else {
            return Err(StructuralValueError::StaleObject);
        };
        if *generation != root.generation() || *domain != root.domain() {
            return Err(StructuralValueError::StaleObject);
        }
        Ok(object)
    }

    pub(super) fn rollback_insert(&mut self, root: RootKey, reused: bool) -> StructuralObject {
        assert!(self.get(root).is_ok());
        let Ok(index) = usize::try_from(root.slot()) else {
            unreachable!("live object slot fits host index")
        };
        assert!(self.live > 0);
        if !reused {
            assert_eq!(index.checked_add(1), Some(self.slots.len()));
        }
        let slot = std::mem::replace(&mut self.slots[index], ObjectSlot::Retired);
        let ObjectSlot::Live { object, .. } = slot else {
            unreachable!("just-inserted structural object");
        };
        if reused {
            self.slots[index] = ObjectSlot::Vacant(root.generation());
            self.free.push(root.slot());
        } else {
            self.slots.pop();
        }
        self.live -= 1;
        object
    }
}

impl From<std::collections::TryReserveError> for StructuralValueError {
    fn from(_: std::collections::TryReserveError) -> Self {
        Self::AllocationFailed
    }
}
