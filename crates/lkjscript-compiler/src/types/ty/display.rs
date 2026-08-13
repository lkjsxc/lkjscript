use super::Type;

impl std::fmt::Display for Type {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        enum Work<'a> {
            Type(&'a Type),
            Text(&'static str),
            Owned(&'a str),
        }

        let mut pending = vec![Work::Type(self)];
        while let Some(item) = pending.pop() {
            match item {
                Work::Text(text) => formatter.write_str(text)?,
                Work::Owned(text) => formatter.write_str(text)?,
                Work::Type(ty) => match ty {
                    Type::Never => formatter.write_str("never")?,
                    Type::Unit => formatter.write_str("unit")?,
                    Type::Bool => formatter.write_str("bool")?,
                    Type::I64 => formatter.write_str("i64")?,
                    Type::F64 => formatter.write_str("f64")?,
                    Type::Str => formatter.write_str("string")?,
                    Type::Bytes => formatter.write_str("bytes")?,
                    Type::ByteVector => formatter.write_str("byte-vector")?,
                    Type::ByteSlice => formatter.write_str("byte-slice")?,
                    Type::ByteSliceMut => formatter.write_str("byte-slice-mut")?,
                    Type::Path => formatter.write_str("path")?,
                    Type::Capability(kind) => {
                        formatter.write_str("capability ")?;
                        formatter.write_str(kind.as_str())?;
                    }
                    Type::Symbol => formatter.write_str("symbol")?,
                    Type::Resource(kind) => formatter.write_str(kind.as_str())?,
                    Type::Product(id) => write!(formatter, "product#{}", id.raw())?,
                    Type::Enum { id, arguments } => {
                        if let Some(name) = crate::types::prelude_name_for_id(*id) {
                            formatter.write_str(name)?;
                        } else {
                            formatter.write_str("enum#")?;
                            for byte in id.bytes() {
                                write!(formatter, "{byte:02x}")?;
                            }
                        }
                        for argument in arguments.iter().rev() {
                            pending.push(Work::Type(argument));
                            pending.push(Work::Text(" "));
                        }
                    }
                    Type::Param(name) => formatter.write_str(name)?,
                    Type::List(inner) => {
                        formatter.write_str("list ")?;
                        pending.push(Work::Type(inner));
                    }
                    Type::Fn { params, ret } => {
                        formatter.write_str("fn inputs")?;
                        pending.push(Work::Type(ret));
                        pending.push(Work::Text(" output "));
                        for parameter in params.iter().rev() {
                            pending.push(Work::Type(parameter));
                            pending.push(Work::Text(" "));
                        }
                    }
                    Type::Forall { vars, body } => {
                        formatter.write_str("forall")?;
                        pending.push(Work::Type(body));
                        pending.push(Work::Text(" "));
                        for variable in vars.iter().rev() {
                            pending.push(Work::Owned(variable));
                            pending.push(Work::Text(" "));
                        }
                    }
                },
            }
        }
        Ok(())
    }
}
