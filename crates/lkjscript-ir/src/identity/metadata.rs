use super::{encoder::Encoder, types::ty};
use crate::*;

pub(super) fn source(out: &mut Encoder, value: &SourceMetadata) {
    let SourceMetadata { id, path } = value;
    out.u32(*id);
    out.string(path);
}
pub(super) fn product(out: &mut Encoder, value: &ProductMetadata) {
    let ProductMetadata { id, name, fields } = value;
    out.u64(id.raw());
    out.string(name);
    out.sequence(fields, |out, value| {
        let ProductField { name, ty: value_ty } = value;
        out.string(name);
        ty(out, value_ty);
    });
}
pub(super) fn enumeration(out: &mut Encoder, value: &EnumMetadata) {
    let EnumMetadata {
        id,
        name,
        type_parameters,
        variants,
        layout,
    } = value;
    out.fixed(&id.bytes());
    out.string(name);
    out.sequence(type_parameters, |out, value| out.string(value));
    out.sequence(variants, |out, value| {
        let EnumVariantMetadata {
            id,
            name,
            source_order,
            physical_tag,
            fields,
        } = value;
        out.fixed(&id.bytes());
        out.string(name);
        out.u64(*source_order);
        out.u64(*physical_tag);
        out.sequence(fields, |out, value| {
            let EnumFieldMetadata {
                id,
                name,
                ty: value_ty,
                indirect,
            } = value;
            out.fixed(&id.bytes());
            out.string(name);
            ty(out, value_ty);
            out.bool(*indirect);
        });
    });
    let EnumLayoutFacts {
        identity,
        recursive,
    } = layout;
    out.fixed(&identity.bytes());
    out.bool(*recursive);
}
pub(super) fn trait_value(out: &mut Encoder, value: &TraitMetadata) {
    let TraitMetadata {
        id,
        name,
        role,
        source,
    } = value;
    out.u32(id.raw());
    out.string(name);
    out.tag(match role {
        TraitRole::Copy => 0,
        TraitRole::Clone => 1,
        TraitRole::Drop => 2,
        TraitRole::Send => 3,
        TraitRole::Sync => 4,
        TraitRole::User => 5,
    });
    out.option(source.as_ref(), |out, value| out.u32(*value));
}
pub(super) fn implementation(out: &mut Encoder, value: &ImplMetadata) {
    let ImplMetadata {
        id,
        trait_id,
        product,
        source,
    } = value;
    out.u32(id.raw());
    out.u32(trait_id.raw());
    out.u64(product.raw());
    out.u32(*source);
}
pub(super) fn effects(out: &mut Encoder, value: EffectSet) {
    out.u16(value.bits());
}
pub(super) fn origin(out: &mut Encoder, value: &Origin) {
    let Origin { source, node } = value;
    out.u32(*source);
    out.u32(*node);
}
