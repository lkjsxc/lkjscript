use crate::{
    EnumId, EnumVariantMetadata, ProductId, Program, RuntimeLayoutId, SsaType, VariantFieldId,
    VariantId,
};

use super::substitute;

pub(crate) fn product_fields(program: &Program, id: ProductId) -> Result<Vec<SsaType>, String> {
    program
        .products
        .iter()
        .find(|definition| definition.id == id)
        .map(|definition| {
            definition
                .fields
                .iter()
                .map(|field| field.ty.clone())
                .collect()
        })
        .ok_or_else(|| "evaluator product metadata is missing".into())
}

pub(crate) fn enum_variant<'a>(
    program: &'a Program,
    ty: &SsaType,
    variant: VariantId,
) -> Result<(&'a EnumVariantMetadata, Vec<SsaType>, RuntimeLayoutId), String> {
    let SsaType::Enum { id, arguments } = ty else {
        return Err("evaluator enum operation has a non-enum type".into());
    };
    let definition = enum_definition(program, *id)?;
    if definition.type_parameters.len() != arguments.len() {
        return Err("evaluator enum substitution arity mismatch".into());
    }
    let selected = definition
        .variants
        .iter()
        .find(|candidate| candidate.id == variant)
        .ok_or_else(|| "evaluator enum variant metadata is missing".to_owned())?;
    let fields = selected
        .fields
        .iter()
        .map(|field| substitute(&field.ty, &definition.type_parameters, arguments))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((selected, fields, definition.layout.identity))
}

pub(crate) fn enum_field_index(
    selected: &EnumVariantMetadata,
    field: VariantFieldId,
) -> Result<usize, String> {
    selected
        .fields
        .iter()
        .position(|candidate| candidate.id == field)
        .ok_or_else(|| "evaluator enum field metadata is missing".into())
}

pub(crate) fn enum_definition(
    program: &Program,
    id: EnumId,
) -> Result<&crate::EnumMetadata, String> {
    program
        .enums
        .iter()
        .find(|definition| definition.id == id)
        .ok_or_else(|| "evaluator enum metadata is missing".into())
}
