use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn binding_ref(&self, binding: BindingId) -> Result<BindingRef> {
        let storage = match self.analyzer.binding(binding)?.kind {
            BindingKind::Parameter | BindingKind::ImmutableLocal | BindingKind::MutableLocal => {
                BindingStorage::Local(self.local_slots.get(&binding).copied().ok_or_else(|| {
                    self.error(format!("binding {} has no HIR local slot", binding.raw()))
                })?)
            }
            BindingKind::Function => BindingStorage::Function,
            BindingKind::BuiltinOperation(_) => {
                return Err(self.error("built-in operation cannot be loaded as a binding"));
            }
        };
        Ok(BindingRef { binding, storage })
    }

    pub(in crate::analyze) fn lookup(&self, name: &str) -> Option<BindingId> {
        self.lookup_lexical(name)
            .or_else(|| self.analyzer.globals.get(name).copied())
            .or_else(|| {
                Operation::from_name(name)
                    .and_then(|operation| self.analyzer.operations.get(&operation).copied())
            })
    }

    pub(in crate::analyze) fn lookup_call(&self, name: &str) -> Option<BindingId> {
        self.lookup_lexical(name)
            .or_else(|| self.analyzer.globals.get(name).copied())
            .or_else(|| {
                Operation::from_name(name)
                    .and_then(|operation| self.analyzer.operations.get(&operation).copied())
            })
    }

    pub(in crate::analyze) fn lookup_lexical(&self, name: &str) -> Option<BindingId> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.get(name) {
                return Some(*binding);
            }
        }
        None
    }

    pub(in crate::analyze) fn expression(&self, ty: Type, kind: ExprKind) -> Expr {
        let effects = self.effects(&kind);
        Expr {
            ty,
            effects,
            origin: self.origin,
            kind,
        }
    }

    pub(in crate::analyze) fn effects(&self, kind: &ExprKind) -> EffectSet {
        match kind {
            ExprKind::LitI64(_)
            | ExprKind::LitF64(_)
            | ExprKind::LitBool(_)
            | ExprKind::LitUnit
            | ExprKind::EmptyList
            | ExprKind::LitNone
            | ExprKind::LitStr(_)
            | ExprKind::MatchUnreachable { .. }
            | ExprKind::QuoteSymbol(_) => EffectSet::PURE,
            ExprKind::Load(_) | ExprKind::Move { .. } | ExprKind::Borrow { .. } => EffectSet::PURE,
            ExprKind::Call { args, .. } => fold_effects(args).union(EffectSet::CONSERVATIVE_CALL),
            ExprKind::Operation {
                operation, args, ..
            } => fold_effects(args).union(operation.effects()),
            ExprKind::Do(expressions) => fold_effects(expressions),
            ExprKind::If {
                condition,
                then_branch,
                else_branch,
            } => condition
                .effects
                .union(then_branch.effects)
                .union(else_branch.effects),
            ExprKind::While {
                condition, body, ..
            } => condition
                .effects
                .union(fold_effects(body))
                .union(EffectSet::MAY_DIVERGE),
            ExprKind::Loop { body, .. } => fold_effects(body).union(EffectSet::MAY_DIVERGE),
            ExprKind::Return { value } | ExprKind::Break { value, .. } => {
                value.effects.union(EffectSet::MAY_DIVERGE)
            }
            ExprKind::Continue { .. } => EffectSet::MAY_DIVERGE,
            ExprKind::Trap { value } => value.effects.union(EffectSet::MAY_TRAP),
            ExprKind::Exit { code } => code
                .effects
                .union(EffectSet::HOST_IO)
                .union(EffectSet::MAY_EXIT),
            ExprKind::Let { bindings, body } => bindings
                .iter()
                .fold(EffectSet::PURE, |effects, binding| {
                    effects.union(binding.value.effects)
                })
                .union(body.effects),
            ExprKind::MutableLocal { initial, body, .. } => initial.effects.union(body.effects),
            ExprKind::SetLocal { value, .. } => value.effects.union(EffectSet::MUTATES_LOCAL),
            ExprKind::ProductValue { fields, .. } | ExprKind::EnumValue { fields, .. } => {
                fold_effects(fields).union(EffectSet::ALLOCATES)
            }
            ExprKind::ProductField { value, .. }
            | ExprKind::EnumIsVariant { value, .. }
            | ExprKind::EnumField { value, .. } => value.effects.union(EffectSet::READS_MEMORY),
            ExprKind::WithProductField {
                value, replacement, ..
            } => value
                .effects
                .union(replacement.effects)
                .union(EffectSet::READS_MEMORY)
                .union(EffectSet::ALLOCATES),
        }
    }

    pub(in crate::analyze) fn diagnostic(&self, diagnostic: AnalysisDiagnostic) -> Error {
        self.analyzer.diagnostic(self.origin, diagnostic)
    }

    pub(in crate::analyze) fn error(&self, message: impl Into<String>) -> Error {
        self.analyzer.error(self.origin, message)
    }
}
