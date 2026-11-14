//! Surface control forms lower to the ordinary checked K operations.
use super::{Builder, Result, a, err, k};

impl Builder<'_, '_> {
    pub(super) fn try_expr(&mut self, expression: usize, span: a::Span) -> Result<(usize, usize)> {
        let (source, ty) = self.expr(expression, None)?;
        let declaration = self.nominal_declaration(ty, span)?;
        let name = &self.compiler.source.declarations[declaration].name;
        let (success_name, failure_name) = match name.as_str() {
            "$std::prelude::Result" => ("Ok", "Err"),
            "$std::prelude::Option" => ("Some", "None"),
            _ => {
                return Err(err(
                    span,
                    "E_TYPE",
                    "? requires the source Result or Option type",
                ));
            }
        };
        let result_ty = self.function.result;
        if self.nominal_declaration(result_ty, span)? != declaration {
            return Err(err(
                span,
                "E_TYPE",
                "? and the function result require the same Result or Option family",
            ));
        }
        let k::TypeKind::Enum { variants, .. } = self.compiler.types[ty].kind.clone() else {
            unreachable!()
        };
        let k::TypeKind::Enum {
            variants: outputs, ..
        } = self.compiler.types[result_ty].kind.clone()
        else {
            unreachable!()
        };
        let success = variants
            .iter()
            .position(|variant: &k::Variant| variant.name == success_name)
            .unwrap();
        let failure = variants
            .iter()
            .position(|variant: &k::Variant| variant.name == failure_name)
            .unwrap();
        let output_failure = outputs
            .iter()
            .position(|variant: &k::Variant| variant.name == failure_name)
            .unwrap();
        let input_fields = &variants[failure].fields;
        let output_fields = &outputs[output_failure].fields;
        let convert_text = input_fields.len() == 1
            && output_fields.len() == 1
            && self.library_string(output_fields[0].ty, span)
            && matches!(
                self.compiler.types[input_fields[0].ty].kind,
                k::TypeKind::Ref { inner, mutable: false }
                    if matches!(self.compiler.types[inner].kind, k::TypeKind::Str)
            );
        let mut fields_differ = input_fields.len() != output_fields.len();
        let mut field = 0;
        while !fields_differ && field < input_fields.len() {
            fields_differ = input_fields[field].ty != output_fields[field].ty;
            field += 1;
        }
        if !convert_text && fields_differ {
            return Err(err(
                span,
                "E_TYPE",
                "? supports equal error types or &str into String; help: use map_err for another conversion",
            ));
        }
        let payload = variants[success].fields[0].ty;
        let result = self.local(payload, "$try", true);
        let condition = self.temp(k::BOOL);
        self.emit(
            k::Op::TagTest {
                dst: condition,
                place: k::Place::local(source),
                variant: success,
            },
            span.line,
        );
        self.emit(k::Op::If { condition }, span.line);
        let before = self.snapshot();
        let value = self
            .read(
                k::Place {
                    local: source,
                    projections: vec![k::Projection::VariantField(success, 0)],
                },
                span.line,
            )?
            .0;
        self.emit(
            k::Op::Assign {
                place: k::Place::local(result),
                src: value,
            },
            span.line,
        );
        self.emit(
            k::Op::DropIf {
                place: k::Place::local(source),
            },
            span.line,
        );
        let succeeded = self.snapshot();
        self.emit(k::Op::Else, span.line);
        self.restore(&before);
        let condition = self.temp(k::BOOL);
        self.emit(
            k::Op::TagTest {
                dst: condition,
                place: k::Place::local(source),
                variant: failure,
            },
            span.line,
        );
        self.emit(k::Op::If { condition }, span.line);
        let mut fields = Vec::new();
        for index in 0..variants[failure].fields.len() {
            let value = self
                .read(
                    k::Place {
                        local: source,
                        projections: vec![k::Projection::VariantField(failure, index)],
                    },
                    span.line,
                )?
                .0;
            let value = if convert_text {
                // This is an ordinary source-library allocation/copy, with no
                // implicit coercion for Result constructors or assignments.
                let arguments: Vec<usize> = Vec::new();
                self.text_method(
                    k::Place {
                        local: value,
                        projections: vec![k::Projection::Deref],
                    },
                    "to_string",
                    &arguments,
                    span,
                )?
                .0
            } else {
                value
            };
            fields.push(value);
        }
        let output = self.temp(result_ty);
        self.emit(
            k::Op::Variant {
                dst: output,
                variant: output_failure,
                fields,
            },
            span.line,
        );
        self.cleanup(0, Some(output), span.line);
        self.emit(k::Op::Return { src: output }, span.line);
        self.emit(k::Op::Else, span.line);
        self.emit(
            k::Op::Trap {
                message: "unreachable ? variant".to_string(),
            },
            span.line,
        );
        self.emit(k::Op::End, span.line);
        self.emit(k::Op::End, span.line);
        self.restore(&succeeded);
        Ok((result, payload))
    }
}

impl Builder<'_, '_> {
    pub(super) fn pattern_initializer(
        &mut self,
        pattern: usize,
        expression: usize,
        expected: Option<usize>,
        annotation: Option<a::TypeId>,
        line: usize,
    ) -> Result<()> {
        if matches!(
            self.compiler.source.patterns[pattern].kind,
            a::Pattern::Bind { by_ref: false, .. }
        ) || (matches!(
            self.compiler.source.patterns[pattern].kind,
            a::Pattern::Wild
        ) && expected
            .is_some_and(|ty| matches!(self.compiler.types[ty].kind, k::TypeKind::Ref { .. })))
        {
            let value = self.expr(expression, expected)?;
            self.annotation_initializer(annotation, k::Place::local(value.0))?;
            let result = self.bind(pattern, value, line);
            self.typed_bindings.clear();
            return result;
        }
        let existing = match &self.compiler.source.exprs[expression].kind {
            a::Expr::Path(path) => {
                self.binding(&path.name).is_some() || self.guard_aliases.contains_key(&path.name)
            }
            a::Expr::Field { .. } | a::Expr::Index { .. } => true,
            a::Expr::Unary { op, .. } => op == "*",
            _ => false,
        };
        let place = if existing {
            self.place(expression)?
        } else {
            k::Place::local(self.expr(expression, expected)?.0)
        };
        let ty =
            super::place_type(&self.compiler.types, self.function, &place).ok_or_else(|| {
                err(
                    self.compiler.source.patterns[pattern].span,
                    "E_TYPE",
                    "invalid binding place",
                )
            })?;
        if expected.is_some_and(|expected| expected != ty) {
            return Err(err(
                self.compiler.source.patterns[pattern].span,
                "E_TYPE",
                "binding type differs",
            ));
        }
        let mut plan = super::patterns::PatternPlan::default();
        let root = self.prepare_pattern(&mut plan, pattern, ty, None)?;
        if !self.patterns_exhaustive(&plan, &[root], ty)? {
            return Err(err(
                self.compiler.source.patterns[pattern].span,
                "E_TYPE",
                "let binding requires an irrefutable pattern",
            ));
        }
        self.annotation_initializer(annotation, place.clone())?;
        let result = self.bind_pattern_plan(&plan, root, place, line);
        self.typed_bindings.clear();
        result
    }
}
