use super::{Value, ValueKind};

impl Value {
    pub const fn as_function(self) -> Option<u64> {
        match self.kind {
            ValueKind::Function => Some(self.payload),
            _ => None,
        }
    }

    pub const fn as_symbol(self) -> Option<u64> {
        match self.kind {
            ValueKind::Symbol => Some(self.payload),
            _ => None,
        }
    }
}
