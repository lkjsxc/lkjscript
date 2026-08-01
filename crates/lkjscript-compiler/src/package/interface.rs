use lkjscript_contracts::ContractDigest;
use lkjscript_core::{Error, Result};

use super::model::LockedWitnessRequirement;

pub(super) fn build(
    bytes: &[u8],
    id: &str,
    contract: ContractDigest,
) -> Result<(Vec<String>, Vec<LockedWitnessRequirement>, String)> {
    let source = std::str::from_utf8(bytes)
        .map_err(|error| Error::msg(format!("module {id} is not UTF-8: {error}")))?;
    let tree = crate::source::validate(source, id, &lkjscript_core::Limits::default())
        .map_err(|error| Error::msg(error.to_string()))?;
    let file = tree
        .files()
        .first()
        .ok_or_else(|| Error::msg(format!("module source is absent: {id}")))?;
    let exports: Vec<_> = crate::source::module_public_names(file)
        .map_err(|error| Error::msg(error.to_string()))?
        .into_iter()
        .collect();
    let requirements = crate::source::module_memory_witness_requirements(file)
        .map_err(|error| Error::msg(error.to_string()))?;
    let requirements = requirements
        .into_iter()
        .map(|requirement| locked_requirement(contract, requirement))
        .collect::<Result<Vec<_>>>()?;
    let mut records = Vec::new();
    for export in &exports {
        frame(&mut records, export.as_bytes())?;
    }
    for requirement in &requirements {
        frame(&mut records, requirement.digest.as_bytes())?;
    }
    let interface = super::graph::framed_hash(
        b"lkjscript.module-interface",
        &[&contract.as_bytes(), id.as_bytes(), &records],
    )?;
    Ok((exports, requirements, interface))
}

fn locked_requirement(
    contract: ContractDigest,
    requirement: crate::source::PublicMemoryWitnessRequirement,
) -> Result<LockedWitnessRequirement> {
    let operations = requirement.operations.join("\0");
    let digest = super::graph::framed_hash(
        b"lkjscript.module-memory-witness",
        &[
            &contract.as_bytes(),
            requirement.export.as_bytes(),
            requirement.parameter.as_bytes(),
            operations.as_bytes(),
        ],
    )?;
    Ok(LockedWitnessRequirement {
        export: requirement.export,
        parameter: requirement.parameter,
        operations: requirement.operations,
        digest,
    })
}

fn frame(output: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    let length =
        u64::try_from(value.len()).map_err(|_| Error::msg("module interface field overflow"))?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}
