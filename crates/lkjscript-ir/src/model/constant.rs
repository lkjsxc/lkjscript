use super::*;

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Unit,
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    StaticBytes(Vec<u8>),
    Symbol(String),
    EmptyList,
}

impl Constant {
    pub fn ty(&self, declared: &SsaType) -> bool {
        matches!(
            (self, declared),
            (Self::Unit, SsaType::Unit)
                | (Self::Bool(_), SsaType::Bool)
                | (Self::I64(_), SsaType::I64)
                | (Self::F64(_), SsaType::F64)
                | (Self::Str(_), SsaType::Str)
                | (Self::StaticBytes(_), SsaType::Bytes)
                | (Self::Symbol(_), SsaType::Symbol)
                | (Self::EmptyList, SsaType::List(_))
        )
    }
}
