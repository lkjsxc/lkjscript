use crate::analyze::*;

impl Analyzer {
    pub(in crate::analyze) fn collect_products(
        &mut self,
        program: &ValidatedSourceTree,
    ) -> Result<()> {
        for (source_index, file) in program.files().iter().enumerate() {
            let source_raw = u32::try_from(source_index)
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
                if field_forms.len() > MAX_PRODUCT_FIELDS {
                    return Err(self.error(
                        source,
                        format!(
                            "product {product_name}: too many fields ({} > {MAX_PRODUCT_FIELDS})",
                            field_forms.len()
                        ),
                    ));
                }
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
                let mut names = HashSet::new();
                let mut fields = Vec::with_capacity(field_forms.len());
                for field_form in field_forms {
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
                    fields.push(ProductField {
                        name: field_name,
                        ty,
                    });
                }
                if product.index() != self.products.len() {
                    return Err(self.error(source, "product declaration order is inconsistent"));
                }
                self.products.push(ProductDefinition {
                    id: product,
                    name: product_name,
                    origin: source,
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
        match ty {
            Type::Product(name) => {
                if self.product_names.contains_key(name) {
                    Ok(())
                } else {
                    Err(format!("unknown product type {name}"))
                }
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
                for argument in arguments {
                    self.validate_product_type(argument)?;
                }
                Ok(())
            }
            Type::Owned(inner) | Type::Ref(inner) | Type::RefMut(inner) => {
                if inner.as_ref() == &Type::Buf {
                    Ok(())
                } else {
                    Err("ownership types accept only exact Buf in this slice".into())
                }
            }
            Type::List(inner) | Type::Option(inner) => {
                if contains_ownership_type(inner) {
                    return Err(
                        "ownership/reference types cannot be stored in List or Option".into(),
                    );
                }
                self.validate_product_type(inner)
            }
            Type::Result(ok, error) => {
                if contains_ownership_type(ok) || contains_ownership_type(error) {
                    return Err("ownership/reference types cannot be stored in Result".into());
                }
                self.validate_product_type(ok)?;
                self.validate_product_type(error)
            }
            Type::Fn { params, ret } => {
                for parameter in params {
                    self.validate_product_type(parameter)?;
                }
                self.validate_product_type(ret)
            }
            Type::Forall { body, .. } => self.validate_product_type(body),
            _ => Ok(()),
        }
    }

    pub(in crate::analyze) fn product_by_name(&self, name: &str) -> Result<&ProductDefinition> {
        let id = self
            .product_names
            .get(name)
            .copied()
            .ok_or_else(|| Error::msg(format!("unknown product type {name}")))?;
        self.products
            .get(id.index())
            .filter(|product| product.id == id)
            .ok_or_else(|| Error::msg(format!("missing HIR product metadata for {name}")))
    }
}
