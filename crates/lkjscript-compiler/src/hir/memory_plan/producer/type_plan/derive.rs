impl TypePlanner<'_> {
    fn derive(&mut self, ty: &Type) -> Result<DerivedType> {
        let deterministic = |mode, dynamic, borrow| DerivedType {
            mode,
            closure: closed(MemoryClosureClass::Deterministic),
            contains_borrow: borrow,
            contains_dynamic_owner: dynamic,
        };
        Ok(match ty {
            Type::Never
            | Type::Unit
            | Type::Bool
            | Type::I64
            | Type::F64
            | Type::Capability(_)
            | Type::Symbol => deterministic(MemoryAggregateMode::Copy, false, false),
            Type::Str | Type::Path => {
                deterministic(MemoryAggregateMode::ImmutableValue, true, false)
            }
            Type::Bytes | Type::ByteVector => {
                deterministic(MemoryAggregateMode::Affine, true, false)
            }
            Type::ByteSlice | Type::ByteSliceMut => {
                deterministic(MemoryAggregateMode::ImmutableValue, false, true)
            }
            Type::Resource(_) => deterministic(MemoryAggregateMode::Affine, true, false),
            Type::List(inner) => self.derive_list(ty, inner)?,
            Type::Param(_) => unresolved(ty, MemoryBlockerReason::UnknownTypeParameter),
            Type::Fn { .. } | Type::Forall { .. } => {
                unresolved(ty, MemoryBlockerReason::CapturedClosure)
            }
            Type::Product(name) => self.derive_product(name)?,
            Type::Enum { id, arguments, .. } => self.derive_enum(id.bytes(), arguments)?,
        })
    }

    fn derive_list(&mut self, ty: &Type, inner: &Type) -> Result<DerivedType> {
        let child = self.intern(inner)?;
        let fact = self.fact(child)?.clone();
        let scalar_element = list_region_element(inner)
            && fact.mode == MemoryAggregateMode::Copy
            && fact.closure.class == MemoryClosureClass::Deterministic
            && !fact.contains_borrow;
        if scalar_element || self.selected_copy_list_element(inner, &fact) {
            return Ok(DerivedType {
                mode: MemoryAggregateMode::ImmutableValue,
                closure: MemoryClosureFact {
                    class: MemoryClosureClass::RegionClosed,
                    blocker_path: Vec::new(),
                    blocker_type: Some(memory_type(ty)),
                    blocker_reason: Some(MemoryBlockerReason::RegionDomainBoundary),
                    mixed_direction: None,
                },
                contains_borrow: false,
                contains_dynamic_owner: false,
            });
        }
        let mut result = unresolved(ty, MemoryBlockerReason::ListElementWitnessRequired);
        result.mode = fact.mode;
        result.contains_borrow = fact.contains_borrow;
        result.contains_dynamic_owner = fact.contains_dynamic_owner;
        Ok(result)
    }

    fn selected_copy_list_element(&self, ty: &Type, fact: &MemoryTypeFact) -> bool {
        matches!(ty, Type::List(_))
            && fact.mode == MemoryAggregateMode::ImmutableValue
            && fact.closure.class == MemoryClosureClass::RegionClosed
            && !fact.contains_borrow
            && !fact.contains_dynamic_owner
            && self.witnesses.iter().find(|item| item.id == fact.witness)
                .filter(|item| item.facts.requirement == MemoryWitnessRequirement::Concrete)
                .and_then(|item| item.facts.list.as_ref())
                .is_some_and(|list| list.selected
                    && list.eligibility == MemoryListElementEligibility::Copy
                    && list.storage == MemoryListStorageKind::SegmentedSessionRegion
                    && list.segment_capacity == 32)
    }

    fn derive_product(&mut self, name: &str) -> Result<DerivedType> {
        let key = DeclarationKey::Product(name.to_owned());
        let definition = self
            .program
            .products
            .iter()
            .find(|item| item.name == name)
            .cloned()
            .ok_or_else(|| {
                Error::msg(format!(
                    "HIR memory plan references unknown product {name}"
                ))
            })?;
        self.charge_fields(definition.fields.len())?;
        if self.graph.is_recursive(&key) {
            return self.derive_recursive(&key, &[]);
        }
        let mut children = Vec::with_capacity(definition.fields.len());
        for (index, field) in definition.fields.iter().enumerate() {
            let id = self.intern(&field.ty)?;
            children.push((
                self.fact(id)?.clone(),
                MemoryTypePathElement::ProductField {
                    index: index_u32(index)?,
                    name: field.name.clone(),
                },
            ));
        }
        let region_capable = definition
            .fields
            .iter()
            .zip(&children)
            .all(|(field, (fact, _))| region_product_field(&field.ty, fact));
        let derived = fold_aggregate(children, false, region_capable);
        if derived.closure.class == MemoryClosureClass::Unresolved {
            return Err(Error::msg(format!(
                "LKJ-MEM-PRODUCT-UNRESOLVED product={name} blocker={:?} path={:?}",
                derived.closure.blocker_reason, derived.closure.blocker_path
            )));
        }
        Ok(derived)
    }

    fn derive_enum(&mut self, id: [u8; 32], arguments: &[Type]) -> Result<DerivedType> {
        let key = DeclarationKey::Enum(id);
        let definition = self
            .program
            .enums
            .iter()
            .find(|item| item.id.bytes() == id)
            .cloned()
            .ok_or_else(|| Error::msg("HIR memory plan references unknown enum"))?;
        if definition.type_parameters.len() != arguments.len() {
            return Err(Error::msg(
                "HIR memory-plan enum substitution arity mismatch",
            ));
        }
        self.charge_variants(definition.variants.len())?;
        self.charge_fields(
            definition
                .variants
                .iter()
                .map(|variant| variant.fields.len())
                .sum(),
        )?;
        if self.graph.is_recursive(&key) {
            return self.derive_recursive(&key, arguments);
        }
        let substitutions: HashMap<_, _> = definition
            .type_parameters
            .iter()
            .cloned()
            .zip(arguments.iter().cloned())
            .collect();
        let mut children = Vec::new();
        for (variant_index, variant) in definition.variants.iter().enumerate() {
            for (field_index, field) in variant.fields.iter().enumerate() {
                let ty = field.ty.subst(&substitutions);
                let child = self.intern(&ty)?;
                let fact = self.fact(child)?.clone();
                children.push((
                    fact,
                    MemoryTypePathElement::EnumVariantField {
                        variant_index: index_u32(variant_index)?,
                        variant: variant.id.bytes(),
                        field_index: index_u32(field_index)?,
                        field: field.id.bytes(),
                    },
                ));
            }
        }
        let derived = fold_aggregate(children, false, false);
        if derived.closure.class == MemoryClosureClass::Unresolved {
            return Err(Error::msg(format!(
                "LKJ-MEM-ENUM-UNRESOLVED enum={} blocker={:?} path={:?}",
                definition.name, derived.closure.blocker_reason, derived.closure.blocker_path
            )));
        }
        Ok(derived)
    }
}

fn list_region_element(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Never | Type::Unit | Type::Bool | Type::I64 | Type::F64
    )
}

fn region_product_field(ty: &Type, fact: &MemoryTypeFact) -> bool {
    matches!(ty, Type::Unit | Type::Bool | Type::I64 | Type::F64)
        || matches!(ty, Type::List(element) if list_region_element(element))
        || matches!(ty, Type::Product(_))
            && fact.closure.class == MemoryClosureClass::RegionClosed
}
