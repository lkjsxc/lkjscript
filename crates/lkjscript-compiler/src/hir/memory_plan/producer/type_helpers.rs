fn fold_aggregate(
    children: Vec<(MemoryTypeFact, MemoryTypePathElement)>,
    recursive: bool,
) -> DerivedType {
    let mut mode = MemoryAggregateMode::Copy;
    let mut contains_borrow = false;
    let mut contains_dynamic_owner = false;
    let mut blocker = None;
    for (fact, path) in children {
        mode = mode.max(fact.mode);
        contains_borrow |= fact.contains_borrow;
        contains_dynamic_owner |= fact.contains_dynamic_owner;
        if fact.closure.class != MemoryClosureClass::Deterministic && blocker.is_none() {
            blocker = Some((fact.closure, path));
        }
    }
    if let Some((mut closure, path)) = blocker {
        closure.blocker_path.insert(0, path);
        if contains_dynamic_owner {
            closure.class = MemoryClosureClass::IllegalMixedBridge;
            closure.mixed_direction = Some(if recursive {
                MemoryMixedBridgeDirection::LegacyContainsDeterministic
            } else { MemoryMixedBridgeDirection::DeterministicContainsLegacy });
        }
        return DerivedType { mode, closure, contains_borrow, contains_dynamic_owner };
    }
    DerivedType {
        mode,
        closure: closed(MemoryClosureClass::Deterministic),
        contains_borrow,
        contains_dynamic_owner,
    }
}

fn type_contains_resource(ty: &Type) -> bool {
    match ty {
        Type::Resource(_) => true,
        Type::List(inner) => type_contains_resource(inner),
        Type::Enum { arguments, .. } => arguments.iter().any(type_contains_resource),
        Type::Fn { params, ret, .. } => {
            params.iter().any(type_contains_resource) || type_contains_resource(ret)
        }
        Type::Forall { body, .. } => type_contains_resource(body),
        _ => false,
    }
}

fn declaration_key(ty: &Type) -> Option<DeclarationKey> {
    match ty {
        Type::Product(name) => Some(DeclarationKey::Product(name.clone())),
        Type::Enum { id, .. } => Some(DeclarationKey::Enum(id.bytes())),
        _ => None,
    }
}

fn is_aggregate(ty: &Type) -> bool { matches!(ty, Type::Product(_) | Type::Enum { .. }) }

fn copy_share(ty: &Type, derived: &DerivedType) -> MemoryCopySharePlan {
    if derived.closure.class == MemoryClosureClass::LegacyClosed {
        return MemoryCopySharePlan::LegacyTracing;
    }
    match ty {
        Type::Symbol => MemoryCopySharePlan::StaticIdentity,
        Type::ByteSlice => MemoryCopySharePlan::BorrowShared,
        Type::ByteSliceMut => MemoryCopySharePlan::BorrowExclusive,
        Type::Resource(_) => MemoryCopySharePlan::ExternalHandle,
        _ => match derived.mode {
            MemoryAggregateMode::Copy => MemoryCopySharePlan::TrivialCopy,
            MemoryAggregateMode::ImmutableValue if is_aggregate(ty) => MemoryCopySharePlan::StructuralCopy,
            MemoryAggregateMode::ImmutableValue => MemoryCopySharePlan::BorrowShared,
            MemoryAggregateMode::Affine => MemoryCopySharePlan::Move,
        },
    }
}

fn closure_error(ty: &Type, closure: &MemoryClosureFact) -> Error {
    Error::msg(format!(
        "LKJ-MEM-MIXED-BRIDGE type={:?} direction={:?} path={:?} blocker-type={:?} reason={:?}",
        memory_type(ty), closure.mixed_direction, closure.blocker_path,
        closure.blocker_type, closure.blocker_reason,
    ))
}

pub(super) fn bounded_add(slot: &mut u64, amount: usize, limit: u64, label: &str) -> Result<()> {
    *slot = slot.checked_add(u64::try_from(amount)
        .map_err(|_| Error::msg(format!("HIR memory-plan {label} charge exceeds u64")))?)
        .ok_or_else(|| Error::msg(format!("HIR memory-plan {label} work overflow")))?;
    if *slot > limit { return Err(Error::msg(format!("HIR memory-plan {label} work exceeds {limit}"))); }
    Ok(())
}

fn base_drop_glues() -> Vec<MemoryDropGluePlan> {
    let mut glues = vec![MemoryDropGluePlan { id: MemoryDropGlueId::new(0),
        kind: MemoryDropGlueKind::ByteVector, drop_path: None }];
    glues.extend(ResourceKind::ALL.into_iter().map(|kind| MemoryDropGluePlan {
        id: resource_glue(kind), kind: MemoryDropGlueKind::Resource(kind), drop_path: None,
    }));
    glues.push(MemoryDropGluePlan { id: bytes_glue(), kind: MemoryDropGlueKind::Bytes,
        drop_path: None });
    glues
}

impl TypePlanner<'_> {
    fn add_structural_drop(
        &mut self,
        ty: &Type,
        derived: &DerivedType,
    ) -> Result<(Option<MemoryDropGlueId>, Option<MemoryDropPathId>)> {
        if derived.closure.class != MemoryClosureClass::Deterministic
            || type_contains_resource(ty)
            || !matches!(ty, Type::Str | Type::Path | Type::Product(_) | Type::Enum { .. })
        {
            return Ok((leaf_glue(ty), None));
        }
        bounded_add(&mut self.fields, 0, MAX_MEMORY_PLAN_AGGREGATE_FIELDS, "aggregate fields")?;
        if u64::try_from(self.drop_paths.len()).unwrap_or(u64::MAX) >= MAX_MEMORY_PLAN_DROP_PATHS {
            return Err(Error::msg("HIR memory-plan drop paths exceed bounded maximum"));
        }
        let path_id = MemoryDropPathId::new(u32::try_from(self.drop_paths.len())
            .map_err(|_| Error::msg("HIR memory-plan drop path identity exceeds u32"))?);
        let branches = self.drop_branches(ty)?;
        self.drop_paths.push(MemoryDropPathPlan { id: path_id, ty: memory_type(ty), branches });
        let glue_id = MemoryDropGlueId::new(u32::try_from(self.glues.len())
            .map_err(|_| Error::msg("HIR memory-plan drop glue identity exceeds u32"))?);
        let kind = match ty {
            Type::Str => MemoryDropGlueKind::String,
            Type::Path => MemoryDropGlueKind::Path,
            Type::Product(name) => MemoryDropGlueKind::Product(name.clone()),
            Type::Enum { id, arguments, .. } => MemoryDropGlueKind::Enum {
                id: id.bytes(), arguments: arguments.iter().map(memory_type).collect(),
            },
            _ => return Err(Error::msg("structural drop requested for non-structural type")),
        };
        self.glues.push(MemoryDropGluePlan { id: glue_id, kind, drop_path: Some(path_id) });
        Ok((Some(glue_id), Some(path_id)))
    }

    fn drop_branches(&self, ty: &Type) -> Result<Vec<MemoryDropBranch>> {
        match ty {
            Type::Str | Type::Path => Ok(vec![MemoryDropBranch { active_variant: None,
                actions: Vec::new() }]),
            Type::Product(name) => {
                let product = self.program.products.iter().find(|item| item.name == *name)
                    .ok_or_else(|| Error::msg("drop path lost product"))?;
                let mut actions = Vec::new();
                for (index, field) in product.fields.iter().enumerate().rev() {
                    if let Some(glue) = self.memo.get(&field.ty).and_then(|id| self.facts.get(id.index()?))
                        .and_then(|fact| fact.drop_glue) {
                        actions.push(MemoryDropAction { path: vec![MemoryDropPathElement::ProductField {
                            index: index_u32(index)?, name: field.name.clone() }], glue });
                    }
                }
                Ok(vec![MemoryDropBranch { active_variant: None, actions }])
            }
            Type::Enum { id, arguments, .. } => self.enum_drop_branches(id.bytes(), arguments),
            _ => Err(Error::msg("drop path requested for non-structural type")),
        }
    }

    fn enum_drop_branches(&self, id: [u8; 32], arguments: &[Type]) -> Result<Vec<MemoryDropBranch>> {
        let item = self.program.enums.iter().find(|item| item.id.bytes() == id)
            .ok_or_else(|| Error::msg("drop path lost enum"))?;
        let substitutions: HashMap<_, _> = item.type_parameters.iter().cloned()
            .zip(arguments.iter().cloned()).collect();
        item.variants.iter().map(|variant| {
            let mut actions = Vec::new();
            for (index, field) in variant.fields.iter().enumerate().rev() {
                let ty = field.ty.subst(&substitutions);
                if let Some(glue) = self.memo.get(&ty).and_then(|id| self.facts.get(id.index()?))
                    .and_then(|fact| fact.drop_glue) {
                    actions.push(MemoryDropAction { path: vec![MemoryDropPathElement::EnumField {
                        variant: variant.id.bytes(), index: index_u32(index)?, field: field.id.bytes(),
                    }], glue });
                }
            }
            Ok(MemoryDropBranch { active_variant: Some(variant.id.bytes()), actions })
        }).collect()
    }
}

fn leaf_glue(ty: &Type) -> Option<MemoryDropGlueId> {
    match ty {
        Type::ByteVector => Some(MemoryDropGlueId::new(0)),
        Type::Bytes => Some(bytes_glue()),
        Type::Resource(kind) => Some(resource_glue(*kind)),
        _ => None,
    }
}
