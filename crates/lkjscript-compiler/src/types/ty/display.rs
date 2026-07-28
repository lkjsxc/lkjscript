use super::Type;

impl std::fmt::Display for Type {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Never => formatter.write_str("never"),
            Self::Unit => formatter.write_str("unit"),
            Self::Bool => formatter.write_str("bool"),
            Self::I64 => formatter.write_str("i64"),
            Self::F64 => formatter.write_str("f64"),
            Self::Str => formatter.write_str("string"),
            Self::Buf => formatter.write_str("buf"),
            Self::Bytes => formatter.write_str("bytes"),
            Self::ByteVector => formatter.write_str("byte-vector"),
            Self::ByteSlice => formatter.write_str("byte-slice"),
            Self::ByteSliceMut => formatter.write_str("byte-slice-mut"),
            Self::Path => formatter.write_str("path"),
            Self::Capability(kind) => write!(formatter, "capability {}", kind.as_str()),
            Self::Symbol => formatter.write_str("symbol"),
            Self::Resource(kind) => formatter.write_str(kind.as_str()),
            Self::Product(name) => write!(formatter, "product {name}"),
            Self::Enum {
                name, arguments, ..
            } => {
                formatter.write_str(name.rsplit(':').next().unwrap_or(name))?;
                for argument in arguments {
                    write!(formatter, " {argument}")?;
                }
                Ok(())
            }
            Self::Param(name) => formatter.write_str(name),
            Self::List(inner) => write!(formatter, "list {inner}"),
            Self::Fn { params, ret } => {
                formatter.write_str("fn inputs")?;
                for parameter in params {
                    write!(formatter, " {parameter}")?;
                }
                write!(formatter, " output {ret}")
            }
            Self::Forall { vars, body } => {
                write!(formatter, "forall {} {body}", vars.join(" "))
            }
        }
    }
}
