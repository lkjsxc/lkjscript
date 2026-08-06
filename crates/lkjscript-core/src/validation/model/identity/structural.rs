use super::{encoder::Encoder, types};
use crate::*;

pub(super) fn structural_type(out: &mut Encoder, value: &StructuralTypeMetadata) {
    let StructuralTypeMetadata {
        id,
        witness,
        identity,
        runtime_type,
        kind,
        layout,
        mode,
    } = value;
    out.u64(id.raw());
    out.fixed(&witness.bytes());
    out.fixed(&identity.bytes());
    types::structural_type(out, *runtime_type);
    type_kind(out, *kind);
    out.u64(layout.raw());
    out.tag(match mode {
        StructuralTypeMode::Copy => 0,
        StructuralTypeMode::Immutable => 1,
        StructuralTypeMode::Affine => 2,
    });
}
fn type_kind(out: &mut Encoder, value: StructuralTypeKind) {
    match value {
        StructuralTypeKind::String => out.tag(0),
        StructuralTypeKind::Path => out.tag(1),
        StructuralTypeKind::Product(id) => {
            out.tag(2);
            out.u64(id.raw());
        }
        StructuralTypeKind::Enum(id) => {
            out.tag(3);
            out.fixed(&id.bytes());
        }
    }
}
pub(super) fn layout(out: &mut Encoder, value: &StructuralLayoutMetadata) {
    let StructuralLayoutMetadata { id, identity, kind } = value;
    out.u64(id.raw());
    out.fixed(&identity.bytes());
    match kind {
        StructuralLayoutKind::String => out.tag(0),
        StructuralLayoutKind::Path => out.tag(1),
        StructuralLayoutKind::Product { product, fields } => {
            out.tag(2);
            out.u64(product.raw());
            out.sequence(fields, field);
        }
        StructuralLayoutKind::Enum {
            enum_id,
            runtime_layout,
            variants,
        } => {
            out.tag(3);
            out.fixed(&enum_id.bytes());
            out.fixed(&runtime_layout.bytes());
            out.sequence(variants, |out, value| {
                let StructuralVariantLayout {
                    variant,
                    source_order,
                    physical_tag,
                    fields,
                } = value;
                out.fixed(&variant.bytes());
                out.u64(*source_order);
                out.u64(*physical_tag);
                out.sequence(fields, field);
            });
        }
    }
}
pub(super) fn representation(out: &mut Encoder, value: &StructuralRepresentationMetadata) {
    let StructuralRepresentationMetadata {
        id,
        type_id,
        witness,
        witness_group,
        witness_member,
        layout,
        category,
        storage,
        route,
    } = value;
    out.u64(id.raw());
    out.u64(type_id.raw());
    out.fixed(&witness.bytes());
    out.fixed(&witness_group.bytes());
    out.u16(*witness_member);
    out.u64(layout.raw());
    out.tag(match category {
        StructuralValueCategory::Owner => 0,
        StructuralValueCategory::View => 1,
        StructuralValueCategory::Destination => 2,
    });
    out.tag(match storage {
        StructuralStorage::Inline => 0,
        StructuralStorage::Static => 1,
        StructuralStorage::Stack => 2,
        StructuralStorage::CallerDestination => 3,
        StructuralStorage::UniqueStructural => 4,
        StructuralStorage::OrdinaryRegion => 5,
        StructuralStorage::SealedRegion => 6,
        StructuralStorage::BorrowedView => 7,
        StructuralStorage::ExternalResource => 8,
    });
    out.fixed(route);
}
pub(super) fn destination(out: &mut Encoder, value: &StructuralDestinationMetadata) {
    let StructuralDestinationMetadata {
        id,
        representation,
        owner_representation,
        active_variant,
        fields,
    } = value;
    out.u64(id.raw());
    out.u64(representation.raw());
    out.u64(owner_representation.raw());
    out.option(active_variant.as_ref(), |out, value| {
        out.fixed(&value.bytes())
    });
    out.sequence(fields, field);
}
pub(super) fn destination_field(out: &mut Encoder, value: &StructuralDestinationFieldRef) {
    let StructuralDestinationFieldRef { destination, field } = value;
    out.u64(destination.raw());
    out.u64(*field);
}
pub(super) fn aggregate_field(out: &mut Encoder, value: &StructuralAggregateFieldRef) {
    let StructuralAggregateFieldRef {
        representation,
        active_variant,
        field: index,
        result,
        result_representation,
    } = value;
    out.u64(representation.raw());
    out.option(active_variant.as_ref(), |out, value| {
        out.fixed(&value.bytes())
    });
    out.u64(*index);
    field(out, result);
    out.option(result_representation.as_ref(), |out, value| {
        out.u64(value.raw())
    });
}
pub(super) fn payload(out: &mut Encoder, value: &StructuralPayloadRef) {
    let StructuralPayloadRef {
        representation,
        variant,
        result,
        result_representation,
    } = value;
    out.u64(representation.raw());
    out.fixed(&variant.bytes());
    field(out, result);
    out.option(result_representation.as_ref(), |out, value| {
        out.u64(value.raw())
    });
}
fn field(out: &mut Encoder, value: &StructuralFieldMetadata) {
    let StructuralFieldMetadata {
        identity,
        runtime_type,
        route,
        resource,
    } = value;
    out.fixed(&identity.bytes());
    out.option(runtime_type.as_ref(), |out, value| {
        types::structural_type(out, *value)
    });
    out.tag(match route {
        StructuralFieldRoute::Copy => 0,
        StructuralFieldRoute::Structural(_) => 1,
        StructuralFieldRoute::Unique => 2,
        StructuralFieldRoute::Resource => 3,
        StructuralFieldRoute::LegacyHeap => 4,
    });
    if let StructuralFieldRoute::Structural(id) = route {
        out.u64(id.raw());
    }
    out.option(resource.as_ref(), |out, value| types::resource(out, *value));
}
