//! Source/result interpretation from exact canonical ordered signatures. A
//! scope relationship certifies validity, never equality of source anchors.
use super::*;
use crate::platform::semantic_id::ParameterId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in super::super) struct Signature {
    pub binder: ReadScopeBinder,
    pub parameters: Vec<(TypeObjectDigest, ParameterUse)>,
    pub categories: Vec<ReadCategory>,
    pub source_scopes: Vec<(usize, ReadScopeArgument)>,
    pub result: CallableResult<usize>,
    pub result_category: ReadCategory,
}

impl Resolver<'_> {
    pub(in super::super) fn read_view_function_signature(
        &mut self,
        declaration: DeclarationReference,
    ) -> Result<Signature> {
        self.step(0)?;
        let function = Oracle(self.source, Some(declaration))
            .function(declaration)
            .ok_or(Failure::Authority)?;
        let header = self.header(
            declaration.package,
            OwnerKey::Declaration(declaration.declaration),
            false,
        )?;
        if !contract::supported_graph_contract(header.contract_version)
            || !matches!(
                header.kind,
                OwnerKind::PureFunction | OwnerKind::TaskFunction
            )
            || matches!(function.effect, FunctionEffect::Pure)
                != (header.kind == OwnerKind::PureFunction)
        {
            return Err(Failure::Authority);
        }
        let binder = ReadScopeBinder::Declaration(declaration);
        self.reserve_entries(
            function
                .parameters
                .len()
                .checked_mul(3)
                .ok_or(Failure::Capacity)?,
        )?;
        let mut parameters = Vec::with_capacity(function.parameters.len());
        let mut ids = BTreeSet::new();
        for id in &function.parameters {
            self.step(0)?;
            if !ids.insert(*id) {
                return Err(Failure::Inventory);
            }
            let p = Oracle(self.source, Some(declaration))
                .parameter(declaration.package, *id)
                .ok_or(Failure::Missing)?;
            if p.header.owner != OwnerKey::Parameter(*id)
                || p.header.kind != OwnerKind::Parameter
                || !contract::supported_graph_contract(p.header.contract_version)
                || p.parent != ParameterParent::Function(declaration.declaration)
            {
                return Err(Failure::Authority);
            }
            if self.contains_read_view(p.ty)? {
                self.current_view(Occurrence {
                    parameter_scope: Some(declaration),
                    context: Context::Lexical(declaration),
                    consumer: (declaration.package, OwnerKey::Parameter(*id)),
                })?;
            }
            if p.resource_requirement.is_some()
                && !matches!(self.object(p.ty)?.form, TypeForm::CapabilityResource { .. })
            {
                return Err(Failure::Category);
            }
            parameters.push((p.ty, p.use_mode));
        }
        let position = |source: ParameterId| {
            function
                .parameters
                .iter()
                .position(|id| *id == source)
                .ok_or(Failure::Authority)
        };
        let relation = match &function.result.relation {
            ResultRelation::Value => ResultRelation::Value,
            ResultRelation::ReadFrom { source } => ResultRelation::ReadFrom {
                source: position(*source)?,
            },
            ResultRelation::ViewFrom { source, scope } => ResultRelation::ViewFrom {
                source: position(*source)?,
                scope: *scope,
            },
        };
        self.reserve_entries(function.read_scope_sources.len())?;
        let sources = function
            .read_scope_sources
            .iter()
            .map(|s| Ok((position(s.source)?, s.scope)))
            .collect::<Result<Vec<_>>>()?;
        self.read_view_signature(
            binder,
            parameters,
            sources,
            CallableResult {
                ty: function.result.ty,
                relation,
            },
            &function.effect,
        )
    }

    pub(in super::super) fn read_view_method_signature(
        &mut self,
        contract: DeclarationReference,
        method: &OwnedMethod,
    ) -> Result<Signature> {
        self.step(0)?;
        let inventory = self.contract(contract)?;
        if inventory
            .methods
            .iter()
            .filter(|m| m.id == method.id)
            .count()
            != 1
            || !inventory.methods.iter().any(|m| m == method)
        {
            return Err(Failure::Authority);
        }
        self.reserve_entries(method.parameters.len() + method.read_scope_sources.len())?;
        let relation = match method.result.relation {
            ResultRelation::Value => ResultRelation::Value,
            ResultRelation::ReadFrom { source } => ResultRelation::ReadFrom {
                source: source as usize,
            },
            ResultRelation::ViewFrom { source, scope } => ResultRelation::ViewFrom {
                source: source as usize,
                scope,
            },
        };
        self.read_view_signature(
            ReadScopeBinder::ContractMethod {
                contract,
                method: method.id,
            },
            method
                .parameters
                .iter()
                .map(|p| (p.ty, p.use_mode))
                .collect(),
            method
                .read_scope_sources
                .iter()
                .map(|s| (s.source as usize, s.scope))
                .collect(),
            CallableResult {
                ty: method.result.ty,
                relation,
            },
            &method.effect,
        )
    }

    fn read_view_signature(
        &mut self,
        binder: ReadScopeBinder,
        parameters: Vec<(TypeObjectDigest, ParameterUse)>,
        source_scopes: Vec<(usize, ReadScopeArgument)>,
        result: CallableResult<usize>,
        effect: &FunctionEffect,
    ) -> Result<Signature> {
        self.step(0)?;
        let declaration = match binder {
            ReadScopeBinder::Declaration(d)
            | ReadScopeBinder::ContractMethod { contract: d, .. } => d,
            _ => return Err(Failure::Authority),
        };
        let context = match binder {
            ReadScopeBinder::ContractMethod { .. } => Context::Contract(declaration),
            _ => Context::Lexical(declaration),
        };
        let occurrence = self.declaration_occurrence(declaration, context);
        let mut view_bearing = !source_scopes.is_empty()
            || !self.scope_parameters(&binder)?.is_empty()
            || matches!(result.relation, ResultRelation::ViewFrom { .. })
            || self.contains_read_view(result.ty)?;
        for (ty, _) in &parameters {
            view_bearing |= self.contains_read_view(*ty)?;
        }
        if view_bearing {
            self.current_view(occurrence)?;
        }
        self.admit_scope_inventory(&binder)?;
        self.reserve_entries(parameters.len() + source_scopes.len())?;
        let mut categories = Vec::with_capacity(parameters.len());
        let mut prior_class = 0;
        for (ty, mode) in &parameters {
            self.step(0)?;
            self.admit_roots(&[*ty], occurrence, false)?;
            let category = self.read_view_category(*ty, occurrence)?;
            let class = match category {
                ReadCategory::Owned => 1,
                ReadCategory::Capability => 2,
                _ => 0,
            };
            if class < prior_class {
                return Err(Failure::Category);
            }
            prior_class = class;
            match category {
                ReadCategory::ReadView if *mode != ParameterUse::Borrow => {
                    return Err(Failure::Category);
                }
                ReadCategory::Owned if *mode == ParameterUse::Unrestricted => {
                    return Err(Failure::Category);
                }
                ReadCategory::Ordinary | ReadCategory::Callable
                    if *mode != ParameterUse::Unrestricted =>
                {
                    return Err(Failure::Category);
                }
                _ => {}
            }
            if let Some(scope) = self.read_view_scope(*ty)? {
                self.admit_scope_argument(&scope, &binder, occurrence)?;
            }
            categories.push(category);
        }
        let mut seen = BTreeSet::new();
        for (source, scope) in &source_scopes {
            self.step(0)?;
            if !seen.insert((*source, *scope))
                || *source >= parameters.len()
                || parameters[*source].1 != ParameterUse::Borrow
                || !matches!(
                    categories[*source],
                    ReadCategory::Owned | ReadCategory::ReadView
                )
            {
                return Err(Failure::Authority);
            }
            self.admit_scope_argument(scope, &binder, occurrence)?;
        }
        self.admit_roots(&[result.ty], occurrence, false)?;
        let result_category = self.read_view_category(result.ty, occurrence)?;
        if matches!(effect, FunctionEffect::Pure)
            && categories.contains(&ReadCategory::Capability)
            && (categories
                .iter()
                .any(|c| matches!(c, ReadCategory::Owned | ReadCategory::ReadView))
                || matches!(
                    result_category,
                    ReadCategory::Owned | ReadCategory::ReadView
                ))
        {
            return Err(Failure::Category);
        }
        match &result.relation {
            ResultRelation::Value => {
                if result_category == ReadCategory::ReadView {
                    return Err(Failure::Category);
                }
            }
            ResultRelation::ReadFrom { source } => {
                self.signature_loan_source(*source, &parameters, &categories, effect)?;
                if result_category != ReadCategory::Owned {
                    return Err(Failure::Category);
                }
            }
            ResultRelation::ViewFrom { source, scope } => {
                self.signature_loan_source(*source, &parameters, &categories, effect)?;
                self.admit_scope_argument(scope, &binder, occurrence)?;
                if result_category != ReadCategory::ReadView
                    || self.read_view_scope(result.ty)? != Some(*scope)
                {
                    return Err(Failure::Category);
                }
                if categories[*source] == ReadCategory::Owned {
                    if !source_scopes.contains(&(*source, *scope)) {
                        return Err(Failure::Authority);
                    }
                } else if self.read_view_scope(parameters[*source].0)? != Some(*scope)
                    && !source_scopes.contains(&(*source, *scope))
                {
                    return Err(Failure::Authority);
                }
            }
        }
        // Named tasks may borrow views, but loans cannot leave a task or an
        // external. Empty Task rows do not establish purity.
        if !matches!(effect, FunctionEffect::Pure)
            && !matches!(result.relation, ResultRelation::Value)
        {
            return Err(Failure::Category);
        }
        Ok(Signature {
            binder,
            parameters,
            categories,
            source_scopes,
            result,
            result_category,
        })
    }

    fn signature_loan_source(
        &self,
        source: usize,
        parameters: &[(TypeObjectDigest, ParameterUse)],
        categories: &[ReadCategory],
        effect: &FunctionEffect,
    ) -> Result<()> {
        if !matches!(effect, FunctionEffect::Pure)
            || source >= parameters.len()
            || parameters[source].1 != ParameterUse::Borrow
            || !matches!(
                categories[source],
                ReadCategory::Owned | ReadCategory::ReadView
            )
        {
            Err(Failure::Authority)
        } else {
            Ok(())
        }
    }

    pub(super) fn read_view_scope(
        &mut self,
        ty: TypeObjectDigest,
    ) -> Result<Option<ReadScopeArgument>> {
        self.step(0)?;
        Ok(match self.object(ty)?.form {
            TypeForm::ReadViewApplied { read_scope, .. }
            | TypeForm::AssociatedReadViewMember { read_scope, .. }
            | TypeForm::AssociatedReadViewProjection { read_scope, .. } => Some(read_scope),
            _ => None,
        })
    }
}
