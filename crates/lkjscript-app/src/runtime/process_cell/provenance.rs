use lkjscript_core::StructuralSliceExt;
use lkjscript_runtime::process_cell_protocol::{
    expected_process_provenance, ProcessBootstrap, ProcessProgramProvenance,
};

pub(super) fn authenticated(
    bootstrap: &ProcessBootstrap,
    program: &lkjscript_compiler::ExecutableProgram,
) -> Result<ProcessProgramProvenance, String> {
    let descriptor = program.prepared().descriptor();
    if descriptor.package_content != bootstrap.package {
        return Err("worker prepared package identity mismatch".into());
    }
    let chunk = program.bytecode();
    let structural = chunk
        .main()
        .return_structural
        .and_then(|id| chunk.structural_representations().get_structural(id))
        .and_then(|representation| {
            chunk
                .structural_types()
                .get_structural(representation.type_id)
        });
    let (return_semantic, root_witness_group, root_witness_member) = match structural {
        Some(value_type) => {
            let witness = chunk
                .memory_witnesses()
                .iter()
                .find(|witness| witness.id == value_type.witness)
                .ok_or_else(|| "worker structural return witness is absent".to_string())?;
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
    let actual = ProcessProgramProvenance {
        platform_revision: bootstrap.platform_revision,
        contract: bootstrap.contract,
        application: bootstrap.application,
        incarnation: bootstrap.incarnation,
        package: bootstrap.package,
        entry: descriptor.entry,
        prepared: program.prepared_identity(),
        return_semantic,
        root_witness_group,
        root_witness_member,
    };
    if actual != expected_process_provenance(bootstrap) {
        return Err("worker prepared identity agreement mismatch".into());
    }
    Ok(actual)
}

fn scalar_return(chunk: &lkjscript_core::ValidatedChunk) -> ([u8; 32], [u8; 32], [u8; 32]) {
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
