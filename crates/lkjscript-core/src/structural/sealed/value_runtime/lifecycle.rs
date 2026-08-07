use super::super::{DomainClass, RootClass, StructuralRootOwnership, StructuralValueKey};
use super::{
    StructuralEventKind, StructuralImage, StructuralObject, StructuralSealResult, StructuralType,
    StructuralValueError, StructuralValueRuntime, TreeFacts,
};

impl StructuralValueRuntime {
    pub fn seal_owned(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<StructuralSealResult, StructuralValueError> {
        let unique = self.resolve_root(key, expected)?;
        self.require_owned_root(unique, expected)?;
        let sealed = self.objects.prepare_sealed_root(unique)?;
        self.roots.preflight_owned_to_sealed(key, sealed)?;
        self.runtime.require_live(unique.domain())?;
        let owner = self.roots.replace_owned_with_sealed(key, sealed)?;
        self.objects.seal_in_place(unique, sealed)?;
        self.runtime.transition_batch(
            &[unique.domain()],
            DomainClass::Unique,
            DomainClass::RegionSealed,
        )?;
        self.note_sealed_publication(true, 0);
        self.record(StructuralEventKind::Seal, owner.get(), 0);
        Ok(StructuralSealResult {
            owner,
            zero_copy_adopted: true,
        })
    }

    pub(in crate::structural::value_runtime) fn publish_sealed_image(
        &mut self,
        image: StructuralImage,
        facts: TreeFacts,
        zero_copy: bool,
        copied_bytes: u64,
    ) -> Result<StructuralSealResult, Box<(StructuralValueError, StructuralImage)>> {
        if let Err(error) = image.validate(facts) {
            return Err(Box::new((error, image)));
        }
        let next_allocation = match self.next_allocation_event() {
            Ok(next) => next,
            Err(error) => return Err(Box::new((error, image))),
        };
        let domain = match self.runtime.allocate(DomainClass::RegionSealed) {
            Ok(domain) => domain,
            Err(error) => return Err(Box::new((error.into(), image))),
        };
        let object = StructuralObject::Sealed {
            image,
            facts,
            owners: 1,
        };
        let (root, reused) = match self.objects.insert(domain, RootClass::SealedPublic, object) {
            Ok(root) => root,
            Err(failure) => {
                let (error, object) = *failure;
                self.runtime.rollback_allocation(domain);
                return Err(Box::new((error, sealed_image(object))));
            }
        };
        match self
            .roots
            .publish(root, StructuralRootOwnership::SealedShared)
        {
            Ok(owner) => {
                self.allocation_events = next_allocation;
                self.note_publication(facts);
                self.note_slot_reuse(reused);
                self.note_sealed_publication(zero_copy, copied_bytes);
                self.record(StructuralEventKind::Allocate, root.slot(), facts.nodes);
                self.record(StructuralEventKind::Seal, owner.get(), copied_bytes);
                Ok(StructuralSealResult {
                    owner,
                    zero_copy_adopted: zero_copy,
                })
            }
            Err(error) => {
                let object = self.objects.rollback_insert(root, reused);
                self.runtime.rollback_allocation(domain);
                Err(Box::new((error.into(), sealed_image(object))))
            }
        }
    }

    pub fn independent_owner(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<StructuralValueKey, StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        match self.objects.get(root)? {
            StructuralObject::Owned { .. } => self.clone_owned(key, expected),
            StructuralObject::Sealed { .. } => self.acquire_sealed(key, expected),
            StructuralObject::Static(_) => Err(StructuralValueError::WrongOwnership),
        }
    }

    pub fn move_owner(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<StructuralValueKey, StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        match self.objects.get(root)? {
            StructuralObject::Owned { .. } => self.move_owned(key, expected),
            StructuralObject::Sealed { .. } => self.move_sealed(key, expected),
            StructuralObject::Static(_) => Err(StructuralValueError::WrongOwnership),
        }
    }

    pub fn acquire_sealed(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<StructuralValueKey, StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        self.require_sealed_root(root, expected)?;
        let owners = self.objects.preflight_sealed_acquire(root)?;
        let acquired = self
            .roots
            .publish(root, StructuralRootOwnership::SealedShared)?;
        self.objects.set_sealed_owner_count(root, owners)?;
        self.metrics.sealed_acquisitions = self.metrics.sealed_acquisitions.saturating_add(1);
        self.metrics.live_sealed_owners = self.metrics.live_sealed_owners.saturating_add(1);
        self.record(StructuralEventKind::SealedAcquire, acquired.get(), 1);
        Ok(acquired)
    }

    pub fn move_sealed(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<StructuralValueKey, StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        self.require_sealed_root(root, expected)?;
        let moved = self.roots.move_sealed(key)?;
        self.metrics.moves = self.metrics.moves.saturating_add(1);
        self.record(StructuralEventKind::Move, key.get(), moved.get());
        Ok(moved)
    }

    pub fn sealed_owners_for(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<u64, StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        self.require_sealed_root(root, expected)?;
        self.objects.sealed_owner_count(root)
    }

    fn note_sealed_publication(&mut self, zero_copy: bool, copied_bytes: u64) {
        self.metrics.sealed_publications = self.metrics.sealed_publications.saturating_add(1);
        self.metrics.zero_copy_adoptions = self
            .metrics
            .zero_copy_adoptions
            .saturating_add(u64::from(zero_copy));
        self.metrics.copied_publication_bytes = self
            .metrics
            .copied_publication_bytes
            .saturating_add(copied_bytes);
        self.metrics.live_sealed_domains = self.metrics.live_sealed_domains.saturating_add(1);
        self.metrics.live_sealed_owners = self.metrics.live_sealed_owners.saturating_add(1);
    }
}

fn sealed_image(object: StructuralObject) -> StructuralImage {
    match object {
        StructuralObject::Sealed { image, .. } => image,
        StructuralObject::Owned { .. } | StructuralObject::Static(_) => {
            unreachable!("sealed publication object")
        }
    }
}
