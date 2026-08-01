use crate::codegen::*;

pub(super) fn install_memory_witnesses(
    chunk: &mut Chunk,
    program: &lkjscript_ir::Program,
    install_structural_routes: bool,
) -> Result<()> {
    for descriptor in &program.memory.witnesses {
        let value_kind = match &descriptor.ty {
            SsaType::Unit => lkjscript_core::MemoryWitnessValueKind::Unit,
            SsaType::Bool => lkjscript_core::MemoryWitnessValueKind::Bool,
            SsaType::I64 => lkjscript_core::MemoryWitnessValueKind::I64,
            SsaType::F64 => lkjscript_core::MemoryWitnessValueKind::F64,
            SsaType::List(_) => lkjscript_core::MemoryWitnessValueKind::List,
            _ if install_structural_routes => executable_structural_route(program, descriptor)
                .map(|representation| {
                    lkjscript_core::MemoryWitnessValueKind::Structural(
                        BytecodeStructuralRepresentationId::new(representation.raw()),
                    )
                })
                .unwrap_or(lkjscript_core::MemoryWitnessValueKind::Unsupported),
            _ => lkjscript_core::MemoryWitnessValueKind::Unsupported,
        };
        chunk
            .memory_witnesses
            .push(lkjscript_core::InstalledMemoryWitness {
                id: BytecodeMemoryWitnessId::new(descriptor.id.bytes()),
                facts: descriptor.facts.clone(),
                dependencies: descriptor
                    .dependencies
                    .iter()
                    .map(|id| BytecodeMemoryWitnessId::new(id.bytes()))
                    .collect(),
                value_kind,
            });
    }
    if !chunk.memory_witnesses.is_empty() {
        chunk.memory_plan = Some(BytecodeMemoryPlanId::new(program.memory.plan.bytes()));
    }
    Ok(())
}

fn executable_structural_route(
    program: &lkjscript_ir::Program,
    descriptor: &lkjscript_ir::MemoryWitnessDescriptor,
) -> Option<lkjscript_ir::StructuralRepresentationId> {
    let representation = descriptor.representation?;
    let route = program
        .memory
        .representations
        .get(representation.index()?)?;
    let ty = program.memory.types.get(route.type_id.index()?)?;
    (route.category == lkjscript_ir::StructuralValueCategory::Owner && ty.witness == descriptor.id)
        .then_some(representation)
}
