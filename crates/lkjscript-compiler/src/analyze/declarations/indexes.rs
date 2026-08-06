use crate::analyze::*;

impl Analyzer {
    pub(in crate::analyze) fn new(program: &ValidatedSourceTree) -> Result<Self> {
        let mut sources = Vec::with_capacity(program.files().len());
        for file in program.files() {
            let raw = u64::try_from(sources.len())
                .map_err(|_| Error::msg("too many source files for HIR SourceId"))?;
            sources.push(Source {
                id: SourceId::new(raw),
                path: file.path.clone(),
            });
        }
        Ok(Self {
            sources,
            bindings: Vec::new(),
            globals: HashMap::new(),
            operations: HashMap::new(),
            product_names: HashMap::new(),
            products: Vec::new(),
            enum_headers: HashMap::new(),
            enums: Vec::new(),
            trait_names: HashMap::new(),
            traits: Vec::new(),
            implementations: Vec::new(),
            implementation_index: HashMap::new(),
            function_bounds: HashMap::new(),
            match_plans: Vec::new(),
            next_loan: 0,
        })
    }

    pub(in crate::analyze) fn install_operations(&mut self) -> Result<()> {
        for operation in Operation::ALL {
            let id = self.add_binding(
                operation.name().to_string(),
                BindingKind::BuiltinOperation(*operation),
                operation.signature(),
                Origin::Builtin,
            )?;
            self.operations.insert(*operation, id);
        }
        Ok(())
    }

    pub(in crate::analyze) fn install_core_traits(&mut self) -> Result<()> {
        for core in CoreTrait::ALL {
            let raw = u64::try_from(self.traits.len())
                .map_err(|_| Error::msg("too many traits for HIR TraitId"))?;
            let id = TraitId::new(raw);
            let name = core.name().to_string();
            self.trait_names.insert(name.clone(), id);
            self.traits.push(TraitDefinition {
                id,
                name,
                origin: Origin::Builtin,
                core: Some(core),
            });
        }
        Ok(())
    }

    pub(in crate::analyze) fn collect_trait_names(
        &mut self,
        program: &ValidatedSourceTree,
    ) -> Result<()> {
        for (source_index, file) in program.files().iter().enumerate() {
            let source = SourceId::new(
                u64::try_from(source_index)
                    .map_err(|_| Error::msg("too many source files for HIR SourceId"))?,
            );
            for form in &file.forms {
                let AstExpr::Call { name, args } = form else {
                    continue;
                };
                if name != "trait" {
                    continue;
                }
                let trait_name =
                    trait_declaration(args).map_err(|message| self.error(source, message))?;
                if !is_declaration_type_name(&trait_name) {
                    return Err(self.error(
                        source,
                        format!("invalid trait declaration name {trait_name}"),
                    ));
                }
                if CoreTrait::ALL.iter().any(|core| core.name() == trait_name) {
                    return Err(self.error(
                        source,
                        format!("trait {trait_name} is compiler-owned and cannot be declared"),
                    ));
                }
                if Operation::from_name(&trait_name).is_some()
                    || is_contextual_name(&trait_name)
                    || is_builtin_type_name(&trait_name)
                {
                    return Err(self.error(
                        source,
                        format!(
                            "trait declaration {trait_name} collides with a reserved \
                             operation, form, or type"
                        ),
                    ));
                }
                if self.trait_names.contains_key(&trait_name) {
                    return Err(
                        self.error(source, format!("duplicate trait declaration {trait_name}"))
                    );
                }
                let id =
                    TraitId::new(u64::try_from(self.traits.len()).map_err(|_| {
                        self.error(source, "too many trait declarations for TraitId")
                    })?);
                self.trait_names.insert(trait_name.clone(), id);
                self.traits.push(TraitDefinition {
                    id,
                    name: trait_name,
                    origin: Origin::Source(source),
                    core: None,
                });
            }
        }
        Ok(())
    }

    pub(in crate::analyze) fn collect_product_names(
        &mut self,
        program: &ValidatedSourceTree,
    ) -> Result<()> {
        for (source_index, file) in program.files().iter().enumerate() {
            let source_raw = u64::try_from(source_index)
                .map_err(|_| Error::msg("too many source files for HIR SourceId"))?;
            let source = SourceId::new(source_raw);
            for form in &file.forms {
                let AstExpr::Call { name, args } = form else {
                    continue;
                };
                if name != "product" {
                    continue;
                }
                let (product_name, _) =
                    product_declaration(args).map_err(|message| self.error(source, message))?;
                if !is_declaration_type_name(&product_name) {
                    return Err(self.error(
                        source,
                        format!("invalid product declaration name {product_name}"),
                    ));
                }
                if Operation::from_name(&product_name).is_some()
                    || is_contextual_name(&product_name)
                    || is_builtin_type_name(&product_name)
                {
                    return Err(self.error(
                        source,
                        format!("product declaration {product_name} collides with a reserved operation, form, or type"),
                    ));
                }
                if self.product_names.contains_key(&product_name) {
                    return Err(self.error(
                        source,
                        format!("duplicate product declaration {product_name}"),
                    ));
                }
                if self.trait_names.contains_key(&product_name) {
                    return Err(self.error(
                        source,
                        format!(
                            "product declaration {product_name} collides with a trait declaration"
                        ),
                    ));
                }
                let raw = u64::try_from(self.product_names.len())
                    .map_err(|_| self.error(source, "product declaration index exceeds u64"))?;
                self.product_names.insert(product_name, ProductId::new(raw));
            }
        }
        Ok(())
    }
}
