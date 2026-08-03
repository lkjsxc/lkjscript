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
        self.types.iter().find(|item| &item.ty == ty)
    }

    pub fn representation(
        &self,
        ty: &SsaType,
        category: StructuralValueCategory,
        storage: StructuralStorage,
    ) -> Option<StructuralRepresentationId> {
        let type_id = self.type_for(ty)?.id;
        let mut candidates = self.representations.iter().filter(|item| {
            item.type_id == type_id && item.category == category && item.storage == storage
        });
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
        let mut candidates = self.representations.iter().filter(|item| {
            item.type_id == type_id
                && item.route == route
                && item.category == category
                && item.storage == storage
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
