use super::*;

impl VerifiedTypes<'_> {
    pub(crate) fn expected_drop(
        &mut self,
        ty: &Type,
        derived: &VerifiedDerived,
    ) -> Result<(Option<MemoryDropGlueId>, Option<MemoryDropPathId>)> {
        if derived.closure.class != MemoryClosureClass::Deterministic
            || verified_type_contains_resource(ty)
            || !matches!(
                ty,
                Type::Str | Type::Path | Type::Product(_) | Type::Enum { .. }
            )
        {
            return Ok((verified_leaf_glue(ty), None));
        }
        let path_raw = self.drop_paths;
        let path = MemoryDropPathId::new(path_raw);
        let expected_path = MemoryDropPathPlan {
            id: path,
            ty: verified_memory_type(ty),
            branches: self.expected_branches(ty)?,
        };
        if path
            .index()
            .and_then(|index| self.plan.drop_paths.get(index))
            != Some(&expected_path)
        {
            return Err(Error::msg(
                "independent memory verifier rejected recursive drop path",
            ));
        }
        let base = 2_u64
            .checked_add(
                u64::try_from(ResourceKind::ALL.len())
                    .map_err(|_| Error::msg("resource count exceeds u64"))?,
            )
            .ok_or_else(|| Error::msg("drop glue identity overflow"))?;
        let glue = MemoryDropGlueId::new(
            base.checked_add(path_raw)
                .ok_or_else(|| Error::msg("drop glue identity overflow"))?,
        );
        let kind = match ty {
            Type::Str => MemoryDropGlueKind::String,
            Type::Path => MemoryDropGlueKind::Path,
            Type::Product(name) => MemoryDropGlueKind::Product(name.clone()),
            Type::Enum { id, arguments, .. } => MemoryDropGlueKind::Enum {
                id: id.bytes(),
                arguments: arguments.iter().map(verified_memory_type).collect(),
            },
            _ => return Err(Error::msg("memory verifier structural drop type mismatch")),
        };
        let expected_glue = MemoryDropGluePlan {
            id: glue,
            kind,
            drop_path: Some(path),
        };
        if glue
            .index()
            .and_then(|index| self.plan.drop_glues.get(index))
            != Some(&expected_glue)
        {
            return Err(Error::msg(
                "independent memory verifier rejected structural drop glue",
            ));
        }
        self.drop_paths = self
            .drop_paths
            .checked_add(1)
            .ok_or_else(|| Error::msg("memory verifier drop-path telemetry overflow"))?;
        Ok((Some(glue), Some(path)))
    }

    fn expected_branches(&self, ty: &Type) -> Result<Vec<MemoryDropBranch>> {
        match ty {
            Type::Str | Type::Path => Ok(vec![MemoryDropBranch {
                active_variant: None,
                actions: Vec::new(),
            }]),
            Type::Product(name) => {
                let item = self.product_definition(name)?;
                let mut actions = Vec::new();
                for (index, field) in item.fields.iter().enumerate().rev() {
                    if let Some(glue) = self
                        .memo
                        .get(&field.ty)
                        .and_then(|id| self.expected.get(id.index()?))
                        .and_then(|item| item.glue)
                    {
                        actions.push(MemoryDropAction {
                            path: vec![MemoryDropPathElement::ProductField {
                                index: index_u32(index)?,
                                name: field.name.clone(),
                            }],
                            glue,
                        });
                    }
                }
                Ok(vec![MemoryDropBranch {
                    active_variant: None,
                    actions,
                }])
            }
            Type::Enum { id, arguments, .. } => self.expected_enum_branches(id.bytes(), arguments),
            _ => Err(Error::msg("memory verifier drop path requested for leaf")),
        }
    }

    fn expected_enum_branches(
        &self,
        id: [u8; 32],
        arguments: &[Type],
    ) -> Result<Vec<MemoryDropBranch>> {
        let item = self.enum_definition(id)?;
        let substitutions: HashMap<_, _> = item
            .type_parameters
            .iter()
            .cloned()
            .zip(arguments.iter().cloned())
            .collect();
        item.variants
            .iter()
            .map(|variant| {
                let mut actions = Vec::new();
                for (index, field) in variant.fields.iter().enumerate().rev() {
                    let ty = field.ty.subst(&substitutions);
                    if let Some(glue) = self
                        .memo
                        .get(&ty)
                        .and_then(|id| self.expected.get(id.index()?))
                        .and_then(|item| item.glue)
                    {
                        actions.push(MemoryDropAction {
                            path: vec![MemoryDropPathElement::EnumField {
                                variant: variant.id.bytes(),
                                index: index_u32(index)?,
                                field: field.id.bytes(),
                            }],
                            glue,
                        });
                    }
                }
                Ok(MemoryDropBranch {
                    active_variant: Some(variant.id.bytes()),
                    actions,
                })
            })
            .collect()
    }
}
