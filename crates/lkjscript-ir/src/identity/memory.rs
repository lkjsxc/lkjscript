use super::{encoder::Encoder, types::ty, witness};
use crate::*;

pub(super) fn structural_memory(out: &mut Encoder, value: &StructuralMemoryMetadata) {
    let StructuralMemoryMetadata {
        plan,
        witness_groups,
        witnesses,
        types,
        layouts,
        representations,
    } = value;
    out.fixed(&plan.bytes());
    out.sequence(witness_groups, witness::group);
    out.sequence(witnesses, witness::descriptor);
    out.sequence(types, |out, value| {
        let StructuralTypeMetadata {
            id,
            witness,
            ty: value_ty,
            layout,
            mode,
        } = value;
        out.u64(id.raw());
        out.fixed(&witness.bytes());
        ty(out, value_ty);
        out.u64(layout.raw());
        structural_type_mode(out, *mode);
    });
    out.sequence(layouts, structural_layout);
    out.sequence(representations, |out, value| {
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
        category_value(out, *category);
        storage_value(out, *storage);
        out.fixed(route);
    });
}

fn structural_layout(out: &mut Encoder, value: &StructuralLayoutMetadata) {
    let StructuralLayoutMetadata { id, identity, kind } = value;
    out.u64(id.raw());
    out.fixed(&identity.bytes());
    match kind {
        StructuralLayoutKind::String => out.tag(0),
        StructuralLayoutKind::Path => out.tag(1),
        StructuralLayoutKind::Product { product, fields } => {
            out.tag(2);
            out.u64(product.raw());
            out.sequence(fields, ty);
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
                out.sequence(fields, ty);
            });
        }
    }
}

fn structural_type_mode(out: &mut Encoder, value: StructuralTypeMode) {
    out.tag(match value {
        StructuralTypeMode::Copy => 0,
        StructuralTypeMode::Immutable => 1,
        StructuralTypeMode::Affine => 2,
    });
}
fn category_value(out: &mut Encoder, value: StructuralValueCategory) {
    out.tag(match value {
        StructuralValueCategory::Owner => 0,
        StructuralValueCategory::View => 1,
        StructuralValueCategory::Destination => 2,
    });
}
fn storage_value(out: &mut Encoder, value: StructuralStorage) {
    out.tag(match value {
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
}

pub(super) fn drop_glue(out: &mut Encoder, value: DropGlueIdentity) {
    match value {
        DropGlueIdentity::ByteVector => out.tag(0),
        DropGlueIdentity::Bytes => out.tag(1),
        DropGlueIdentity::Resource(kind) => {
            out.tag(2);
            super::types::resource(out, kind);
        }
        DropGlueIdentity::Structural(glue) => {
            out.tag(3);
            structural_glue(out, glue);
        }
    }
}

fn structural_glue(out: &mut Encoder, value: StructuralDropGlueIdentity) {
    match value {
        StructuralDropGlueIdentity::String { type_id, layout }
        | StructuralDropGlueIdentity::Path { type_id, layout }
        | StructuralDropGlueIdentity::Destination { type_id, layout } => {
            out.tag(match value {
                StructuralDropGlueIdentity::String { .. } => 0,
                StructuralDropGlueIdentity::Path { .. } => 1,
                _ => 4,
            });
            out.u64(type_id.raw());
            out.u64(layout.raw());
        }
        StructuralDropGlueIdentity::Product {
            type_id,
            product,
            layout,
        } => {
            out.tag(2);
            out.u64(type_id.raw());
            out.u64(product.raw());
            out.u64(layout.raw());
        }
        StructuralDropGlueIdentity::Enum {
            type_id,
            enum_id,
            layout,
            runtime_layout,
        } => {
            out.tag(3);
            out.u64(type_id.raw());
            out.fixed(&enum_id.bytes());
            out.u64(layout.raw());
            out.fixed(&runtime_layout.bytes());
        }
    }
}
