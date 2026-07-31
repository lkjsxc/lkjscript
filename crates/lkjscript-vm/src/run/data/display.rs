use super::*;

pub(in crate::run) fn value<J: RuntimeTier>(vm: &Vm<'_, J>, value: Value) -> Result<String> {
    if value.is_invalid() {
        return Err(Error::msg("invalid VM value escaped initialized storage"));
    }
    if value.is_unit() {
        return Ok("unit".into());
    }
    if value.is_empty_list() {
        return Ok("empty-list".into());
    }
    if let Some(boolean) = value.as_bool() {
        return Ok(boolean.to_string());
    }
    if let Some(number) = value.as_i64() {
        return Ok(number.to_string());
    }
    if let Some(number) = value.as_f64() {
        return Ok(number.to_string());
    }
    if let Some(resource) = value.as_resource() {
        return Ok(format!("resource#{resource}"));
    }
    if let Some(prototype) = value.as_function() {
        return Ok(format!("#<fn:{prototype}>"));
    }
    if let Some(symbol) = value.as_symbol() {
        return match vm.chunk.constants().get(symbol as usize) {
            Some(Constant::Symbol(text)) => Ok(text.clone()),
            _ => Err(Error::msg("invalid symbol constant index")),
        };
    }
    if value.as_static_string().is_some() {
        return crate::run::structural_ops::copy_string(vm, value);
    }
    if value.as_structural_root().is_some() {
        if let Ok(text) = crate::run::structural_ops::copy_string(vm, value) {
            return Ok(text);
        }
        if let Ok(path) = crate::run::structural_ops::copy_path(vm, value) {
            return Ok(format!("#<path:{}>", path.len()));
        }
        return Ok("#<structural-owner>".into());
    }
    let object = vm.arena.get(value)?.clone();
    match object {
        HeapObj::Pair { car, cdr } => Ok(format!(
            "({} . {})",
            self::value(vm, car)?,
            self::value(vm, cdr)?
        )),
        HeapObj::Product { product, .. } => Ok(format!("#<product:{}>", product.raw())),
        HeapObj::Enum {
            layout,
            physical_tag,
            active_payload,
        } => display_enum(vm, layout.bytes(), physical_tag, &active_payload),
    }
}

fn display_enum<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    layout: [u8; 32],
    tag: u16,
    payload: &[Value],
) -> Result<String> {
    if layout == lkjscript_core::OPTION_LAYOUT {
        return match (tag, payload) {
            (0, [value]) => Ok(format!("some({})", self::value(vm, *value)?)),
            (1, []) => Ok("none".into()),
            _ => Err(Error::msg("malformed option value")),
        };
    }
    if layout == lkjscript_core::RESULT_LAYOUT {
        return match (tag, payload) {
            (0, [value]) => Ok(format!("ok({})", self::value(vm, *value)?)),
            (1, [value]) => Ok(format!("err({})", self::value(vm, *value)?)),
            _ => Err(Error::msg("malformed result value")),
        };
    }
    let mut fields = Vec::with_capacity(payload.len());
    for field in payload {
        fields.push(self::value(vm, *field)?);
    }
    Ok(format!("#<enum:{tag}>({})", fields.join(", ")))
}
