mod model;

pub(crate) use model::PreparationProvenance;
pub use model::PreparedProgram;

use lkjscript_contracts::{
    PackageProvenanceKind, PreparedContractDigests, PreparedProgramDescriptor,
};
use lkjscript_core::{validated_bytecode_identity, Error, Result, ValidatedChunk};
use lkjscript_ir::{specialize_native_transport, verified_program_identity, VerifiedProgram};

use crate::HirMemoryPlan;

pub(crate) fn bind(
    ssa: VerifiedProgram,
    bytecode: ValidatedChunk,
    memory_plan: &HirMemoryPlan,
    provenance: PreparationProvenance,
) -> Result<(PreparedProgram, VerifiedProgram, ValidatedChunk)> {
    let semantic_ssa = verified_program_identity(&ssa)
        .map_err(|error| Error::msg(error.to_string()))?
        .bytes();
    let native_specialization_ssa = specialize_native_transport(&ssa)
        .ok()
        .map(|(native, _)| {
            verified_program_identity(&native)
                .map(|identity| identity.bytes())
                .map_err(|error| Error::msg(error.to_string()))
        })
        .transpose()?;
    let validated_bytecode = validated_bytecode_identity(&bytecode)?.bytes();
    let descriptor = PreparedProgramDescriptor {
        package_kind: provenance.kind,
        package_content: provenance.package_content,
        package_root: provenance.package_root,
        entry: provenance.entry,
        module_memory_closure: provenance.module_memory_closure,
        memory_plan: memory_plan.id.as_bytes(),
        witness_closure: provenance.witness_closure,
        semantic_ssa,
        native_specialization_ssa,
        validated_bytecode,
        contracts: contract_digests()?,
    };
    let identity = descriptor
        .identity()
        .map_err(|error| Error::msg(error.to_string()))?;
    let ssa = ssa
        .bind_prepared_identity(identity)
        .map_err(|error| Error::msg(error.to_string()))?;
    let bytecode = bytecode.bind_prepared_identity(identity)?;
    Ok((
        PreparedProgram {
            descriptor,
            identity,
        },
        ssa,
        bytecode,
    ))
}

pub(crate) fn development(
    source_identity: [u8; 32],
    path: &str,
    plan: &HirMemoryPlan,
) -> Result<PreparationProvenance> {
    let path_len = u64::try_from(path.len())
        .map_err(|_| Error::host("development package path length exceeds u64"))?;
    let source_capacity = 40_usize
        .checked_add(path.len())
        .ok_or_else(|| Error::host("development package source identity size overflow"))?;
    let mut source = Vec::new();
    source
        .try_reserve_exact(source_capacity)
        .map_err(|_| Error::host("development package source identity allocation failed"))?;
    source.extend_from_slice(&source_identity);
    source.extend_from_slice(&path_len.to_be_bytes());
    source.extend_from_slice(path.as_bytes());
    let content = domain_hash(b"lkjscript.development-package-content", &source)?;
    let mut witness = Vec::new();
    for group in &plan.witness_groups {
        let member_count = u64::try_from(group.members.len())
            .map_err(|_| Error::host("development witness member count exceeds u64"))?;
        let member_bytes = group
            .members
            .len()
            .checked_mul(72)
            .and_then(|bytes| bytes.checked_add(41))
            .ok_or_else(|| Error::host("development witness group size overflow"))?;
        witness
            .try_reserve(member_bytes)
            .map_err(|_| Error::host("development witness group allocation failed"))?;
        witness.extend_from_slice(&group.id.as_bytes());
        witness.push(u8::from(group.recursive));
        witness.extend_from_slice(&member_count.to_be_bytes());
        for member in &group.members {
            witness.extend_from_slice(&member.witness.as_bytes());
            witness.extend_from_slice(&member.ordinal.to_be_bytes());
            witness.extend_from_slice(&member.semantic_identity);
        }
    }
    for member in &plan.witnesses {
        let dependencies = lkjscript_contracts::canonical_executable_memory_witness_dependencies(
            &member.facts.dependencies,
        );
        let additional = 32_usize
            .checked_add(dependencies.len())
            .ok_or_else(|| Error::host("development witness dependency size overflow"))?;
        witness
            .try_reserve(additional)
            .map_err(|_| Error::host("development witness dependency allocation failed"))?;
        witness.extend_from_slice(&member.id.as_bytes());
        witness.extend_from_slice(&dependencies);
    }
    Ok(PreparationProvenance {
        kind: PackageProvenanceKind::Development,
        package_content: content,
        package_root: domain_hash(b"lkjscript.development-package-root", path.as_bytes())?,
        entry: domain_hash(b"lkjscript.development-package-entry", path.as_bytes())?,
        module_memory_closure: domain_hash(
            b"lkjscript.development-module-memory",
            &plan.id.as_bytes(),
        )?,
        witness_closure: domain_hash(b"lkjscript.development-witness-closure", &witness)?,
    })
}

pub(crate) fn locked(value: crate::package::PreparedPackageFacts) -> PreparationProvenance {
    PreparationProvenance {
        kind: PackageProvenanceKind::Locked,
        package_content: value.package_content,
        package_root: value.package_root,
        entry: value.entry,
        module_memory_closure: value.module_memory_closure,
        witness_closure: value.witness_closure,
    }
}

fn contract_digests() -> Result<PreparedContractDigests> {
    let contracts =
        lkjscript_contracts::current_contracts().map_err(|error| Error::msg(error.to_string()))?;
    let get = |name| {
        contracts
            .get(name)
            .map(|value| value.digest().as_bytes())
            .ok_or_else(|| Error::msg(format!("prepared contract is not registered: {name}")))
    };
    Ok(PreparedContractDigests {
        prepared_program: get(lkjscript_contracts::PREPARED_PROGRAM)?,
        runtime_calls: lkjscript_contracts::RUNTIME_CALLS_DIGEST.as_bytes(),
        native_layout: lkjscript_contracts::NATIVE_LAYOUT_DIGEST.as_bytes(),
        verified_ssa: lkjscript_contracts::VERIFIED_SSA_DIGEST.as_bytes(),
        bytecode: get(lkjscript_contracts::BYTECODE)?,
    })
}

fn domain_hash(domain: &[u8], value: &[u8]) -> Result<[u8; 32]> {
    let value_len = u64::try_from(value.len())
        .map_err(|_| Error::host("prepared identity value length exceeds u64"))?;
    let capacity = domain
        .len()
        .checked_add(8)
        .and_then(|length| length.checked_add(value.len()))
        .ok_or_else(|| Error::host("prepared identity byte count overflow"))?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(capacity)
        .map_err(|_| Error::host("prepared identity allocation failed"))?;
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(&value_len.to_be_bytes());
    bytes.extend_from_slice(value);
    Ok(lkjscript_contracts::sha256(&bytes))
}
