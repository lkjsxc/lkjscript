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
            Type::Param(_) => verified_legacy(ty, MemoryBlockerReason::UnknownTypeParameter),
            Type::Fn { .. } | Type::Forall { .. } => {
                verified_legacy(ty, MemoryBlockerReason::CapturedClosure)
            }
            Type::Product(name) => self.product(name)?,
            Type::Enum { id, arguments, .. } => self.enum_type(id.bytes(), arguments)?,
        })
    }

    fn list(&mut self, ty: &Type, inner: &Type) -> Result<VerifiedDerived> {
        let child = self.intern(inner)?;
        let fact = self.expected(child)?.clone();
        let mut result = verified_legacy(ty, MemoryBlockerReason::ListPair);
        result.mode = fact.derived.mode;
        result.contains_borrow = fact.derived.contains_borrow;
        result.contains_dynamic_owner = fact.derived.contains_dynamic_owner;
        if fact.derived.contains_dynamic_owner {
            result.closure.class = MemoryClosureClass::IllegalMixedBridge;
            result.closure.blocker_path = vec![MemoryTypePathElement::TypeArgument(0)];
            result.closure.mixed_direction =
                Some(MemoryMixedBridgeDirection::LegacyContainsDeterministic);
        }
        Ok(result)
    }

    fn product(&mut self, name: &str) -> Result<VerifiedDerived> {
        let key = VerifiedDeclarationKey::Product(name.to_owned());
        let item = self
            .program
            .products
            .iter()
            .find(|item| item.name == name)
            .cloned()
            .ok_or_else(|| Error::msg("memory verifier lost product"))?;
        verified_add(
            &mut self.fields,
            item.fields.len(),
            MAX_MEMORY_PLAN_AGGREGATE_FIELDS,
        )?;
        if self.graph.is_recursive(&key) {
            return self.recursive(&key, &[]);
        }
        let mut children = Vec::new();
        for (index, field) in item.fields.iter().enumerate() {
            let id = self.intern(&field.ty)?;
            children.push((
                self.expected(id)?.derived.clone(),
                MemoryTypePathElement::ProductField {
                    index: index_u32(index)?,
                    name: field.name.clone(),
                },
            ));
        }
        Ok(verified_fold(children))
    }

    fn enum_type(&mut self, id: [u8; 32], arguments: &[Type]) -> Result<VerifiedDerived> {
        let key = VerifiedDeclarationKey::Enum(id);
        let item = self
            .program
            .enums
            .iter()
            .find(|item| item.id.bytes() == id)
            .cloned()
            .ok_or_else(|| Error::msg("memory verifier lost enum"))?;
        if item.type_parameters.len() != arguments.len() {
            return Err(Error::msg("memory verifier enum arity mismatch"));
        }
        verified_add(
            &mut self.variants,
            item.variants.len(),
            MAX_MEMORY_PLAN_AGGREGATE_VARIANTS,
        )?;
        verified_add(
            &mut self.fields,
            item.variants.iter().map(|v| v.fields.len()).sum(),
            MAX_MEMORY_PLAN_AGGREGATE_FIELDS,
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
                        variant_index: index_u32(vi)?,
                        variant: variant.id.bytes(),
                        field_index: index_u32(fi)?,
                        field: field.id.bytes(),
                    },
                ));
            }
        }
        Ok(verified_fold(children))
    }
}

include!("derive/recursive.rs");
