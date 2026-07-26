use crate::analyze::*;

impl<'a> Resolver<'a> {
    pub(in crate::analyze) fn new(
        analyzer: &'a mut Analyzer,
        origin: SourceId,
        function_scope: HashMap<String, BindingId>,
        local_slots: HashMap<BindingId, u8>,
        type_variables: HashSet<String>,
        parameter_count: usize,
    ) -> Self {
        let local_places = local_slots
            .iter()
            .map(|(binding, slot)| (*binding, PlaceId::new(u32::from(*slot))))
            .collect();
        Self {
            analyzer,
            origin,
            scopes: vec![function_scope],
            local_slots,
            local_places,
            type_variables,
            next_slot: parameter_count,
            max_slots: parameter_count,
            next_place: u32::try_from(parameter_count).unwrap_or(u32::MAX),
        }
    }

    pub(in crate::analyze) fn place(&self, binding: BindingId) -> Result<PlaceId> {
        self.local_places.get(&binding).copied().ok_or_else(|| {
            self.error(format!(
                "binding {} has no whole-local PlaceId",
                binding.raw()
            ))
        })
    }

    pub(in crate::analyze) fn allocate_place(&mut self, binding: BindingId) -> Result<PlaceId> {
        let place = PlaceId::new(self.next_place);
        self.next_place = self
            .next_place
            .checked_add(1)
            .ok_or_else(|| self.error("too many ownership places"))?;
        self.local_places.insert(binding, place);
        Ok(place)
    }

    pub(in crate::analyze) fn allocate_loan(&mut self) -> Result<LoanId> {
        let loan = LoanId::new(self.analyzer.next_loan);
        self.analyzer.next_loan = self
            .analyzer
            .next_loan
            .checked_add(1)
            .ok_or_else(|| self.error("too many ownership loans in program closure"))?;
        Ok(loan)
    }

    pub(in crate::analyze) fn local_count(&self) -> Result<u8> {
        u8::try_from(self.max_slots)
            .map_err(|_| self.error("expression needs more than 255 bytecode local slots"))
    }

    pub(in crate::analyze) fn resolve_expr(&mut self, expression: &AstExpr) -> Result<Expr> {
        match expression {
            AstExpr::LitUnit => Ok(self.expression(Type::Unit, ExprKind::LitUnit)),
            AstExpr::LitBool(value) => Ok(self.expression(Type::Bool, ExprKind::LitBool(*value))),
            AstExpr::LitI64(value) => Ok(self.expression(Type::I64, ExprKind::LitI64(*value))),
            AstExpr::LitF64(value) => Ok(self.expression(Type::F64, ExprKind::LitF64(*value))),
            AstExpr::LitStr(value) => {
                Ok(self.expression(Type::Str, ExprKind::LitStr(value.clone())))
            }
            AstExpr::Symbol(name) => self.resolve_load(name),
            AstExpr::List(_) => Err(self.error("raw list literal needs typed construction")),
            AstExpr::Call { name, args } => self.resolve_call(name, args),
        }
    }

    pub(in crate::analyze) fn resolve_load(&self, name: &str) -> Result<Expr> {
        let binding = self.lookup(name).ok_or_else(|| {
            self.diagnostic(AnalysisDiagnostic::UnknownName {
                usage: NameUse::Symbol,
                name: name.to_string(),
            })
        })?;
        let resolved = self.analyzer.binding(binding)?;
        if matches!(resolved.kind, BindingKind::BuiltinOperation(_)) {
            return Err(self.error(format!(
                "operation {name} is not a first-class value; call it directly"
            )));
        }
        if resolved.kind == BindingKind::Function
            && self
                .analyzer
                .function_bounds
                .get(&binding)
                .is_some_and(|bounds| !bounds.is_empty())
        {
            return Err(self.error(format!(
                "bounded generic function {name} is not a first-class value in the marker slice; call it directly"
            )));
        }
        let ty = resolved.ty.clone();
        let binding = self.binding_ref(binding)?;
        Ok(self.expression(ty, ExprKind::Load(binding)))
    }
}

pub(in crate::analyze) fn context_only_form(name: &str) -> bool {
    matches!(
        name,
        "fn" | "def"
            | "main"
            | "sig"
            | "params"
            | "forall"
            | "bounds"
            | "bound"
            | "type"
            | "import"
            | "name"
            | "product"
            | "fields"
            | "variant"
            | "variant-field"
            | "enum"
            | "variants"
            | "trait"
            | "impl"
            | "for"
            | "arms"
            | "arm"
            | "wildcard"
            | "binding"
            | "bool-pattern"
            | "i64-pattern"
            | "variant-pattern"
            | "variant-field-pattern"
            | "product-pattern"
            | "product-field-pattern"
    )
}
