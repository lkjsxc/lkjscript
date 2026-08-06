use crate::analyze::*;
use std::collections::BTreeSet;

pub(super) fn pattern(
    program: &hir::Program,
    origin: SourceId,
    pattern: &MatchPattern,
    expected: &Type,
    locals: &mut BTreeSet<BindingId>,
    places: &mut BTreeSet<PlaceId>,
) -> Result<()> {
    if &pattern.ty() != expected {
        return Err(Error::msg("match plan pattern type is stale"));
    }
    match pattern {
        MatchPattern::Wildcard { .. } => Ok(()),
        MatchPattern::Binding { local: item } => local(program, origin, item, locals, places),
        MatchPattern::Bool(_) if expected == &Type::Bool => Ok(()),
        MatchPattern::I64(_) if expected == &Type::I64 => Ok(()),
        MatchPattern::Variant {
            ty,
            enum_id,
            variant,
            layout,
            fields,
        } => {
            let Type::Enum { id, arguments, .. } = ty else {
                return Err(Error::msg("variant pattern lost enum type"));
            };
            if id != enum_id {
                return Err(Error::msg("variant pattern EnumId is stale"));
            }
            let definition = program
                .enums
                .iter()
                .find(|item| item.id == *enum_id)
                .ok_or_else(|| Error::msg("variant pattern EnumId is unknown"))?;
            if definition.layout.identity != *layout {
                return Err(Error::msg("variant pattern layout is stale"));
            }
            let selected = definition
                .variants
                .iter()
                .find(|item| item.id == *variant)
                .ok_or_else(|| Error::msg("variant pattern VariantId is stale"))?;
            if fields.len() != selected.fields.len() {
                return Err(Error::msg("variant pattern field count is stale"));
            }
            let substitutions: HashMap<_, _> = definition
                .type_parameters
                .iter()
                .cloned()
                .zip(arguments.iter().cloned())
                .collect();
            for (field, declared) in fields.iter().zip(&selected.fields) {
                let field_ty = declared.ty.subst(&substitutions);
                if field.name != declared.name || field.field_index != declared.source_order {
                    return Err(Error::msg(
                        "variant pattern field identity/type/order is stale",
                    ));
                }
                match (&field.projection, &field.pattern) {
                    (None, MatchPattern::Wildcard { .. }) => {}
                    (Some(projection), pattern) if projection.ty == field_ty => {
                        local(program, origin, projection, locals, places)?;
                        pattern_child(program, origin, pattern, &field_ty, locals, places)?;
                    }
                    _ => {
                        return Err(Error::msg(
                            "variant pattern wildcard/projection metadata is stale",
                        ))
                    }
                }
            }
            Ok(())
        }
        MatchPattern::Product {
            ty,
            product,
            fields,
        } => {
            let Type::Product(name) = ty else {
                return Err(Error::msg("product pattern lost product type"));
            };
            let definition = program
                .products
                .iter()
                .find(|item| item.id == *product && item.name == *name)
                .ok_or_else(|| Error::msg("product pattern identity is stale"))?;
            if fields.len() != definition.fields.len() {
                return Err(Error::msg("product pattern field count is stale"));
            }
            for (field, declared) in fields.iter().zip(&definition.fields) {
                if field.name != declared.name || field.field_index != declared.source_order {
                    return Err(Error::msg(
                        "product pattern field identity/type/order is stale",
                    ));
                }
                match (&field.projection, &field.pattern) {
                    (None, MatchPattern::Wildcard { .. }) => {}
                    (Some(projection), pattern) if projection.ty == declared.ty => {
                        local(program, origin, projection, locals, places)?;
                        pattern_child(program, origin, pattern, &declared.ty, locals, places)?;
                    }
                    _ => {
                        return Err(Error::msg(
                            "product pattern wildcard/projection metadata is stale",
                        ))
                    }
                }
            }
            Ok(())
        }
        _ => Err(Error::msg("match plan literal/type mismatch")),
    }
}

fn pattern_child(
    program: &hir::Program,
    origin: SourceId,
    value: &MatchPattern,
    ty: &Type,
    locals: &mut BTreeSet<BindingId>,
    places: &mut BTreeSet<PlaceId>,
) -> Result<()> {
    pattern(program, origin, value, ty, locals, places)
}

pub(super) fn local(
    program: &hir::Program,
    origin: SourceId,
    local: &MatchLocal,
    locals: &mut BTreeSet<BindingId>,
    places: &mut BTreeSet<PlaceId>,
) -> Result<()> {
    let binding = program
        .binding(local.binding)
        .ok_or_else(|| Error::msg("match plan local binding is stale"))?;
    if binding.ty != local.ty
        || binding.kind != BindingKind::ImmutableLocal
        || binding.origin != Origin::Source(origin)
    {
        return Err(Error::msg("match plan local binding type/origin is stale"));
    }
    if !locals.insert(local.binding) || !places.insert(local.place) {
        return Err(Error::msg("match plan reuses a binding or place identity"));
    }
    Ok(())
}
