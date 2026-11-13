//! Formatting expands to source String/Formatter calls. K still sees ordinary
//! references, calls, aggregates and control flow; formatting owns no authority.
use super::{Builder, Result, a, err, k, place_type};
use crate::syntax::formatting::{self, Part, Specification};
use std::collections::BTreeMap;
impl Builder<'_, '_> {
    pub(super) fn text_equality_values(
        &mut self,
        operation: &str,
        left: usize,
        right: usize,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let equal =
            self.source_library_call("str", "__lkj_eq", BTreeMap::new(), vec![left, right], span)?;
        if operation == "==" {
            Ok(equal)
        } else {
            let dst = self.temp(k::BOOL);
            self.emit(
                k::Op::Unary {
                    dst,
                    op: "!".to_string(),
                    src: equal.0,
                },
                span.line,
            );
            Ok((dst, k::BOOL))
        }
    }

    pub(super) fn source_library_call(
        &mut self,
        module: &str,
        name: &str,
        substitutions: BTreeMap<String, usize>,
        arguments: Vec<usize>,
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let source = self.compiler.names.library_function(module, name, span)?;
        let function = self.compiler.instantiate(source, substitutions)?;
        let signature = &self.compiler.functions[function];
        if signature.params != arguments.len()
            || signature
                .locals
                .iter()
                .zip(&arguments)
                .any(|(parameter, value)| parameter.ty != self.ty(*value))
        {
            return Err(err(
                span,
                "E_TYPE",
                "source library operation has incompatible arguments",
            ));
        }
        let ty = signature.result;
        let dst = self.temp(ty);
        self.emit(
            k::Op::Call {
                dst,
                function,
                args: arguments,
            },
            span.line,
        );
        Ok((dst, ty))
    }
    pub(super) fn source_text_method(
        &mut self,
        target: k::Place,
        name: &str,
        args: &[usize],
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let pattern = matches!(
            name,
            "starts_with" | "ends_with" | "contains" | "find" | "rfind" | "replace"
        );
        let count = if name == "replace" {
            2
        } else if pattern || name == "repeat" || name == "eq_ignore_ascii_case" {
            1
        } else {
            0
        };
        if args.len() != count {
            return Err(err(span, "E_TYPE", "text method argument count differs"));
        }
        let receiver = self.borrow(target, false, span.line)?.0;
        let mut values = vec![receiver];
        let mut substitutions = BTreeMap::new();
        for (index, arg) in args.iter().enumerate() {
            let expected = if name == "repeat" {
                Some(k::U64)
            } else if name == "eq_ignore_ascii_case" || (name == "replace" && index == 1) {
                Some(self.ty(receiver))
            } else {
                None
            };
            let value = self.expr(*arg, expected)?;
            if pattern && index == 0 {
                substitutions.insert("P".to_string(), value.1);
            }
            values.push(value.0);
        }
        self.source_library_call("str", &format!("__lkj_{name}"), substitutions, values, span)
    }
    fn format_constant(&mut self, ty: usize, value: i64, line: usize) -> usize {
        let dst = self.temp(ty);
        self.emit(k::Op::Int { dst, value }, line);
        dst
    }
    fn format_text(&mut self, text: String, line: usize) -> usize {
        let inner = self.compiler.intern(String::new(), k::TypeKind::Str);
        let ty = self.compiler.reference(inner, false);
        let dst = self.temp(ty);
        self.emit(
            k::Op::Text {
                dst,
                bytes: text.into_bytes(),
            },
            line,
        );
        dst
    }
    fn formatter_type(&mut self, span: a::Span) -> Result<usize> {
        let path = self.compiler.names.library_type("fmt", "Formatter", span)?;
        self.compiler.named_type(&path, &BTreeMap::new(), span)
    }
    fn format_argument(&mut self, expression: a::ExprId, span: a::Span) -> Result<(usize, usize)> {
        let mut place = self.addressable(expression)?;
        let mut ty = place_type(&self.compiler.types, self.function, &place)
            .ok_or_else(|| err(span, "E_TYPE", "format argument is not addressable"))?;
        while let k::TypeKind::Ref { inner, .. } | k::TypeKind::Box { inner } =
            self.compiler.types[ty].kind
        {
            place.projections.push(k::Projection::Deref);
            ty = inner;
        }
        let reference = self.borrow(place, false, span.line)?.0;
        Ok((reference, ty))
    }
    fn format_settings(
        &mut self,
        formatter: usize,
        specification: &Specification,
        span: a::Span,
    ) -> Result<()> {
        let receiver = self.borrow(k::Place::local(formatter), true, span.line)?.0;
        let width = self.format_constant(k::U64, specification.width as i64, span.line);
        let flags = self.format_constant(k::U8, i64::from(specification.flags), span.line);
        let fill = self.format_constant(
            self.compiler.type_names["char"],
            specification.fill as i64,
            span.line,
        );
        let unit = self
            .source_library_call(
                "fmt",
                "__lkj_settings",
                BTreeMap::new(),
                vec![receiver, width, flags, fill],
                span,
            )?
            .0;
        self.cleanup_place(k::Place::local(unit), span.line);
        Ok(())
    }
    pub(super) fn format_macro(
        &mut self,
        name: &str,
        args: &[usize],
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let writing = name != "format";
        let format_index = usize::from(writing);
        let Some(&literal) = args.get(format_index) else {
            return Err(err(span, "E_FORMAT", "format macro requires a literal"));
        };
        let a::Expr::String(text) = &self.compiler.source.exprs[literal].kind else {
            return Err(err(span, "E_FORMAT", "format argument must be a literal"));
        };
        let parts = formatting::parse(text, span)?;
        let target = if writing {
            Some(self.addressable(args[0])?)
        } else {
            None
        };
        let mut arguments = Vec::new();
        for argument in &args[format_index + 1..] {
            arguments.push(self.format_argument(*argument, span)?);
        }
        let mut output = None;
        let formatter = if let Some(mut target) = target {
            let mut ty = place_type(&self.compiler.types, self.function, &target)
                .ok_or_else(|| err(span, "E_TYPE", "format output is not addressable"))?;
            while let k::TypeKind::Ref { inner, .. } = self.compiler.types[ty].kind {
                target.projections.push(k::Projection::Deref);
                ty = inner;
            }
            let formatter_type = self.formatter_type(span)?;
            if ty != formatter_type && !self.has_declared_method(ty, "write_str", span)? {
                return Err(err(
                    span,
                    "E_TRAIT",
                    "write! requires the source Write trait in scope",
                ));
            }
            let receiver = self.borrow(target, true, span.line)?.0;
            self.source_library_call(
                "fmt",
                if ty == formatter_type {
                    "__lkj_nested"
                } else {
                    "__lkj_new"
                },
                BTreeMap::new(),
                vec![receiver],
                span,
            )?
            .0
        } else {
            let string = self
                .source_library_call("string", "__lkj_new", BTreeMap::new(), Vec::new(), span)?
                .0;
            self.function.locals[string].mutable = true;
            output = Some(string);
            let receiver = self.borrow(k::Place::local(string), true, span.line)?.0;
            self.source_library_call("fmt", "__lkj_new", BTreeMap::new(), vec![receiver], span)?
                .0
        };
        self.function.locals[formatter].mutable = true;
        let (initial, result_type) =
            self.source_library_call("fmt", "__lkj_success", BTreeMap::new(), Vec::new(), span)?;
        let result = self.local(result_type, "$format_result", true);
        self.emit(
            k::Op::Assign {
                place: k::Place::local(result),
                src: initial,
            },
            span.line,
        );
        for part in parts {
            let first = self.live.len();
            let success = self.temp(k::BOOL);
            self.emit(
                k::Op::TagTest {
                    dst: success,
                    place: k::Place::local(result),
                    variant: 0,
                },
                span.line,
            );
            self.emit(k::Op::If { condition: success }, span.line);
            let before = self.snapshot();
            let produced = match part {
                Part::Text(text) => {
                    let receiver = self.borrow(k::Place::local(formatter), true, span.line)?.0;
                    let text = self.format_text(text, span.line);
                    self.source_library_call(
                        "fmt",
                        "__lkj_literal",
                        BTreeMap::new(),
                        vec![receiver, text],
                        span,
                    )?
                }
                Part::Value {
                    argument,
                    specification,
                    ..
                } => {
                    let index = argument
                        .parse::<usize>()
                        .map_err(|_| err(span, "E_FORMAT", "format argument was not normalized"))?;
                    let Some(&(reference, ty)) = arguments.get(index) else {
                        return Err(err(span, "E_FORMAT", "format argument is out of bounds"));
                    };
                    self.format_settings(formatter, &specification, span)?;
                    let receiver = self.borrow(k::Place::local(formatter), true, span.line)?.0;
                    let value = self.read(k::Place::local(reference), span.line)?.0;
                    let mut substitutions = BTreeMap::new();
                    substitutions.insert("T".to_string(), ty);
                    let helper = if matches!(self.compiler.types[ty].kind, k::TypeKind::Str)
                        && specification.method == "display"
                    {
                        "__lkj_display_str".to_string()
                    } else {
                        format!("__lkj_{}", specification.method)
                    };
                    self.source_library_call(
                        "fmt",
                        &helper,
                        substitutions,
                        vec![receiver, value],
                        span,
                    )?
                }
            };
            if produced.1 != result_type {
                return Err(err(span, "E_TYPE", "format helper result differs"));
            }
            self.cleanup_place(k::Place::local(result), span.line);
            self.emit(
                k::Op::Assign {
                    place: k::Place::local(result),
                    src: produced.0,
                },
                span.line,
            );
            self.cleanup(first, None, span.line);
            let after = self.snapshot();
            self.emit(k::Op::Else, span.line);
            self.restore(&before);
            self.emit(k::Op::End, span.line);
            self.merge_carriers(&after);
        }
        self.cleanup_place(k::Place::local(formatter), span.line);
        for (reference, _) in arguments {
            self.cleanup_place(k::Place::local(reference), span.line);
        }
        if let Some(output) = output {
            let success = self.temp(k::BOOL);
            self.emit(
                k::Op::TagTest {
                    dst: success,
                    place: k::Place::local(result),
                    variant: 0,
                },
                span.line,
            );
            self.emit(k::Op::If { condition: success }, span.line);
            let valid = self.snapshot();
            self.emit(k::Op::Else, span.line);
            self.emit(
                k::Op::Trap {
                    message: "formatting trait returned an error".to_string(),
                },
                span.line,
            );
            self.emit(k::Op::End, span.line);
            self.restore(&valid);
            self.cleanup_place(k::Place::local(result), span.line);
            Ok((output, self.ty(output)))
        } else {
            Ok((result, result_type))
        }
    }
}
