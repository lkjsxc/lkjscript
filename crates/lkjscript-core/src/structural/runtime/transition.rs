use super::{SlotState, StructuralRuntime};
use crate::structural::{DomainClass, DomainKey, StructuralError};

impl StructuralRuntime {
    pub(in crate::structural) fn transition_batch(
        &mut self,
        keys: &[DomainKey],
        expected: DomainClass,
        next: DomainClass,
    ) -> Result<(), StructuralError> {
        for (index, &key) in keys.iter().enumerate() {
            self.require_live(key)?;
            if key.class() != expected {
                return Err(StructuralError::WrongDomainClass {
                    expected,
                    actual: key.class(),
                });
            }
            if keys[..index].contains(&key) {
                return Err(StructuralError::DuplicateDependency);
            }
        }
        for &key in keys {
            self.slots[key.slot() as usize].state = SlotState::Live(next);
        }
        Ok(())
    }

    pub(in crate::structural) fn require_live(
        &self,
        key: DomainKey,
    ) -> Result<(), StructuralError> {
        if key.runtime() != self.identity {
            return Err(StructuralError::WrongRuntime);
        }
        let Some(slot) = self.slots.get(key.slot() as usize) else {
            return Err(StructuralError::StaleDomain(key));
        };
        if slot.generation != key.generation() {
            return Err(StructuralError::StaleDomain(key));
        }
        match slot.state {
            SlotState::Live(class) if class == key.class() => Ok(()),
            SlotState::Live(actual) => Err(StructuralError::WrongDomainClass {
                expected: key.class(),
                actual,
            }),
            SlotState::Vacant | SlotState::Retired => Err(StructuralError::StaleDomain(key)),
        }
    }
}
