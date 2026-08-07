use lkjscript_contracts::ContractDigest;
use lkjscript_core::{Error, Result};

use super::analysis::ModuleAnalysis;
use super::model::PackageMemoryInterface;

pub(super) struct ModuleInterface {
    pub(super) exports: Vec<String>,
    pub(super) records: Vec<PackageMemoryInterface>,
    pub(super) memory_digest: String,
    pub(super) interface_digest: String,
}

pub(super) fn build(
    id: &str,
    public: bool,
    contract: ContractDigest,
    analysis: &ModuleAnalysis,
) -> Result<ModuleInterface> {
    let file = analysis
        .source
        .files()
        .iter()
        .find(|file| file.origin.logical_path() == analysis.logical_id)
        .ok_or_else(|| Error::msg(format!("module source is absent: {id}")))?;
    let exports = if public {
        crate::source::module_public_names(file)
            .map_err(crate::source::SourceDiagnostic::into_core)?
            .into_iter()
            .collect()
    } else {
        Vec::new()
    };
    let mut records = Vec::with_capacity(exports.len());
    for name in &exports {
        records.push(export_record(
            id,
            &analysis.logical_id,
            name,
            contract,
            analysis,
        )?);
    }
    let memory_bytes = serde_json::to_vec(&records)
        .map_err(|error| Error::msg(format!("encode module memory interface: {error}")))?;
    let memory_digest = super::graph::framed_hash(
        b"lkjscript.module-memory-interface",
        &[&contract.as_bytes(), id.as_bytes(), &memory_bytes],
    )?;
    let export_bytes = serde_json::to_vec(&exports)
        .map_err(|error| Error::msg(format!("encode module exports: {error}")))?;
    let interface_digest = super::graph::framed_hash(
        b"lkjscript.module-interface",
        &[
            &contract.as_bytes(),
            id.as_bytes(),
            &export_bytes,
            memory_digest.as_bytes(),
        ],
    )?;
    Ok(ModuleInterface {
        exports,
        records,
        memory_digest,
        interface_digest,
    })
}

fn export_record(
    module: &str,
    logical_module: &str,
    name: &str,
    contract: ContractDigest,
    analysis: &ModuleAnalysis,
) -> Result<PackageMemoryInterface> {
    let (declaration_identity, declaration_kind) =
        super::interface_values::declaration(logical_module, name, analysis)?;
    let export_identity = super::graph::framed_hash(
        b"lkjscript.package-export",
        &[
            &contract.as_bytes(),
            module.as_bytes(),
            declaration_identity.as_bytes(),
            name.as_bytes(),
        ],
    )?;
    let internal = crate::source::module_names::internal_name(logical_module, name);
    let function = analysis.hir.functions.iter().find(|function| {
        analysis
            .hir
            .binding(function.binding)
            .is_some_and(|binding| binding.name == internal)
    });
    let fields = match function {
        Some(function) => super::interface_function::fields(function, analysis)?,
        None => super::interface_function::not_applicable(),
    };
    let contract_hex = contract.to_hex();
    let content = serde_json::to_vec(&(
        &declaration_identity,
        &export_identity,
        name,
        declaration_kind,
        &fields.types,
        &fields.traits,
        &fields.requirements,
        &fields.parameters,
        fields.result,
        &fields.equality,
        &fields.snapshot,
        &contract_hex,
    ))
    .map_err(|error| Error::msg(format!("encode package memory interface: {error}")))?;
    let digest = super::graph::framed_hash(
        b"lkjscript.package-memory-interface",
        &[&contract.as_bytes(), &content],
    )?;
    Ok(PackageMemoryInterface {
        declaration_identity,
        export_identity,
        name: name.into(),
        declaration_kind: declaration_kind.into(),
        type_parameters: fields.types,
        trait_parameters: fields.traits,
        memory_requirements: fields.requirements,
        parameter_modes: fields.parameters,
        result_mode: fields.result,
        equality_constraints: fields.equality,
        semantic_snapshot_constraints: fields.snapshot,
        module_interface_contract: contract_hex,
        package_memory_interface_sha256: digest,
    })
}
