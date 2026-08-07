use std::num::NonZeroU64;

use super::object_slab::{ObjectSlab, ObjectSlot, StructuralObject};
use crate::structural::{DomainClass, RootClass, RootKey, StructuralValueError};

impl ObjectSlab {
    pub(in crate::structural::value_runtime) fn prepare_sealed_root(
        &self,
        unique: RootKey,
    ) -> Result<RootKey, StructuralValueError> {
        if unique.domain().class() != DomainClass::Unique
            || unique.class() != RootClass::UniquePublic
        {
            return Err(StructuralValueError::WrongOwnership);
        }
        let StructuralObject::Owned { image, .. } = self.get(unique)? else {
            return Err(StructuralValueError::WrongOwnership);
        };
        let value_type = image.root().value_type();
        Ok(RootKey::from_parts(
            unique.domain().with_class(DomainClass::RegionSealed),
            RootClass::SealedPublic,
            unique.slot(),
            unique.generation(),
            value_type.layout,
            value_type.semantic_type,
        ))
    }

    pub(in crate::structural::value_runtime) fn seal_in_place(
        &mut self,
        unique: RootKey,
        sealed: RootKey,
    ) -> Result<(), StructuralValueError> {
        let expected = self.prepare_sealed_root(unique)?;
        if expected != sealed {
            return Err(StructuralValueError::InvariantViolation);
        }
        let index =
            usize::try_from(unique.slot()).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        let live = std::mem::replace(&mut self.slots[index], ObjectSlot::Retired);
        let ObjectSlot::Live {
            generation,
            domain: _,
            object: StructuralObject::Owned { image, facts },
        } = live
        else {
            return Err(StructuralValueError::InvariantViolation);
        };
        self.slots[index] = ObjectSlot::Live {
            generation,
            domain: sealed.domain(),
            object: StructuralObject::Sealed {
                image,
                facts,
                owners: 1,
            },
        };
        Ok(())
    }

    pub(in crate::structural::value_runtime) fn sealed_owner_count(
        &self,
        root: RootKey,
    ) -> Result<u64, StructuralValueError> {
        let StructuralObject::Sealed { owners, .. } = self.get(root)? else {
            return Err(StructuralValueError::WrongOwnership);
        };
        Ok(*owners)
    }

    pub(in crate::structural::value_runtime) fn preflight_sealed_acquire(
        &self,
        root: RootKey,
    ) -> Result<u64, StructuralValueError> {
        self.sealed_owner_count(root)?
            .checked_add(1)
            .ok_or(StructuralValueError::OwnerOverflow)
    }

    pub(in crate::structural::value_runtime) fn set_sealed_owner_count(
        &mut self,
        root: RootKey,
        owners: u64,
    ) -> Result<(), StructuralValueError> {
        if owners == 0 {
            return Err(StructuralValueError::InvariantViolation);
        }
        let StructuralObject::Sealed {
            owners: current, ..
        } = self.get_mut(root)?
        else {
            return Err(StructuralValueError::WrongOwnership);
        };
        *current = owners;
        Ok(())
    }

    pub(in crate::structural::value_runtime) fn sealed_facts(
        &self,
        root: RootKey,
    ) -> Result<super::TreeFacts, StructuralValueError> {
        let StructuralObject::Sealed { facts, .. } = self.get(root)? else {
            return Err(StructuralValueError::WrongOwnership);
        };
        Ok(*facts)
    }

    pub(in crate::structural::value_runtime) fn preflight_take(
        &mut self,
        root: RootKey,
    ) -> Result<(), StructuralValueError> {
        self.get(root)?;
        if root.generation().get() < u64::MAX {
            self.free.try_reserve(1)?;
        }
        Ok(())
    }

    pub(in crate::structural::value_runtime) fn take(
        &mut self,
        root: RootKey,
    ) -> Result<StructuralObject, StructuralValueError> {
        self.preflight_take(root)?;
        let index =
            usize::try_from(root.slot()).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        let replacement = if root.generation().get() == u64::MAX {
            ObjectSlot::Retired
        } else {
            let generation = NonZeroU64::new(root.generation().get() + 1)
                .ok_or(StructuralValueError::InvariantViolation)?;
            self.free.push(root.slot());
            ObjectSlot::Vacant(generation)
        };
        let ObjectSlot::Live { object, .. } =
            std::mem::replace(&mut self.slots[index], replacement)
        else {
            return Err(StructuralValueError::InvariantViolation);
        };
        self.live -= 1;
        Ok(object)
    }
}
