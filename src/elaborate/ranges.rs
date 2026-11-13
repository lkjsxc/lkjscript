//! Range syntax borrows a checked subview. The source and bounds are evaluated
//! once; K owns access, bounds, UTF-8 and result-origin validation.
use super::{Builder, Result, a, err, k, place_type};
use std::collections::BTreeMap;

impl Builder<'_, '_> {
    pub(super) fn is_range(&self, index: usize) -> bool {
        matches!(
            self.compiler.source.exprs[index].kind,
            a::Expr::Range { .. }
        )
    }

    pub(super) fn library_string(&self, ty: usize, span: a::Span) -> bool {
        let Some((name, _)) = self.compiler.applications.get(&ty) else {
            return false;
        };
        self.compiler
            .names
            .library_type("string", "String", span)
            .ok()
            .and_then(|path| path.declaration)
            .is_some_and(|declaration| {
                self.compiler.declaration_names.get(name) == Some(&declaration)
            })
    }

    pub(super) fn range_referent(&mut self, ty: usize, span: a::Span) -> Result<usize> {
        match self.compiler.types[ty].kind {
            k::TypeKind::Vec { inner }
            | k::TypeKind::Array { inner, .. }
            | k::TypeKind::Slice { inner, .. } => Ok(self.compiler.intern(
                String::new(),
                k::TypeKind::Slice {
                    inner,
                    mutable: false,
                },
            )),
            k::TypeKind::Str => Ok(ty),
            _ if self.library_string(ty, span) => {
                Ok(self.compiler.intern(String::new(), k::TypeKind::Str))
            }
            _ => Err(err(
                span,
                "E_TYPE",
                "range source needs Vec, array, slice or str",
            )),
        }
    }

    // A range is initially a place, and method selection may later require an
    // exclusive receiver. Only these unnamed, compiler-created borrows may be
    // promoted; a user reference or an ordinary reference value never is.
    pub(super) fn promote_range_place(&mut self, place: &k::Place, span: a::Span) -> Result<()> {
        let mut pending = vec![place.local];
        while let Some(local) = pending.pop() {
            let Some(instruction) = self.range_borrows.get(&local).copied() else {
                continue;
            };
            let source = match &self.function.body[instruction].op {
                k::Op::Borrow {
                    place,
                    mutable: false,
                    ..
                }
                | k::Op::ViewRange {
                    place,
                    mutable: false,
                    ..
                } => place.clone(),
                _ => continue,
            };
            let k::TypeKind::Ref { inner, .. } = self.compiler.types[self.ty(local)].kind else {
                unreachable!("range temporary is a reference")
            };
            if matches!(self.compiler.types[inner].kind, k::TypeKind::Str) {
                return Err(err(
                    span,
                    "E_UNSUPPORTED",
                    "mutable text views are not implemented",
                ));
            }
            self.guard_write(&source, span)?;
            self.function.locals[local].ty = self.compiler.reference(inner, true);
            match &mut self.function.body[instruction].op {
                k::Op::Borrow { mutable, .. } | k::Op::ViewRange { mutable, .. } => *mutable = true,
                _ => unreachable!(),
            }
            pending.push(source.local);
        }
        Ok(())
    }

    pub(super) fn range_place(
        &mut self,
        value: usize,
        index: usize,
        mutable: bool,
        span: a::Span,
    ) -> Result<k::Place> {
        let a::Expr::Range {
            start,
            end,
            inclusive,
        } = self.compiler.source.exprs[index].kind.clone()
        else {
            unreachable!("range index")
        };
        let mut source = self.addressable_with_mutability(value, mutable)?;
        let mut base = place_type(&self.compiler.types, self.function, &source)
            .ok_or_else(|| err(span, "E_TYPE", "invalid range base"))?;
        while let k::TypeKind::Ref { inner, .. } | k::TypeKind::Box { inner } =
            self.compiler.types[base].kind
        {
            source.projections.push(k::Projection::Deref);
            base = inner;
        }
        let referent = self.range_referent(base, span)?;
        if mutable && matches!(self.compiler.types[referent].kind, k::TypeKind::Str) {
            return Err(err(
                span,
                "E_UNSUPPORTED",
                "mutable text views are not implemented",
            ));
        }
        if self.library_string(base, span) {
            let input = self.borrow(source, false, span.line)?.0;
            let function = self
                .compiler
                .names
                .library_function("string", "__lkj_as_str", span)?;
            let function = self.compiler.instantiate(function, BTreeMap::new())?;
            let result = self.compiler.functions[function].result;
            let output = self.temp(result);
            self.emit(
                k::Op::Call {
                    dst: output,
                    function,
                    args: vec![input],
                },
                span.line,
            );
            source = k::Place {
                local: output,
                projections: vec![k::Projection::Deref],
            };
        }
        // Capture the source borrow before evaluating either bound. A bound
        // expression cannot replace the owner beneath the selected receiver.
        let input = self.borrow(source, mutable, span.line)?.0;
        self.range_borrows
            .insert(input, self.function.body.len() - 1);
        let source = k::Place {
            local: input,
            projections: vec![k::Projection::Deref],
        };
        let start = match start {
            Some(start) => self.expr(start, Some(k::U64))?.0,
            None => self.integer("0", Some(k::U64), false, span)?.0,
        };
        let end = match end {
            Some(end) => self.expr(end, Some(k::U64))?.0,
            None => {
                let receiver = self.borrow(source.clone(), false, span.line)?.0;
                let dst = self.temp(k::U64);
                let name = if matches!(self.compiler.types[referent].kind, k::TypeKind::Str) {
                    "str_len"
                } else {
                    "vec_len"
                };
                self.emit(
                    k::Op::Intrinsic {
                        dst,
                        name: name.to_string(),
                        args: vec![receiver],
                    },
                    span.line,
                );
                dst
            }
        };
        let ty = self.compiler.reference(referent, mutable);
        let dst = self.temp(ty);
        let instruction = self.function.body.len();
        self.emit(
            k::Op::ViewRange {
                dst,
                place: source,
                start,
                end,
                inclusive,
                mutable,
            },
            span.line,
        );
        self.range_borrows.insert(dst, instruction);
        Ok(k::Place {
            local: dst,
            projections: vec![k::Projection::Deref],
        })
    }
}
