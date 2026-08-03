use super::super::{
    DomainClass, RootClass, StructuralRootOwnership, StructuralRootTableStats, StructuralValueKey,
};
use super::{
    StaticStructuralArtifact, StructuralEventKind, StructuralObject, StructuralType,
    StructuralValueError, StructuralValueRuntime,
};

impl StructuralValueRuntime {
    pub fn register_static(
        &mut self,
        artifact: StaticStructuralArtifact,
    ) -> Result<StructuralValueKey, StructuralValueError> {
        self.validate_static(artifact)?;
        let domain = self.runtime.allocate(DomainClass::Static)?;
        let (root, reused) = match self.objects.insert(
            domain,
            RootClass::StaticPublic,
            StructuralObject::Static(artifact),
        ) {
            Ok(root) => root,
            Err(failure) => {
                let (error, _) = *failure;
                self.runtime.rollback_allocation(domain);
                return Err(error);
            }
        };
        match self.roots.publish(root, StructuralRootOwnership::Static) {
            Ok(key) => {
                self.metrics.live_objects = self.metrics.live_objects.saturating_add(1);
                self.note_slot_reuse(reused);
                self.record(StructuralEventKind::StaticRegister, key.slot(), 0);
                Ok(key)
            }
            Err(error) => {
                self.objects.rollback_insert(root, reused);
                self.runtime.rollback_allocation(domain);
                Err(error.into())
            }
        }
    }

    pub fn unregister_static(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<(), StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        let StructuralObject::Static(artifact) = self.objects.get(root)? else {
            return Err(StructuralValueError::WrongPayloadKind);
        };
        self.require_type(artifact.value_type, expected)?;
        self.runtime.preflight_release(&[root.domain()])?;
        self.objects.preflight_take(root)?;
        let root = self.roots.unregister_static(key)?;
        let StructuralObject::Static(_) = self.objects.take(root)? else {
            return Err(StructuralValueError::InvariantViolation);
        };
        self.runtime.release(root.domain())?;
        self.metrics.live_objects = self.metrics.live_objects.saturating_sub(1);
        self.record(StructuralEventKind::StaticUnregister, key.slot(), 0);
        Ok(())
    }

    pub fn verify_empty(&self) -> Result<(), StructuralValueError> {
        self.roots.assert_no_live_roots()?;
        if self.metrics.live_destinations != 0 {
            return Err(StructuralValueError::LiveDestination);
        }
        if self.metrics.live_views != 0 {
            return Err(StructuralValueError::LiveView);
        }
        if self.objects.live != 0 {
            return Err(StructuralValueError::LiveObject);
        }
        if self.metrics.release_backlog != 0 {
            return Err(StructuralValueError::ReleaseBacklog);
        }
        self.runtime.validate()?;
        Ok(())
    }

    pub const fn root_stats(&self) -> StructuralRootTableStats {
        self.roots.stats()
    }
}
