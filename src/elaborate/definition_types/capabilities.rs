//! Source-shaped counterparts of the shared capability method inventory.
use super::super::capabilities::Value;
use super::{Infer, Result, Shape, Variable, a, err};

impl Infer<'_, '_> {
    pub(super) fn capability_type(&mut self, value: Value, span: a::Span) -> Result<Variable> {
        Ok(match value {
            Value::Unit => self.atom("()"),
            Value::Byte => self.atom("u8"),
            Value::Count => self.atom("usize"),
            Value::Error => self.atom("i64"),
            Value::Boolean => self.atom("bool"),
            Value::Stdout => self.atom("Stdout"),
            Value::Stdin => self.atom("Stdin"),
            Value::Stderr => self.atom("Stderr"),
            Value::Directory => self.atom("Directory"),
            Value::File => self.atom("File"),
            Value::Process => self.atom("Process"),
            Value::Clock => self.atom("Clock"),
            Value::Child => self.atom("Child"),
            Value::FileRef | Value::DirectoryRef => {
                let inner = self.atom(if matches!(value, Value::FileRef) {
                    "File"
                } else {
                    "Directory"
                });
                self.shaped(Shape::Ref(matches!(value, Value::DirectoryRef), inner))
            }
            Value::ArgumentsView => {
                let byte = self.atom("u8");
                let bytes = self.shaped(Shape::Vec(byte));
                let values = self.shaped(Shape::Slice(bytes));
                self.shaped(Shape::Ref(false, values))
            }
            Value::Text => {
                let text = self.atom("str");
                self.shaped(Shape::Ref(false, text))
            }
            Value::Bytes | Value::MutableBytes => {
                let byte = self.atom("u8");
                let bytes = self.shaped(Shape::Slice(byte));
                self.shaped(Shape::Ref(matches!(value, Value::MutableBytes), bytes))
            }
            Value::Arguments => {
                let byte = self.atom("u8");
                let bytes = self.shaped(Shape::Vec(byte));
                self.shaped(Shape::Vec(bytes))
            }
            Value::ResultDirectory
            | Value::ResultFile
            | Value::ResultCount
            | Value::ResultBytes
            | Value::ResultUnit
            | Value::ResultChild
            | Value::ResultNames
            | Value::ResultStatus
            | Value::ResultMaybeStatus => {
                let success = match value {
                    Value::ResultDirectory => self.capability_type(Value::Directory, span)?,
                    Value::ResultFile => self.capability_type(Value::File, span)?,
                    Value::ResultChild => self.capability_type(Value::Child, span)?,
                    Value::ResultNames => self.capability_type(Value::Arguments, span)?,
                    Value::ResultStatus => self.atom("i64"),
                    Value::ResultMaybeStatus => {
                        let status = self.atom("i64");
                        let path = self.names.library_type("prelude", "Option", span)?;
                        let declaration = path.declaration.ok_or_else(|| {
                            err(span, "E_TYPE", "source Option declaration is missing")
                        })?;
                        self.shaped(Shape::Nominal(declaration, vec![status]))
                    }
                    Value::ResultCount => self.atom("usize"),
                    Value::ResultBytes => {
                        let byte = self.atom("u8");
                        self.shaped(Shape::Vec(byte))
                    }
                    _ => self.atom("()"),
                };
                let error = self.atom("i64");
                let path = self.names.library_type("prelude", "Result", span)?;
                let Some(declaration) = path.declaration else {
                    return Err(err(span, "E_TYPE", "source Result declaration is missing"));
                };
                self.shaped(Shape::Nominal(declaration, vec![success, error]))
            }
        })
    }
}
