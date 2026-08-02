use super::super::SemanticDagType;
use super::model::{SealedSemanticDagError, SealedSemanticDagMetrics};
use super::TypedSealedDagStore;
use crate::structural::{SealedRegionMetrics, StructuralLimits, StructuralRuntime};

#[derive(Debug)]
pub struct SealedSemanticDagRuntime {
    pub(super) runtime: StructuralRuntime,
    pub(super) stores: Vec<TypedSealedDagStore>,
    pub(super) limits: StructuralLimits,
}

impl SealedSemanticDagRuntime {
    pub fn new(limits: StructuralLimits) -> Result<Self, SealedSemanticDagError> {
        let runtime = StructuralRuntime::new(limits)?;
        Ok(Self {
            limits: runtime.limits(),
            runtime,
            stores: Vec::new(),
        })
    }

    pub fn metrics(&self) -> SealedSemanticDagMetrics {
        let mut sealed = SealedRegionMetrics::default();
        for typed in &self.stores {
            add_metrics(&mut sealed, typed.store.metrics());
        }
        SealedSemanticDagMetrics {
            typed_stores: u32::try_from(self.stores.len()).unwrap_or(u32::MAX),
            runtime: self.runtime.metrics(),
            sealed,
        }
    }

    pub fn validate(&self) -> Result<(), SealedSemanticDagError> {
        self.runtime.validate()?;
        for typed in &self.stores {
            typed.store.validate(&self.runtime)?;
        }
        Ok(())
    }

    pub(super) fn store(
        &self,
        index: u32,
        value_type: SemanticDagType,
    ) -> Result<&TypedSealedDagStore, SealedSemanticDagError> {
        self.stores
            .get(index as usize)
            .filter(|typed| typed.value_type == value_type)
            .ok_or(SealedSemanticDagError::CorruptRegion)
    }

    pub(super) fn store_mut(
        &mut self,
        index: u32,
        value_type: SemanticDagType,
    ) -> Result<&mut TypedSealedDagStore, SealedSemanticDagError> {
        self.stores
            .get_mut(index as usize)
            .filter(|typed| typed.value_type == value_type)
            .ok_or(SealedSemanticDagError::CorruptRegion)
    }
}

fn add_metrics(total: &mut SealedRegionMetrics, next: SealedRegionMetrics) {
    total.builders_created = total.builders_created.saturating_add(next.builders_created);
    total.regions_sealed = total.regions_sealed.saturating_add(next.regions_sealed);
    total.regions_destroyed = total
        .regions_destroyed
        .saturating_add(next.regions_destroyed);
    total.roots_published = total.roots_published.saturating_add(next.roots_published);
    total.retains = total.retains.saturating_add(next.retains);
    total.releases = total.releases.saturating_add(next.releases);
    total.weak_upgrades = total.weak_upgrades.saturating_add(next.weak_upgrades);
    total.rejected_cycles = total.rejected_cycles.saturating_add(next.rejected_cycles);
    total.dependency_edges = total.dependency_edges.saturating_add(next.dependency_edges);
    total.release_work = total.release_work.saturating_add(next.release_work);
    total.bytes_allocated = total.bytes_allocated.saturating_add(next.bytes_allocated);
}
