fn fold_aggregate(
    children: Vec<(MemoryTypeFact, MemoryTypePathElement)>,
    recursive: bool,
    region_capable: bool,
) -> DerivedType {
    let mut mode = MemoryAggregateMode::Copy;
    let mut contains_borrow = false;
    let mut contains_dynamic_owner = false;
    let mut region = None;
    let mut blocker = None;
    for (fact, path) in children {
        mode = mode.max(fact.mode);
        contains_borrow |= fact.contains_borrow;
        contains_dynamic_owner |= fact.contains_dynamic_owner;
        match fact.closure.class {
            MemoryClosureClass::Deterministic => {}
            MemoryClosureClass::RegionClosed if region.is_none() => {
                region = Some((fact.closure, path));
            }
            MemoryClosureClass::RegionClosed => {}
            MemoryClosureClass::Unresolved | MemoryClosureClass::IllegalDomainBridge
                if blocker.is_none() =>
            {
                blocker = Some((fact.closure, path));
            }
            MemoryClosureClass::Unresolved | MemoryClosureClass::IllegalDomainBridge => {}
        }
    }
    if let Some((mut closure, path)) = blocker {
        closure.blocker_path.insert(0, path);
        if contains_dynamic_owner {
            closure.class = MemoryClosureClass::IllegalDomainBridge;
            closure.mixed_direction = Some(if recursive {
                MemoryMixedBridgeDirection::UnresolvedContainsDeterministic
            } else { MemoryMixedBridgeDirection::DeterministicContainsUnresolved });
        }
        return DerivedType { mode, closure, contains_borrow, contains_dynamic_owner };
    }
    if let Some((mut closure, path)) = region {
        closure.blocker_path.insert(0, path);
        if contains_dynamic_owner || contains_borrow || recursive {
            closure.class = MemoryClosureClass::IllegalDomainBridge;
            closure.mixed_direction = Some(MemoryMixedBridgeDirection::DeterministicContainsUnresolved);
        } else if !region_capable {
            closure.class = MemoryClosureClass::Unresolved;
        }
        return DerivedType {
            mode,
            closure,
            contains_borrow,
            contains_dynamic_owner,
        };
    }
    DerivedType {
        mode,
        closure: closed(MemoryClosureClass::Deterministic),
        contains_borrow,
        contains_dynamic_owner: true,
    }
}

include!("type_plan/type_helpers_resources.rs");

fn copy_share(ty: &Type, derived: &DerivedType) -> MemoryCopySharePlan {
    if derived.closure.class == MemoryClosureClass::RegionClosed {
        return MemoryCopySharePlan::RegionHandleCopy;
    }
    if derived.closure.class != MemoryClosureClass::Deterministic {
        return MemoryCopySharePlan::Unsupported;
    }
    match ty {
        Type::Symbol => MemoryCopySharePlan::StaticIdentity,
        Type::ByteSlice => MemoryCopySharePlan::BorrowShared,
        Type::ByteSliceMut => MemoryCopySharePlan::BorrowExclusive,
        Type::Resource(_) => MemoryCopySharePlan::ExternalHandle,
        Type::List(_) => MemoryCopySharePlan::RegionHandleCopy,
        Type::Product(_) | Type::Enum { .. } => MemoryCopySharePlan::StructuralCopy,
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

pub(super) fn checked_observe(slot: &mut u64, amount: usize, label: &str) -> Result<()> {
    *slot = slot
        .checked_add(
            u64::try_from(amount)
                .map_err(|_| Error::msg(format!("HIR memory-plan {label} exceeds u64")))?,
        )
        .ok_or_else(|| Error::msg(format!("HIR memory-plan {label} telemetry overflow")))?;
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
        let path_id = MemoryDropPathId::new(u64::try_from(self.drop_paths.len())
            .map_err(|_| Error::msg("HIR memory-plan drop path identity exceeds u64"))?);
        let branches = self.drop_branches(ty)?;
        self.drop_paths.push(MemoryDropPathPlan { id: path_id, ty: memory_type(ty), branches });
        let glue_id = MemoryDropGlueId::new(u64::try_from(self.glues.len())
            .map_err(|_| Error::msg("HIR memory-plan drop glue identity exceeds u64"))?);
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
                let product = self.product(name)?;
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
        let item = self.enumeration(id)?;
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
