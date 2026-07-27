use super::{EnumId, RuntimeLayoutId, Type};
use lkjscript_core::{
    PreludeEnum, NUMERIC_ERROR_ID, NUMERIC_ERROR_LAYOUT, OPTION_ID, OPTION_LAYOUT, RESULT_ID,
    RESULT_LAYOUT, SYSTEM_ERROR_ID, SYSTEM_ERROR_LAYOUT, UTF8_ERROR_ID, UTF8_ERROR_LAYOUT,
};

pub fn prelude_type(kind: PreludeEnum, arguments: Vec<Type>) -> Type {
    let (id, name) = match kind {
        PreludeEnum::Option => (OPTION_ID, "option"),
        PreludeEnum::Result => (RESULT_ID, "result"),
        PreludeEnum::NumericError => (NUMERIC_ERROR_ID, "numeric-error"),
        PreludeEnum::Utf8Error => (UTF8_ERROR_ID, "utf8-error"),
        PreludeEnum::SystemError => (SYSTEM_ERROR_ID, "system-error"),
    };
    Type::Enum {
        id: EnumId::new(id),
        name: name.into(),
        arguments,
    }
}

pub fn option_type(value: Type) -> Type {
    prelude_type(PreludeEnum::Option, vec![value])
}

pub fn result_type(ok: Type, error: Type) -> Type {
    prelude_type(PreludeEnum::Result, vec![ok, error])
}

pub fn numeric_error_type() -> Type {
    prelude_type(PreludeEnum::NumericError, Vec::new())
}

pub fn utf8_error_type() -> Type {
    prelude_type(PreludeEnum::Utf8Error, Vec::new())
}

pub fn system_error_type() -> Type {
    prelude_type(PreludeEnum::SystemError, Vec::new())
}

pub const fn prelude_layout(kind: PreludeEnum) -> RuntimeLayoutId {
    RuntimeLayoutId::new(match kind {
        PreludeEnum::Option => OPTION_LAYOUT,
        PreludeEnum::Result => RESULT_LAYOUT,
        PreludeEnum::NumericError => NUMERIC_ERROR_LAYOUT,
        PreludeEnum::Utf8Error => UTF8_ERROR_LAYOUT,
        PreludeEnum::SystemError => SYSTEM_ERROR_LAYOUT,
    })
}
