impl TypePlanner<'_> {
    fn producer_semantic_descriptor(
        &self,
        root: &Type,
    ) -> Result<lkjscript_contracts::SemanticDescriptor> {
        let mut pending = VecDeque::from([root]);
        let mut declarations = BTreeMap::new();
        while let Some(ty) = pending.pop_front() {
            match ty {
                Type::Product(name) => {
                    let item = self
                        .program
                        .products
                        .iter()
                        .find(|item| item.name == *name)
                        .ok_or_else(|| {
                            Error::msg("semantic closure lost product declaration")
                        })?;
                    if declarations.contains_key(&item.identity) {
                        continue;
                    }
                    let fields = item
                        .fields
                        .iter()
                        .map(|field| {
                            Ok(lkjscript_contracts::SemanticProductField {
                                identity: field.identity,
                                source_order: field.source_order,
                                ty: self.producer_semantic_type(&field.ty)?,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    pending.extend(item.fields.iter().map(|field| &field.ty));
                    declarations.insert(
                        item.identity,
                        lkjscript_contracts::SemanticDeclaration::Product(
                            lkjscript_contracts::SemanticProductDeclaration {
                                identity: item.identity,
                                fields,
                            },
                        ),
                    );
                }
                Type::Enum { id, arguments, .. } => {
                    pending.extend(arguments);
                    let item = self
                        .program
                        .enums
                        .iter()
                        .find(|item| item.id == *id)
                        .ok_or_else(|| Error::msg("semantic closure lost enum declaration"))?;
                    if declarations.contains_key(&id.bytes()) {
                        continue;
                    }
                    let variants = item
                        .variants
                        .iter()
                        .map(|variant| {
                            let fields = variant
                                .fields
                                .iter()
                                .map(|field| {
                                    Ok(lkjscript_contracts::SemanticEnumVariantField {
                                        identity: field.id.bytes(),
                                        source_order: field.source_order,
                                        ty: self.producer_semantic_type(&field.ty)?,
                                        indirect: field.indirect,
                                    })
                                })
                                .collect::<Result<Vec<_>>>()?;
                            Ok(lkjscript_contracts::SemanticEnumVariant {
                                identity: variant.id.bytes(),
                                source_order: variant.source_order,
                                fields,
                            })
                        })
                        .collect::<Result<Vec<_>>>()?;
                    for variant in &item.variants {
                        pending.extend(variant.fields.iter().map(|field| &field.ty));
                    }
                    declarations.insert(
                        id.bytes(),
                        lkjscript_contracts::SemanticDeclaration::Enum(
                            lkjscript_contracts::SemanticEnumDeclaration {
                                identity: id.bytes(),
                                type_parameters: item.type_parameters.clone(),
                                variants,
                            },
                        ),
                    );
                }
                Type::List(inner) => pending.push_back(inner),
                Type::Fn { params, ret } => {
                    pending.extend(params);
                    pending.push_back(ret);
                }
                Type::Forall { body, .. } => pending.push_back(body),
                _ => {}
            }
        }
        let descriptor = lkjscript_contracts::SemanticDescriptor {
            root: self.producer_semantic_type(root)?,
            declarations: declarations.into_values().collect(),
        };
        lkjscript_contracts::validate_semantic_descriptor(&descriptor)
            .map_err(|error| Error::msg(error.to_string()))?;
        Ok(descriptor)
    }
}
