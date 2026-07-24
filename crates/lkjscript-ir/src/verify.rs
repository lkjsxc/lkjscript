use std::collections::{HashMap, HashSet};

use crate::{
    Block, BlockId, CallTarget, EffectSet, FailureBehavior, Function, FunctionId,
    GenericInstantiation, ImplId, Instruction, InstructionKind, IrError, ProductId, Program,
    RuntimeOp, Safepoint, Signature, SsaType, Terminator, TraitId, TraitRole, TraitWitnessKind,
    ValueId,
};

#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedProgram(Program);

impl VerifiedProgram {
    pub fn program(&self) -> &Program {
        &self.0
    }

    pub fn into_program(self) -> Program {
        self.0
    }
}

pub fn verify(program: Program) -> crate::Result<VerifiedProgram> {
    verify_program(&program)?;
    Ok(VerifiedProgram(program))
}

pub const TRAIT_VERIFY_MAX_DEPTH: usize = 32;
pub const TRAIT_VERIFY_MAX_WORK: usize = 256;
const TYPE_VERIFY_MAX_DEPTH: usize = 64;
const TYPE_VERIFY_MAX_WORK: usize = 4_096;

fn verify_program(program: &Program) -> crate::Result<()> {
    if program.sources.iter().enumerate().any(|(index, source)| {
        source.id != u32::try_from(index).unwrap_or(u32::MAX) || source.path.is_empty()
    }) {
        return fail("SSA source metadata must have dense IDs and non-empty paths");
    }
    if program
        .products
        .iter()
        .enumerate()
        .any(|(index, product)| product.id.index() != Some(index) || product.name.is_empty())
    {
        return fail("SSA products must have dense IDs and non-empty names");
    }
    let mut product_names = HashSet::new();
    for product in &program.products {
        if !product_names.insert(product.name.as_str()) {
            return fail(format!("duplicate SSA product name {}", product.name));
        }
        let mut fields = HashSet::new();
        for field in &product.fields {
            if field.name.is_empty() || !fields.insert(field.name.as_str()) {
                return fail(format!(
                    "SSA product {} has an empty or duplicate field",
                    product.name
                ));
            }
            verify_type(program, &field.ty, &[])?;
        }
    }
    verify_trait_metadata(program)?;
    if program.functions.is_empty() {
        return fail("SSA program has no functions");
    }
    if program
        .main
        .index()
        .is_none_or(|index| index >= program.functions.len())
    {
        return fail("SSA program has an invalid main FunctionId");
    }
    let mut function_names = HashSet::new();
    for (index, function) in program.functions.iter().enumerate() {
        if function.id.index() != Some(index) {
            return fail("SSA functions must have dense IDs in storage order");
        }
        if function.name.is_empty() || !function_names.insert(function.name.as_str()) {
            return fail(format!(
                "SSA function {} has an empty or duplicate name",
                function.id.raw()
            ));
        }
        verify_function(program, function)?;
    }
    let main = function(program, program.main)?;
    if !main.signature.type_parameters.is_empty() || !main.signature.parameters.is_empty() {
        return fail("SSA main must be monomorphic and have no parameters");
    }
    Ok(())
}

fn verify_trait_metadata(program: &Program) -> crate::Result<()> {
    let core = [
        (TraitRole::Copy, "Copy"),
        (TraitRole::Clone, "Clone"),
        (TraitRole::Drop, "Drop"),
        (TraitRole::Send, "Send"),
        (TraitRole::Sync, "Sync"),
    ];
    if program.traits.len() < core.len() {
        return fail("SSA trait metadata is missing compiler-owned core traits");
    }
    let mut names = HashSet::new();
    for (index, trait_metadata) in program.traits.iter().enumerate() {
        if trait_metadata.id.index() != Some(index)
            || trait_metadata.name.is_empty()
            || !names.insert(trait_metadata.name.as_str())
        {
            return fail("SSA traits must have dense IDs and unique non-empty names");
        }
        if let Some((role, name)) = core.get(index) {
            if trait_metadata.role != *role
                || trait_metadata.name != *name
                || trait_metadata.source.is_some()
            {
                return fail("SSA compiler-owned trait identity is not canonical");
            }
        } else if trait_metadata.role != TraitRole::User
            || trait_metadata
                .source
                .is_none_or(|source| source as usize >= program.sources.len())
        {
            return fail("SSA source trait has invalid role or source identity");
        }
    }
    let mut coherent = HashSet::new();
    for (index, implementation) in program.implementations.iter().enumerate() {
        if implementation.id.index() != Some(index) {
            return fail("SSA implementations must have dense IDs");
        }
        let trait_metadata = trait_by_id(program, implementation.trait_id)?;
        if trait_metadata.role != TraitRole::User {
            return fail("SSA explicit implementation targets a compiler-owned core trait");
        }
        let _product = product_by_id(program, implementation.product)?;
        if implementation.source as usize >= program.sources.len() {
            return fail("SSA implementation has an invalid source identity");
        }
        if !coherent.insert((implementation.trait_id, implementation.product)) {
            return fail("SSA has overlapping marker implementations");
        }
    }
    Ok(())
}

fn verify_function(program: &Program, function: &Function) -> crate::Result<()> {
    let type_parameters: Vec<&str> = function
        .signature
        .type_parameters
        .iter()
        .map(String::as_str)
        .collect();
    let mut seen_type_parameters = HashSet::new();
    if type_parameters
        .iter()
        .any(|name| name.is_empty() || !seen_type_parameters.insert(*name))
    {
        return fail(format!(
            "SSA function {} has invalid type parameters",
            function.name
        ));
    }
    let mut seen_bounds = HashSet::new();
    for bound in &function.signature.bounds {
        if !type_parameters.contains(&bound.parameter.as_str()) {
            return fail(format!(
                "SSA function {} has a bound on undeclared parameter {}",
                function.name, bound.parameter
            ));
        }
        let trait_metadata = trait_by_id(program, bound.trait_id)?;
        if matches!(trait_metadata.role, TraitRole::Clone | TraitRole::Drop) {
            return fail(format!(
                "SSA function {} uses a core trait that requires unavailable methods",
                function.name
            ));
        }
        if !seen_bounds.insert((bound.parameter.as_str(), bound.trait_id)) {
            return fail(format!(
                "SSA function {} has duplicate trait bounds",
                function.name
            ));
        }
    }
    for ty in &function.signature.parameters {
        verify_type(program, ty, &type_parameters)?;
    }
    verify_type(program, &function.signature.result, &type_parameters)?;

    if function.blocks.is_empty() {
        return fail(format!("SSA function {} has no blocks", function.name));
    }
    if function
        .entry
        .index()
        .is_none_or(|index| index >= function.blocks.len())
    {
        return fail(format!(
            "SSA function {} has an invalid entry",
            function.name
        ));
    }
    let mut block_ids = HashSet::new();
    for block in &function.blocks {
        if block
            .id
            .index()
            .is_none_or(|index| index >= function.blocks.len())
            || !block_ids.insert(block.id)
        {
            return fail(format!(
                "SSA function {} has missing or duplicate BlockIds",
                function.name
            ));
        }
    }
    if block_ids.len() != function.blocks.len() {
        return fail(format!(
            "SSA function {} does not have dense BlockIds",
            function.name
        ));
    }

    let (types, definitions) = collect_values(program, function, &type_parameters)?;
    let entry = block(function, function.entry)?;
    if entry.parameters.len() != function.signature.parameters.len() {
        return fail(format!(
            "SSA function {} entry parameter arity does not match signature",
            function.name
        ));
    }
    for (parameter, expected) in entry.parameters.iter().zip(&function.signature.parameters) {
        if &parameter.ty != expected {
            return fail(format!(
                "SSA function {} entry parameter type mismatch",
                function.name
            ));
        }
    }

    let predecessors = predecessors(function)?;
    let dominators = dominators(function, &predecessors)?;
    let reachable = reachable(function)?;
    for block in &function.blocks {
        verify_block(
            program,
            function,
            block,
            &types,
            &definitions,
            &dominators,
            &type_parameters,
        )?;
    }
    verify_loops(function, &dominators, &reachable)?;
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct Definition {
    block: BlockId,
    instruction: Option<usize>,
}

fn collect_values(
    program: &Program,
    function: &Function,
    type_parameters: &[&str],
) -> crate::Result<(Vec<SsaType>, HashMap<ValueId, Definition>)> {
    let mut values: HashMap<ValueId, (SsaType, Definition)> = HashMap::new();
    for block in &function.blocks {
        for parameter in &block.parameters {
            verify_type(program, &parameter.ty, type_parameters)?;
            let definition = Definition {
                block: block.id,
                instruction: None,
            };
            if values
                .insert(parameter.id, (parameter.ty.clone(), definition))
                .is_some()
            {
                return fail(format!(
                    "SSA function {} has duplicate ValueId {}",
                    function.name,
                    parameter.id.raw()
                ));
            }
        }
        for (instruction_index, instruction) in block.instructions.iter().enumerate() {
            verify_type(program, &instruction.ty, type_parameters)?;
            let definition = Definition {
                block: block.id,
                instruction: Some(instruction_index),
            };
            if values
                .insert(instruction.id, (instruction.ty.clone(), definition))
                .is_some()
            {
                return fail(format!(
                    "SSA function {} has duplicate ValueId {}",
                    function.name,
                    instruction.id.raw()
                ));
            }
        }
    }
    let mut types = vec![SsaType::Unit; values.len()];
    let mut definitions = HashMap::with_capacity(values.len());
    for raw in 0..values.len() {
        let raw = u32::try_from(raw).map_err(|_| IrError::new("SSA ValueId count exceeds u32"))?;
        let id = ValueId::new(raw);
        let Some((ty, definition)) = values.remove(&id) else {
            return fail(format!(
                "SSA function {} has missing ValueId {}",
                function.name, raw
            ));
        };
        let Some(slot) = types.get_mut(usize::try_from(raw).unwrap_or(usize::MAX)) else {
            return fail("SSA ValueId indexing failed");
        };
        *slot = ty;
        definitions.insert(id, definition);
    }
    Ok((types, definitions))
}

fn verify_block(
    program: &Program,
    function: &Function,
    block: &Block,
    types: &[SsaType],
    definitions: &HashMap<ValueId, Definition>,
    dominators: &[HashSet<BlockId>],
    type_parameters: &[&str],
) -> crate::Result<()> {
    if let Some(frame) = &block.metadata.frame_state {
        verify_frame_state(
            function,
            block.id,
            None,
            frame,
            types,
            definitions,
            dominators,
        )?;
    }
    for (index, instruction) in block.instructions.iter().enumerate() {
        for operand in instruction.kind.operands() {
            verify_available(
                function,
                block.id,
                Some(index),
                operand,
                definitions,
                dominators,
            )?;
        }
        if let Some(frame) = &instruction.metadata.frame_state {
            verify_frame_state(
                function,
                block.id,
                Some(index),
                frame,
                types,
                definitions,
                dominators,
            )?;
        }
        verify_instruction(program, function, instruction, types, type_parameters)?;
    }
    for operand in block.terminator.operands() {
        verify_available(
            function,
            block.id,
            Some(block.instructions.len()),
            operand,
            definitions,
            dominators,
        )?;
    }
    verify_terminator(program, function, block, types)
}

fn verify_instruction(
    program: &Program,
    _function: &Function,
    instruction: &Instruction,
    types: &[SsaType],
    type_parameters: &[&str],
) -> crate::Result<()> {
    let expected_effects = match &instruction.kind {
        InstructionKind::Constant(constant) => {
            if !constant.ty(&instruction.ty) {
                return fail(format!(
                    "SSA value {} constant type mismatch",
                    instruction.id.raw()
                ));
            }
            EffectSet::PURE
        }
        InstructionKind::Copy(value) => {
            if value_type(types, *value)? != &instruction.ty {
                return fail(format!(
                    "SSA value {} copy type mismatch",
                    instruction.id.raw()
                ));
            }
            EffectSet::PURE
        }
        InstructionKind::FunctionRef(target) => {
            let callee = function_by_id(program, *target)?;
            if !callee.signature.bounds.is_empty() {
                return fail("SSA bounded generic function cannot be a first-class value");
            }
            if instruction.ty != SsaType::Function(Box::new(callee.signature.clone())) {
                return fail(format!(
                    "SSA value {} function-reference type mismatch",
                    instruction.id.raw()
                ));
            }
            EffectSet::PURE
        }
        InstructionKind::Runtime {
            operation,
            arguments,
            signature,
        } => {
            verify_resolved_signature(signature, arguments, &instruction.ty, types)?;
            verify_runtime_signature(*operation, signature)?;
            operation.effects()
        }
        InstructionKind::Call {
            target,
            arguments,
            signature,
            instantiation,
        } => {
            verify_resolved_signature(signature, arguments, &instruction.ty, types)?;
            match target {
                CallTarget::Direct(target) => {
                    let callee = function_by_id(program, *target)?;
                    verify_call_compatibility(
                        program,
                        &callee.signature,
                        signature,
                        instantiation.as_ref(),
                        type_parameters,
                    )?;
                    callee.effects
                }
                CallTarget::Indirect(target) => {
                    let target_ty = value_type(types, *target)?;
                    let SsaType::Function(target_signature) = target_ty else {
                        return fail(format!(
                            "SSA value {} has a non-function indirect call target",
                            instruction.id.raw()
                        ));
                    };
                    if !target_signature.bounds.is_empty() {
                        return fail("SSA indirect call target has unsupported marker bounds");
                    }
                    verify_call_compatibility(
                        program,
                        target_signature,
                        signature,
                        instantiation.as_ref(),
                        type_parameters,
                    )?;
                    EffectSet::CONSERVATIVE_CALL
                }
            }
        }
        InstructionKind::ProductValue { product, fields } => {
            let metadata = product_by_id(program, *product)?;
            if instruction.ty != SsaType::Product(*product) || fields.len() != metadata.fields.len()
            {
                return fail(format!(
                    "SSA value {} malformed product construction",
                    instruction.id.raw()
                ));
            }
            for (value, field) in fields.iter().zip(&metadata.fields) {
                if value_type(types, *value)? != &field.ty {
                    return fail(format!(
                        "SSA value {} product field type mismatch",
                        instruction.id.raw()
                    ));
                }
            }
            EffectSet::ALLOCATES
        }
        InstructionKind::ProductField {
            product,
            field,
            value,
        } => {
            let metadata = product_by_id(program, *product)?;
            let Some(field_metadata) = metadata.fields.get(usize::from(*field)) else {
                return fail("SSA product field index is out of range");
            };
            if value_type(types, *value)? != &SsaType::Product(*product)
                || instruction.ty != field_metadata.ty
            {
                return fail("SSA product field type or identity mismatch");
            }
            EffectSet::READS_MEMORY
        }
        InstructionKind::WithProductField {
            product,
            field,
            value,
            replacement,
        } => {
            let metadata = product_by_id(program, *product)?;
            let Some(field_metadata) = metadata.fields.get(usize::from(*field)) else {
                return fail("SSA replacement field index is out of range");
            };
            if value_type(types, *value)? != &SsaType::Product(*product)
                || value_type(types, *replacement)? != &field_metadata.ty
                || instruction.ty != SsaType::Product(*product)
            {
                return fail("SSA product replacement type or identity mismatch");
            }
            EffectSet::READS_MEMORY.union(EffectSet::ALLOCATES)
        }
    };
    if instruction.metadata.effects != expected_effects {
        return fail(format!(
            "SSA value {} has invalid effect metadata",
            instruction.id.raw()
        ));
    }
    let expected_safepoint = if matches!(instruction.kind, InstructionKind::Call { .. })
        || expected_effects.contains(EffectSet::ALLOCATES)
        || expected_effects.contains(EffectSet::HOST_IO)
    {
        Safepoint::Required
    } else {
        Safepoint::None
    };
    if instruction.metadata.safepoint != expected_safepoint
        || (expected_safepoint == Safepoint::Required && instruction.metadata.frame_state.is_none())
    {
        return fail(format!(
            "SSA value {} has invalid safepoint metadata",
            instruction.id.raw()
        ));
    }
    let expected_failure = failure_behavior(expected_effects);
    if instruction.metadata.failure != expected_failure {
        return fail(format!(
            "SSA value {} has invalid failure metadata",
            instruction.id.raw()
        ));
    }
    verify_type(program, &instruction.ty, type_parameters)
}

fn failure_behavior(effects: EffectSet) -> FailureBehavior {
    let trap = effects.contains(EffectSet::MAY_TRAP);
    let outcome = effects.contains(EffectSet::MAY_EXIT) || effects.contains(EffectSet::ALLOCATES);
    match (trap, outcome) {
        (false, false) => FailureBehavior::None,
        (true, false) => FailureBehavior::Trap,
        (false, true) => FailureBehavior::StructuredOutcome,
        (true, true) => FailureBehavior::TrapOrOutcome,
    }
}

fn verify_terminator(
    _program: &Program,
    function: &Function,
    block: &Block,
    types: &[SsaType],
) -> crate::Result<()> {
    match &block.terminator {
        Terminator::Branch { target, arguments } => {
            verify_edge(function, *target, arguments, types)?;
        }
        Terminator::ConditionalBranch {
            condition,
            true_target,
            true_arguments,
            false_target,
            false_arguments,
        } => {
            if value_type(types, *condition)? != &SsaType::Bool {
                return fail("SSA conditional branch condition is not Bool");
            }
            verify_edge(function, *true_target, true_arguments, types)?;
            verify_edge(function, *false_target, false_arguments, types)?;
        }
        Terminator::Return(value) => {
            if value_type(types, *value)? != function.signature.result.as_ref() {
                return fail(format!(
                    "SSA function {} returns the wrong type",
                    function.name
                ));
            }
        }
        Terminator::Trap { message } => {
            if message.is_empty() {
                return fail("SSA trap terminator has an empty diagnostic");
            }
        }
        Terminator::Exit { code } => {
            if value_type(types, *code)? != &SsaType::I64 {
                return fail("SSA exit terminator code is not I64");
            }
        }
        Terminator::Outcome { detail, .. } => {
            if let Some(detail) = detail {
                if value_type(types, *detail)? != &SsaType::Str {
                    return fail("SSA structured-outcome detail is not Str");
                }
            }
        }
    }
    Ok(())
}

fn verify_edge(
    function: &Function,
    target: BlockId,
    arguments: &[ValueId],
    types: &[SsaType],
) -> crate::Result<()> {
    let target = block(function, target)?;
    if target.parameters.len() != arguments.len() {
        return fail(format!(
            "SSA edge to block {} has {} arguments for {} parameters",
            target.id.raw(),
            arguments.len(),
            target.parameters.len()
        ));
    }
    for (argument, parameter) in arguments.iter().zip(&target.parameters) {
        if value_type(types, *argument)? != &parameter.ty {
            return fail(format!(
                "SSA edge to block {} has a block-argument type mismatch",
                target.id.raw()
            ));
        }
    }
    Ok(())
}

fn verify_frame_state(
    function: &Function,
    block: BlockId,
    instruction: Option<usize>,
    frame: &crate::FrameState,
    types: &[SsaType],
    definitions: &HashMap<ValueId, Definition>,
    dominators: &[HashSet<BlockId>],
) -> crate::Result<()> {
    let mut bindings = HashSet::new();
    let mut slots = HashSet::new();
    let mut previous_binding = None;
    for local in &frame.locals {
        if previous_binding.is_some_and(|previous| previous >= local.binding) {
            return fail("SSA frame locals are not in stable BindingId order");
        }
        previous_binding = Some(local.binding);
        if !bindings.insert(local.binding) || !slots.insert(local.slot) {
            return fail("SSA frame state has duplicate bindings or local slots");
        }
        let _ty = value_type(types, local.value)?;
        verify_available(
            function,
            block,
            instruction,
            local.value,
            definitions,
            dominators,
        )?;
    }
    for value in &frame.operand_stack {
        let _ty = value_type(types, *value)?;
        verify_available(
            function,
            block,
            instruction,
            *value,
            definitions,
            dominators,
        )?;
    }
    Ok(())
}

fn verify_available(
    function: &Function,
    use_block: BlockId,
    use_instruction: Option<usize>,
    value: ValueId,
    definitions: &HashMap<ValueId, Definition>,
    dominators: &[HashSet<BlockId>],
) -> crate::Result<()> {
    let Some(definition) = definitions.get(&value).copied() else {
        return fail(format!(
            "SSA function {} uses missing ValueId {}",
            function.name,
            value.raw()
        ));
    };
    if definition.block == use_block {
        match (definition.instruction, use_instruction) {
            (None, _) => Ok(()),
            (Some(definition), Some(usage)) if definition < usage => Ok(()),
            _ => fail(format!(
                "SSA function {} uses ValueId {} before definition",
                function.name,
                value.raw()
            )),
        }
    } else {
        let Some(use_dominators) = use_block.index().and_then(|index| dominators.get(index)) else {
            return fail("SSA dominance metadata is inconsistent");
        };
        if use_dominators.contains(&definition.block) {
            Ok(())
        } else {
            fail(format!(
                "SSA ValueId {} does not dominate its use in function {}",
                value.raw(),
                function.name
            ))
        }
    }
}

fn predecessors(function: &Function) -> crate::Result<Vec<Vec<BlockId>>> {
    let mut result = vec![Vec::new(); function.blocks.len()];
    for block in &function.blocks {
        for successor in successors(&block.terminator) {
            let Some(slot) = successor.index().and_then(|index| result.get_mut(index)) else {
                return fail("SSA terminator references a missing block");
            };
            slot.push(block.id);
        }
    }
    Ok(result)
}

fn reachable(function: &Function) -> crate::Result<HashSet<BlockId>> {
    let mut result = HashSet::new();
    let mut work = vec![function.entry];
    while let Some(current) = work.pop() {
        if !result.insert(current) {
            continue;
        }
        let current = block(function, current)?;
        work.extend(successors(&current.terminator));
    }
    Ok(result)
}

fn dominators(
    function: &Function,
    predecessors: &[Vec<BlockId>],
) -> crate::Result<Vec<HashSet<BlockId>>> {
    let all: HashSet<BlockId> = function.blocks.iter().map(|block| block.id).collect();
    let mut result = vec![all.clone(); function.blocks.len()];
    let Some(entry_index) = function.entry.index() else {
        return fail("SSA entry BlockId cannot be indexed");
    };
    let Some(entry) = result.get_mut(entry_index) else {
        return fail("SSA entry BlockId is missing");
    };
    *entry = HashSet::from([function.entry]);
    let mut changed = true;
    while changed {
        changed = false;
        for block in &function.blocks {
            if block.id == function.entry {
                continue;
            }
            let Some(preds) = block.id.index().and_then(|index| predecessors.get(index)) else {
                return fail("SSA predecessor metadata is inconsistent");
            };
            let mut next = if let Some(first) = preds.first() {
                result
                    .get(first.index().unwrap_or(usize::MAX))
                    .cloned()
                    .ok_or_else(|| IrError::new("SSA predecessor BlockId is invalid"))?
            } else {
                HashSet::new()
            };
            for predecessor in preds.iter().skip(1) {
                let Some(other) = result.get(predecessor.index().unwrap_or(usize::MAX)) else {
                    return fail("SSA predecessor BlockId is invalid");
                };
                next.retain(|candidate| other.contains(candidate));
            }
            next.insert(block.id);
            let Some(current) = block.id.index().and_then(|index| result.get_mut(index)) else {
                return fail("SSA dominator metadata is inconsistent");
            };
            if *current != next {
                *current = next;
                changed = true;
            }
        }
    }
    Ok(result)
}

fn verify_loops(
    function: &Function,
    dominators: &[HashSet<BlockId>],
    reachable: &HashSet<BlockId>,
) -> crate::Result<()> {
    let mut headers = HashSet::new();
    for block in &function.blocks {
        if !reachable.contains(&block.id) {
            continue;
        }
        for successor in successors(&block.terminator) {
            let Some(block_dominators) = block.id.index().and_then(|index| dominators.get(index))
            else {
                return fail("SSA loop dominance metadata is inconsistent");
            };
            if block_dominators.contains(&successor) {
                let target = block_by_id(function, successor)?;
                if !target.metadata.loop_header {
                    return fail(format!(
                        "SSA backedge targets unmarked loop header {}",
                        successor.raw()
                    ));
                }
                headers.insert(successor);
            }
        }
    }
    for block in &function.blocks {
        if block.metadata.loop_header && !headers.contains(&block.id) {
            return fail(format!(
                "SSA block {} is marked loop-header without a backedge",
                block.id.raw()
            ));
        }
        if block.metadata.loop_header && block.metadata.frame_state.is_none() {
            return fail(format!(
                "SSA loop header {} has no frame state",
                block.id.raw()
            ));
        }
    }
    Ok(())
}

fn verify_resolved_signature(
    signature: &Signature,
    arguments: &[ValueId],
    result: &SsaType,
    types: &[SsaType],
) -> crate::Result<()> {
    if !signature.type_parameters.is_empty()
        || !signature.bounds.is_empty()
        || signature.parameters.len() != arguments.len()
        || signature.result.as_ref() != result
    {
        return fail("SSA call has an unresolved or inconsistent signature");
    }
    for (argument, parameter) in arguments.iter().zip(&signature.parameters) {
        if value_type(types, *argument)? != parameter {
            return fail("SSA call argument type does not match resolved signature");
        }
    }
    Ok(())
}

fn verify_call_compatibility(
    program: &Program,
    declared: &Signature,
    resolved: &Signature,
    instantiation: Option<&GenericInstantiation>,
    caller_type_parameters: &[&str],
) -> crate::Result<()> {
    if declared.parameters.len() != resolved.parameters.len() {
        return fail("SSA call arity does not match callee");
    }
    let permitted: HashSet<&str> = declared
        .type_parameters
        .iter()
        .map(String::as_str)
        .collect();
    let mut substitutions: HashMap<&str, SsaType> = HashMap::new();
    for (declared, resolved) in declared.parameters.iter().zip(&resolved.parameters) {
        bind_type(declared, resolved, &permitted, &mut substitutions)?;
    }
    let expected_result = substitute_type(&declared.result, &substitutions);
    if expected_result != *resolved.result {
        return fail("SSA call result type does not match callee");
    }
    if declared.type_parameters.is_empty() {
        if instantiation.is_some() {
            return fail("SSA monomorphic call carries generic instantiation facts");
        }
        return Ok(());
    }
    let instantiation = instantiation
        .ok_or_else(|| IrError::new("SSA generic call is missing instantiation facts"))?;
    if instantiation.substitutions.len() != declared.type_parameters.len() {
        return fail("SSA generic call has a non-canonical substitution count");
    }
    for (parameter, fact) in declared
        .type_parameters
        .iter()
        .zip(&instantiation.substitutions)
    {
        if fact.parameter != *parameter || substitutions.get(parameter.as_str()) != Some(&fact.ty) {
            return fail("SSA generic call substitution identity does not match inference");
        }
        verify_type(program, &fact.ty, caller_type_parameters)?;
    }
    if instantiation.witnesses.len() != declared.bounds.len() {
        return fail("SSA generic call witness count does not match bounds");
    }
    let mut seen = HashSet::new();
    for (bound, witness) in declared.bounds.iter().zip(&instantiation.witnesses) {
        let expected_type = substitutions
            .get(bound.parameter.as_str())
            .ok_or_else(|| IrError::new("SSA trait bound parameter was not inferred"))?;
        if witness.trait_id != bound.trait_id || &witness.ty != expected_type {
            return fail("SSA trait witness type or trait does not match its bound");
        }
        if !seen.insert((witness.trait_id, witness.ty.clone())) {
            return fail("SSA generic call has duplicate trait witnesses");
        }
        verify_witness(program, witness)?;
    }
    Ok(())
}

fn verify_witness(program: &Program, witness: &crate::TraitWitness) -> crate::Result<()> {
    let trait_metadata = trait_by_id(program, witness.trait_id)?;
    match witness.kind {
        TraitWitnessKind::AutoTrait => {
            if !trait_metadata.role.is_auto() {
                return fail("SSA auto-trait witness references a non-auto trait");
            }
            let mut work = 0;
            let mut active = HashSet::new();
            if !auto_trait_holds(
                program,
                trait_metadata.role,
                &witness.ty,
                0,
                &mut work,
                &mut active,
            )? {
                return fail("SSA auto-trait witness asserts an unsupported type fact");
            }
        }
        TraitWitnessKind::Explicit(implementation_id) => {
            let implementation = impl_by_id(program, implementation_id)?;
            let SsaType::Product(product) = witness.ty else {
                return fail("SSA explicit marker witness does not target a product");
            };
            if implementation.trait_id != witness.trait_id || implementation.product != product {
                return fail(
                    "SSA explicit marker witness identity does not match trait and product",
                );
            }
        }
    }
    Ok(())
}

fn auto_trait_holds(
    program: &Program,
    role: TraitRole,
    ty: &SsaType,
    depth: usize,
    work: &mut usize,
    active: &mut HashSet<ProductId>,
) -> crate::Result<bool> {
    if depth > TRAIT_VERIFY_MAX_DEPTH {
        return fail(format!(
            "SSA auto-trait verification depth exceeded {TRAIT_VERIFY_MAX_DEPTH}"
        ));
    }
    *work = work
        .checked_add(1)
        .ok_or_else(|| IrError::new("SSA auto-trait work overflow"))?;
    if *work > TRAIT_VERIFY_MAX_WORK {
        return fail(format!(
            "SSA auto-trait verification work exceeded {TRAIT_VERIFY_MAX_WORK}"
        ));
    }
    match role {
        TraitRole::Copy => match ty {
            SsaType::Unit
            | SsaType::Bool
            | SsaType::I64
            | SsaType::F64
            | SsaType::Str
            | SsaType::Symbol => Ok(true),
            SsaType::Buf | SsaType::Handle | SsaType::Function(_) | SsaType::TypeParameter(_) => {
                Ok(false)
            }
            SsaType::List(inner) | SsaType::Option(inner) => {
                auto_trait_holds(program, role, inner, depth + 1, work, active)
            }
            SsaType::Result(ok, error) => {
                Ok(
                    auto_trait_holds(program, role, ok, depth + 1, work, active)?
                        && auto_trait_holds(program, role, error, depth + 1, work, active)?,
                )
            }
            SsaType::Product(product) => {
                if !active.insert(*product) {
                    return fail(format!(
                        "SSA auto-trait verification encountered product cycle at {}",
                        product.raw()
                    ));
                }
                let metadata = product_by_id(program, *product)?;
                let mut result = true;
                for field in &metadata.fields {
                    if !auto_trait_holds(program, role, &field.ty, depth + 1, work, active)? {
                        result = false;
                        break;
                    }
                }
                active.remove(product);
                Ok(result)
            }
        },
        TraitRole::Send | TraitRole::Sync => Ok(matches!(
            ty,
            SsaType::Unit | SsaType::Bool | SsaType::I64 | SsaType::F64
        )),
        TraitRole::Clone | TraitRole::Drop | TraitRole::User => Ok(false),
    }
}

fn bind_type<'a>(
    declared: &'a SsaType,
    resolved: &SsaType,
    permitted: &HashSet<&'a str>,
    substitutions: &mut HashMap<&'a str, SsaType>,
) -> crate::Result<()> {
    match (declared, resolved) {
        (SsaType::TypeParameter(name), resolved) if permitted.contains(name.as_str()) => {
            if let Some(previous) = substitutions.get(name.as_str()) {
                if previous != resolved {
                    return fail("SSA generic call has conflicting type substitutions");
                }
            } else {
                substitutions.insert(name, resolved.clone());
            }
            Ok(())
        }
        (SsaType::List(left), SsaType::List(right))
        | (SsaType::Option(left), SsaType::Option(right)) => {
            bind_type(left, right, permitted, substitutions)
        }
        (SsaType::Result(left_ok, left_err), SsaType::Result(right_ok, right_err)) => {
            bind_type(left_ok, right_ok, permitted, substitutions)?;
            bind_type(left_err, right_err, permitted, substitutions)
        }
        (SsaType::Function(left), SsaType::Function(right)) => {
            if left.type_parameters != right.type_parameters
                || left.bounds != right.bounds
                || left.parameters.len() != right.parameters.len()
            {
                return fail("SSA generic function type identity or arity mismatch");
            }
            let nested_permitted: HashSet<&str> = permitted
                .iter()
                .copied()
                .filter(|name| !left.type_parameters.iter().any(|nested| nested == name))
                .collect();
            for (left, right) in left.parameters.iter().zip(&right.parameters) {
                bind_type(left, right, &nested_permitted, substitutions)?;
            }
            bind_type(
                &left.result,
                &right.result,
                &nested_permitted,
                substitutions,
            )
        }
        (left, right) if left == right => Ok(()),
        _ => fail("SSA resolved call type is incompatible with declaration"),
    }
}

fn substitute_type(ty: &SsaType, substitutions: &HashMap<&str, SsaType>) -> SsaType {
    match ty {
        SsaType::TypeParameter(name) => substitutions
            .get(name.as_str())
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        SsaType::List(item) => SsaType::List(Box::new(substitute_type(item, substitutions))),
        SsaType::Option(item) => SsaType::Option(Box::new(substitute_type(item, substitutions))),
        SsaType::Result(ok, err) => SsaType::Result(
            Box::new(substitute_type(ok, substitutions)),
            Box::new(substitute_type(err, substitutions)),
        ),
        SsaType::Function(signature) => {
            let nested_substitutions: HashMap<&str, SsaType> = substitutions
                .iter()
                .filter(|(name, _)| {
                    !signature
                        .type_parameters
                        .iter()
                        .any(|nested| nested == **name)
                })
                .map(|(name, ty)| (*name, ty.clone()))
                .collect();
            SsaType::Function(Box::new(Signature {
                type_parameters: signature.type_parameters.clone(),
                bounds: signature.bounds.clone(),
                parameters: signature
                    .parameters
                    .iter()
                    .map(|ty| substitute_type(ty, &nested_substitutions))
                    .collect(),
                result: Box::new(substitute_type(&signature.result, &nested_substitutions)),
            }))
        }
        _ => ty.clone(),
    }
}

fn verify_runtime_signature(operation: RuntimeOp, signature: &Signature) -> crate::Result<()> {
    let parameters = &signature.parameters;
    let result = signature.result.as_ref();
    let exact = |expected: &[SsaType], result_type: &SsaType| {
        parameters == expected && result == result_type
    };
    let valid = match operation {
        RuntimeOp::Add | RuntimeOp::Subtract | RuntimeOp::Multiply | RuntimeOp::Divide => {
            parameters.len() == 2
                && parameters.iter().all(is_numeric)
                && result
                    == if parameters.iter().any(|ty| ty == &SsaType::F64) {
                        &SsaType::F64
                    } else {
                        &SsaType::I64
                    }
        }
        RuntimeOp::EqualValue => {
            parameters.len() == 2
                && parameters[0] == parameters[1]
                && supports_value_equality(&parameters[0])
                && result == &SsaType::Bool
        }
        RuntimeOp::SameObject => {
            parameters.len() == 2
                && parameters[0] == parameters[1]
                && matches!(parameters[0], SsaType::Buf | SsaType::Handle)
                && result == &SsaType::Bool
        }
        RuntimeOp::ListEqual => {
            parameters.len() == 2
                && parameters[0] == parameters[1]
                && matches!(&parameters[0], SsaType::List(item) if supports_value_equality(item))
                && result == &SsaType::Bool
        }
        RuntimeOp::F64BitsEqual => exact(&[SsaType::F64, SsaType::F64], &SsaType::Bool),
        RuntimeOp::Less | RuntimeOp::LessEqual | RuntimeOp::Greater | RuntimeOp::GreaterEqual => {
            parameters.len() == 2 && parameters.iter().all(is_numeric) && result == &SsaType::Bool
        }
        RuntimeOp::Not => exact(&[SsaType::Bool], &SsaType::Bool),
        RuntimeOp::BitAnd | RuntimeOp::BitOr | RuntimeOp::BitXor => {
            exact(&[SsaType::I64, SsaType::I64], &SsaType::I64)
        }
        RuntimeOp::Cons => {
            matches!(parameters.as_slice(), [item, SsaType::List(tail)] if item == tail.as_ref())
                && result == &parameters[1]
        }
        RuntimeOp::Car => {
            matches!(parameters.as_slice(), [SsaType::List(item)] if item.as_ref() == result)
        }
        RuntimeOp::Cdr => {
            matches!(parameters.as_slice(), [SsaType::List(_)]) && result == &parameters[0]
        }
        RuntimeOp::IsEmptyList => {
            matches!(parameters.as_slice(), [SsaType::List(_)]) && result == &SsaType::Bool
        }
        RuntimeOp::Print | RuntimeOp::WriteStr => exact(&[SsaType::Str], &SsaType::Unit),
        RuntimeOp::Flush => exact(&[], &SsaType::Unit),
        RuntimeOp::ReadByte => exact(&[], &SsaType::I64),
        RuntimeOp::WriteByte => exact(&[SsaType::I64], &SsaType::Unit),
        RuntimeOp::EmptyStr => exact(&[], &SsaType::Str),
        RuntimeOp::ArgCount => exact(&[], &SsaType::I64),
        RuntimeOp::Arg => exact(&[SsaType::I64], &SsaType::Option(Box::new(SsaType::Str))),
        RuntimeOp::BufNew => exact(&[SsaType::I64], &SsaType::Buf),
        RuntimeOp::BufLen => exact(&[SsaType::Buf], &SsaType::I64),
        RuntimeOp::BufRef | RuntimeOp::BufGetU32 => {
            exact(&[SsaType::Buf, SsaType::I64], &SsaType::I64)
        }
        RuntimeOp::BufSet | RuntimeOp::BufSetU32 => {
            exact(&[SsaType::Buf, SsaType::I64, SsaType::I64], &SsaType::Unit)
        }
        RuntimeOp::BufClone | RuntimeOp::BufFromStr => match operation {
            RuntimeOp::BufClone => exact(&[SsaType::Buf], &SsaType::Buf),
            RuntimeOp::BufFromStr => exact(&[SsaType::Str], &SsaType::Buf),
            _ => false,
        },
        RuntimeOp::BufToStr => exact(&[SsaType::Buf], &system_result(SsaType::Str)),
        RuntimeOp::BufSlice => exact(
            &[SsaType::Buf, SsaType::I64, SsaType::I64],
            &system_result(SsaType::Buf),
        ),
        RuntimeOp::StrLen => exact(&[SsaType::Str], &SsaType::I64),
        RuntimeOp::StrRef => exact(&[SsaType::Str, SsaType::I64], &SsaType::I64),
        RuntimeOp::StrAppend => exact(&[SsaType::Str, SsaType::Str], &SsaType::Str),
        RuntimeOp::StrSlice => exact(&[SsaType::Str, SsaType::I64, SsaType::I64], &SsaType::Str),
        RuntimeOp::StrFromByte | RuntimeOp::StrFromI64 => exact(&[SsaType::I64], &SsaType::Str),
        RuntimeOp::StrFromF64 => exact(&[SsaType::F64], &SsaType::Str),
        RuntimeOp::StdinHandle => exact(&[], &SsaType::Handle),
        RuntimeOp::SysIsatty => exact(&[SsaType::Handle], &system_result(SsaType::Bool)),
        RuntimeOp::SysClose => exact(&[SsaType::Handle], &system_result(SsaType::Unit)),
        RuntimeOp::SysReadByte => exact(&[SsaType::Handle], &system_result(SsaType::I64)),
        RuntimeOp::SysWriteByte => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysReadInto | RuntimeOp::SysWriteFrom => exact(
            &[SsaType::Handle, SsaType::Buf, SsaType::I64, SsaType::I64],
            &system_result(SsaType::I64),
        ),
        RuntimeOp::SysTtyGuardSave => exact(&[SsaType::Buf], &system_result(SsaType::Unit)),
        RuntimeOp::SysTtyGuardClear => exact(&[], &system_result(SsaType::Unit)),
        RuntimeOp::SysOpenRead
        | RuntimeOp::SysOpenWrite
        | RuntimeOp::SysOpenAppend
        | RuntimeOp::SysOpenCreateNew
        | RuntimeOp::SysOpenDir => exact(&[SsaType::Str], &system_result(SsaType::Handle)),
        RuntimeOp::SysFsync => exact(&[SsaType::Handle], &system_result(SsaType::Unit)),
        RuntimeOp::SysTruncate => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysRename => exact(&[SsaType::Str, SsaType::Str], &system_result(SsaType::Unit)),
        RuntimeOp::SysRandomFill => exact(
            &[SsaType::Buf, SsaType::I64, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSha256 => exact(
            &[SsaType::Buf, SsaType::I64, SsaType::I64],
            &system_result(SsaType::Buf),
        ),
        RuntimeOp::SysSqliteOpen => exact(
            &[SsaType::Str, SsaType::I64],
            &system_result(SsaType::Handle),
        ),
        RuntimeOp::SysSqliteClose
        | RuntimeOp::SysSqliteFinalize
        | RuntimeOp::SysSqliteReset
        | RuntimeOp::SysSqliteClearBindings => {
            exact(&[SsaType::Handle], &system_result(SsaType::Unit))
        }
        RuntimeOp::SysSqliteBusyTimeout | RuntimeOp::SysSqliteBindNull => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSqliteExec => exact(
            &[SsaType::Handle, SsaType::Str],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSqlitePrepare => exact(
            &[SsaType::Handle, SsaType::Str],
            &system_result(SsaType::Handle),
        ),
        RuntimeOp::SysSqliteBindI64 => exact(
            &[SsaType::Handle, SsaType::I64, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSqliteBindF64 => exact(
            &[SsaType::Handle, SsaType::I64, SsaType::F64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSqliteBindText => exact(
            &[SsaType::Handle, SsaType::I64, SsaType::Str],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSqliteBindBytes => exact(
            &[SsaType::Handle, SsaType::I64, SsaType::Buf],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysSqliteStep
        | RuntimeOp::SysSqliteColumnCount
        | RuntimeOp::SysSqliteChanges
        | RuntimeOp::SysSqliteLastInsertRowid
        | RuntimeOp::SysSqliteExtendedResultCode => {
            exact(&[SsaType::Handle], &system_result(SsaType::I64))
        }
        RuntimeOp::SysSqliteColumnType => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::I64),
        ),
        RuntimeOp::SysSqliteColumnI64 => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::Option(Box::new(SsaType::I64))),
        ),
        RuntimeOp::SysSqliteColumnF64 => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::Option(Box::new(SsaType::F64))),
        ),
        RuntimeOp::SysSqliteColumnText => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::Option(Box::new(SsaType::Str))),
        ),
        RuntimeOp::SysSqliteColumnBytes => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::Option(Box::new(SsaType::Buf))),
        ),
        RuntimeOp::SysSqliteBackup => exact(
            &[SsaType::Handle, SsaType::Str, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysPathExists => exact(&[SsaType::Str], &system_result(SsaType::Bool)),
        RuntimeOp::SysWaitMs => exact(&[SsaType::I64], &system_result(SsaType::Unit)),
        RuntimeOp::SysNowMs => exact(&[], &system_result(SsaType::I64)),
        RuntimeOp::SysSocket => exact(&[], &system_result(SsaType::Handle)),
        RuntimeOp::SysBind | RuntimeOp::SysListen => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::SysAccept => exact(&[SsaType::Handle], &system_result(SsaType::Handle)),
        RuntimeOp::SysRecv => exact(&[SsaType::Handle], &system_result(SsaType::Str)),
        RuntimeOp::SysSend => exact(
            &[SsaType::Handle, SsaType::Str],
            &system_result(SsaType::I64),
        ),
        RuntimeOp::SysPoll => exact(
            &[SsaType::Handle, SsaType::I64],
            &system_result(SsaType::I64),
        ),
        RuntimeOp::SysTtyGet | RuntimeOp::SysTtySet => exact(
            &[SsaType::Handle, SsaType::Buf],
            &system_result(SsaType::Unit),
        ),
        RuntimeOp::Ok => {
            matches!((parameters.as_slice(), result), ([value], SsaType::Result(ok, _)) if value == ok.as_ref())
        }
        RuntimeOp::Err => {
            matches!((parameters.as_slice(), result), ([value], SsaType::Result(_, err)) if value == err.as_ref())
        }
        RuntimeOp::IsOk => {
            matches!(parameters.as_slice(), [SsaType::Result(_, _)]) && result == &SsaType::Bool
        }
        RuntimeOp::UnwrapOk => {
            matches!(parameters.as_slice(), [SsaType::Result(ok, _)] if ok.as_ref() == result)
        }
        RuntimeOp::UnwrapErr => {
            matches!(parameters.as_slice(), [SsaType::Result(_, err)] if err.as_ref() == result)
        }
        RuntimeOp::Some => {
            matches!((parameters.as_slice(), result), ([value], SsaType::Option(item)) if value == item.as_ref())
        }
        RuntimeOp::IsSome => {
            matches!(parameters.as_slice(), [SsaType::Option(_)]) && result == &SsaType::Bool
        }
        RuntimeOp::UnwrapSome => {
            matches!(parameters.as_slice(), [SsaType::Option(item)] if item.as_ref() == result)
        }
    };
    if valid {
        Ok(())
    } else {
        fail(format!(
            "SSA runtime operation {operation:?} has an impossible signature"
        ))
    }
}

fn system_result(success: SsaType) -> SsaType {
    SsaType::Result(Box::new(success), Box::new(SsaType::Str))
}

fn supports_value_equality(ty: &SsaType) -> bool {
    match ty {
        SsaType::Unit
        | SsaType::Bool
        | SsaType::I64
        | SsaType::F64
        | SsaType::Str
        | SsaType::Symbol => true,
        SsaType::Option(item) => supports_value_equality(item),
        SsaType::Result(ok, err) => supports_value_equality(ok) && supports_value_equality(err),
        _ => false,
    }
}

fn is_numeric(ty: &SsaType) -> bool {
    matches!(ty, SsaType::I64 | SsaType::F64)
}

fn verify_type(program: &Program, ty: &SsaType, type_parameters: &[&str]) -> crate::Result<()> {
    let mut work = 0;
    verify_type_at(program, ty, type_parameters, 0, &mut work)
}

fn verify_type_at(
    program: &Program,
    ty: &SsaType,
    type_parameters: &[&str],
    depth: usize,
    work: &mut usize,
) -> crate::Result<()> {
    if depth > TYPE_VERIFY_MAX_DEPTH {
        return fail(format!("SSA type nesting exceeds {TYPE_VERIFY_MAX_DEPTH}"));
    }
    *work = work
        .checked_add(1)
        .ok_or_else(|| IrError::new("SSA type verification work overflow"))?;
    if *work > TYPE_VERIFY_MAX_WORK {
        return fail(format!(
            "SSA type verification work exceeds {TYPE_VERIFY_MAX_WORK}"
        ));
    }
    match ty {
        SsaType::Product(product) => {
            let _metadata = product_by_id(program, *product)?;
            Ok(())
        }
        SsaType::List(item) | SsaType::Option(item) => {
            verify_type_at(program, item, type_parameters, depth + 1, work)
        }
        SsaType::Result(ok, err) => {
            verify_type_at(program, ok, type_parameters, depth + 1, work)?;
            verify_type_at(program, err, type_parameters, depth + 1, work)
        }
        SsaType::Function(signature) => {
            let mut names = HashSet::new();
            if signature
                .type_parameters
                .iter()
                .any(|name| name.is_empty() || !names.insert(name.as_str()))
            {
                return fail("SSA function type has invalid type parameters");
            }
            let nested_parameters: Vec<&str> = signature
                .type_parameters
                .iter()
                .map(String::as_str)
                .collect();
            let mut nested_scope: Vec<&str> = type_parameters
                .iter()
                .copied()
                .filter(|outer| !nested_parameters.contains(outer))
                .collect();
            nested_scope.extend(nested_parameters.iter().copied());
            let mut bounds = HashSet::new();
            for bound in &signature.bounds {
                if !nested_parameters.contains(&bound.parameter.as_str())
                    || !bounds.insert((bound.parameter.as_str(), bound.trait_id))
                {
                    return fail("SSA function type has malformed trait bounds");
                }
                let trait_metadata = trait_by_id(program, bound.trait_id)?;
                if matches!(trait_metadata.role, TraitRole::Clone | TraitRole::Drop) {
                    return fail("SSA function type uses an unavailable core trait bound");
                }
            }
            for parameter in &signature.parameters {
                verify_type_at(program, parameter, &nested_scope, depth + 1, work)?;
            }
            verify_type_at(program, &signature.result, &nested_scope, depth + 1, work)
        }
        SsaType::TypeParameter(name) => {
            if type_parameters.contains(&name.as_str()) {
                Ok(())
            } else {
                fail(format!("SSA has unbound type parameter {name}"))
            }
        }
        _ => Ok(()),
    }
}

fn successors(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Branch { target, .. } => vec![*target],
        Terminator::ConditionalBranch {
            true_target,
            false_target,
            ..
        } => vec![*true_target, *false_target],
        _ => Vec::new(),
    }
}

fn function(program: &Program, id: FunctionId) -> crate::Result<&Function> {
    function_by_id(program, id)
}

fn function_by_id(program: &Program, id: FunctionId) -> crate::Result<&Function> {
    id.index()
        .and_then(|index| program.functions.get(index))
        .filter(|function| function.id == id)
        .ok_or_else(|| IrError::new(format!("missing SSA FunctionId {}", id.raw())))
}

fn trait_by_id(program: &Program, id: TraitId) -> crate::Result<&crate::TraitMetadata> {
    id.index()
        .and_then(|index| program.traits.get(index))
        .filter(|trait_metadata| trait_metadata.id == id)
        .ok_or_else(|| IrError::new(format!("missing SSA TraitId {}", id.raw())))
}

fn impl_by_id(program: &Program, id: ImplId) -> crate::Result<&crate::ImplMetadata> {
    id.index()
        .and_then(|index| program.implementations.get(index))
        .filter(|implementation| implementation.id == id)
        .ok_or_else(|| IrError::new(format!("missing SSA ImplId {}", id.raw())))
}

fn product_by_id(program: &Program, id: ProductId) -> crate::Result<&crate::ProductMetadata> {
    id.index()
        .and_then(|index| program.products.get(index))
        .filter(|product| product.id == id)
        .ok_or_else(|| IrError::new(format!("missing SSA ProductId {}", id.raw())))
}

fn block(function: &Function, id: BlockId) -> crate::Result<&Block> {
    block_by_id(function, id)
}

fn block_by_id(function: &Function, id: BlockId) -> crate::Result<&Block> {
    id.index()
        .and_then(|index| {
            function
                .blocks
                .iter()
                .find(|block| block.id == id && index < function.blocks.len())
        })
        .ok_or_else(|| IrError::new(format!("missing SSA BlockId {}", id.raw())))
}

fn value_type(types: &[SsaType], id: ValueId) -> crate::Result<&SsaType> {
    id.index()
        .and_then(|index| types.get(index))
        .ok_or_else(|| IrError::new(format!("missing SSA ValueId {}", id.raw())))
}

fn fail<T>(message: impl Into<String>) -> crate::Result<T> {
    Err(IrError::new(message))
}
