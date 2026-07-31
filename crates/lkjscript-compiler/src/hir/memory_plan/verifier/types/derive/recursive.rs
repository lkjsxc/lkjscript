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
        let mut mode = MemoryAggregateMode::Copy;
        let mut dynamic = false;
        let mut contains_borrow = false;
        let mut dynamic_blocker = None;
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
                            let child = self.intern(argument)?;
                            let fact = self.expected(child)?.clone();
                            mode = mode.max(fact.derived.mode);
                            dynamic |= fact.derived.contains_dynamic_owner;
                            contains_borrow |= fact.derived.contains_borrow;
                            if fact.derived.contains_dynamic_owner && dynamic_blocker.is_none() {
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
                let fact = self.expected(child)?.clone();
                mode = mode.max(fact.derived.mode);
                dynamic |= fact.derived.contains_dynamic_owner;
                contains_borrow |= fact.derived.contains_borrow;
                if fact.derived.contains_dynamic_owner && dynamic_blocker.is_none() {
                    dynamic_blocker = Some((fact, vec![path]));
                }
            }
        }
        if let Some((fact, path)) = dynamic_blocker {
            return Ok(verified_recursive_mixed(fact, path));
        }
        let root = verified_recursive_root(self.program, key, arguments)?;
        let mut result = verified_legacy(&root, MemoryBlockerReason::RecursiveDeclarationScc);
        result.mode = mode;
        result.contains_borrow = contains_borrow;
        result.contains_dynamic_owner = dynamic;
        Ok(result)
    }
}
