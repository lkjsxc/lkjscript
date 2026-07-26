use super::{EnumId, RuntimeLayoutId, Type, VariantId};
use lkjscript_core::{NumericError, NUMERIC_ERROR_ID, NUMERIC_ERROR_LAYOUT};

pub const NUMERIC_ERROR_NAME: &str = "NumericError";

pub fn numeric_error_type() -> Type {
    Type::Enum {
        id: EnumId::new(NUMERIC_ERROR_ID),
        name: NUMERIC_ERROR_NAME.into(),
        arguments: Vec::new(),
    }
}

pub const fn numeric_error_variant(error: NumericError) -> VariantId {
    VariantId::new(error.variant_id())
}

pub const fn numeric_error_layout() -> RuntimeLayoutId {
    RuntimeLayoutId::new(NUMERIC_ERROR_LAYOUT)
}
