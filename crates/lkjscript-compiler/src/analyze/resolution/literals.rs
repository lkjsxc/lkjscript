use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn resolve_empty_list(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let element = parse_type_form(args)
            .map_err(|message| self.error(format!("empty-list: {message}")))?;
        self.analyzer
            .validate_product_type(&element)
            .map_err(|message| self.error(format!("empty-list: {message}")))?;
        let mut parameters = HashSet::new();
        collect_type_params(&element, &mut parameters);
        if let Some(parameter) = parameters
            .into_iter()
            .find(|parameter| !self.type_variables.contains(*parameter))
        {
            return Err(self.error(format!(
                "empty-list: type parameter {parameter} is not declared by forall"
            )));
        }
        Ok(self.expression(Type::List(Box::new(element)), ExprKind::EmptyList))
    }

    pub(in crate::analyze) fn resolve_none(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let value_type =
            parse_type_form(args).map_err(|message| self.error(format!("none: {message}")))?;
        let value_type = self
            .analyzer
            .resolve_enum_type(
                &value_type,
                &self.type_variables.iter().cloned().collect::<Vec<_>>(),
            )
            .map_err(|message| self.error(format!("none: {message}")))?;
        self.analyzer
            .validate_product_type(&value_type)
            .map_err(|message| self.error(format!("none: {message}")))?;
        let mut parameters = HashSet::new();
        collect_type_params(&value_type, &mut parameters);
        if let Some(parameter) = parameters
            .into_iter()
            .find(|parameter| !self.type_variables.contains(*parameter))
        {
            return Err(self.error(format!(
                "none: type parameter {parameter} is not declared by forall"
            )));
        }
        Ok(self.expression(
            crate::types::option_type(value_type),
            ExprKind::EnumValue {
                enum_id: EnumId::new(lkjscript_core::OPTION_ID),
                variant: VariantId::new(lkjscript_core::OPTION_NONE_ID),
                layout: crate::types::prelude_layout(lkjscript_core::PreludeEnum::Option),
                fields: Vec::new(),
            },
        ))
    }

    pub(in crate::analyze) fn resolve_quote(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let symbol = match args {
            [AstExpr::Symbol(symbol)] => symbol.clone(),
            [_] => return Err(self.error("quote accepts only a symbol")),
            _ => return Err(self.error("quote expects one symbol")),
        };
        Ok(self.expression(Type::Symbol, ExprKind::QuoteSymbol(symbol)))
    }
}
