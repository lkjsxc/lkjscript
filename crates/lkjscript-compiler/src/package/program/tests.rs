#![allow(clippy::expect_used)]

use super::PreparationProvenance;
use lkjscript_contracts::PreparedProgramDescriptor;
use lkjscript_core::Limits;

#[test]
fn independent_prepared_reconstruction_rejects_self_consistent_descriptor_tampering() {
    let source = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n7\n/main\n";
    let program = crate::compile_source(source, "prepared-verifier.lkjscript", &Limits::default())
        .expect("compile prepared verifier fixture");
    let descriptor = program.prepared().descriptor();
    let provenance = provenance(descriptor);
    super::verifier::verify(
        descriptor,
        program.prepared_identity(),
        program.ssa(),
        program.bytecode(),
        program.memory_plan(),
        program.profile(),
        &provenance,
    )
    .expect("independent prepared reconstruction");

    let mut forged = descriptor;
    forged.semantic_ssa = [0x44; 32];
    assert!(super::verifier::verify(
        forged,
        forged.identity().expect("forged descriptor identity"),
        program.ssa(),
        program.bytecode(),
        program.memory_plan(),
        program.profile(),
        &provenance,
    )
    .is_err());
}

fn provenance(descriptor: PreparedProgramDescriptor) -> PreparationProvenance {
    PreparationProvenance {
        kind: descriptor.package_kind,
        package_content: descriptor.package_content,
        package_root: descriptor.package_root,
        entry: descriptor.entry,
        module_memory_closure: descriptor.module_memory_closure,
        witness_closure: descriptor.witness_closure,
    }
}
