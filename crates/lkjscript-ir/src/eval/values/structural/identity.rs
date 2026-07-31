use std::num::NonZeroU64;

use lkjscript_core::{LayoutIdentity, SemanticTypeIdentity, StructuralKind, StructuralType};

use crate::{Program, SsaType};

pub(crate) fn structural_type(program: &Program, ty: &SsaType) -> Result<StructuralType, String> {
    let kind = match ty {
        SsaType::Unit => StructuralKind::Unit,
        SsaType::Bool => StructuralKind::Bool,
        SsaType::I64 => StructuralKind::I64,
        SsaType::F64 => StructuralKind::F64,
        SsaType::Str => StructuralKind::String,
        SsaType::Path => StructuralKind::Path,
        SsaType::Bytes => StructuralKind::Bytes,
        SsaType::ByteVector => StructuralKind::ByteVector,
        SsaType::Product(_) => StructuralKind::Product,
        SsaType::Enum { .. } => StructuralKind::Enum,
        SsaType::Symbol => StructuralKind::Static,
        _ => {
            return Err(format!(
                "type {ty:?} has no evaluator structural representation"
            ))
        }
    };
    let semantic = fingerprint(0x8f3f_73b5_cf1c_9ade, ty);
    let layout = match ty {
        SsaType::Enum { id, .. } => {
            let definition = program
                .enums
                .iter()
                .find(|definition| definition.id == *id)
                .ok_or_else(|| "evaluator structural enum metadata is missing".to_owned())?;
            fingerprint_bytes(0x9c2a_45d1_76e8_03bf, &definition.layout.identity.bytes())
        }
        SsaType::Product(id) => mix(fingerprint_tag(0xe55a_7341_0a0f_b861, 12), id.raw() as u64),
        _ => fingerprint(0x4d7c_51a9_284e_b603, ty),
    };
    Ok(StructuralType::new(
        LayoutIdentity::new(nonzero(layout)),
        SemanticTypeIdentity::new(nonzero(semantic)),
        kind,
    ))
}

fn fingerprint(mut state: u64, ty: &SsaType) -> u64 {
    state = fingerprint_tag(state, type_tag(ty));
    match ty {
        SsaType::Capability(kind) => mix(state, *kind as u64),
        SsaType::Resource(kind) => mix(state, *kind as u64),
        SsaType::StructuralDestination(id) => mix(state, id.raw() as u64),
        SsaType::Product(id) => mix(state, id.raw() as u64),
        SsaType::Enum { id, arguments } => {
            state = fingerprint_bytes(state, &id.bytes());
            for argument in arguments {
                state = fingerprint(state, argument);
            }
            state
        }
        SsaType::List(inner) => fingerprint(state, inner),
        SsaType::Function(signature) => {
            for parameter in &signature.parameters {
                state = fingerprint(state, parameter);
            }
            fingerprint(state, &signature.result)
        }
        SsaType::TypeParameter(name) => fingerprint_bytes(state, name.as_bytes()),
        _ => state,
    }
}

const fn type_tag(ty: &SsaType) -> u8 {
    match ty {
        SsaType::Unit => 1,
        SsaType::Bool => 2,
        SsaType::I64 => 3,
        SsaType::F64 => 4,
        SsaType::Str => 5,
        SsaType::Symbol => 6,
        SsaType::Bytes => 7,
        SsaType::ByteVector => 8,
        SsaType::ByteSlice => 9,
        SsaType::ByteSliceMut => 10,
        SsaType::Path => 11,
        SsaType::Capability(_) => 12,
        SsaType::Resource(_) => 13,
        SsaType::StructuralDestination(_) => 14,
        SsaType::Product(_) => 15,
        SsaType::Enum { .. } => 16,
        SsaType::List(_) => 17,
        SsaType::Function(_) => 18,
        SsaType::TypeParameter(_) => 19,
    }
}

const fn fingerprint_tag(state: u64, tag: u8) -> u64 {
    mix(state, tag as u64)
}

fn fingerprint_bytes(mut state: u64, bytes: &[u8]) -> u64 {
    for &byte in bytes {
        state = mix(state, u64::from(byte));
    }
    state
}

const fn mix(state: u64, value: u64) -> u64 {
    (state ^ value).wrapping_mul(0x0000_0100_0000_01b3)
}

const fn nonzero(value: u64) -> NonZeroU64 {
    match NonZeroU64::new(value) {
        Some(value) => value,
        None => NonZeroU64::MIN,
    }
}
