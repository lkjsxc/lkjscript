use crate::analyze::*;

impl Resolver<'_> {
    pub(super) fn parse_match_pattern(
        &mut self,
        form: &AstExpr,
        expected: &Type,
    ) -> Result<MatchPattern> {
        crate::stack::grow(|| self.parse_match_pattern_inner(form, expected))
    }

    fn parse_match_pattern_inner(
        &mut self,
        form: &AstExpr,
        expected: &Type,
    ) -> Result<MatchPattern> {
        let AstExpr::Call { name, args } = form else {
            return Err(self.error("match arm pattern must be one closed pattern marker"));
        };
        match name.as_str() {
            "wildcard" if args.is_empty() => Ok(MatchPattern::Wildcard {
                ty: expected.clone(),
            }),
            "binding" => self.parse_binding_pattern(args, expected),
            "bool-pattern" => match args.as_slice() {
                [AstExpr::LitBool(value)] if expected == &Type::Bool => {
                    Ok(MatchPattern::Bool(*value))
                }
                [_] => Err(self.error("bool-pattern requires a Bool scrutinee")),
                _ => Err(self.error("bool-pattern contains exactly one Bool literal")),
            },
            "i64-pattern" => match args.as_slice() {
                [AstExpr::LitI64(value)] if expected == &Type::I64 => Ok(MatchPattern::I64(*value)),
                [_] => Err(self.error("i64-pattern requires an I64 scrutinee")),
                _ => Err(self.error("i64-pattern contains exactly one I64 literal")),
            },
            "variant-pattern" => self.parse_variant_pattern(args, expected),
            "product-pattern" => self.parse_product_pattern(args, expected),
            _ => Err(self.error(format!("unknown or malformed closed match pattern {name}/"))),
        }
    }

    fn parse_binding_pattern(&mut self, args: &[AstExpr], expected: &Type) -> Result<MatchPattern> {
        let [name_form] = args else {
            return Err(self.error("binding pattern expects exactly name/"));
        };
        let name = declared_name_form(name_form, "binding pattern")
            .map_err(|message| self.error(message))?;
        if self
            .scopes
            .last()
            .is_some_and(|scope| scope.contains_key(&name))
        {
            return Err(self.error(format!("duplicate pattern binding {name}")));
        }
        let local =
            self.allocate_match_local(name.clone(), expected.clone(), BindingKind::ImmutableLocal)?;
        let Some(scope) = self.scopes.last_mut() else {
            return Err(Error::msg("missing match arm scope"));
        };
        scope.insert(name, local.binding);
        Ok(MatchPattern::Binding { local })
    }

    pub(super) fn resolve_pattern_type(&self, form: &AstExpr) -> Result<Type> {
        let AstExpr::Call { name, args } = form else {
            return Err(self.error("pattern type must be type/"));
        };
        if name != "type" {
            return Err(self.error("pattern must state type/ first"));
        }
        let unresolved =
            parse_type_form(self.analyzer, args).map_err(|message| self.error(message))?;
        let parameters: Vec<_> = self.type_variables.iter().cloned().collect();
        let ty = self
            .analyzer
            .resolve_enum_type(&unresolved, &parameters)
            .map_err(|message| self.error(message))?;
        self.analyzer
            .validate_product_type(&ty)
            .map_err(|message| self.error(message))?;
        Ok(ty)
    }

    pub(super) fn allocate_hidden_match_local(&mut self, ty: Type) -> Result<MatchLocal> {
        let name = format!("$match{}", self.analyzer.bindings.len());
        self.allocate_match_local(name, ty, BindingKind::MatchTemporary)
    }

    fn allocate_match_local(
        &mut self,
        name: String,
        ty: Type,
        kind: BindingKind,
    ) -> Result<MatchLocal> {
        let slot = self.next_slot;
        self.next_slot = self
            .next_slot
            .checked_add(1)
            .ok_or_else(|| self.error("match local slot count overflow"))?;
        self.max_slots = self.max_slots.max(self.next_slot);
        let binding =
            self.analyzer
                .add_binding(name, kind, ty.clone(), Origin::Source(self.origin))?;
        self.local_slots.insert(binding, slot);
        let place = self.allocate_place(binding)?;
        Ok(MatchLocal {
            binding,
            place,
            slot,
            ty,
        })
    }
}
