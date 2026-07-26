use super::*;

pub(super) fn ssa_value_type(
    function: &Function,
    value: ValueId,
) -> Result<&SsaType, LoweringError> {
    function
        .blocks
        .iter()
        .find_map(|block| {
            block
                .parameters
                .iter()
                .find(|parameter| parameter.id == value)
                .map(|parameter| &parameter.ty)
                .or_else(|| {
                    block
                        .instructions
                        .iter()
                        .find(|instruction| instruction.id == value)
                        .map(|instruction| &instruction.ty)
                })
        })
        .ok_or_else(|| {
            LoweringError::new(
                LoweringFailureCode::InvalidFunction,
                Some(function.id),
                "enum operand type is absent",
            )
        })
}

pub(super) fn substitute_enum_type(ty: &SsaType, names: &[String], values: &[SsaType]) -> SsaType {
    match ty {
        SsaType::TypeParameter(name) => names
            .iter()
            .position(|candidate| candidate == name)
            .and_then(|index| values.get(index))
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        SsaType::List(inner) => SsaType::List(Box::new(substitute_enum_type(inner, names, values))),
        SsaType::Enum { id, arguments } => SsaType::Enum {
            id: *id,
            arguments: arguments
                .iter()
                .map(|argument| substitute_enum_type(argument, names, values))
                .collect(),
        },
        _ => ty.clone(),
    }
}
