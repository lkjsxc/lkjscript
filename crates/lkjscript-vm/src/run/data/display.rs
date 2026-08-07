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
        return match vm.chunk.constant(symbol) {
            Some(Constant::Symbol(text)) => Ok(text.clone()),
            _ => Err(Error::msg("invalid symbol constant index")),
        };
    }
    if value.as_static_string().is_some() {
        return crate::run::structural_ops::copy_string(vm, value);
    }
    if value.as_region_product_word().is_some() {
        return Ok("#<region-product>".into());
    }
    if value.as_segmented_list().is_some() {
        let mut cursor = value;
        let mut elements = Vec::new();
        loop {
            let Some((head, tail)) = vm.list_view(cursor)? else {
                return Ok(format!("({})", elements.join(" ")));
            };
            elements
                .try_reserve(1)
                .map_err(|_| Error::msg("list display allocation failed"))?;
            elements.push(self::value(vm, head)?);
            cursor = tail;
        }
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
    Err(Error::msg("value display category is unsupported"))
}
