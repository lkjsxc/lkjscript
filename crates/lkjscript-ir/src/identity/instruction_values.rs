use super::{writer::Writer, IdentityError};
use crate::*;

type IdentityResult<T = ()> = std::result::Result<T, IdentityError>;

pub(super) fn constant(out: &mut Writer, value: &Constant) -> IdentityResult {
    match value {
        Constant::Unit => out.u8(0),
        Constant::Bool(value) => {
            out.u8(1)?;
            out.bool(*value)
        }
        Constant::I64(value) => {
            out.u8(2)?;
            out.i64(*value)
        }
        Constant::F64(value) => {
            out.u8(3)?;
            out.u64(value.to_bits())
        }
        Constant::Str(value) => text(out, 4, value),
        Constant::Symbol(value) => text(out, 5, value),
        Constant::EmptyList => out.u8(6),
    }
}

pub(super) fn call_target(out: &mut Writer, value: &CallTarget) -> IdentityResult {
    match value {
        CallTarget::Direct(function) => {
            out.u8(0)?;
            out.u32(function.raw())
        }
        CallTarget::Indirect(value) => {
            out.u8(1)?;
            out.u32(value.raw())
        }
    }
}

pub(super) fn place_value(
    out: &mut Writer,
    tag: u8,
    place: PlaceId,
    value: ValueId,
) -> IdentityResult {
    out.u8(tag)?;
    out.u32(place.raw())?;
    out.u32(value.raw())
}

pub(super) fn tagged_value(out: &mut Writer, tag: u8, value: ValueId) -> IdentityResult {
    out.u8(tag)?;
    out.u32(value.raw())
}

pub(super) fn product_field(
    out: &mut Writer,
    tag: u8,
    product: ProductId,
    field: u8,
    value: ValueId,
) -> IdentityResult {
    out.u8(tag)?;
    out.u16(product.raw())?;
    out.u8(field)?;
    out.u32(value.raw())
}

pub(super) fn enum_prefix(
    out: &mut Writer,
    tag: u8,
    enum_id: EnumId,
    variant: VariantId,
    layout: RuntimeLayoutId,
) -> IdentityResult {
    out.u8(tag)?;
    out.fixed(&enum_id.bytes())?;
    out.fixed(&variant.bytes())?;
    out.fixed(&layout.bytes())
}

fn text(out: &mut Writer, tag: u8, value: &str) -> IdentityResult {
    out.u8(tag)?;
    out.string(value)
}
