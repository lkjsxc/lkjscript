use super::encoder::Encoder;
use crate::*;

pub(super) fn product(out: &mut Encoder, value: &ProductMetadata) {
    let ProductMetadata {
        id,
        identity,
        region,
        name,
        fields,
        region_fields,
    } = value;
    out.u64(id.raw());
    out.fixed(&identity.bytes());
    out.bool(*region);
    out.string(name);
    out.sequence(fields, |out, value| out.string(value));
    out.sequence(region_fields, region_field);
}
fn region_field(out: &mut Encoder, value: &RegionProductFieldKind) {
    match value {
        RegionProductFieldKind::Unit => out.tag(0),
        RegionProductFieldKind::Bool => out.tag(1),
        RegionProductFieldKind::I64 => out.tag(2),
        RegionProductFieldKind::F64 => out.tag(3),
        RegionProductFieldKind::List => out.tag(4),
        RegionProductFieldKind::Product(id) => {
            out.tag(5);
            out.u64(id.raw());
        }
    }
}
pub(super) fn product_field(out: &mut Encoder, value: &ProductFieldRef) {
    let ProductFieldRef { product, field } = value;
    out.u64(product.raw());
    out.u64(*field);
}
pub(super) fn enumeration(out: &mut Encoder, value: &EnumMetadata) {
    let EnumMetadata {
        id,
        name,
        type_parameter_count,
        layout,
        variants,
    } = value;
    out.fixed(&id.bytes());
    out.string(name);
    out.u64(*type_parameter_count);
    out.fixed(&layout.bytes());
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
            let EnumFieldMetadata { id, name } = value;
            out.fixed(&id.bytes());
            out.string(name);
        });
    });
}
pub(super) fn enum_construction(out: &mut Encoder, value: &EnumConstructionRef) {
    let EnumConstructionRef {
        enum_id,
        variant,
        layout,
        substitution_arity,
    } = value;
    enum_ref(out, *enum_id, *variant, *layout);
    out.u64(*substitution_arity);
}
pub(super) fn enum_variant(out: &mut Encoder, value: &EnumVariantRef) {
    let EnumVariantRef {
        enum_id,
        variant,
        layout,
    } = value;
    enum_ref(out, *enum_id, *variant, *layout);
}
pub(super) fn enum_field(out: &mut Encoder, value: &EnumFieldRef) {
    let EnumFieldRef {
        enum_id,
        variant,
        field,
        layout,
    } = value;
    enum_ref(out, *enum_id, *variant, *layout);
    out.fixed(&field.bytes());
}
fn enum_ref(out: &mut Encoder, id: EnumId, variant: VariantId, layout: RuntimeLayoutId) {
    out.fixed(&id.bytes());
    out.fixed(&variant.bytes());
    out.fixed(&layout.bytes());
}
