use super::*;

impl VerifiedTypes<'_> {
    pub(crate) fn derive(&mut self, ty: &Type) -> Result<VerifiedDerived> {
        let deterministic = |mode, dynamic, borrow| VerifiedDerived {
            mode,
            closure: verified_closed(MemoryClosureClass::Deterministic),
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
            Type::List(inner) => self.list(ty, inner)?,
            Type::Param(_) => verified_unresolved(ty, MemoryBlockerReason::UnknownTypeParameter),
            Type::Fn { .. } | Type::Forall { .. } => {
                verified_unresolved(ty, MemoryBlockerReason::CapturedClosure)
            }
            Type::Product(id) => self.product(*id)?,
            Type::Enum { id, arguments, .. } => self.enum_type(id.bytes(), arguments)?,
        })
    }

    fn list(&mut self, ty: &Type, inner: &Type) -> Result<VerifiedDerived> {
        let child = self.intern(inner)?;
        let fact = self.expected(child)?.clone();
        let scalar_element = verified_list_region_element(inner)
            && fact.derived.mode == MemoryAggregateMode::Copy
            && fact.derived.closure.class == MemoryClosureClass::Deterministic
            && !fact.derived.contains_borrow;
        if scalar_element || self.verified_selected_list_element(inner, &fact) {
            return Ok(VerifiedDerived {
                mode: MemoryAggregateMode::ImmutableValue,
                closure: MemoryClosureFact {
                    class: MemoryClosureClass::RegionClosed,
                    blocker_path: Vec::new(),
                    blocker_type: Some(verified_memory_type(ty)),
                    blocker_reason: Some(MemoryBlockerReason::RegionDomainBoundary),
                    mixed_direction: None,
                },
                contains_borrow: false,
                contains_dynamic_owner: false,
            });
        }
        let mut result = verified_unresolved(ty, MemoryBlockerReason::ListElementWitnessRequired);
        result.mode = fact.derived.mode;
        result.contains_borrow = fact.derived.contains_borrow;
        result.contains_dynamic_owner = fact.derived.contains_dynamic_owner;
        Ok(result)
    }

    fn product(&mut self, id: hir::ProductId) -> Result<VerifiedDerived> {
        let key = VerifiedDeclarationKey::Product(id);
        let item = self.product_definition(id)?.clone();
        verified_observe(&mut self.fields, item.fields.len())?;
        if self.graph.is_recursive(&key) {
            return self.recursive(&key, &[]);
        }
        let mut children = Vec::new();
        for (index, field) in item.fields.iter().enumerate() {
            let id = self.intern(&field.ty)?;
            children.push((
                self.expected(id)?.derived.clone(),
                MemoryTypePathElement::ProductField {
                    index: index_u64(index)?,
                    field: field.identity,
                },
            ));
        }
        let region_capable = item
            .fields
            .iter()
            .zip(&children)
            .all(|(field, (derived, _))| verified_region_product_field(&field.ty, derived));
        let derived = verified_fold(children, region_capable);
        if derived.closure.class == MemoryClosureClass::Unresolved {
            return Err(Error::msg(format!(
                "memory verifier rejects unresolved product {}",
                item.name
            )));
        }
        Ok(derived)
    }

    fn enum_type(&mut self, id: [u8; 32], arguments: &[Type]) -> Result<VerifiedDerived> {
        let key = VerifiedDeclarationKey::Enum(id);
        let item = self.enum_definition(id)?.clone();
        if item.type_parameters.len() != arguments.len() {
            return Err(Error::msg("memory verifier enum arity mismatch"));
        }
        verified_observe(&mut self.variants, item.variants.len())?;
        verified_observe(
            &mut self.fields,
            item.variants.iter().map(|v| v.fields.len()).sum(),
        )?;
        if self.graph.is_recursive(&key) {
            return self.recursive(&key, arguments);
        }
        let substitutions: HashMap<_, _> = item
            .type_parameters
            .iter()
            .cloned()
            .zip(arguments.iter().cloned())
            .collect();
        let mut children = Vec::new();
        for (vi, variant) in item.variants.iter().enumerate() {
            for (fi, field) in variant.fields.iter().enumerate() {
                let ty = field.ty.subst(&substitutions);
                let child = self.intern(&ty)?;
                children.push((
                    self.expected(child)?.derived.clone(),
                    MemoryTypePathElement::EnumVariantField {
                        variant_index: index_u64(vi)?,
                        variant: variant.id.bytes(),
                        field_index: index_u64(fi)?,
                        field: field.id.bytes(),
                    },
                ));
            }
        }
        let derived = verified_fold(children, false);
        if derived.closure.class == MemoryClosureClass::Unresolved {
            return Err(Error::msg(format!(
                "memory verifier rejects unresolved enum {}",
                item.name
            )));
        }
        Ok(derived)
    }
}

include!("derive/lists.rs");

fn verified_list_region_element(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Never | Type::Unit | Type::Bool | Type::I64 | Type::F64
    )
}

fn verified_region_product_field(ty: &Type, derived: &VerifiedDerived) -> bool {
    matches!(ty, Type::Unit | Type::Bool | Type::I64 | Type::F64)
        || matches!(ty, Type::List(_)) && derived.closure.class == MemoryClosureClass::RegionClosed
        || matches!(ty, Type::Product(_))
            && derived.closure.class == MemoryClosureClass::RegionClosed
}

include!("derive/recursive.rs");
