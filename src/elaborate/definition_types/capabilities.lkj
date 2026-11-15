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
            Value::Listener => self.atom("Listener"),
            Value::Connector => self.atom("Connector"),
            Value::Readiness => self.atom("Readiness"),
            Value::Entropy => self.atom("Entropy"),
            Value::Stop => self.atom("Stop"),
            Value::Pid => self.atom("u32"),
            Value::Port => self.atom("u16"),
            Value::FileRef | Value::DirectoryRef | Value::MutableFileRef | Value::ListenerRef | Value::ClockRef | Value::StopRef => {
                let inner = self.atom(match value {
                    Value::FileRef | Value::MutableFileRef => "File",
                    Value::ListenerRef => "Listener",
                    Value::ClockRef => "Clock",
                    Value::StopRef => "Stop",
                    _ => "Directory",
                });
                self.shaped(Shape::Ref(!matches!(value, Value::FileRef), inner))
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
            Value::MaybeBytes => {
                let byte = self.atom("u8");
                let bytes = self.shaped(Shape::Vec(byte));
                let path = self.names.library_type("prelude", "Option", span)?;
                let declaration = path.declaration.ok_or_else(|| err(span, "E_TYPE", "source Option declaration is missing"))?;
                self.shaped(Shape::Nominal(declaration, vec![bytes]))
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
            | Value::ResultListener
            | Value::ResultConnector
            | Value::ResultReadiness
            | Value::ResultStop
            | Value::ResultBoolean
            | Value::ResultPid
            | Value::ResultEvents
            | Value::ResultPort
            | Value::ResultNames
            | Value::ResultStatus
            | Value::ResultMaybeStatus => {
                let success = match value {
                    Value::ResultDirectory => self.capability_type(Value::Directory, span)?,
                    Value::ResultFile => self.capability_type(Value::File, span)?,
                    Value::ResultChild => self.capability_type(Value::Child, span)?,
                    Value::ResultListener => self.capability_type(Value::Listener, span)?,
                    Value::ResultConnector => self.capability_type(Value::Connector, span)?,
                    Value::ResultReadiness => self.capability_type(Value::Readiness, span)?,
                    Value::ResultStop => self.capability_type(Value::Stop, span)?,
                    Value::ResultBoolean => self.capability_type(Value::Boolean, span)?,
                    Value::ResultPid => self.capability_type(Value::Pid, span)?,
                    Value::ResultPort => self.capability_type(Value::Port, span)?,
                    Value::ResultNames => self.capability_type(Value::Arguments, span)?,
                    Value::ResultStatus => self.atom("i64"),
                    Value::ResultEvents => {
                        let token = self.atom("usize");
                        let flags = self.atom("u8");
                        let pair = self.shaped(Shape::Tuple(vec![token, flags]));
                        self.shaped(Shape::Vec(pair))
                    }

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
