use crate::analyze::*;

pub(crate) fn analyze_interface_program(source: &ValidatedSourceTree) -> Result<hir::Program> {
    let mut analyzer = Analyzer::new(source)?;
    analyzer.install_operations()?;
    analyzer.install_prelude_enums()?;
    analyzer.install_core_traits()?;
    analyzer.collect_trait_names(source)?;
    analyzer.collect_product_names(source)?;
    analyzer.collect_enum_names(source)?;
    analyzer.collect_enums(source)?;
    analyzer.collect_products(source)?;
    analyzer.collect_implementations(source)?;
    let (pending, main) = analyzer.collect_interface_headers(source)?;
    let mut functions = Vec::with_capacity(pending.len());
    for function in pending {
        functions.push(analyzer.resolve_function(
            function.binding,
            function.origin,
            function.parsed,
            function.bounds,
        )?);
    }
    let main = match main {
        Some(main) => analyzer.resolve_main(main)?,
        None => declaration_root(source)?,
    };
    let global_layout = analyzer.build_global_layout(&functions)?;
    let mut program = hir::Program {
        sources: analyzer.sources,
        bindings: analyzer.bindings,
        products: analyzer.products,
        enums: analyzer.enums,
        traits: analyzer.traits,
        implementations: analyzer.implementations,
        match_plans: analyzer.match_plans,
        functions,
        main,
        global_layout,
    };
    resolution::matching::lower_semantic_matches(&mut program)?;
    crate::ownership::check(&program)?;
    crate::effects::infer(&mut program);
    Ok(program)
}

impl Analyzer {
    fn collect_interface_headers<'a>(
        &mut self,
        program: &'a ValidatedSourceTree,
    ) -> Result<(Vec<PendingFunction<'a>>, Option<PendingMain<'a>>)> {
        let mut functions = Vec::new();
        let mut main = None;
        for (source_index, file) in program.files().iter().enumerate() {
            let source = SourceId::new(
                u64::try_from(source_index)
                    .map_err(|_| Error::msg("too many source files for HIR SourceId"))?,
            );
            let is_root = file.path == program.root_path();
            for form in &file.forms {
                match form {
                    AstExpr::Call { name, .. }
                        if matches!(
                            name.as_str(),
                            "import" | "product" | "enum" | "trait" | "impl"
                        ) => {}
                    AstExpr::Call { name, args } if name == "def" => {
                        functions.push(self.collect_definition(source, args)?);
                    }
                    AstExpr::Call { name, args } if name == "main" => {
                        if !is_root {
                            return Err(self.error(source, "imported file may not declare main"));
                        }
                        if main.is_some() {
                            return Err(
                                self.error(source, "executable root declares duplicate main")
                            );
                        }
                        main = Some(self.pending_interface_main(source, args)?);
                    }
                    AstExpr::Call { name, .. } if name == "do" => {
                        return Err(self.error(source, "top-level do was removed; use root main"));
                    }
                    other => {
                        return Err(
                            self.error(source, format!("unsupported top-level form: {other:?}"))
                        );
                    }
                }
            }
        }
        Ok((functions, main))
    }

    fn pending_interface_main<'a>(
        &self,
        source: SourceId,
        args: &'a [AstExpr],
    ) -> Result<PendingMain<'a>> {
        let (param_names, param_types, return_type, body) = parse_main(self, args)
            .map_err(|message| self.error(source, format!("main: {message}")))?;
        let param_types = self.resolve_main_parameter_types(source, param_types)?;
        let return_type = self
            .resolve_enum_type(&return_type, &[])
            .map_err(|message| self.error(source, format!("main: {message}")))?;
        self.validate_product_type(&return_type)
            .map_err(|message| self.error(source, format!("main: {message}")))?;
        if matches!(return_type, Type::ByteSlice | Type::ByteSliceMut) {
            return Err(self.error(source, "main cannot return a lexical reference"));
        }
        let mut free = HashSet::new();
        collect_type_params(&return_type, &mut free);
        if let Some(parameter) = free.into_iter().next() {
            return Err(self.error(
                source,
                format!("main: return type contains unbound parameter {parameter}"),
            ));
        }
        Ok(PendingMain {
            origin: source,
            param_names,
            param_types,
            return_type,
            body,
        })
    }
}

fn declaration_root(source: &ValidatedSourceTree) -> Result<Main> {
    let index = source
        .files()
        .iter()
        .position(|file| file.path == source.root_path())
        .ok_or_else(|| Error::msg("declaration analysis root source is absent"))?;
    let origin = SourceId::new(
        u64::try_from(index).map_err(|_| Error::msg("declaration root SourceId exceeds u64"))?,
    );
    Ok(Main {
        origin: Origin::Source(origin),
        params: Vec::new(),
        param_places: Vec::new(),
        param_types: Vec::new(),
        return_type: Type::Unit,
        arity: 0,
        local_count: 0,
        body: Expr {
            ty: Type::Unit,
            effects: EffectSet::PURE,
            origin: Origin::Source(origin),
            kind: ExprKind::LitUnit,
        },
    })
}
