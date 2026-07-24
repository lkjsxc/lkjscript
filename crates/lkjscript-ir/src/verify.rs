use std::collections::{HashMap, HashSet};

use crate::{
    Block, BlockId, CallTarget, EffectSet, FailureBehavior, Function, FunctionId, Instruction,
    InstructionKind, IrError, ProductId, Program, RuntimeOp, Safepoint, Signature, SsaType,
    Terminator, ValueId,
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
        } => {
            verify_resolved_signature(signature, arguments, &instruction.ty, types)?;
            match target {
                CallTarget::Direct(target) => {
                    let callee = function_by_id(program, *target)?;
                    verify_call_compatibility(&callee.signature, signature)?;
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
                    verify_call_compatibility(target_signature, signature)?;
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

fn verify_call_compatibility(declared: &Signature, resolved: &Signature) -> crate::Result<()> {
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
    Ok(())
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
            if left.parameters.len() != right.parameters.len() {
                return fail("SSA generic function type arity mismatch");
            }
            for (left, right) in left.parameters.iter().zip(&right.parameters) {
                bind_type(left, right, permitted, substitutions)?;
            }
            bind_type(&left.result, &right.result, permitted, substitutions)
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
        SsaType::Function(signature) => SsaType::Function(Box::new(Signature {
            type_parameters: signature.type_parameters.clone(),
            parameters: signature
                .parameters
                .iter()
                .map(|ty| substitute_type(ty, substitutions))
                .collect(),
            result: Box::new(substitute_type(&signature.result, substitutions)),
        })),
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
    match ty {
        SsaType::Product(product) => {
            let _metadata = product_by_id(program, *product)?;
            Ok(())
        }
        SsaType::List(item) | SsaType::Option(item) => verify_type(program, item, type_parameters),
        SsaType::Result(ok, err) => {
            verify_type(program, ok, type_parameters)?;
            verify_type(program, err, type_parameters)
        }
        SsaType::Function(signature) => {
            for parameter in &signature.parameters {
                verify_type(program, parameter, type_parameters)?;
            }
            verify_type(program, &signature.result, type_parameters)
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
