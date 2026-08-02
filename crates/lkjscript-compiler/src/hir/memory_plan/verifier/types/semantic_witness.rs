use std::collections::VecDeque;

impl VerifiedTypes<'_> {
    fn producer_semantic_descriptor(&self, root: &Type) -> Result<lkjscript_contracts::SemanticDescriptor> {
        let mut pending = VecDeque::from([root.clone()]);
        let mut declarations = BTreeMap::new();
        while let Some(ty) = pending.pop_front() {
            match ty {
                Type::Product(name) => {
                    let item = self.program.products.iter().find(|item| item.name == name)
                        .ok_or_else(|| Error::msg("semantic closure lost product declaration"))?;
                    if declarations.contains_key(&item.identity) { continue; }
                    let fields = item.fields.iter().map(|field| Ok(lkjscript_contracts::SemanticProductField {
                        identity: field.identity, source_order: field.source_order,
                        ty: self.producer_semantic_type(&field.ty)?,
                    })).collect::<Result<Vec<_>>>()?;
                    for field in &item.fields { pending.push_back(field.ty.clone()); }
                    declarations.insert(item.identity, lkjscript_contracts::SemanticDeclaration::Product(
                        lkjscript_contracts::SemanticProductDeclaration { identity: item.identity, fields }));
                }
                Type::Enum { id, arguments, .. } => {
                    for argument in arguments { pending.push_back(argument); }
                    let item = self.program.enums.iter().find(|item| item.id == id)
                        .ok_or_else(|| Error::msg("semantic closure lost enum declaration"))?;
                    if declarations.contains_key(&id.bytes()) { continue; }
                    let variants = item.variants.iter().map(|variant| {
                        let fields = variant.fields.iter().map(|field| Ok(lkjscript_contracts::SemanticEnumVariantField {
                            identity: field.id.bytes(), source_order: field.source_order,
                            ty: self.producer_semantic_type(&field.ty)?, indirect: field.indirect,
                        })).collect::<Result<Vec<_>>>()?;
                        Ok(lkjscript_contracts::SemanticEnumVariant {
                            identity: variant.id.bytes(), source_order: variant.source_order, fields,
                        })
                    }).collect::<Result<Vec<_>>>()?;
                    for variant in &item.variants { for field in &variant.fields { pending.push_back(field.ty.clone()); } }
                    declarations.insert(id.bytes(), lkjscript_contracts::SemanticDeclaration::Enum(
                        lkjscript_contracts::SemanticEnumDeclaration { identity: id.bytes(),
                            type_parameters: item.type_parameters.clone(), variants }));
                }
                Type::List(inner) => pending.push_back(*inner),
                Type::Fn { params, ret } => { pending.extend(params); pending.push_back(*ret); }
                Type::Forall { body, .. } => pending.push_back(*body),
                _ => {}
            }
        }
        let descriptor = lkjscript_contracts::SemanticDescriptor {
            root: self.producer_semantic_type(root)?, declarations: declarations.into_values().collect(),
        };
        lkjscript_contracts::validate_semantic_descriptor(&descriptor)
            .map_err(|error| Error::msg(error.to_string()))?;
        Ok(descriptor)
    }

    fn producer_semantic_type(&self, ty: &Type) -> Result<lkjscript_contracts::SemanticType> {
        use lkjscript_contracts::{SemanticPrimitiveKind as P, SemanticType as S};
        Ok(match ty {
            Type::Never=>S::Primitive(P::Never), Type::Unit=>S::Primitive(P::Unit),
            Type::Bool=>S::Primitive(P::Bool), Type::I64=>S::Primitive(P::I64),
            Type::F64=>S::Primitive(P::F64), Type::Str=>S::Primitive(P::String),
            Type::Bytes=>S::Primitive(P::Bytes), Type::Path=>S::Primitive(P::Path),
            Type::ByteVector=>S::Primitive(P::ByteVector), Type::ByteSlice=>S::Primitive(P::ByteSlice),
            Type::ByteSliceMut=>S::Primitive(P::ByteSliceMut), Type::Symbol=>S::Primitive(P::Symbol),
            Type::Capability(kind)=>S::Capability(*kind), Type::Resource(kind)=>S::Resource(*kind),
            Type::Product(name)=>S::Product(self.program.products.iter().find(|item| item.name == *name)
                .ok_or_else(|| Error::msg("semantic type lost product identity"))?.identity),
            Type::Enum{id,arguments,..}=>S::Enum{identity:id.bytes(),arguments:arguments.iter()
                .map(|item| self.producer_semantic_type(item)).collect::<Result<_>>()?},
            Type::Param(name)=>S::Parameter(name.clone()),
            Type::List(item)=>S::List(Box::new(self.producer_semantic_type(item)?)),
            Type::Fn{params,ret}=>S::Function{parameters:params.iter().map(|item| self.producer_semantic_type(item))
                .collect::<Result<_>>()?,result:Box::new(self.producer_semantic_type(ret)?)},
            Type::Forall{vars,body}=>S::ForAll{parameters:vars.clone(),body:Box::new(self.producer_semantic_type(body)?)},
        })
    }

    fn producer_witness_dependencies(&self, root: &Type) -> Result<Vec<lkjscript_contracts::ExecutableMemoryWitnessDependency>> {
        use lkjscript_contracts::ExecutableMemoryWitnessRole as R;
        let mut output = Vec::new();
        match root {
            Type::List(element) => self.producer_dependency(root, element, R::ListElement, &mut output)?,
            Type::Product(name) => {
                let item = self.program.products.iter().find(|item| item.name == *name)
                    .ok_or_else(|| Error::msg("witness roles lost product"))?;
                for field in &item.fields { self.producer_dependency(root, &field.ty, R::ProductField {
                    product:item.identity, field:field.identity, source_order:field.source_order }, &mut output)?; }
            }
            Type::Enum{id,arguments,..} => {
                let item = self.program.enums.iter().find(|item| item.id == *id)
                    .ok_or_else(|| Error::msg("witness roles lost enum"))?;
                if arguments.len()!=item.type_parameters.len() { return Err(Error::msg("witness enum argument arity mismatch")); }
                for (index, argument) in arguments.iter().enumerate() { self.producer_dependency(root, argument,
                    R::TypeArgument{constructor:id.bytes(),index:u16::try_from(index).map_err(|_| Error::msg("type argument order overflow"))?}, &mut output)?; }
                let substitutions: HashMap<_,_> = item.type_parameters.iter().cloned().zip(arguments.iter().cloned()).collect();
                for variant in &item.variants { for field in &variant.fields {
                    self.producer_dependency(root, &field.ty.subst(&substitutions), R::EnumVariantField {
                        enumeration:id.bytes(),variant:variant.id.bytes(),field:field.id.bytes(),
                        variant_source_order:variant.source_order,field_source_order:field.source_order}, &mut output)?;
                }}
            }
            _ => {}
        }
        Ok(output)
    }

    fn producer_dependency(&self, root: &Type, child: &Type, role: lkjscript_contracts::ExecutableMemoryWitnessRole,
        output: &mut Vec<lkjscript_contracts::ExecutableMemoryWitnessDependency>) -> Result<()> {
        use lkjscript_contracts::{ExecutableMemoryWitnessDependency as D, ExecutableMemoryWitnessTarget as T};
        let local = verifier_semantic_declaration_key(root).and_then(|left| self.graph.component(&left).map(|component|(left,component)))
            .and_then(|(left,component)| verifier_semantic_declaration_key(child).filter(|right| self.graph.is_recursive(&left)
                && self.graph.component(right)==Some(component)))
            .and_then(|_| lkjscript_contracts::direct_nominal(&self.producer_semantic_type(child).ok()?));
        let target = if let Some(identity)=local { T::LocalSemantic(identity) } else {
            let fact = self.memo.get(child).copied().and_then(|id| self.expected.get(id.index()?))
                .ok_or_else(|| Error::msg("memory verifier dependency child was not independently reconstructed"))?;
            T::ExternalWitness(fact.witness.as_bytes())
        };
        output.push(D { role, target }); Ok(())
    }
}

fn verifier_semantic_declaration_key(ty: &Type) -> Option<VerifiedDeclarationKey> {
    match ty {
        Type::Product(name) => Some(VerifiedDeclarationKey::Product(name.clone())),
        Type::Enum { id, .. } => Some(VerifiedDeclarationKey::Enum(id.bytes())),
        _ => None,
    }
}
