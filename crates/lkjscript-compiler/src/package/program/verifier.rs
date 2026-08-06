use lkjscript_contracts::{
    PreparedContractDigests, PreparedProgramDescriptor, PreparedProgramIdentity,
};
use lkjscript_core::{validated_bytecode_identity, Error, Result, ValidatedChunk};
use lkjscript_ir::{specialize_native_transport, verified_program_identity, VerifiedProgram};

use super::PreparationProvenance;
use crate::HirMemoryPlan;

pub(super) fn verify(
    descriptor: PreparedProgramDescriptor,
    identity: PreparedProgramIdentity,
    ssa: &VerifiedProgram,
    bytecode: &ValidatedChunk,
    plan: &HirMemoryPlan,
    provenance: &PreparationProvenance,
) -> Result<()> {
    let semantic_ssa = verified_program_identity(ssa)
        .map_err(|error| Error::msg(error.to_string()))?
        .bytes();
    let native_lowerable_ssa = match specialize_native_transport(ssa) {
        Ok((native, _)) => verified_program_identity(&native)
            .map_err(|error| Error::msg(error.to_string()))?
            .bytes(),
        Err(_) => semantic_ssa,
    };
    let reconstructed = PreparedProgramDescriptor {
        platform_revision: lkjscript_contracts::PLATFORM_REVISION,
        package_kind: provenance.kind,
        package_content: provenance.package_content,
        package_root: provenance.package_root,
        entry: provenance.entry,
        module_memory_closure: provenance.module_memory_closure,
        memory_plan: plan.id.as_bytes(),
        witness_closure: provenance.witness_closure,
        semantic_ssa,
        native_lowerable_ssa,
        validated_bytecode: validated_bytecode_identity(bytecode)?.bytes(),
        contracts: reconstruct_contract_digests()?,
    };
    if reconstructed != descriptor {
        return Err(Error::msg(
            "prepared descriptor does not match independent reconstruction",
        ));
    }
    let reconstructed_identity = reconstructed
        .identity()
        .map_err(|error| Error::msg(error.to_string()))?;
    if reconstructed_identity != identity {
        return Err(Error::msg(
            "prepared identity does not match independent reconstruction",
        ));
    }
    ssa.require_prepared_identity(identity)
        .map_err(|error| Error::msg(error.to_string()))?;
    bytecode.require_prepared_identity(identity)?;
    Ok(())
}

fn reconstruct_contract_digests() -> Result<PreparedContractDigests> {
    let contracts =
        lkjscript_contracts::current_contracts().map_err(|error| Error::msg(error.to_string()))?;
    let lookup = |name| {
        contracts
            .get(name)
            .map(|value| value.digest().as_bytes())
            .ok_or_else(|| Error::msg(format!("prepared verifier contract is absent: {name}")))
    };
    let runtime_control = lookup(lkjscript_contracts::RUNTIME_CONTROL)?;
    let mut codec = Vec::from(b"lkjscript.process-outcome-codec".as_slice());
    codec.extend_from_slice(&runtime_control);
    codec.extend_from_slice(&lkjscript_contracts::STRUCTURAL_OWNERSHIP_DOMAINS_DIGEST.as_bytes());
    Ok(PreparedContractDigests {
        prepared_program: lookup(lkjscript_contracts::PREPARED_PROGRAM)?,
        runtime_calls: lkjscript_contracts::RUNTIME_CALLS_DIGEST.as_bytes(),
        native_layout: lkjscript_contracts::NATIVE_LAYOUT_DIGEST.as_bytes(),
        verified_ssa: lkjscript_contracts::VERIFIED_SSA_DIGEST.as_bytes(),
        bytecode: lookup(lkjscript_contracts::BYTECODE)?,
        runtime_control,
        process_outcome_codec: lkjscript_contracts::sha256(&codec),
    })
}
