//! Equality lowers through the selected builtin trait implementation, never a
//! method-name search. Operand materialization is shared by scalar and nominal
//! paths so a failed hint cannot cause a source expression to run twice.
use super::generic_check::BuiltinMethod;
use super::{Builder, Result, a, err, ir, place_type};

impl Builder<'_, '_> {
    /// Relations borrow aggregates and read scalar operands exactly once, before
    /// evaluating the right expression. Derived operations are ordinary checked IR.
    pub(super) fn ordering_expr(
        &mut self,
        operation: &str,
        left: usize,
        right: usize,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let first_temporary = self.live.len();
        let expected = self
            .equality_hint(left)
            .or_else(|| self.equality_hint(right));
        let (left_place, operand_type) = self.equality_operand(left, expected, span)?;
        let result = if self.compiler.depends_on_rigid(operand_type) {
            let function = self
                .compiler
                .comparison_contract(operand_type, operation, span)?;
            let left = self.borrow(left_place, false, span.line)?.0;
            let (right_place, right_type) =
                self.equality_operand(right, Some(operand_type), span)?;
            if right_type != operand_type {
                return Err(err(span, "E_TYPE", "comparison operand types differ"));
            }
            let right = self.borrow(right_place, false, span.line)?.0;
            self.equality_call(function, false, left, right, span)
        } else {
            let (left_place, target) = self.equality_referent(left_place, operand_type);
            if matches!(
                self.compiler.types[target].kind,
                ir::TypeKind::Bool
                    | ir::TypeKind::Char
                    | ir::TypeKind::Int { .. }
                    | ir::TypeKind::F64
            ) {
                let left = self.read(left_place, span.line)?.0;
                let (right_place, right_type) =
                    self.equality_operand(right, Some(operand_type), span)?;
                if right_type != operand_type {
                    return Err(err(span, "E_TYPE", "comparison operand types differ"));
                }
                let (right_place, _) = self.equality_referent(right_place, right_type);
                let right = self.read(right_place, span.line)?.0;
                let result = self.temp(ir::BOOL);
                if target == ir::BOOL {
                    let negated = self.temp(ir::BOOL);
                    self.emit(
                        ir::Op::Unary {
                            dst: negated,
                            op: "!".to_string(),
                            src: if operation == "<" || operation == "<=" {
                                left
                            } else {
                                right
                            },
                        },
                        span.line,
                    );
                    self.emit(
                        ir::Op::Binary {
                            dst: result,
                            op: if operation == "<" || operation == ">" {
                                "&"
                            } else {
                                "|"
                            }
                            .to_string(),
                            left: negated,
                            right: if operation == "<" || operation == "<=" {
                                right
                            } else {
                                left
                            },
                        },
                        span.line,
                    );
                } else {
                    self.emit(
                        ir::Op::Binary {
                            dst: result,
                            op: operation.to_string(),
                            left,
                            right,
                        },
                        span.line,
                    );
                }
                (result, ir::BOOL)
            } else {
                let method = match operation {
                    "<" => BuiltinMethod::Lt,
                    "<=" => BuiltinMethod::Le,
                    ">" => BuiltinMethod::Gt,
                    ">=" => BuiltinMethod::Ge,
                    _ => unreachable!(),
                };
                let function = self.compiler.builtin_operation(target, method, span)?;
                let left = self.borrow(left_place, false, span.line)?.0;
                let (right_place, right_type) =
                    self.equality_operand(right, Some(operand_type), span)?;
                if right_type != operand_type {
                    return Err(err(span, "E_TYPE", "comparison operand types differ"));
                }
                let (right_place, _) = self.equality_referent(right_place, right_type);
                let right = self.borrow(right_place, false, span.line)?.0;
                self.equality_call(function, false, left, right, span)
            }
        };
        let mut local = self.live.len();
        while local > first_temporary {
            local -= 1;
            if self.live.at(local)
                && matches!(
                    self.compiler.types[self.ty(local)].kind,
                    ir::TypeKind::Ref { .. }
                )
            {
                self.emit(
                    ir::Op::Drop {
                        place: ir::Place::local(local),
                    },
                    span.line,
                );
            }
        }
        Ok(result)
    }

    fn equality_hint(&mut self, expression: usize) -> Option<usize> {
        // A method named `len` need not be the collection builtin. The actual
        // method result is authoritative once the expression has been checked.
        if matches!(
            self.compiler.source.exprs[expression].kind,
            a::Expr::Method { .. }
        ) {
            None
        } else {
            self.hint(expression)
        }
    }

    fn equality_operand(
        &mut self,
        expression: usize,
        expected: Option<usize>,
        span: a::Span,
    ) -> Result<(ir::Place, usize)> {
        let is_place = match &self.compiler.source.exprs[expression].kind {
            a::Expr::Path(path) => self.binding(&path.name).is_some(),
            a::Expr::Field { .. } | a::Expr::Index { .. } => true,
            a::Expr::Unary { op, .. } => op == "*",
            _ => false,
        };
        let place = if is_place {
            self.place(expression)?
        } else {
            let value = self.expr(expression, expected)?;
            ir::Place::local(value.0)
        };
        let ty = place_type(&self.compiler.types, self.function, &place)
            .ok_or_else(|| err(span, "E_TYPE", "invalid equality operand"))?;
        Ok((place, ty))
    }

    fn equality_referent(&self, mut place: ir::Place, mut ty: usize) -> (ir::Place, usize) {
        while let ir::TypeKind::Ref { inner, .. } = self.compiler.types[ty].kind {
            place.projections.push(ir::Projection::Deref);
            ty = inner;
        }
        (place, ty)
    }

    fn equality_implementation(
        &mut self,
        target: usize,
        operation: &str,
        span: a::Span,
    ) -> Result<(usize, bool)> {
        let method = if operation == "==" {
            BuiltinMethod::Eq
        } else {
            BuiltinMethod::Ne
        };
        Ok((
            self.compiler.builtin_operation(target, method, span)?,
            false,
        ))
    }

    fn equality_call(
        &mut self,
        function: usize,
        negate: bool,
        left: usize,
        right: usize,
        span: a::Span,
    ) -> (usize, usize) {
        let dst = self.temp(ir::BOOL);
        self.emit(
            ir::Op::Call {
                dst,
                function,
                args: vec![left, right],
            },
            span.line,
        );
        if negate {
            let result = self.temp(ir::BOOL);
            self.emit(
                ir::Op::Unary {
                    dst: result,
                    op: "!".to_string(),
                    src: dst,
                },
                span.line,
            );
            (result, ir::BOOL)
        } else {
            (dst, ir::BOOL)
        }
    }

    pub(super) fn equality_expr(
        &mut self,
        operation: &str,
        left: usize,
        right: usize,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let first_temporary = self.live.len();
        let expected = self
            .equality_hint(left)
            .or_else(|| self.equality_hint(right));
        let (left_place, operand_type) = self.equality_operand(left, expected, span)?;
        let result = if self.compiler.depends_on_rigid(operand_type) {
            // This also handles a rigid result of a call whose type was not
            // available to the definition checker's initial hint-based path.
            let function = self
                .compiler
                .comparison_contract(operand_type, operation, span)?;
            let left = self.borrow(left_place, false, span.line)?.0;
            let (right_place, right_type) =
                self.equality_operand(right, Some(operand_type), span)?;
            if right_type != operand_type {
                return Err(err(span, "E_TYPE", "equality operand types differ"));
            }
            let right = self.borrow(right_place, false, span.line)?.0;
            self.equality_call(function, false, left, right, span)
        } else {
            let (left_place, target) = self.equality_referent(left_place, operand_type);
            if matches!(self.compiler.types[target].kind, ir::TypeKind::Str)
                || self.library_string(target, span)
            {
                let left_string = self.library_string(target, span);
                let mut left = self.borrow(left_place, false, span.line)?.0;
                let (right_place, right_type) = self.equality_operand(right, None, span)?;
                let (right_place, right_target) = self.equality_referent(right_place, right_type);
                let right_string = self.library_string(right_target, span);
                let depth = |mut ty: usize| {
                    let mut count = 0;
                    while let ir::TypeKind::Ref { inner, .. } = self.compiler.types[ty].kind {
                        ty = inner;
                        count += 1;
                    }
                    count
                };
                let left_depth = depth(operand_type);
                let right_depth = depth(right_type);
                let compatible = if left_string == right_string {
                    operand_type == right_type
                } else if left_string {
                    matches!(self.compiler.types[right_target].kind, ir::TypeKind::Str)
                        && (right_depth == left_depth || right_depth == left_depth + 1)
                } else {
                    right_string && (left_depth == right_depth || left_depth == right_depth + 1)
                };
                if !compatible {
                    return Err(err(
                        span,
                        "E_TYPE",
                        &format!(
                            "equality operand types differ: {} and {}",
                            self.compiler.display_type(operand_type),
                            self.compiler.display_type(right_type)
                        ),
                    ));
                }
                let mut right = self.borrow(right_place, false, span.line)?.0;
                if left_string && right_string {
                    let (function, negate) =
                        self.equality_implementation(target, operation, span)?;
                    self.equality_call(function, negate, left, right, span)
                } else {
                    if left_string {
                        left = self
                            .source_library_call(
                                "string",
                                "__lkjscript_as_str",
                                std::collections::BTreeMap::new(),
                                vec![left],
                                span,
                            )?
                            .0;
                    }
                    if right_string {
                        right = self
                            .source_library_call(
                                "string",
                                "__lkjscript_as_str",
                                std::collections::BTreeMap::new(),
                                vec![right],
                                span,
                            )?
                            .0;
                    }
                    self.text_equality_values(operation, left, right, span)?
                }
            } else {
                match self.compiler.types[target].kind.clone() {
                    ir::TypeKind::Bool
                    | ir::TypeKind::Char
                    | ir::TypeKind::Int { .. }
                    | ir::TypeKind::F64 => {
                        // Read scalars before evaluating the right expression. A
                        // later assignment must not change the left operand value.
                        let left = self.read(left_place, span.line)?.0;
                        let (right_place, right_type) =
                            self.equality_operand(right, Some(operand_type), span)?;
                        if right_type != operand_type {
                            return Err(err(span, "E_TYPE", "equality operand types differ"));
                        }
                        let (right_place, _) = self.equality_referent(right_place, right_type);
                        let right = self.read(right_place, span.line)?.0;
                        let dst = self.temp(ir::BOOL);
                        self.emit(
                            ir::Op::Binary {
                                dst,
                                op: operation.to_string(),
                                left,
                                right,
                            },
                            span.line,
                        );
                        (dst, ir::BOOL)
                    }
                    ir::TypeKind::Str => {
                        let left = self.borrow(left_place, false, span.line)?.0;
                        let (right_place, right_type) =
                            self.equality_operand(right, Some(operand_type), span)?;
                        if right_type != operand_type {
                            return Err(err(span, "E_TYPE", "equality operand types differ"));
                        }
                        let (right_place, _) = self.equality_referent(right_place, right_type);
                        let right = self.borrow(right_place, false, span.line)?.0;
                        self.text_equality_values(operation, left, right, span)?
                    }
                    ir::TypeKind::Unit
                    | ir::TypeKind::Struct { .. }
                    | ir::TypeKind::Enum { .. }
                    | ir::TypeKind::Box { .. }
                    | ir::TypeKind::Array { .. } => {
                        let (function, negate) =
                            self.equality_implementation(target, operation, span)?;
                        let left = self.borrow(left_place, false, span.line)?.0;
                        let (right_place, right_type) =
                            self.equality_operand(right, Some(operand_type), span)?;
                        if right_type != operand_type {
                            return Err(err(span, "E_TYPE", "equality operand types differ"));
                        }
                        let (right_place, _) = self.equality_referent(right_place, right_type);
                        let right = self.borrow(right_place, false, span.line)?.0;
                        self.equality_call(function, negate, left, right, span)
                    }
                    ir::TypeKind::Slice { inner, .. } => {
                        let function = self.compiler.sequence_instance(
                            "__lkjscript_slice_equal",
                            inner,
                            span,
                        )?;
                        let left = self.borrow(left_place, false, span.line)?.0;
                        let (right_place, right_type) =
                            self.equality_operand(right, Some(operand_type), span)?;
                        if right_type != operand_type {
                            return Err(err(span, "E_TYPE", "equality operand types differ"));
                        }
                        let (right_place, _) = self.equality_referent(right_place, right_type);
                        let right = self.borrow(right_place, false, span.line)?.0;
                        self.equality_call(function, operation == "!=", left, right, span)
                    }
                    ir::TypeKind::Vec { inner } => {
                        let left = self.borrow(left_place, false, span.line)?.0;
                        let (right_place, right_type) =
                            self.equality_operand(right, Some(operand_type), span)?;
                        if right_type != operand_type {
                            return Err(err(span, "E_TYPE", "equality operand types differ"));
                        }
                        let (right_place, _) = self.equality_referent(right_place, right_type);
                        let right = self.borrow(right_place, false, span.line)?.0;
                        self.vector_equality_values(operation, left, right, inner, span)?
                    }
                    _ => {
                        return Err(err(
                            span,
                            "E_UNSUPPORTED",
                            "equality for this type is not implemented yet",
                        ));
                    }
                }
            }
        };
        // Operand reference temporaries do not escape a boolean comparison.
        // End their loans here while retaining owned operand temporaries until
        // their ordinary enclosing statement cleanup (and destructor order).
        let mut local = self.live.len();
        while local > first_temporary {
            local -= 1;
            if self.live.at(local)
                && matches!(
                    self.compiler.types[self.ty(local)].kind,
                    ir::TypeKind::Ref { .. }
                )
            {
                self.emit(
                    ir::Op::Drop {
                        place: ir::Place::local(local),
                    },
                    span.line,
                );
            }
        }
        Ok(result)
    }
}
