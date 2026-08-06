use crate::analyze::*;

impl Analyzer {
    pub(in crate::analyze) fn collect_implementations(
        &mut self,
        program: &ValidatedSourceTree,
    ) -> Result<()> {
        let mut coherent = HashSet::new();
        for (source_index, file) in program.files().iter().enumerate() {
            let source = SourceId::new(
                u32::try_from(source_index)
                    .map_err(|_| Error::msg("too many source files for HIR SourceId"))?,
            );
            for form in &file.forms {
                let AstExpr::Call { name, args } = form else {
                    continue;
                };
                if name != "impl" {
                    continue;
                }
                let (trait_name, target) =
                    impl_declaration(args).map_err(|message| self.error(source, message))?;
                let trait_id = self.trait_names.get(&trait_name).copied().ok_or_else(|| {
                    self.error(
                        source,
                        format!("impl references unknown trait {trait_name}"),
                    )
                })?;
                let trait_definition = self
                    .traits
                    .get(trait_id.index().unwrap_or(usize::MAX))
                    .ok_or_else(|| self.error(source, "impl resolved an unknown TraitId"))?;
                if trait_definition.core.is_some() {
                    return Err(self.error(
                        source,
                        format!(
                            "core trait {trait_name} cannot be explicitly implemented in the \
                             marker-trait slice"
                        ),
                    ));
                }
                let Type::Product(product_name) = &target else {
                    return Err(self.error(
                        source,
                        "marker impl target must be one exact nominal Product type",
                    ));
                };
                let product = self
                    .product_names
                    .get(product_name)
                    .copied()
                    .ok_or_else(|| {
                        self.error(
                            source,
                            format!("impl references unknown product {product_name}"),
                        )
                    })?;
                if !coherent.insert((trait_id, product)) {
                    return Err(self.error(
                        source,
                        format!(
                            "overlapping marker impl for trait {trait_name} and product \
                             {product_name} in the current program closure"
                        ),
                    ));
                }
                let id = ImplId::new(
                    u32::try_from(self.implementations.len())
                        .map_err(|_| self.error(source, "too many implementations for ImplId"))?,
                );
                self.implementations.push(ImplDefinition {
                    id,
                    trait_id,
                    product,
                    origin: source,
                });
                self.implementation_index.insert((trait_id, product), id);
            }
        }
        Ok(())
    }
}
