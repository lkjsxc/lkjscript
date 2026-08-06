use std::path::Path;
use std::sync::Arc;

use lkjscript_contracts::PreparedProgramIdentity;
use lkjscript_core::{CapabilityKind, StructuralSliceExt, ValidatedChunk};

use crate::{ApplicationManifest, PackageContentId, RuntimeError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct PreparedProcessIdentity {
    pub(crate) entry: [u8; 32],
    pub(crate) prepared: PreparedProgramIdentity,
    pub(crate) return_semantic: [u8; 32],
    pub(crate) root_witness_group: [u8; 32],
    pub(crate) root_witness_member: [u8; 32],
}

pub(crate) fn prepare_isolated(
    entry: &Path,
    package_root: &Path,
    package: PackageContentId,
    application: &ApplicationManifest,
) -> Result<(Arc<ValidatedChunk>, PreparedProcessIdentity), RuntimeError> {
    let (verified_root, package_manifest, content) =
        lkjscript_compiler::package::verify_content(entry)
            .map_err(|error| process_error(error.to_string()))?;
    if verified_root != package_root || content.as_bytes() != package.bytes() {
        return Err(process_error("parent package content identity mismatch"));
    }
    let program = lkjscript_compiler::compile_path(entry)
        .map_err(|error| process_error(error.to_string()))?;
    validate_grants(
        program.bytecode().required_capabilities(),
        &application.capabilities,
        &package_manifest.capabilities,
    )?;
    let identity = process_identity(&program).map_err(process_error)?;
    if program.prepared().descriptor().package_content != package.bytes() {
        return Err(process_error("parent prepared package identity mismatch"));
    }
    Ok((Arc::new(program.into_bytecode()), identity))
}

pub(crate) fn process_identity(
    program: &lkjscript_compiler::ExecutableProgram,
) -> Result<PreparedProcessIdentity, String> {
    let descriptor = program.prepared().descriptor();
    let chunk = program.bytecode();
    let structural = chunk
        .main()
        .return_structural
        .and_then(|id| chunk.structural_representations().get_structural(id))
        .and_then(|value| chunk.structural_types().get_structural(value.type_id));
    let (return_semantic, root_witness_group, root_witness_member) = match structural {
        Some(value_type) => {
            let witness = chunk
                .memory_witnesses()
                .iter()
                .find(|witness| witness.id == value_type.witness)
                .ok_or_else(|| "parent structural return witness is absent".to_string())?;
            let mut semantic = Vec::from(b"lkjscript.process-return-semantic".as_slice());
            semantic.extend_from_slice(&value_type.runtime_type.semantic_type.get().to_be_bytes());
            (
                lkjscript_contracts::sha256(&semantic),
                witness.group.bytes(),
                witness.id.bytes(),
            )
        }
        None => scalar_return(chunk),
    };
    Ok(PreparedProcessIdentity {
        entry: descriptor.entry,
        prepared: program.prepared_identity(),
        return_semantic,
        root_witness_group,
        root_witness_member,
    })
}

fn scalar_return(chunk: &ValidatedChunk) -> ([u8; 32], [u8; 32], [u8; 32]) {
    let mut value = Vec::from(b"lkjscript.process-scalar-return".as_slice());
    value.push(u8::from(chunk.main().return_copy_kind.is_some()));
    value.push(u8::from(chunk.main().return_unique.is_some()));
    value.push(u8::from(chunk.main().return_resource.is_some()));
    let semantic = lkjscript_contracts::sha256(&value);
    let mut group = Vec::from(b"lkjscript.process-scalar-root-group".as_slice());
    group.extend_from_slice(&semantic);
    let group = lkjscript_contracts::sha256(&group);
    let mut member = Vec::from(b"lkjscript.process-scalar-root-member".as_slice());
    member.extend_from_slice(&group);
    (semantic, group, lkjscript_contracts::sha256(&member))
}

fn validate_grants(
    required: &[CapabilityKind],
    granted: &[CapabilityKind],
    package: &[String],
) -> Result<(), RuntimeError> {
    for capability in required {
        if granted.binary_search(capability).is_err() {
            return Err(RuntimeError::CapabilityNotGranted(*capability));
        }
    }
    for capability in granted {
        if package
            .binary_search_by_key(&capability.as_str(), String::as_str)
            .is_err()
        {
            return Err(process_error(format!(
                "package lacks {} grant",
                capability.as_str()
            )));
        }
        if !matches!(
            capability,
            CapabilityKind::Arguments | CapabilityKind::Stdio | CapabilityKind::Clock
        ) {
            return Err(process_error("isolated worker capability is unsupported"));
        }
    }
    Ok(())
}

fn process_error(message: impl Into<String>) -> RuntimeError {
    RuntimeError::ProcessCell(message.into())
}
