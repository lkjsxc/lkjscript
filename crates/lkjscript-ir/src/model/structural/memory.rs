use super::*;

impl Default for StructuralMemoryMetadata {
    fn default() -> Self {
        Self {
            plan: MemoryPlanId::new([0; 32]),
            witness_groups: Vec::new(),
            witnesses: Vec::new(),
            types: Vec::new(),
            layouts: Vec::new(),
            representations: Vec::new(),
        }
    }
}

impl StructuralMemoryMetadata {
    pub fn witness(&self, id: MemoryWitnessId) -> Option<&MemoryWitnessDescriptor> {
        self.witnesses
            .binary_search_by_key(&id, |item| item.id)
            .ok()
            .and_then(|index| self.witnesses.get(index))
    }

    pub fn type_for(&self, ty: &SsaType) -> Option<&StructuralTypeMetadata> {
        self.types
            .binary_search_by(|item| item.ty.cmp(ty))
            .ok()
            .and_then(|index| self.types.get(index))
    }

    fn representations_for(
        &self,
        type_id: StructuralTypeId,
    ) -> &[StructuralRepresentationMetadata] {
        let start = self
            .representations
            .partition_point(|item| item.type_id < type_id);
        let end = self
            .representations
            .partition_point(|item| item.type_id <= type_id);
        self.representations.get(start..end).unwrap_or(&[])
    }

    pub fn representation(
        &self,
        ty: &SsaType,
        category: StructuralValueCategory,
        storage: StructuralStorage,
    ) -> Option<StructuralRepresentationId> {
        let type_id = self.type_for(ty)?.id;
        let mut candidates = self
            .representations_for(type_id)
            .iter()
            .filter(|item| item.category == category && item.storage == storage);
        let selected = candidates.next()?;
        candidates.next().is_none().then_some(selected.id)
    }

    pub fn representation_by_route(
        &self,
        ty: &SsaType,
        route: [u8; 32],
        category: StructuralValueCategory,
        storage: StructuralStorage,
    ) -> Option<StructuralRepresentationId> {
        let type_id = self.type_for(ty)?.id;
        let mut candidates = self.representations_for(type_id).iter().filter(|item| {
            item.route == route && item.category == category && item.storage == storage
        });
        let selected = candidates.next()?;
        candidates.next().is_none().then_some(selected.id)
    }

    pub fn is_owned(&self, ty: &SsaType) -> bool {
        self.type_for(ty)
            .is_some_and(|item| item.mode != StructuralTypeMode::Copy)
    }

    pub fn is_immutable(&self, ty: &SsaType) -> bool {
        self.type_for(ty)
            .is_some_and(|item| item.mode == StructuralTypeMode::Immutable)
    }
}
