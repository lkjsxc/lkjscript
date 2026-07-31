use super::*;

pub(super) fn preflight_enum_instruction(
    program: &lkjscript_ir::Program,
    function: &Function,
    instruction: &Instruction,
    layouts: &LayoutInterner,
) -> Result<(), LoweringError> {
    match &instruction.kind {
        InstructionKind::EnumValue {
            enum_id,
            variant,
            layout,
            fields,
        } => {
            let definition = enum_definition(program, function.id, *enum_id)?;
            let selected = enum_variant(definition, function.id, *variant)?;
            let arguments = enum_type_arguments(&instruction.ty, *enum_id, function.id)?;
            validate_enum_header(definition, *layout, arguments, function.id, layouts)?;
            if fields.len() != selected.fields.len() {
                return invalid_enum(function.id, "enum construction field count mismatch");
            }
            for (value, field) in fields.iter().zip(&selected.fields) {
                let actual = ssa_value_type(function, *value)?;
                let expected =
                    substitute_enum_type(&field.ty, &definition.type_parameters, arguments);
                if actual != &expected {
                    return invalid_enum(function.id, "enum field substitution mismatch");
                }
                lower_type(function.id, actual, layouts)?;
            }
            Ok(())
        }
        InstructionKind::EnumIsVariant {
            enum_id,
            variant,
            layout,
            value,
        } => {
            let definition = enum_definition(program, function.id, *enum_id)?;
            enum_variant(definition, function.id, *variant)?;
            let arguments =
                enum_type_arguments(ssa_value_type(function, *value)?, *enum_id, function.id)?;
            validate_enum_header(definition, *layout, arguments, function.id, layouts)
        }
        InstructionKind::EnumField {
            enum_id,
            variant,
            field,
            layout,
            value,
        } => {
            let definition = enum_definition(program, function.id, *enum_id)?;
            let selected = enum_variant(definition, function.id, *variant)?;
            let field = selected
                .fields
                .iter()
                .find(|candidate| candidate.id == *field)
                .ok_or_else(|| enum_error(function.id, "enum projection field is absent"))?;
            let arguments =
                enum_type_arguments(ssa_value_type(function, *value)?, *enum_id, function.id)?;
            validate_enum_header(definition, *layout, arguments, function.id, layouts)?;
            let expected = substitute_enum_type(&field.ty, &definition.type_parameters, arguments);
            if instruction.ty != expected {
                return invalid_enum(function.id, "enum projection substitution mismatch");
            }
            Ok(())
        }
        _ => invalid_enum(function.id, "non-enum instruction reached enum preflight"),
    }
}

pub(super) fn enum_definition(
    program: &lkjscript_ir::Program,
    function: FunctionId,
    enum_id: lkjscript_ir::EnumId,
) -> Result<&lkjscript_ir::EnumMetadata, LoweringError> {
    program
        .enums
        .iter()
        .find(|definition| definition.id == enum_id)
        .ok_or_else(|| enum_error(function, "enum metadata is absent"))
}

pub(super) fn enum_variant(
    definition: &lkjscript_ir::EnumMetadata,
    function: FunctionId,
    variant: lkjscript_ir::VariantId,
) -> Result<&lkjscript_ir::EnumVariantMetadata, LoweringError> {
    definition
        .variants
        .iter()
        .find(|candidate| candidate.id == variant)
        .ok_or_else(|| enum_error(function, "enum variant metadata is absent"))
}

pub(super) fn enum_type_arguments(
    ty: &SsaType,
    enum_id: lkjscript_ir::EnumId,
    function: FunctionId,
) -> Result<&[SsaType], LoweringError> {
    match ty {
        SsaType::Enum { id, arguments } if *id == enum_id => Ok(arguments),
        _ => invalid_enum(function, "enum concrete type identity mismatch"),
    }
}

fn validate_enum_header(
    definition: &lkjscript_ir::EnumMetadata,
    layout: lkjscript_ir::RuntimeLayoutId,
    arguments: &[SsaType],
    function: FunctionId,
    layouts: &LayoutInterner,
) -> Result<(), LoweringError> {
    if definition.layout.identity != layout
        || arguments.len() != definition.type_parameters.len()
        || arguments.len() > 16
    {
        return invalid_enum(function, "enum layout/substitution metadata mismatch");
    }
    for argument in arguments {
        lower_type(function, argument, layouts)?;
    }
    Ok(())
}

fn invalid_enum<T>(function: FunctionId, detail: &str) -> Result<T, LoweringError> {
    Err(enum_error(function, detail))
}

fn enum_error(function: FunctionId, detail: &str) -> LoweringError {
    LoweringError::new(
        LoweringFailureCode::UnsupportedOperation,
        Some(function),
        detail,
    )
}
