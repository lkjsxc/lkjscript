//! Equality lowers through the selected builtin trait implementation, never a
//! method-name search. Operand materialization is shared by scalar and nominal
//! paths so a failed hint cannot cause a source expression to run twice.
use super::{Builder, Result, a, err, k, place_type};
use crate::generics::{TraitIdentity, implementation_witness};
use std::collections::BTreeMap;

impl Builder<'_, '_> {
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
    ) -> Result<(k::Place, usize)> {
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
            k::Place::local(value.0)
        };
        let ty = place_type(&self.compiler.types, self.function, &place)
            .ok_or_else(|| err(span, "E_TYPE", "invalid equality operand"))?;
        Ok((place, ty))
    }

    fn equality_referent(&self, mut place: k::Place, mut ty: usize) -> (k::Place, usize) {
        while let k::TypeKind::Ref { inner, .. } = self.compiler.types[ty].kind {
            place.projections.push(k::Projection::Deref);
            ty = inner;
        }
        (place, ty)
    }

    fn equality_method(
        &mut self,
        method: usize,
        target: usize,
        substitutions: &BTreeMap<String, usize>,
        span: a::Span,
    ) -> Result<usize> {
        let source = self.compiler.source.functions[method].clone();
        let reference = self.compiler.reference(target, false);
        if source.params.len() != 2
            || source.params[0].name != "self"
            || source.generics.iter().any(|parameter| !parameter.lifetime)
        {
            return Err(err(
                span,
                "E_TRAIT",
                "PartialEq method requires (&self, &Self) -> bool without type parameters",
            ));
        }
        for parameter in &source.params {
            if self.compiler.resolve(parameter.ty, substitutions)? != reference {
                return Err(err(
                    span,
                    "E_TRAIT",
                    "PartialEq method parameters must be shared references to the exact Self type",
                ));
            }
        }
        if self.compiler.resolve(source.result, substitutions)? != k::BOOL {
            return Err(err(span, "E_TRAIT", "PartialEq method result must be bool"));
        }
        if source.body.is_none() {
            return Err(err(
                span,
                "E_UNSUPPORTED",
                "PartialEq method has no executable body",
            ));
        }
        self.compiler.instantiate(method, substitutions.clone())
    }

    fn equality_implementation(
        &mut self,
        target: usize,
        operation: &str,
        span: a::Span,
    ) -> Result<(usize, bool)> {
        let identity = TraitIdentity::Builtin("PartialEq".to_string());
        let empty = BTreeMap::new();
        let assumptions = self
            .compiler
            .contracts
            .as_ref()
            .map_or(&empty, |c| &c.rigid);
        let selected = implementation_witness(
            self.compiler.source,
            &self.compiler.types,
            &self.compiler.applications,
            target,
            &identity,
            assumptions,
        );
        let Some(selected) = selected else {
            return Err(if self.compiler.satisfies_identity(target, &identity) {
                err(
                    span,
                    "E_UNSUPPORTED",
                    "PartialEq has no executable implementation; derived comparisons are not implemented yet",
                )
            } else {
                err(
                    span,
                    "E_TRAIT",
                    "equality requires the builtin PartialEq trait",
                )
            });
        };
        let implementation = self.compiler.source.impls[selected.implementation].clone();
        let mut substitutions = selected.substitutions;
        substitutions.insert("Self".to_string(), target);
        self.compiler
            .bounds(&implementation.generics, &substitutions)?;
        let mut equal = None;
        let mut unequal = None;
        for method in implementation.methods {
            let slot = match self.compiler.source.functions[method].name.as_str() {
                "eq" => &mut equal,
                "ne" => &mut unequal,
                _ => continue,
            };
            if slot.replace(method).is_some() {
                return Err(err(span, "E_TRAIT", "duplicate PartialEq method"));
            }
        }
        let equal =
            equal.ok_or_else(|| err(span, "E_TRAIT", "PartialEq implementation requires eq"))?;
        let equal = self.equality_method(equal, target, &substitutions, span)?;
        let unequal = match unequal {
            Some(method) => Some(self.equality_method(method, target, &substitutions, span)?),
            None => None,
        };
        Ok(if operation == "!=" {
            unequal.map_or((equal, true), |method| (method, false))
        } else {
            (equal, false)
        })
    }

    fn equality_call(
        &mut self,
        function: usize,
        negate: bool,
        left: usize,
        right: usize,
        span: a::Span,
    ) -> (usize, usize) {
        let dst = self.temp(k::BOOL);
        self.emit(
            k::Op::Call {
                dst,
                function,
                args: vec![left, right],
            },
            span.line,
        );
        if negate {
            let result = self.temp(k::BOOL);
            self.emit(
                k::Op::Unary {
                    dst: result,
                    op: "!".to_string(),
                    src: dst,
                },
                span.line,
            );
            (result, k::BOOL)
        } else {
            (dst, k::BOOL)
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
            match self.compiler.types[target].kind.clone() {
                k::TypeKind::Bool | k::TypeKind::Int { .. } | k::TypeKind::F64 => {
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
                    let dst = self.temp(k::BOOL);
                    self.emit(
                        k::Op::Binary {
                            dst,
                            op: operation.to_string(),
                            left,
                            right,
                        },
                        span.line,
                    );
                    (dst, k::BOOL)
                }
                k::TypeKind::Struct { .. } | k::TypeKind::Enum { .. } => {
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
                k::TypeKind::Vec { inner } => {
                    let left = self.borrow(left_place, false, span.line)?.0;
                    let (right_place, right_type) =
                        self.equality_operand(right, Some(operand_type), span)?;
                    if right_type != operand_type {
                        return Err(err(span, "E_TYPE", "equality operand types differ"));
                    }
                    let (right_place, _) = self.equality_referent(right_place, right_type);
                    let right = self.borrow(right_place, false, span.line)?.0;
                    self.vector_compare_values(operation, left, right, inner, span)?
                }
                _ => {
                    return Err(err(
                        span,
                        "E_UNSUPPORTED",
                        "equality for this type is not implemented yet",
                    ));
                }
            }
        };
        // Operand reference temporaries do not escape a boolean comparison.
        // End their loans here while retaining owned operand temporaries until
        // their ordinary enclosing statement cleanup (and destructor order).
        for local in (first_temporary..self.live.len()).rev() {
            if self.live[local]
                && matches!(
                    self.compiler.types[self.ty(local)].kind,
                    k::TypeKind::Ref { .. }
                )
            {
                self.emit(
                    k::Op::Drop {
                        place: k::Place::local(local),
                    },
                    span.line,
                );
            }
        }
        Ok(result)
    }
}
