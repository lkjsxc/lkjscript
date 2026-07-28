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
            Type::Bytes | Type::ByteVector | Type::Resource(_) => {}
            Type::ByteSliceMut => {
                return Err(self.error(
                    "byte-slice-mut forwarding is unsupported in the initial ownership slice",
                ));
            }
            _ => {
                return Err(self
                    .error("move requires affine bytes, byte-vector, or a typed resource place"))
            }
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
                self.error("borrow expects exactly one whole byte-vector local or parameter name")
            );
        };
        let binding = self.lookup_lexical(name).ok_or_else(|| {
            self.error(format!(
                "borrow target {name} is not a whole local or parameter"
            ))
        })?;
        let owner_ty = self.analyzer.binding(binding)?.ty.clone();
        if owner_ty != Type::ByteVector {
            return Err(self.error(
                "borrow target must have exact type byte-vector; reborrow and legacy Buf are unsupported",
            ));
        }
        let place = self.place(binding)?;
        let loan = self.allocate_loan()?;
        let binding = self.binding_ref(binding)?;
        let ty = match kind {
            BorrowKind::Shared => Type::ByteSlice,
            BorrowKind::Mutable => Type::ByteSliceMut,
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
