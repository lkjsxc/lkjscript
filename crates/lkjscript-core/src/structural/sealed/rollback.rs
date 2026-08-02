use super::{SealedBuilder, SealedRegionStore};
use crate::structural::{DomainClass, StructuralRuntime};

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub(crate) fn rollback_dropless_builder(
        &mut self,
        runtime: &mut StructuralRuntime,
        builder: SealedBuilder<T, D>,
    ) {
        assert_eq!(runtime.identity(), self.runtime);
        let Some(index) = self.records.iter().position(|(key, _)| *key == builder.key) else {
            unreachable!("sealed builder belongs to its originating store");
        };
        let record = &self.records[index].1;
        assert_eq!(builder.key.class(), DomainClass::RegionBuilding);
        assert_eq!(record.owners, 0);
        assert_eq!(record.loans, 0);
        assert!(record.drops.as_slice().is_empty());
        self.records.swap_remove(index);
        runtime.rollback_allocation(builder.key);
    }
}
