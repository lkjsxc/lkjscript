use super::{writer::Writer, IdentityError};
use crate::*;

type IdentityResult<T = ()> = std::result::Result<T, IdentityError>;

pub(super) fn origin(out: &mut Writer, value: Origin) -> IdentityResult {
    out.u32(value.source)?;
    out.u32(value.node)
}

pub(super) fn signature(out: &mut Writer, value: &Signature) -> IdentityResult {
    out.sequence(&value.type_parameters, |out, item| out.string(item))?;
    out.sequence(&value.bounds, trait_bound)?;
    out.sequence(&value.parameters, ssa_type)?;
    ssa_type(out, &value.result)
}

pub(super) fn ssa_type(out: &mut Writer, value: &SsaType) -> IdentityResult {
    match value {
        SsaType::Unit => out.u8(0),
        SsaType::Bool => out.u8(1),
        SsaType::I64 => out.u8(2),
        SsaType::F64 => out.u8(3),
        SsaType::Str => out.u8(4),
        SsaType::Symbol => out.u8(5),
        SsaType::Buf => out.u8(6),
        SsaType::Path => out.u8(7),
        SsaType::Capability(kind) => {
            out.u8(8)?;
            out.string(kind.as_str())
        }
        SsaType::Owned(inner) => nested(out, 9, inner),
        SsaType::Ref(inner) => nested(out, 10, inner),
        SsaType::RefMut(inner) => nested(out, 11, inner),
        SsaType::Handle => out.u8(12),
        SsaType::Product(id) => {
            out.u8(13)?;
            out.u16(id.raw())
        }
        SsaType::Enum { id, arguments } => {
            out.u8(14)?;
            out.fixed(&id.bytes())?;
            out.sequence(arguments, ssa_type)
        }
        SsaType::List(inner) => nested(out, 15, inner),
        SsaType::Function(signature_value) => {
            out.u8(16)?;
            signature(out, signature_value)
        }
        SsaType::TypeParameter(name) => {
            out.u8(17)?;
            out.string(name)
        }
    }
}

pub(super) fn instantiation(out: &mut Writer, value: &GenericInstantiation) -> IdentityResult {
    out.sequence(&value.substitutions, |out, item| {
        out.string(&item.parameter)?;
        ssa_type(out, &item.ty)
    })?;
    out.sequence(&value.witnesses, |out, item| {
        out.u32(item.trait_id.raw())?;
        ssa_type(out, &item.ty)?;
        match item.kind {
            TraitWitnessKind::AutoTrait => out.u8(0),
            TraitWitnessKind::Explicit(id) => {
                out.u8(1)?;
                out.u32(id.raw())
            }
        }
    })
}

pub(super) fn trait_role(out: &mut Writer, value: TraitRole) -> IdentityResult {
    out.u8(match value {
        TraitRole::Copy => 0,
        TraitRole::Clone => 1,
        TraitRole::Drop => 2,
        TraitRole::Send => 3,
        TraitRole::Sync => 4,
        TraitRole::User => 5,
    })
}

fn trait_bound(out: &mut Writer, value: &TraitBound) -> IdentityResult {
    out.string(&value.parameter)?;
    out.u32(value.trait_id.raw())
}

fn nested(out: &mut Writer, tag: u8, value: &SsaType) -> IdentityResult {
    out.u8(tag)?;
    ssa_type(out, value)
}
