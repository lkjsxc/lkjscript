use super::{Value, ValueKind};

impl Value {
    pub const fn as_function(self) -> Option<u32> {
        match self.kind {
            ValueKind::Function => Some(self.payload as u32),
            _ => None,
        }
    }

    pub const fn as_symbol(self) -> Option<u32> {
        match self.kind {
            ValueKind::Symbol => Some(self.payload as u32),
            _ => None,
        }
    }
}
