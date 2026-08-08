use crate::analyze::*;

impl Analyzer {
    pub(in crate::analyze) fn collect_products(
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
                let (product_name, field_forms) =
                    product_declaration(args).map_err(|message| self.error(source, message))?;
                let product = self
                    .product_names
                    .get(&product_name)
                    .copied()
                    .ok_or_else(|| {
                        self.error(
                            source,
                            format!("unknown product declaration {product_name}"),
                        )
                    })?;
                let declaration = program
                    .declarations()
                    .iter()
                    .find(|declaration| {
                        declaration.kind() == crate::source::DeclarationKind::Product
                            && declaration.name() == product_name
                            && declaration.origin().logical_path() == file.origin.logical_path
                    })
                    .ok_or_else(|| self.error(source, "product declaration identity is missing"))?;
                let identity = declaration.key().digest();
                if identity == [0; 32] {
                    return Err(self.error(source, "product declaration identity is unresolved"));
                }
                let mut names = HashSet::new();
                let mut fields = Vec::with_capacity(field_forms.len());
                for (field_order, field_form) in field_forms.iter().enumerate() {
                    let (field_name, ty) = parse_product_field(field_form).map_err(|message| {
                        self.error(source, format!("product {product_name}: {message}"))
                    })?;
                    let ty = self.resolve_enum_type(&ty, &[]).map_err(|message| {
                        self.error(source, format!("product {product_name}: {message}"))
                    })?;
                    if !names.insert(field_name.clone()) {
                        return Err(self.error(
                            source,
                            format!("product {product_name}: duplicate field {field_name}"),
                        ));
                    }
                    self.validate_product_type(&ty).map_err(|message| {
                        self.error(
                            source,
                            format!("product {product_name} field {field_name}: {message}"),
                        )
                    })?;
                    if contains_ownership_type(&ty) {
                        return Err(self.error(
                            source,
                            format!(
                                "product {product_name} field {field_name}: ownership/reference \
                                 types cannot be stored in products"
                            ),
                        ));
                    }
                    let mut free = HashSet::new();
                    collect_type_params(&ty, &mut free);
                    if let Some(parameter) = free.into_iter().next() {
                        return Err(self.error(
                            source,
                            format!(
                                "product {product_name} field {field_name}: type contains unbound \
                                 parameter {parameter}"
                            ),
                        ));
                    }
                    let source_order = u64::try_from(field_order)
                        .map_err(|_| self.error(source, "product field order exceeds u64"))?;
                    let field_identity =
                        crate::source::product_field_identity(identity, &field_name, source_order)
                            .map_err(|_| {
                                self.error(source, "cannot encode stable product field identity")
                            })?;
                    if field_identity == [0; 32] {
                        return Err(
                            self.error(source, "stable product field identity is unresolved")
                        );
                    }
                    fields.push(ProductField {
                        identity: field_identity,
                        source_order,
                        name: field_name,
                        ty,
                    });
                }
                if product.index() != Some(self.products.len()) {
                    return Err(self.error(source, "product declaration order is inconsistent"));
                }
                self.products.push(ProductDefinition {
                    id: product,
                    identity,
                    name: product_name,
                    origin: Origin::Source(source),
                    fields,
                });
            }
        }
        Ok(())
    }

    pub(in crate::analyze) fn validate_product_type(
        &self,
        ty: &Type,
    ) -> std::result::Result<(), String> {
        let mut pending = vec![ty];
        while let Some(ty) = pending.pop() {
            match ty {
                Type::Never => return Err("Never is not a storage, field, or ABI type".into()),
                Type::Product(name) if !self.product_names.contains_key(name) => {
                    return Err(format!("unknown product type {name}"));
                }
                Type::Enum {
                    id,
                    name,
                    arguments,
                } => {
                    let Some((expected, parameters)) = self.enum_headers.get(name) else {
                        return Err(format!("unknown enum type {name}"));
                    };
                    if id != expected || arguments.len() != parameters.len() {
                        return Err(format!("enum type {name} has invalid identity or arity"));
                    }
                    pending.extend(arguments);
                }
                Type::List(inner) => {
                    if contains_ownership_type(inner) {
                        return Err("ownership/reference types cannot be stored in List".into());
                    }
                    pending.push(inner);
                }
                Type::Fn { params, ret } => {
                    pending.push(ret);
                    pending.extend(params);
                }
                Type::Forall { body, .. } => pending.push(body),
                _ => {}
            }
        }
        Ok(())
    }

    pub(in crate::analyze) fn product_by_name(&self, name: &str) -> Result<&ProductDefinition> {
        let id = self
            .product_names
            .get(name)
            .copied()
            .ok_or_else(|| Error::msg(format!("unknown product type {name}")))?;
        self.products
            .get(id.index().ok_or_else(|| {
                Error::msg(format!("ProductId for {name} exceeds host index width"))
            })?)
            .filter(|product| product.id == id)
            .ok_or_else(|| Error::msg(format!("missing HIR product metadata for {name}")))
    }
}
