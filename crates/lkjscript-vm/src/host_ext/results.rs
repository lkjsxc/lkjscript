use super::*;
use lkjscript_core::SystemErrorKind;

fn enum_value(
    arena: &mut Arena,
    layout: [u8; 32],
    physical_tag: u16,
    payload: Vec<Value>,
) -> Result<Value> {
    arena.alloc(HeapObj::Enum {
        layout: lkjscript_core::RuntimeLayoutId::new(layout),
        physical_tag,
        active_payload: payload,
    })
}

pub fn result_ok(arena: &mut Arena, value: Value) -> Result<Value> {
    enum_value(arena, lkjscript_core::RESULT_LAYOUT, 0, vec![value])
}

pub fn result_err(arena: &mut Arena, value: Value) -> Result<Value> {
    enum_value(arena, lkjscript_core::RESULT_LAYOUT, 1, vec![value])
}

pub fn language_result(
    arena: &mut Arena,
    kind: SystemErrorKind,
    result: Result<Value>,
) -> Result<Value> {
    if kind == SystemErrorKind::Utf8 {
        return Err(Error::msg(
            "SystemError.Utf8 requires a structured Utf8Error payload",
        ));
    }
    match result {
        Ok(value) => result_ok(arena, value),
        Err(error) => {
            let code = option_none(arena)?;
            let detail = arena.alloc(HeapObj::Str(error.to_string()))?;
            let detail = option_some(arena, detail)?;
            let error = enum_value(
                arena,
                lkjscript_core::SYSTEM_ERROR_LAYOUT,
                kind.physical_tag(),
                vec![code, detail],
            )?;
            result_err(arena, error)
        }
    }
}

pub fn utf8_result(
    arena: &mut Arena,
    result: std::result::Result<Value, lkjscript_core::Utf8Failure>,
) -> Result<Value> {
    match result {
        Ok(value) => result_ok(arena, value),
        Err(error) => {
            let error = utf8_error(arena, error)?;
            result_err(arena, error)
        }
    }
}

pub fn system_utf8_error(arena: &mut Arena, error: lkjscript_core::Utf8Failure) -> Result<Value> {
    let error = utf8_error(arena, error)?;
    let system = enum_value(
        arena,
        lkjscript_core::SYSTEM_ERROR_LAYOUT,
        SystemErrorKind::Utf8.physical_tag(),
        vec![error],
    )?;
    result_err(arena, system)
}

fn utf8_error(arena: &mut Arena, error: lkjscript_core::Utf8Failure) -> Result<Value> {
    let offset =
        i64::try_from(error.offset).map_err(|_| Error::msg("UTF-8 error offset exceeds I64"))?;
    let offset = Value::from_i64(offset);
    enum_value(
        arena,
        lkjscript_core::UTF8_ERROR_LAYOUT,
        error.kind.physical_tag(),
        vec![offset],
    )
}

pub fn option_none(arena: &mut Arena) -> Result<Value> {
    enum_value(arena, lkjscript_core::OPTION_LAYOUT, 1, Vec::new())
}

pub fn option_some(arena: &mut Arena, value: Value) -> Result<Value> {
    enum_value(arena, lkjscript_core::OPTION_LAYOUT, 0, vec![value])
}
