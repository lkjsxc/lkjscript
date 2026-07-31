impl VerifiedTypes<'_> {
    fn recursive(
        &mut self,
        key: &VerifiedDeclarationKey,
        arguments: &[Type],
    ) -> Result<VerifiedDerived> {
        let component = self
            .graph
            .component(key)
            .ok_or_else(|| Error::msg("memory verifier lost SCC"))?;
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
                verified_recursive_substitutions(self.program, &declaration, key, arguments)?;
            for (field, path) in verified_recursive_fields(self.program, &declaration)? {
                let field = field.subst(&substitutions);
                if verified_key(&field).and_then(|item| self.graph.component(&item))
                    == Some(component)
                {
                    if let Type::Enum { arguments, .. } = &field {
                        for (index, argument) in arguments.iter().enumerate() {
                            if verified_type_mentions_component(argument, &self.graph, component) {
                                return Err(Error::msg(
                                    "LKJ-MEM-RECURSIVE-NONREGULAR transformed recursive type argument",
                                ));
                            }
                            let argument_id = self.intern(argument)?;
                            let fact = self.expected(argument_id)?.clone();
                            let mut argument_path = vec![path.clone()];
                            argument_path
                                .push(MemoryTypePathElement::TypeArgument(index_u32(index)?));
                            verified_fold_recursive_fact(
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
                if verified_type_mentions_component(&field, &self.graph, component) {
                    return Err(Error::msg(
                        "LKJ-MEM-RECURSIVE-NONREGULAR wrapped recursive field",
                    ));
                }
                let field_id = self.intern(&field)?;
                let fact = self.expected(field_id)?.clone();
                verified_fold_recursive_fact(
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
                return Err(Error::msg(
                    "independent memory verifier reconstructed recursive mixed closure",
                ));
            }
            return Err(Error::msg(format!(
                "memory verifier rejects recursive unresolved blocker={:?} path={:?} mode={mode:?}",
                closure.blocker_reason, closure.blocker_path
            )));
        }
        Ok(VerifiedDerived {
            mode,
            closure: verified_closed(MemoryClosureClass::Deterministic),
            contains_borrow,
            contains_dynamic_owner: true,
        })
    }
}

fn verified_type_mentions_component(
    ty: &Type,
    graph: &VerifiedDeclarationGraph,
    component: usize,
) -> bool {
    let mut declarations = Vec::new();
    verified_collect_declarations(ty, &mut declarations);
    declarations
        .iter()
        .any(|key| graph.component(key) == Some(component))
}

#[allow(clippy::too_many_arguments)]
fn verified_fold_recursive_fact(
    fact: VerifiedExpectedType,
    path: Vec<MemoryTypePathElement>,
    mode: &mut MemoryAggregateMode,
    dynamic: &mut bool,
    contains_borrow: &mut bool,
    unresolved_blocker: &mut Option<(MemoryClosureFact, Vec<MemoryTypePathElement>)>,
    affine_path: &mut Option<Vec<MemoryTypePathElement>>,
) {
    *mode = (*mode).max(fact.derived.mode);
    *dynamic |= fact.derived.contains_dynamic_owner;
    *contains_borrow |= fact.derived.contains_borrow;
    if fact.derived.mode == MemoryAggregateMode::Affine && affine_path.is_none() {
        *affine_path = Some(path.clone());
    }
    match fact.derived.closure.class {
        MemoryClosureClass::RegionClosed
        | MemoryClosureClass::Unresolved
        | MemoryClosureClass::IllegalDomainBridge
            if unresolved_blocker.is_none() =>
        {
            *unresolved_blocker = Some((fact.derived.closure, path));
        }
        _ => {}
    }
}
