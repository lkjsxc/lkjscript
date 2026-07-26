use std::collections::HashSet;

use crate::verify::*;
use crate::{
    EnumMetadata, EnumVariantMetadata, Program, RuntimeLayoutId, SsaType, VariantFieldId, VariantId,
};

pub(crate) fn verify_enum_metadata(program: &Program) -> crate::Result<()> {
    let mut enum_ids = HashSet::new();
    let mut names = HashSet::new();
    let mut layouts = HashSet::new();
    let mut variants = HashSet::new();
    let mut fields = HashSet::new();
    for definition in &program.enums {
        if !definition.id.is_resolved()
            || !definition.layout.identity.is_resolved()
            || definition.name.is_empty()
            || !enum_ids.insert(definition.id)
            || !names.insert(definition.name.as_str())
            || !layouts.insert(definition.layout.identity)
            || definition.variants.is_empty()
        {
            return fail("SSA enum has invalid or duplicate identity/name/layout");
        }
        let mut parameters = HashSet::new();
        if definition
            .type_parameters
            .iter()
            .any(|name| name.is_empty() || !parameters.insert(name.as_str()))
        {
            return fail("SSA enum has invalid type parameters");
        }
        let scope: Vec<_> = definition
            .type_parameters
            .iter()
            .map(String::as_str)
            .collect();
        let mut tags = HashSet::new();
        for variant in &definition.variants {
            verify_variant(program, variant, &scope, &mut variants, &mut fields)?;
            if !tags.insert(variant.physical_tag)
                || usize::from(variant.physical_tag) >= definition.variants.len()
            {
                return fail("SSA enum has malformed physical tags");
            }
        }
        let recursive = definition
            .variants
            .iter()
            .flat_map(|v| &v.fields)
            .any(|field| contains_enum(&field.ty, definition.id));
        if recursive != definition.layout.recursive {
            return fail("SSA enum recursive layout fact is invalid");
        }
    }
    Ok(())
}

fn verify_variant(
    program: &Program,
    variant: &EnumVariantMetadata,
    scope: &[&str],
    variants: &mut HashSet<VariantId>,
    fields: &mut HashSet<VariantFieldId>,
) -> crate::Result<()> {
    if !variant.id.is_resolved() || variant.name.is_empty() || !variants.insert(variant.id) {
        return fail("SSA enum variant has invalid or duplicate identity/name");
    }
    let mut names = HashSet::new();
    for field in &variant.fields {
        if !field.id.is_resolved()
            || field.name.is_empty()
            || !fields.insert(field.id)
            || !names.insert(field.name.as_str())
        {
            return fail("SSA enum field has invalid or duplicate identity/name");
        }
        verify_type(program, &field.ty, scope)?;
        if contains_ownership_type(&field.ty)
            || field.indirect != contains_any_enum(&field.ty)
            || field.traced != is_traced(&field.ty)
        {
            return fail("SSA enum field has invalid storage/layout facts");
        }
    }
    Ok(())
}

pub(crate) fn enum_by_id(program: &Program, id: crate::EnumId) -> crate::Result<&EnumMetadata> {
    program
        .enums
        .iter()
        .find(|definition| definition.id == id)
        .ok_or_else(|| crate::IrError::new("missing SSA EnumId"))
}

pub(crate) fn variant_by_id(
    definition: &EnumMetadata,
    id: VariantId,
) -> crate::Result<&EnumVariantMetadata> {
    definition
        .variants
        .iter()
        .find(|variant| variant.id == id)
        .ok_or_else(|| crate::IrError::new("missing SSA VariantId for enum"))
}

pub(crate) fn check_layout(
    definition: &EnumMetadata,
    layout: RuntimeLayoutId,
) -> crate::Result<()> {
    if definition.layout.identity == layout {
        Ok(())
    } else {
        fail("SSA enum runtime layout identity mismatch")
    }
}

pub(crate) fn substitute(ty: &SsaType, parameters: &[String], arguments: &[SsaType]) -> SsaType {
    match ty {
        SsaType::TypeParameter(name) => parameters
            .iter()
            .position(|parameter| parameter == name)
            .and_then(|index| arguments.get(index))
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        SsaType::List(inner) => SsaType::List(Box::new(substitute(inner, parameters, arguments))),
        SsaType::Option(inner) => {
            SsaType::Option(Box::new(substitute(inner, parameters, arguments)))
        }
        SsaType::Result(ok, error) => SsaType::Result(
            Box::new(substitute(ok, parameters, arguments)),
            Box::new(substitute(error, parameters, arguments)),
        ),
        SsaType::Enum {
            id,
            arguments: nested,
        } => SsaType::Enum {
            id: *id,
            arguments: nested
                .iter()
                .map(|item| substitute(item, parameters, arguments))
                .collect(),
        },
        _ => ty.clone(),
    }
}

fn contains_enum(ty: &SsaType, id: crate::EnumId) -> bool {
    match ty {
        SsaType::Enum {
            id: nested,
            arguments,
        } => *nested == id || arguments.iter().any(|t| contains_enum(t, id)),
        SsaType::List(t) | SsaType::Option(t) => contains_enum(t, id),
        SsaType::Result(a, b) => contains_enum(a, id) || contains_enum(b, id),
        _ => false,
    }
}

fn contains_any_enum(ty: &SsaType) -> bool {
    match ty {
        SsaType::Enum { .. } => true,
        SsaType::List(t) | SsaType::Option(t) => contains_any_enum(t),
        SsaType::Result(a, b) => contains_any_enum(a) || contains_any_enum(b),
        _ => false,
    }
}

fn is_traced(ty: &SsaType) -> bool {
    matches!(
        ty,
        SsaType::Str
            | SsaType::Symbol
            | SsaType::Buf
            | SsaType::Product(_)
            | SsaType::Enum { .. }
            | SsaType::List(_)
            | SsaType::Option(_)
            | SsaType::Result(_, _)
            | SsaType::Function(_)
    )
}
