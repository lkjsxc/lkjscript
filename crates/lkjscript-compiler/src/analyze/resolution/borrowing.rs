use crate::analyze::*;

impl Resolver<'_> {
    pub(in crate::analyze) fn resolve_move(&mut self, args: &[AstExpr]) -> Result<Expr> {
        let [AstExpr::Symbol(name)] = args else {
            return Err(self.error("move expects exactly one whole local or parameter name"));
        };
        let binding = self.lookup_lexical(name).ok_or_else(|| {
            self.error(format!(
                "move target {name} is not a whole local or parameter"
            ))
        })?;
        let ty = self.analyzer.binding(binding)?.ty.clone();
        match ty {
            Type::Owned(ref inner) if inner.as_ref() == &Type::Buf => {}
            Type::RefMut(_) => {
                return Err(
                    self.error("RefMut forwarding is unsupported in the initial ownership slice")
                );
            }
            _ => return Err(self.error("move requires an affine Owned Buf place")),
        }
        let place = self.place(binding)?;
        let binding = self.binding_ref(binding)?;
        Ok(self.expression(ty, ExprKind::Move { place, binding }))
    }

    pub(in crate::analyze) fn resolve_borrow(
        &mut self,
        args: &[AstExpr],
        kind: BorrowKind,
    ) -> Result<Expr> {
        let [AstExpr::Symbol(name)] = args else {
            return Err(
                self.error("borrow expects exactly one whole Owned Buf local or parameter name")
            );
        };
        let binding = self.lookup_lexical(name).ok_or_else(|| {
            self.error(format!(
                "borrow target {name} is not a whole local or parameter"
            ))
        })?;
        let owner_ty = self.analyzer.binding(binding)?.ty.clone();
        if owner_ty != Type::Owned(Box::new(Type::Buf)) {
            return Err(self.error(
                "borrow target must have exact type Owned Buf; reborrow and legacy Buf are unsupported",
            ));
        }
        let place = self.place(binding)?;
        let loan = self.allocate_loan()?;
        let binding = self.binding_ref(binding)?;
        let ty = match kind {
            BorrowKind::Shared => Type::Ref(Box::new(Type::Buf)),
            BorrowKind::Mutable => Type::RefMut(Box::new(Type::Buf)),
        };
        Ok(self.expression(
            ty,
            ExprKind::Borrow {
                place,
                loan,
                kind,
                binding,
            },
        ))
    }
}
