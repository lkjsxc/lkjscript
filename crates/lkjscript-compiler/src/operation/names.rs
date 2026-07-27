use crate::operation::*;
use lkjscript_contracts::{operation_by_id, operation_by_source_name, OperationIdentity};

impl Operation {
    pub fn from_name(name: &str) -> Option<Self> {
        let record = operation_by_source_name(name)?;
        Self::ALL.get(record.identity.index()).copied()
    }

    pub const fn identity(self) -> OperationIdentity {
        OperationIdentity::new(self as u16)
    }

    pub fn name(self) -> &'static str {
        operation_by_id(self.identity()).map_or("invalid-operation", |record| record.source_name)
    }
}
