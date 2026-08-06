use std::num::NonZeroU64;

use super::super::*;

pub fn runtime_structural_type(
    program: Option<&Program>,
    ty: &SsaType,
) -> Result<Option<lkjscript_core::StructuralType>> {
    let kind = match ty {
        SsaType::Unit => lkjscript_core::StructuralKind::Unit,
        SsaType::Bool => lkjscript_core::StructuralKind::Bool,
        SsaType::I64 => lkjscript_core::StructuralKind::I64,
        SsaType::F64 => lkjscript_core::StructuralKind::F64,
        SsaType::Str => lkjscript_core::StructuralKind::String,
        SsaType::Path => lkjscript_core::StructuralKind::Path,
        SsaType::Bytes => lkjscript_core::StructuralKind::Bytes,
        SsaType::ByteVector => lkjscript_core::StructuralKind::ByteVector,
        SsaType::Product(_) => lkjscript_core::StructuralKind::Product,
        SsaType::Enum { .. } => lkjscript_core::StructuralKind::Enum,
        SsaType::Symbol => lkjscript_core::StructuralKind::Static,
        SsaType::ByteSlice
        | SsaType::ByteSliceMut
        | SsaType::Capability(_)
        | SsaType::Resource(_)
        | SsaType::StructuralDestination(_)
        | SsaType::List(_)
        | SsaType::Function(_)
        | SsaType::TypeParameter(_) => return Ok(None),
    };
    let semantic = runtime_structural_semantic_type(program, ty)?;
    let layout = match ty {
        SsaType::Enum { id, .. } => {
            let program = program.ok_or_else(|| {
                IrError::new("enum runtime identity requires structural program metadata")
            })?;
            let definition = program
                .enums
                .iter()
                .find(|definition| definition.id == *id)
                .ok_or_else(|| IrError::new("structural enum runtime identity is missing"))?;
            fingerprint_bytes(0x9c2a_45d1_76e8_03bf, &definition.layout.identity.bytes())
        }
        SsaType::Product(id) => runtime_product_layout_identity(
            runtime_product_identity(
                program.ok_or_else(|| {
                    IrError::new("product runtime identity requires structural program metadata")
                })?,
                *id,
            )?
            .bytes(),
        )
        .get(),
        _ => fingerprint(0x4d7c_51a9_284e_b603, ty),
    };
    Ok(Some(lkjscript_core::StructuralType::new(
        lkjscript_core::LayoutIdentity::new(nonzero(layout)),
        semantic,
        kind,
    )))
}

pub fn runtime_structural_semantic_type(
    program: Option<&Program>,
    ty: &SsaType,
) -> Result<lkjscript_core::SemanticTypeIdentity> {
    if let SsaType::Product(id) = ty {
        let program = program.ok_or_else(|| {
            IrError::new("product semantic identity requires structural program metadata")
        })?;
        return Ok(runtime_product_semantic_type(
            runtime_product_identity(program, *id)?.bytes(),
        ));
    }
    Ok(lkjscript_core::SemanticTypeIdentity::new(nonzero(
        fingerprint(0x8f3f_73b5_cf1c_9ade, ty),
    )))
}

pub fn runtime_product_identity(program: &Program, id: ProductId) -> Result<RuntimeLayoutId> {
    let product = program
        .products
        .iter()
        .find(|product| product.id == id)
        .ok_or_else(|| IrError::new("product runtime identity metadata is missing"))?;
    runtime_product_contract_identity(program.memory.plan, &product.name)
}

pub fn runtime_product_contract_identity(
    plan: MemoryPlanId,
    name: &str,
) -> Result<RuntimeLayoutId> {
    lkjscript_core::runtime_product_contract_identity(
        lkjscript_core::MemoryPlanId::new(plan.bytes()),
        name,
    )
    .map(|identity| RuntimeLayoutId::new(identity.bytes()))
    .map_err(|error| IrError::new(error.to_string()))
}

pub fn runtime_product_semantic_type(identity: [u8; 32]) -> lkjscript_core::SemanticTypeIdentity {
    lkjscript_core::product_semantic_identity(lkjscript_core::RuntimeLayoutId::new(identity))
}

pub fn runtime_product_layout_identity(identity: [u8; 32]) -> NonZeroU64 {
    nonzero(
        lkjscript_core::product_layout_identity(lkjscript_core::RuntimeLayoutId::new(identity))
            .get(),
    )
}

fn fingerprint(mut state: u64, ty: &SsaType) -> u64 {
    state = fingerprint_tag(state, type_tag(ty));
    match ty {
        SsaType::Capability(kind) => mix(state, *kind as u64),
        SsaType::Resource(kind) => mix(state, *kind as u64),
        SsaType::StructuralDestination(id) => mix(state, id.raw()),
        SsaType::Product(id) => mix(state, id.raw()),
        SsaType::Enum { id, arguments } => {
            state = fingerprint_bytes(state, &id.bytes());
            arguments.iter().fold(state, fingerprint)
        }
        SsaType::List(inner) => fingerprint(state, inner),
        SsaType::Function(signature) => {
            state = signature.parameters.iter().fold(state, fingerprint);
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
