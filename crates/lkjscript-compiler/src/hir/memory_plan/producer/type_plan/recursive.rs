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
        let mut mode = MemoryAggregateMode::Copy;
        let mut dynamic = false;
        let mut contains_borrow = false;
        let mut dynamic_blocker = None;
        for declaration in keys {
            let substitutions =
                recursive_substitutions(self.program, &declaration, key, arguments)?;
            for (field, path) in recursive_fields(self.program, &declaration)? {
                let field = field.subst(&substitutions);
                if declaration_key(&field).and_then(|item| self.graph.component(&item))
                    == Some(component)
                {
                    if let Type::Enum { arguments, .. } = &field {
                        for (index, argument) in arguments.iter().enumerate() {
                            let child = self.intern(argument)?;
                            let fact = self.fact(child)?.clone();
                            mode = mode.max(fact.mode);
                            dynamic |= fact.contains_dynamic_owner;
                            contains_borrow |= fact.contains_borrow;
                            if fact.contains_dynamic_owner && dynamic_blocker.is_none() {
                                let mut argument_path = vec![path.clone()];
                                argument_path
                                    .push(MemoryTypePathElement::TypeArgument(index_u32(index)?));
                                dynamic_blocker = Some((fact, argument_path));
                            }
                        }
                    }
                    continue;
                }
                let child = self.intern(&field)?;
                let fact = self.fact(child)?.clone();
                mode = mode.max(fact.mode);
                dynamic |= fact.contains_dynamic_owner;
                contains_borrow |= fact.contains_borrow;
                if fact.contains_dynamic_owner && dynamic_blocker.is_none() {
                    dynamic_blocker = Some((fact, vec![path]));
                }
            }
        }
        if let Some((fact, path)) = dynamic_blocker {
            return Ok(recursive_mixed(fact, path));
        }
        let root = recursive_root_type(self.program, key, arguments)?;
        let mut result = legacy(&root, MemoryBlockerReason::RecursiveDeclarationScc);
        result.mode = mode;
        result.contains_borrow = contains_borrow;
        result.contains_dynamic_owner = dynamic;
        Ok(result)
    }
}
