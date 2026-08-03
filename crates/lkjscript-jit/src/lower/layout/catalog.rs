use super::*;

pub(in crate::lower) struct StructuralCatalog {
    pub(super) plan: lkjscript_ir::MemoryPlanId,
    pub(super) types: HashMap<SsaType, lkjscript_native::StructuralTypeIdentity>,
    pub(super) type_ids: HashMap<lkjscript_ir::StructuralTypeId, SsaType>,
    pub(super) modes: HashMap<SsaType, lkjscript_ir::StructuralTypeMode>,
    pub(super) layouts: HashMap<lkjscript_ir::StructuralTypeId, lkjscript_ir::StructuralLayoutKind>,
    pub(super) representations: HashMap<
        lkjscript_ir::StructuralRepresentationId,
        (
            lkjscript_ir::StructuralTypeId,
            lkjscript_ir::StructuralValueCategory,
            lkjscript_ir::StructuralStorage,
        ),
    >,
}

impl Default for StructuralCatalog {
    fn default() -> Self {
        Self {
            plan: lkjscript_ir::MemoryPlanId::new([0; 32]),
            types: HashMap::new(),
            type_ids: HashMap::new(),
            modes: HashMap::new(),
            layouts: HashMap::new(),
            representations: HashMap::new(),
        }
    }
}

impl StructuralCatalog {
    pub(in crate::lower) fn build(program: &lkjscript_ir::Program) -> Result<Self, LoweringError> {
        let mut catalog = Self {
            plan: program.memory.plan,
            ..Self::default()
        };
        for item in &program.memory.types {
            let layout = program
                .memory
                .layouts
                .get(item.layout.index().unwrap_or(usize::MAX))
                .filter(|layout| layout.id == item.layout)
                .ok_or_else(|| invalid_structural("structural type layout is missing"))?;
            let kind = match layout.kind {
                lkjscript_ir::StructuralLayoutKind::String => {
                    lkjscript_native::StructuralKind::String
                }
                lkjscript_ir::StructuralLayoutKind::Path => lkjscript_native::StructuralKind::Path,
                lkjscript_ir::StructuralLayoutKind::Product { .. } => {
                    lkjscript_native::StructuralKind::Product
                }
                lkjscript_ir::StructuralLayoutKind::Enum { .. } => {
                    lkjscript_native::StructuralKind::Enum
                }
            };
            let runtime_type = lkjscript_ir::runtime_structural_type(Some(program), &item.ty)
                .map_err(|error| invalid_structural(&error.to_string()))?
                .ok_or_else(|| invalid_structural("structural runtime type is missing"))?;
            let value_type = lkjscript_native::StructuralTypeIdentity::new(
                runtime_type.layout.get(),
                runtime_type.semantic_type.get(),
                kind,
                item.mode == lkjscript_ir::StructuralTypeMode::Copy,
            );
            if catalog.types.insert(item.ty.clone(), value_type).is_some()
                || catalog.modes.insert(item.ty.clone(), item.mode).is_some()
                || catalog.type_ids.insert(item.id, item.ty.clone()).is_some()
                || catalog
                    .layouts
                    .insert(item.id, layout.kind.clone())
                    .is_some()
            {
                return Err(invalid_structural(
                    "structural type identities are not unique",
                ));
            }
        }
        for representation in &program.memory.representations {
            if !catalog.type_ids.contains_key(&representation.type_id)
                || catalog
                    .representations
                    .insert(
                        representation.id,
                        (
                            representation.type_id,
                            representation.category,
                            representation.storage,
                        ),
                    )
                    .is_some()
            {
                return Err(invalid_structural(
                    "structural representation metadata is inconsistent",
                ));
            }
        }
        Ok(catalog)
    }

    pub(in crate::lower) fn selected(&self, ty: &SsaType) -> bool {
        self.types.contains_key(ty)
    }

    pub(in crate::lower) fn value_type(
        &self,
        ty: &SsaType,
    ) -> Option<lkjscript_native::StructuralTypeIdentity> {
        self.types
            .get(ty)
            .copied()
            .or_else(|| scalar_structural_type(self.plan, ty))
    }

    pub(in crate::lower) fn copy_type(&self, ty: &SsaType) -> bool {
        self.modes
            .get(ty)
            .is_some_and(|mode| *mode == lkjscript_ir::StructuralTypeMode::Copy)
    }

    pub(in crate::lower) fn owner_type(&self, ty: &SsaType) -> Option<ValueType> {
        self.types.get(ty).copied().map(ValueType::StructuralOwner)
    }

    pub(in crate::lower) fn type_id(
        &self,
        ty: &SsaType,
    ) -> Result<lkjscript_ir::StructuralTypeId, LoweringError> {
        let mut matches = self
            .type_ids
            .iter()
            .filter_map(|(type_id, candidate)| (candidate == ty).then_some(*type_id));
        let selected = matches
            .next()
            .ok_or_else(|| invalid_structural("structural type identity is absent"))?;
        if matches.next().is_some() {
            return Err(invalid_structural("structural type identity is ambiguous"));
        }
        Ok(selected)
    }

    pub(in crate::lower) fn representation(
        &self,
        representation: lkjscript_ir::StructuralRepresentationId,
        category: lkjscript_ir::StructuralValueCategory,
    ) -> Result<(lkjscript_ir::StructuralTypeId, SsaType), LoweringError> {
        let (type_id, actual, _) = self
            .representations
            .get(&representation)
            .copied()
            .ok_or_else(|| invalid_structural("structural representation is absent"))?;
        if actual != category {
            return Err(invalid_structural(
                "structural representation has the wrong value category",
            ));
        }
        let ty = self
            .type_ids
            .get(&type_id)
            .cloned()
            .ok_or_else(|| invalid_structural("structural representation type is absent"))?;
        Ok((type_id, ty))
    }

    pub(in crate::lower) fn representation_storage(
        &self,
        representation: lkjscript_ir::StructuralRepresentationId,
        category: lkjscript_ir::StructuralValueCategory,
    ) -> Result<lkjscript_native::StructuralStorageRoute, LoweringError> {
        let (_, actual, storage) = self
            .representations
            .get(&representation)
            .copied()
            .ok_or_else(|| invalid_structural("structural representation is absent"))?;
        if actual != category {
            return Err(invalid_structural(
                "structural representation has the wrong value category",
            ));
        }
        match storage {
            lkjscript_ir::StructuralStorage::UniqueStructural => {
                Ok(lkjscript_native::StructuralStorageRoute::Unique)
            }
            lkjscript_ir::StructuralStorage::SealedRegion => {
                Ok(lkjscript_native::StructuralStorageRoute::Sealed)
            }
            _ => Err(invalid_structural(
                "native structural representation storage is unsupported",
            )),
        }
    }
}
