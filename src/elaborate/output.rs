//! Capability methods lower to checked IR primitives or ordinary source helpers.
use super::{Builder, Result, a, capabilities, err, ir, methods};
use std::collections::BTreeMap;

impl Builder<'_, '_> {
    fn capability_type(&mut self, value: capabilities::Value, span: a::Span) -> Result<usize> {
        use capabilities::Value;
        Ok(match value {
            Value::Unit => ir::UNIT,
            Value::Byte => ir::U8,
            Value::Count => ir::U64,
            Value::Error => ir::I64,
            Value::Boolean => ir::BOOL,
            Value::Stdout => self.compiler.intern(String::new(), ir::TypeKind::Stdout),
            Value::Stdin => self.compiler.intern(String::new(), ir::TypeKind::Stdin),
            Value::Stderr => self.compiler.intern(String::new(), ir::TypeKind::Stderr),
            Value::Directory => self.compiler.intern(String::new(), ir::TypeKind::Directory),
            Value::File => self.compiler.intern(String::new(), ir::TypeKind::File),
            Value::Text => {
                let text = self.compiler.intern(String::new(), ir::TypeKind::Str);
                self.compiler.reference(text, false)
            }
            Value::Bytes | Value::MutableBytes => {
                let bytes = self.compiler.intern(
                    String::new(),
                    ir::TypeKind::Slice {
                        inner: ir::U8,
                        mutable: false,
                    },
                );
                self.compiler
                    .reference(bytes, matches!(value, Value::MutableBytes))
            }
            Value::Arguments => {
                let bytes = self
                    .compiler
                    .intern(String::new(), ir::TypeKind::Vec { inner: ir::U8 });
                self.compiler
                    .intern(String::new(), ir::TypeKind::Vec { inner: bytes })
            }
            Value::ResultDirectory
            | Value::ResultFile
            | Value::ResultCount
            | Value::ResultBytes
            | Value::ResultUnit => {
                let success = match value {
                    Value::ResultDirectory => self.capability_type(Value::Directory, span)?,
                    Value::ResultFile => self.capability_type(Value::File, span)?,
                    Value::ResultCount => ir::U64,
                    Value::ResultBytes => self
                        .compiler
                        .intern(String::new(), ir::TypeKind::Vec { inner: ir::U8 }),
                    _ => ir::UNIT,
                };
                let path = self
                    .compiler
                    .names
                    .library_type("prelude", "Result", span)?;
                let mut arguments = BTreeMap::new();
                arguments.insert("T".to_string(), success);
                arguments.insert("E".to_string(), ir::I64);
                self.compiler.named_type(&path, &arguments, span)?
            }
        })
    }
    pub(super) fn output_method(
        &mut self,
        target: ir::Place,
        base: usize,
        name: &str,
        args: &[usize],
        span: a::Span,
    ) -> Result<(usize, usize)> {
        let method = capabilities::method(methods::receiver(&self.compiler.types[base].kind), name)
            .ok_or_else(|| {
                err(
                    span,
                    "E_TYPE",
                    &format!(
                        "capability {} has no method {name}",
                        self.compiler.display_type(base)
                    ),
                )
            })?;
        let (parameters, result) = method.signature();
        if args.len() != parameters.len() {
            return Err(err(
                span,
                "E_TYPE",
                &format!(
                    "capability method {name} expects {} arguments, found {}",
                    parameters.len(),
                    args.len()
                ),
            ));
        }
        let result = self.capability_type(result, span)?;
        let receiver = self.borrow(target, true, span.line)?.0;
        let mut values = vec![receiver];
        let mut index = 0;
        while index < parameters.len() {
            let expected = self.capability_type(parameters[index], span)?;
            let value = self.expr(args[index], Some(expected))?;
            if value.1 != expected {
                return Err(err(
                    self.compiler.source.exprs[args[index]].span,
                    "E_TYPE",
                    &format!(
                        "{name} argument {} expects {}, found {}",
                        index + 1,
                        self.compiler.display_type(expected),
                        self.compiler.display_type(value.1)
                    ),
                ));
            }
            values.push(value.0);
            index += 1;
        }
        if let Some(operation) = method.intrinsic() {
            let dst = self.temp(result);
            self.emit(
                ir::Op::Intrinsic {
                    dst,
                    name: operation.to_string(),
                    args: values,
                },
                span.line,
            );
            Ok((dst, result))
        } else {
            let helper = method.helper().expect("source capability operation");
            let value = self.source_library_call("io", helper, BTreeMap::new(), values, span)?;
            if value.1 != result {
                return Err(err(
                    span,
                    "E_TYPE",
                    &format!(
                        "source IO helper {helper} returns {}, expected {}",
                        self.compiler.display_type(value.1),
                        self.compiler.display_type(result)
                    ),
                ));
            }
            Ok(value)
        }
    }
}
