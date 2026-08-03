impl TypePlanner<'_> {
    fn producer_witness_dependencies(
        &self,
        root: &Type,
    ) -> Result<Vec<lkjscript_contracts::ExecutableMemoryWitnessDependency>> {
        use lkjscript_contracts::ExecutableMemoryWitnessRole as R;
        let mut output = Vec::new();
        match root {
            Type::List(element) => {
                self.producer_dependency(root, element, R::ListElement, &mut output)?;
            }
            Type::Product(name) => {
                let item = self
                    .program
                    .products
                    .iter()
                    .find(|item| item.name == *name)
                    .ok_or_else(|| Error::msg("witness roles lost product"))?;
                for field in &item.fields {
                    self.producer_dependency(
                        root,
                        &field.ty,
                        R::ProductField {
                            product: item.identity,
                            field: field.identity,
                            source_order: field.source_order,
                        },
                        &mut output,
                    )?;
                }
            }
            Type::Enum { id, arguments, .. } => {
                let item = self
                    .program
                    .enums
                    .iter()
                    .find(|item| item.id == *id)
                    .ok_or_else(|| Error::msg("witness roles lost enum"))?;
                if arguments.len() != item.type_parameters.len() {
                    return Err(Error::msg("witness enum argument arity mismatch"));
                }
                for (index, argument) in arguments.iter().enumerate() {
                    self.producer_dependency(
                        root,
                        argument,
                        R::TypeArgument {
                            constructor: id.bytes(),
                            index: u16::try_from(index)
                                .map_err(|_| Error::msg("type argument order overflow"))?,
                        },
                        &mut output,
                    )?;
                }
                let substitutions: HashMap<_, _> = item
                    .type_parameters
                    .iter()
                    .cloned()
                    .zip(arguments.iter().cloned())
                    .collect();
                for variant in &item.variants {
                    for field in &variant.fields {
                        self.producer_dependency(
                            root,
                            &field.ty.subst(&substitutions),
                            R::EnumVariantField {
                                enumeration: id.bytes(),
                                variant: variant.id.bytes(),
                                field: field.id.bytes(),
                                variant_source_order: variant.source_order,
                                field_source_order: field.source_order,
                            },
                            &mut output,
                        )?;
                    }
                }
            }
            _ => {}
        }
        Ok(output)
    }

    fn producer_dependency(
        &self,
        root: &Type,
        child: &Type,
        role: lkjscript_contracts::ExecutableMemoryWitnessRole,
        output: &mut Vec<lkjscript_contracts::ExecutableMemoryWitnessDependency>,
    ) -> Result<()> {
        use lkjscript_contracts::{
            ExecutableMemoryWitnessDependency as D, ExecutableMemoryWitnessTarget as T,
        };
        let local = declaration_key(root)
            .and_then(|left| self.graph.component(&left).map(|component| (left, component)))
            .is_some_and(|(left, component)| {
                self.graph.is_recursive(&left)
                    && declaration_key(child)
                        .is_some_and(|right| self.graph.component(&right) == Some(component))
            });
        let target = if local {
            T::LocalMember(0)
        } else {
            let fact = self.memo.get(child).copied()
                .and_then(|id| self.facts.get(id.index()?))
                .ok_or_else(|| Error::msg(
                    "witness dependency child was not independently interned"))?;
            T::ExternalMember { group: [0; 32], member: fact.witness.as_bytes() }
        };
        output.push(D { role, target });
        Ok(())
    }
}
