use super::{functions, types, writer::Writer, IdentityError};
use crate::*;

type IdentityResult<T = ()> = std::result::Result<T, IdentityError>;

pub(super) fn program(out: &mut Writer, value: &Program) -> IdentityResult {
    out.sequence(&value.sources, source)?;
    out.sequence(&value.products, product)?;
    out.sequence(&value.enums, enum_metadata)?;
    out.sequence(&value.traits, trait_metadata)?;
    out.sequence(&value.implementations, implementation)?;
    out.sequence(&value.functions, functions::function)?;
    out.u32(value.main.raw())
}

fn source(out: &mut Writer, value: &SourceMetadata) -> IdentityResult {
    out.u32(value.id)?;
    out.string(&value.path)
}

fn product(out: &mut Writer, value: &ProductMetadata) -> IdentityResult {
    out.u16(value.id.raw())?;
    out.string(&value.name)?;
    out.sequence(&value.fields, |out, field| {
        out.string(&field.name)?;
        types::ssa_type(out, &field.ty)
    })
}

fn enum_metadata(out: &mut Writer, value: &EnumMetadata) -> IdentityResult {
    out.fixed(&value.id.bytes())?;
    out.string(&value.name)?;
    out.sequence(&value.type_parameters, |out, item| out.string(item))?;
    out.sequence(&value.variants, enum_variant)?;
    out.fixed(&value.layout.identity.bytes())?;
    out.bool(value.layout.recursive)
}

fn enum_variant(out: &mut Writer, value: &EnumVariantMetadata) -> IdentityResult {
    out.fixed(&value.id.bytes())?;
    out.string(&value.name)?;
    out.u16(value.physical_tag)?;
    out.sequence(&value.fields, |out, field| {
        out.fixed(&field.id.bytes())?;
        out.string(&field.name)?;
        types::ssa_type(out, &field.ty)?;
        out.bool(field.indirect)?;
        out.bool(field.traced)
    })
}

fn trait_metadata(out: &mut Writer, value: &TraitMetadata) -> IdentityResult {
    out.u32(value.id.raw())?;
    out.string(&value.name)?;
    types::trait_role(out, value.role)?;
    out.option(value.source.as_ref(), |out, source| out.u32(*source))
}

fn implementation(out: &mut Writer, value: &ImplMetadata) -> IdentityResult {
    out.u32(value.id.raw())?;
    out.u32(value.trait_id.raw())?;
    out.u16(value.product.raw())?;
    out.u32(value.source)
}
