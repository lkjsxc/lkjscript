use super::super::StructuralValueKey;
use super::{
    StructuralDisposeReport, StructuralEventKind, StructuralObject, StructuralOwnerKind,
    StructuralType, StructuralValueError, StructuralValueRuntime,
};

impl StructuralValueRuntime {
    pub fn dispose_owner(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<StructuralDisposeReport, StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        match self.objects.get(root)? {
            StructuralObject::Owned { facts, .. } => {
                let facts = *facts;
                self.drop_owned(key, expected)?;
                Ok(StructuralDisposeReport {
                    ownership: StructuralOwnerKind::Unique,
                    final_release: true,
                    nodes_reclaimed: facts.nodes,
                    bytes_reclaimed: facts.bytes,
                    release_work: facts.nodes,
                })
            }
            StructuralObject::Sealed { .. } => self.dispose_sealed(key, root, expected),
            StructuralObject::Static(_) => Err(StructuralValueError::WrongOwnership),
        }
    }

    fn dispose_sealed(
        &mut self,
        key: StructuralValueKey,
        root: super::super::RootKey,
        expected: StructuralType,
    ) -> Result<StructuralDisposeReport, StructuralValueError> {
        self.require_sealed_root(root, expected)?;
        let owners = self.objects.sealed_owner_count(root)?;
        let facts = self.objects.sealed_facts(root)?;
        let final_release = owners == 1;
        if final_release {
            self.runtime.preflight_release(&[root.domain()])?;
            self.objects.preflight_take(root)?;
        }
        self.roots.release_sealed(key)?;
        if final_release {
            let StructuralObject::Sealed { image, .. } = self.objects.take(root)? else {
                return Err(StructuralValueError::InvariantViolation);
            };
            self.runtime.release(root.domain())?;
            self.note_object_removed(facts);
            drop(image);
            self.metrics.live_sealed_domains = self.metrics.live_sealed_domains.saturating_sub(1);
            self.metrics.sealed_nodes_reclaimed = self
                .metrics
                .sealed_nodes_reclaimed
                .saturating_add(facts.nodes);
            self.metrics.string_bytes_released = self
                .metrics
                .string_bytes_released
                .saturating_add(facts.string_bytes);
            self.metrics.path_bytes_released = self
                .metrics
                .path_bytes_released
                .saturating_add(facts.path_bytes);
        } else {
            self.objects.set_sealed_owner_count(root, owners - 1)?;
        }
        self.metrics.drops = self.metrics.drops.saturating_add(1);
        self.metrics.releases = self.metrics.releases.saturating_add(1);
        self.metrics.sealed_releases = self.metrics.sealed_releases.saturating_add(1);
        self.metrics.live_sealed_owners = self.metrics.live_sealed_owners.saturating_sub(1);
        self.metrics.release_work = self.metrics.release_work.saturating_add(1);
        self.metrics.sealed_release_work = self.metrics.sealed_release_work.saturating_add(1);
        self.record(StructuralEventKind::SealedRelease, key.get(), 1);
        Ok(StructuralDisposeReport {
            ownership: StructuralOwnerKind::Sealed,
            final_release,
            nodes_reclaimed: if final_release { facts.nodes } else { 0 },
            bytes_reclaimed: if final_release { facts.bytes } else { 0 },
            release_work: 1,
        })
    }
}
