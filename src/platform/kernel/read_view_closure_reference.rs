//! Complete view-dependent function discovery before the legacy flow/cache path.
//! Both arms, unused initializers and complete signature inventories are roots.
use super::*;

impl Resolver<'_> {
    pub(in super::super) fn function_uses_read_view(
        &mut self,
        declaration: DeclarationReference,
    ) -> Result<bool> {
        self.step(0)?;
        let f = Oracle(self.source, Some(declaration))
            .function(declaration)
            .ok_or(Failure::Authority)?;
        if !f.read_scope_parameters.is_empty()
            || !f.read_scope_sources.is_empty()
            || matches!(f.result.relation, ResultRelation::ViewFrom { .. })
        {
            return Ok(true);
        }
        if self.contains_read_view(f.result.ty)? {
            return Ok(true);
        }
        for id in &f.parameters {
            let p = Oracle(self.source, Some(declaration))
                .parameter(declaration.package, *id)
                .ok_or(Failure::Missing)?;
            if self.contains_read_view(p.ty)? {
                return Ok(true);
            }
        }
        // Imported signatures have no fabricated body. Their complete inventory
        // is checked by the source closure; source body obligations stay local.
        if declaration.package != self.source.root.package_id {
            return Ok(false);
        }
        let body = match self
            .source
            .owners
            .get(&OwnerKey::Declaration(declaration.declaration))
        {
            Some(OwnerRecord::Declaration(d)) => match &d.payload {
                DeclarationPayload::Function(f) => f.body,
                _ => return Err(Failure::Authority),
            },
            _ => return Err(Failure::Missing),
        };
        self.reserve_entries(1)?;
        let mut pending = vec![body];
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            self.step(0)?;
            if seen.contains(&id) {
                continue;
            }
            self.reserve_entries(1)?;
            seen.insert(id);
            let expression = match self.source.owners.get(&OwnerKey::Expression(id)) {
                Some(OwnerRecord::Expression(e)) => e,
                _ => return Err(Failure::Missing),
            };
            if matches!(
                expression.operation,
                ExpressionOperation::CreateReadView { .. }
                    | ExpressionOperation::BorrowReadView { .. }
                    | ExpressionOperation::ReborrowReadView { .. }
                    | ExpressionOperation::MoveReadView { .. }
                    | ExpressionOperation::DropReadView { .. }
                    | ExpressionOperation::ReadViewDataField { .. }
                    | ExpressionOperation::BorrowReadViewField { .. }
            ) {
                return Ok(true);
            }
            for ty in expression.type_roots() {
                if self.contains_read_view(ty)? {
                    return Ok(true);
                }
            }
            let target = match &expression.operation {
                ExpressionOperation::Call { function, .. }
                | ExpressionOperation::ImplementationCall { function, .. }
                | ExpressionOperation::FunctionValue { function, .. } => Some(*function),
                _ => None,
            };
            if let Some(target) = target
                && let Some(signature) = Oracle(self.source, Some(target)).function(target)
            {
                if !signature.read_scope_parameters.is_empty()
                    || !signature.read_scope_sources.is_empty()
                    || self.contains_read_view(signature.result.ty)?
                {
                    return Ok(true);
                }
                for parameter in &signature.parameters {
                    let p = Oracle(self.source, Some(target))
                        .parameter(target.package, *parameter)
                        .ok_or(Failure::Missing)?;
                    if self.contains_read_view(p.ty)? {
                        return Ok(true);
                    }
                }
            }
            if let ExpressionOperation::MethodCall {
                contract, method, ..
            } = &expression.operation
            {
                let c = self.contract(*contract)?;
                let m = c
                    .methods
                    .iter()
                    .find(|m| m.id == *method)
                    .ok_or(Failure::Inventory)?;
                if !m.read_scope_parameters.is_empty()
                    || !m.read_scope_sources.is_empty()
                    || self.object(m.result.ty)?.form.is_read_view()
                    || m.parameters
                        .iter()
                        .any(|p| self.object(p.ty).is_ok_and(|t| t.form.is_read_view()))
                {
                    return Ok(true);
                }
            }
            let children = expression.children();
            self.reserve_entries(children.len())?;
            pending.extend(children.into_iter().map(|c| c.expression));
            if let ExpressionOperation::Let { bindings, .. } = &expression.operation {
                for id in bindings {
                    self.step(0)?;
                    let b = match self.source.owners.get(&OwnerKey::Binding(*id)) {
                        Some(OwnerRecord::Binding(b)) => b,
                        _ => return Err(Failure::Missing),
                    };
                    if b.kind == BindingKind::ReadView
                        || b.kind == BindingKind::ReadViewBorrow
                        || b.declared_type
                            .is_some_and(|t| self.object(t).is_ok_and(|t| t.form.is_read_view()))
                    {
                        return Ok(true);
                    }
                    if let Some(value) = b.value {
                        self.reserve_entries(1)?;
                        pending.push(value);
                    }
                }
            }
        }
        Ok(false)
    }
}
