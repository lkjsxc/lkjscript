use crate::analyze::*;

impl Analyzer {
    pub(in crate::analyze) fn collect_headers<'a>(
        &mut self,
        program: &'a ValidatedSourceTree,
    ) -> Result<(Vec<PendingFunction<'a>>, PendingMain<'a>)> {
        let mut functions = Vec::new();
        let mut main = None;
        for (source_index, file) in program.files().iter().enumerate() {
            let source_raw = u32::try_from(source_index)
                .map_err(|_| Error::msg("too many source files for HIR SourceId"))?;
            let source = SourceId::new(source_raw);
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
                        let (param_names, param_types, return_type, body) = parse_main(args)
                            .map_err(|message| self.error(source, format!("main: {message}")))?;
                        let param_types = self.resolve_main_parameter_types(source, param_types)?;
                        let return_type = self
                            .resolve_enum_type(&return_type, &[])
                            .map_err(|message| self.error(source, format!("main: {message}")))?;
                        self.validate_product_type(&return_type)
                            .map_err(|message| self.error(source, format!("main: {message}")))?;
                        if matches!(return_type, Type::ByteSlice | Type::ByteSliceMut) {
                            return Err(self.error(
                                source,
                                "main cannot return a lexical reference in the initial ownership slice",
                            ));
                        }
                        let mut free = HashSet::new();
                        collect_type_params(&return_type, &mut free);
                        if let Some(parameter) = free.into_iter().next() {
                            return Err(self.error(
                                source,
                                format!("main: return type contains unbound parameter {parameter}"),
                            ));
                        }
                        main = Some(PendingMain {
                            origin: source,
                            param_names,
                            param_types,
                            return_type,
                            body,
                        });
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
        let main =
            main.ok_or_else(|| Error::msg("executable root must declare exactly one main"))?;
        Ok((functions, main))
    }

    pub(in crate::analyze) fn collect_definition<'a>(
        &mut self,
        origin: SourceId,
        args: &'a [AstExpr],
    ) -> Result<PendingFunction<'a>> {
        let name = definition_name(args).map_err(|message| self.error(origin, message))?;
        let [_, AstExpr::Call {
            name: tag,
            args: fn_args,
        }] = args
        else {
            return Err(self.error(
                origin,
                format!("def {name}: top-level def must declare an immutable fn"),
            ));
        };
        if tag != "fn" {
            return Err(self.error(
                origin,
                format!("def {name}: top-level def must declare an immutable fn"),
            ));
        }
        let mut parsed = parse_function(fn_args)
            .map_err(|message| self.error(origin, format!("def {name}: {message}")))?;
        self.resolve_function_enum_types(&mut parsed)
            .map_err(|message| self.error(origin, format!("def {name}: {message}")))?;
        validate_function_header(&name, &parsed).map_err(|message| self.error(origin, message))?;
        for ty in parsed
            .signature_params
            .iter()
            .chain(parsed.param_types.iter())
            .chain(std::iter::once(&parsed.signature_return))
        {
            self.validate_product_type(ty)
                .map_err(|message| self.error(origin, format!("def {name}: {message}")))?;
        }
        if matches!(
            parsed.signature_return,
            Type::ByteSlice | Type::ByteSliceMut
        ) {
            return Err(self.error(
                origin,
                format!("def {name}: lexical references cannot be returned in the initial ownership slice"),
            ));
        }
        let monomorphic = Type::Fn {
            params: parsed.signature_params.clone(),
            ret: Box::new(parsed.signature_return.clone()),
        };
        let ty = if parsed.forall_vars.is_empty() {
            monomorphic
        } else {
            Type::Forall {
                vars: parsed.forall_vars.clone(),
                body: Box::new(monomorphic),
            }
        };
        let binding = self.add_global(origin, name, BindingKind::Function, ty)?;
        let mut bounds = Vec::with_capacity(parsed.bounds.len());
        let mut seen = HashSet::new();
        for bound in &parsed.bounds {
            if !parsed
                .forall_vars
                .iter()
                .any(|variable| variable == &bound.parameter)
            {
                return Err(self.error(
                    origin,
                    format!(
                        "bound parameter {} is not declared by forall",
                        bound.parameter
                    ),
                ));
            }
            let trait_id = self
                .trait_names
                .get(&bound.trait_name)
                .copied()
                .ok_or_else(|| {
                    self.error(
                        origin,
                        format!("bound references unknown trait {}", bound.trait_name),
                    )
                })?;
            let trait_definition = self
                .traits
                .get(trait_id.index().unwrap_or(usize::MAX))
                .ok_or_else(|| self.error(origin, "bound resolved an unknown TraitId"))?;
            if matches!(
                trait_definition.core,
                Some(CoreTrait::Clone | CoreTrait::Drop)
            ) {
                return Err(self.error(
                    origin,
                    format!(
                        "core trait {} requires methods and is unavailable in the marker-trait slice",
                        trait_definition.name
                    ),
                ));
            }
            if !seen.insert((bound.parameter.as_str(), trait_id)) {
                return Err(self.error(
                    origin,
                    format!("duplicate bound {} {}", bound.parameter, bound.trait_name),
                ));
            }
            bounds.push(TraitBound {
                parameter: bound.parameter.clone(),
                trait_id,
            });
        }
        self.function_bounds.insert(binding, bounds.clone());
        Ok(PendingFunction {
            binding,
            origin,
            parsed,
            bounds,
        })
    }
}
