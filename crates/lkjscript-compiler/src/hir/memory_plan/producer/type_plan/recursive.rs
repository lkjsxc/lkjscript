impl TypePlanner<'_> {
    fn derive_recursive(
        &mut self,
        key: &DeclarationKey,
        arguments: &[Type],
    ) -> Result<DerivedType> {
        let component = self
            .graph
            .component(key)
            .ok_or_else(|| Error::msg("recursive declaration lost SCC"))?;
        let keys: Vec<_> = self
            .graph
            .keys
            .iter()
            .filter(|item| self.graph.component(item) == Some(component))
            .cloned()
            .collect();
        let mut mode = MemoryAggregateMode::ImmutableValue;
        let mut dynamic = false;
        let mut contains_borrow = false;
        let mut unresolved_blocker = None;
        let mut affine_path = None;
        for declaration in keys {
            let substitutions =
                self.recursive_substitutions(&declaration, key, arguments)?;
            for (field, path) in self.recursive_fields(&declaration)? {
                let field = field.subst(&substitutions);
                if declaration_key(&field).and_then(|item| self.graph.component(&item))
                    == Some(component)
                {
                    if let Type::Enum { arguments, .. } = &field {
                        for (index, argument) in arguments.iter().enumerate() {
                            if type_mentions_component(argument, &self.graph, component)? {
                                return Err(Error::msg(
                                    "LKJ-MEM-RECURSIVE-NONREGULAR transformed recursive type argument",
                                ));
                            }
                            let argument_id = self.intern(argument)?;
                            let fact = self.fact(argument_id)?.clone();
                            let mut argument_path = vec![path.clone()];
                            argument_path
                                .push(MemoryTypePathElement::TypeArgument(index_u32(index)?));
                            fold_recursive_fact(
                                fact,
                                argument_path,
                                &mut mode,
                                &mut dynamic,
                                &mut contains_borrow,
                                &mut unresolved_blocker,
                                &mut affine_path,
                            );
                        }
                    }
                    continue;
                }
                if type_mentions_component(&field, &self.graph, component)? {
                    return Err(Error::msg(
                        "LKJ-MEM-RECURSIVE-NONREGULAR wrapped recursive field",
                    ));
                }
                let field_id = self.intern(&field)?;
                let fact = self.fact(field_id)?.clone();
                fold_recursive_fact(
                    fact,
                    vec![path],
                    &mut mode,
                    &mut dynamic,
                    &mut contains_borrow,
                    &mut unresolved_blocker,
                    &mut affine_path,
                );
            }
        }
        if affine_path.is_some() || contains_borrow {
            return Err(Error::msg(format!(
                "LKJ-MEM-RECURSIVE-AFFINE path={:?} mode={mode:?} contains-borrow={contains_borrow}",
                affine_path
            )));
        }
        if let Some((mut closure, path)) = unresolved_blocker {
            closure.blocker_path.splice(0..0, path);
            if dynamic {
                closure.class = MemoryClosureClass::IllegalDomainBridge;
                closure.mixed_direction =
                    Some(MemoryMixedBridgeDirection::UnresolvedContainsDeterministic);
            }
            let derived = DerivedType {
                mode,
                closure,
                contains_borrow,
                contains_dynamic_owner: dynamic,
            };
            if derived.closure.class == MemoryClosureClass::IllegalDomainBridge {
                return Err(closure_error(
                    &self.recursive_root_type(key, arguments)?,
                    &derived.closure,
                ));
            }
            return Err(Error::msg(format!(
                "LKJ-MEM-AGGREGATE-UNRESOLVED blocker={:?} path={:?}",
                derived.closure.blocker_reason, derived.closure.blocker_path
            )));
        }
        Ok(DerivedType {
            mode,
            closure: closed(MemoryClosureClass::Deterministic),
            contains_borrow,
            contains_dynamic_owner: true,
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn type_mentions_component(
    ty: &Type,
    graph: &DeclarationGraph,
    component: usize,
) -> Result<bool> {
    let mut declarations = Vec::new();
    collect_declarations(ty, &mut declarations)?;
    Ok(declarations
        .iter()
        .any(|key| graph.component(key) == Some(component)))
}

fn fold_recursive_fact(
    fact: MemoryTypeFact,
    path: Vec<MemoryTypePathElement>,
    mode: &mut MemoryAggregateMode,
    dynamic: &mut bool,
    contains_borrow: &mut bool,
    unresolved_blocker: &mut Option<(MemoryClosureFact, Vec<MemoryTypePathElement>)>,
    affine_path: &mut Option<Vec<MemoryTypePathElement>>,
) {
    *mode = (*mode).max(fact.mode);
    *dynamic |= fact.contains_dynamic_owner;
    *contains_borrow |= fact.contains_borrow;
    if fact.mode == MemoryAggregateMode::Affine && affine_path.is_none() {
        *affine_path = Some(path.clone());
    }
    match fact.closure.class {
        MemoryClosureClass::RegionClosed
        | MemoryClosureClass::Unresolved
        | MemoryClosureClass::IllegalDomainBridge
            if unresolved_blocker.is_none() =>
        {
            *unresolved_blocker = Some((fact.closure, path));
        }
        _ => {}
    }
}
