use std::collections::{HashMap, HashSet};

use crate::{
    verify, Block, BlockId, CallTarget, Constant, EffectSet, FailureBehavior, FrameState,
    InstructionKind, Program, RuntimeOp, Safepoint, Terminator, ValueId, VerifiedProgram,
};

pub fn normalize_baseline(program: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let folded = constant_fold_and_propagate(program)?;
    let copied = copy_propagate(&folded)?;
    let simplified = simplify_branches(&copied)?;
    let reachable = unreachable_blocks(&simplified)?;
    let forwarded = empty_block_forwarding(&reachable)?;
    let dead = effect_aware_dce(&forwarded)?;
    let direct = direct_call_resolution(&dead)?;
    canonical_block_order(&direct)
}

pub fn constant_fold_and_propagate(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        let mut constants = HashMap::new();
        for block in &mut function.blocks {
            for instruction in &mut block.instructions {
                let replacement = match &instruction.kind {
                    InstructionKind::Constant(constant) => Some(constant.clone()),
                    InstructionKind::Copy(source) => constants.get(source).cloned(),
                    InstructionKind::Runtime {
                        operation,
                        arguments,
                        ..
                    } => {
                        let arguments: Option<Vec<Constant>> = arguments
                            .iter()
                            .map(|argument| constants.get(argument).cloned())
                            .collect();
                        arguments.and_then(|arguments| fold_runtime(*operation, &arguments))
                    }
                    _ => None,
                };
                if let Some(constant) = replacement {
                    instruction.kind = InstructionKind::Constant(constant.clone());
                    instruction.metadata.effects = EffectSet::PURE;
                    instruction.metadata.safepoint = Safepoint::None;
                    instruction.metadata.failure = FailureBehavior::None;
                    instruction.metadata.frame_state = None;
                    constants.insert(instruction.id, constant);
                }
            }
        }
    }
    finish(program)
}

pub fn copy_propagate(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        let mut copies = HashMap::new();
        for block in &function.blocks {
            for instruction in &block.instructions {
                if let InstructionKind::Copy(source) = instruction.kind {
                    copies.insert(instruction.id, resolve_copy(source, &copies));
                }
            }
        }
        rewrite_function_values(function, |value| resolve_copy(value, &copies));
        for block in &mut function.blocks {
            block
                .instructions
                .retain(|instruction| !matches!(instruction.kind, InstructionKind::Copy(_)));
        }
    }
    compact_values(&mut program)?;
    finish(program)
}

pub fn unreachable_blocks(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        let mut reachable = HashSet::new();
        let mut work = vec![function.entry];
        while let Some(current) = work.pop() {
            if !reachable.insert(current) {
                continue;
            }
            let Some(block) = function.blocks.iter().find(|block| block.id == current) else {
                continue;
            };
            work.extend(successors(&block.terminator));
        }
        function
            .blocks
            .retain(|block| reachable.contains(&block.id));
    }
    compact_blocks(&mut program)?;
    compact_values(&mut program)?;
    finish(program)
}

pub fn simplify_branches(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        let constants: HashMap<ValueId, bool> = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .filter_map(|instruction| match instruction.kind {
                InstructionKind::Constant(Constant::Bool(value)) => Some((instruction.id, value)),
                _ => None,
            })
            .collect();
        for block in &mut function.blocks {
            let replacement = match &block.terminator {
                Terminator::ConditionalBranch {
                    condition,
                    true_target,
                    true_arguments,
                    false_target,
                    false_arguments,
                } => constants.get(condition).map(|condition| {
                    if *condition {
                        Terminator::Branch {
                            target: *true_target,
                            arguments: true_arguments.clone(),
                        }
                    } else {
                        Terminator::Branch {
                            target: *false_target,
                            arguments: false_arguments.clone(),
                        }
                    }
                }),
                _ => None,
            };
            if let Some(replacement) = replacement {
                block.terminator = replacement;
            }
        }
        // Verification runs after every isolated pass. Remove blocks made
        // unreachable by a proven constant edge here so they cannot retain
        // stale cross-block ownership/value uses until the later dedicated
        // unreachable-block pass.
        let mut reachable = HashSet::new();
        let mut work = vec![function.entry];
        while let Some(current) = work.pop() {
            if !reachable.insert(current) {
                continue;
            }
            if let Some(block) = function.blocks.iter().find(|block| block.id == current) {
                work.extend(successors(&block.terminator));
            }
        }
        function
            .blocks
            .retain(|block| reachable.contains(&block.id));
        clear_stale_loop_headers(function);
    }
    compact_blocks(&mut program)?;
    compact_values(&mut program)?;
    finish(program)
}

pub fn empty_block_forwarding(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        loop {
            let candidate = function.blocks.iter().find_map(|block| {
                let Terminator::Branch { target, arguments } = &block.terminator else {
                    return None;
                };
                if block.id == function.entry
                    || block.metadata.loop_header
                    || !block.instructions.is_empty()
                    || target == &block.id
                    || arguments.len() != block.parameters.len()
                    || !arguments.iter().all(|argument| {
                        block
                            .parameters
                            .iter()
                            .any(|parameter| parameter.id == *argument)
                    })
                {
                    return None;
                }
                Some((
                    block.id,
                    *target,
                    block.parameters.clone(),
                    arguments.clone(),
                ))
            });
            let Some((candidate, target, parameters, outgoing)) = candidate else {
                break;
            };
            let mut changed = false;
            for block in &mut function.blocks {
                changed |= forward_terminator(
                    &mut block.terminator,
                    candidate,
                    target,
                    &parameters,
                    &outgoing,
                );
            }
            if !changed {
                break;
            }
            function.blocks.retain(|block| block.id != candidate);
        }
    }
    compact_blocks(&mut program)?;
    compact_values(&mut program)?;
    finish(program)
}

pub fn effect_aware_dce(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        let definitions: HashMap<ValueId, Vec<ValueId>> = function
            .blocks
            .iter()
            .flat_map(|block| &block.instructions)
            .map(|instruction| (instruction.id, instruction.kind.operands()))
            .collect();
        let mut live = HashSet::new();
        for block in &function.blocks {
            live.extend(block.terminator.operands());
            if let Some(frame) = &block.metadata.frame_state {
                add_frame_values(&mut live, frame);
            }
            for instruction in &block.instructions {
                if !instruction.metadata.effects.is_pure()
                    || instruction.metadata.safepoint == Safepoint::Required
                    || matches!(
                        instruction.kind,
                        InstructionKind::PlaceInit { .. }
                            | InstructionKind::PlaceEnd { .. }
                            | InstructionKind::Move { .. }
                            | InstructionKind::Borrow { .. }
                    )
                {
                    live.insert(instruction.id);
                }
                if let Some(frame) = &instruction.metadata.frame_state {
                    add_frame_values(&mut live, frame);
                }
            }
        }
        let mut pending: Vec<ValueId> = live.iter().copied().collect();
        while let Some(value) = pending.pop() {
            if let Some(operands) = definitions.get(&value) {
                for operand in operands {
                    if live.insert(*operand) {
                        pending.push(*operand);
                    }
                }
            }
        }
        for block in &mut function.blocks {
            block.instructions.retain(|instruction| {
                live.contains(&instruction.id)
                    || matches!(
                        instruction.kind,
                        InstructionKind::PlaceInit { .. }
                            | InstructionKind::PlaceEnd { .. }
                            | InstructionKind::Move { .. }
                            | InstructionKind::Borrow { .. }
                    )
                    || !instruction.metadata.effects.is_pure()
                    || instruction.metadata.safepoint == Safepoint::Required
            });
        }
    }
    compact_values(&mut program)?;
    finish(program)
}

pub fn direct_call_resolution(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    let function_effects: Vec<EffectSet> = program
        .functions
        .iter()
        .map(|function| function.effects)
        .collect();
    for function in &mut program.functions {
        let mut references = HashMap::new();
        for block in &function.blocks {
            for instruction in &block.instructions {
                match instruction.kind {
                    InstructionKind::FunctionRef(target) => {
                        references.insert(instruction.id, target);
                    }
                    InstructionKind::Copy(source) => {
                        if let Some(target) = references.get(&source).copied() {
                            references.insert(instruction.id, target);
                        }
                    }
                    _ => {}
                }
            }
        }
        for block in &mut function.blocks {
            for instruction in &mut block.instructions {
                let replacement = match &instruction.kind {
                    InstructionKind::Call {
                        target: CallTarget::Indirect(value),
                        arguments,
                        signature,
                        instantiation,
                    } => references.get(value).map(|target| InstructionKind::Call {
                        target: CallTarget::Direct(*target),
                        arguments: arguments.clone(),
                        signature: signature.clone(),
                        instantiation: instantiation.clone(),
                    }),
                    _ => None,
                };
                if let Some(replacement) = replacement {
                    instruction.kind = replacement;
                    let target = match &instruction.kind {
                        InstructionKind::Call {
                            target: CallTarget::Direct(target),
                            ..
                        } => *target,
                        _ => continue,
                    };
                    if let Some(effects) = function_effects
                        .get(target.index().unwrap_or(usize::MAX))
                        .copied()
                    {
                        instruction.metadata.effects = effects;
                        instruction.metadata.failure = failure_behavior(effects);
                    }
                }
            }
        }
    }
    finish(program)
}

pub fn canonical_block_order(verified: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let mut program = verified.program().clone();
    for function in &mut program.functions {
        let blocks: HashMap<BlockId, Block> = function
            .blocks
            .iter()
            .cloned()
            .map(|block| (block.id, block))
            .collect();
        let mut order = Vec::with_capacity(function.blocks.len());
        let mut seen = HashSet::new();
        canonical_walk(function.entry, &blocks, &mut seen, &mut order);
        let mut remaining: Vec<BlockId> = blocks
            .keys()
            .copied()
            .filter(|id| !seen.contains(id))
            .collect();
        remaining.sort_unstable();
        order.extend(remaining);
        function.blocks = order
            .into_iter()
            .filter_map(|id| blocks.get(&id).cloned())
            .collect();
    }
    compact_blocks(&mut program)?;
    compact_values(&mut program)?;
    finish(program)
}

fn fold_runtime(operation: RuntimeOp, arguments: &[Constant]) -> Option<Constant> {
    use RuntimeOp as Op;
    match (operation, arguments) {
        (Op::Add, [Constant::I64(left), Constant::I64(right)]) => {
            left.checked_add(*right).map(Constant::I64)
        }
        (Op::Subtract, [Constant::I64(left), Constant::I64(right)]) => {
            left.checked_sub(*right).map(Constant::I64)
        }
        (Op::Multiply, [Constant::I64(left), Constant::I64(right)]) => {
            left.checked_mul(*right).map(Constant::I64)
        }
        (Op::Divide, [Constant::I64(left), Constant::I64(right)]) => {
            left.checked_div(*right).map(Constant::I64)
        }
        (Op::BitAnd, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::I64(left & right))
        }
        (Op::BitOr, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::I64(left | right))
        }
        (Op::BitXor, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::I64(left ^ right))
        }
        (Op::Not, [Constant::Bool(value)]) => Some(Constant::Bool(!value)),
        (Op::Less, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::Bool(left < right))
        }
        (Op::LessEqual, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::Bool(left <= right))
        }
        (Op::Greater, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::Bool(left > right))
        }
        (Op::GreaterEqual, [Constant::I64(left), Constant::I64(right)]) => {
            Some(Constant::Bool(left >= right))
        }
        (Op::EqualValue, [left, right]) => fold_equal(left, right).map(Constant::Bool),
        (Op::IsEmptyList, [Constant::EmptyList]) => Some(Constant::Bool(true)),
        _ => None,
    }
}

fn fold_equal(left: &Constant, right: &Constant) -> Option<bool> {
    match (left, right) {
        (Constant::Unit, Constant::Unit) | (Constant::None, Constant::None) => Some(true),
        (Constant::Bool(left), Constant::Bool(right)) => Some(left == right),
        (Constant::I64(left), Constant::I64(right)) => Some(left == right),
        (Constant::Str(left), Constant::Str(right))
        | (Constant::Symbol(left), Constant::Symbol(right)) => Some(left == right),
        _ => None,
    }
}

fn resolve_copy(mut value: ValueId, copies: &HashMap<ValueId, ValueId>) -> ValueId {
    let mut remaining = copies.len().saturating_add(1);
    while remaining > 0 {
        let Some(next) = copies.get(&value).copied() else {
            break;
        };
        value = next;
        remaining -= 1;
    }
    value
}

fn forward_terminator(
    terminator: &mut Terminator,
    candidate: BlockId,
    target: BlockId,
    parameters: &[crate::BlockParameter],
    outgoing: &[ValueId],
) -> bool {
    match terminator {
        Terminator::Branch {
            target: edge_target,
            arguments,
        } if *edge_target == candidate => {
            let incoming = arguments.clone();
            *edge_target = target;
            *arguments = substitute_edge(parameters, outgoing, &incoming);
            true
        }
        Terminator::ConditionalBranch {
            true_target,
            true_arguments,
            false_target,
            false_arguments,
            ..
        } => {
            let mut changed = false;
            if *true_target == candidate {
                let incoming = true_arguments.clone();
                *true_target = target;
                *true_arguments = substitute_edge(parameters, outgoing, &incoming);
                changed = true;
            }
            if *false_target == candidate {
                let incoming = false_arguments.clone();
                *false_target = target;
                *false_arguments = substitute_edge(parameters, outgoing, &incoming);
                changed = true;
            }
            changed
        }
        _ => false,
    }
}

fn substitute_edge(
    parameters: &[crate::BlockParameter],
    outgoing: &[ValueId],
    incoming: &[ValueId],
) -> Vec<ValueId> {
    let substitutions: HashMap<ValueId, ValueId> = parameters
        .iter()
        .zip(incoming)
        .map(|(parameter, incoming)| (parameter.id, *incoming))
        .collect();
    outgoing
        .iter()
        .map(|value| substitutions.get(value).copied().unwrap_or(*value))
        .collect()
}

fn canonical_walk(
    current: BlockId,
    blocks: &HashMap<BlockId, Block>,
    seen: &mut HashSet<BlockId>,
    order: &mut Vec<BlockId>,
) {
    if !seen.insert(current) {
        return;
    }
    order.push(current);
    let Some(block) = blocks.get(&current) else {
        return;
    };
    for successor in successors(&block.terminator) {
        canonical_walk(successor, blocks, seen, order);
    }
}

fn clear_stale_loop_headers(function: &mut crate::Function) {
    let stale: Vec<BlockId> = function
        .blocks
        .iter()
        .filter(|block| block.metadata.loop_header)
        .filter_map(|header| {
            let mut seen = HashSet::new();
            let mut work = successors(&header.terminator);
            let mut cyclic = false;
            while let Some(current) = work.pop() {
                if current == header.id {
                    cyclic = true;
                    break;
                }
                if !seen.insert(current) {
                    continue;
                }
                if let Some(block) = function.blocks.iter().find(|block| block.id == current) {
                    work.extend(successors(&block.terminator));
                }
            }
            (!cyclic).then_some(header.id)
        })
        .collect();
    for id in stale {
        if let Some(block) = function.blocks.iter_mut().find(|block| block.id == id) {
            block.metadata.loop_header = false;
            block.metadata.frame_state = None;
        }
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

fn add_frame_values(live: &mut HashSet<ValueId>, frame: &FrameState) {
    live.extend(frame.locals.iter().map(|local| local.value));
    live.extend(frame.operand_stack.iter().copied());
}

fn failure_behavior(effects: EffectSet) -> FailureBehavior {
    match (
        effects.contains(EffectSet::MAY_TRAP),
        effects.contains(EffectSet::MAY_EXIT) || effects.contains(EffectSet::ALLOCATES),
    ) {
        (false, false) => FailureBehavior::None,
        (true, false) => FailureBehavior::Trap,
        (false, true) => FailureBehavior::StructuredOutcome,
        (true, true) => FailureBehavior::TrapOrOutcome,
    }
}

fn compact_blocks(program: &mut Program) -> crate::Result<()> {
    for function in &mut program.functions {
        let mut mapping = HashMap::with_capacity(function.blocks.len());
        for (index, block) in function.blocks.iter().enumerate() {
            let raw = u32::try_from(index)
                .map_err(|_| crate::IrError::new("SSA block count exceeds u32"))?;
            mapping.insert(block.id, BlockId::new(raw));
        }
        function.entry = mapped_block(&mapping, function.entry)?;
        for block in &mut function.blocks {
            block.id = mapped_block(&mapping, block.id)?;
            match &mut block.terminator {
                Terminator::Branch { target, .. } => *target = mapped_block(&mapping, *target)?,
                Terminator::ConditionalBranch {
                    true_target,
                    false_target,
                    ..
                } => {
                    *true_target = mapped_block(&mapping, *true_target)?;
                    *false_target = mapped_block(&mapping, *false_target)?;
                }
                _ => {}
            }
        }
    }
    compact_places(program)
}

fn compact_places(program: &mut Program) -> crate::Result<()> {
    for function in &mut program.functions {
        let mut referenced = HashSet::new();
        for block in &function.blocks {
            referenced.extend(
                block
                    .parameters
                    .iter()
                    .filter_map(|parameter| parameter.owner_place),
            );
            for instruction in &block.instructions {
                match instruction.kind {
                    InstructionKind::PlaceInit { place, .. }
                    | InstructionKind::PlaceEnd { place }
                    | InstructionKind::Move { place, .. }
                    | InstructionKind::Borrow { place, .. } => {
                        referenced.insert(place);
                    }
                    _ => {}
                }
            }
        }
        let retained: Vec<_> = function
            .places
            .iter()
            .filter(|place| referenced.contains(&place.id))
            .cloned()
            .collect();
        let mut mapping = HashMap::with_capacity(retained.len());
        for (index, place) in retained.iter().enumerate() {
            let raw = u32::try_from(index)
                .map_err(|_| crate::IrError::new("SSA place count exceeds u32"))?;
            mapping.insert(place.id, crate::PlaceId::new(raw));
        }
        function.places = retained
            .into_iter()
            .map(|mut place| {
                place.id = mapped_place(&mapping, place.id)?;
                Ok(place)
            })
            .collect::<crate::Result<Vec<_>>>()?;
        for block in &mut function.blocks {
            for parameter in &mut block.parameters {
                if let Some(place) = parameter.owner_place {
                    parameter.owner_place = Some(mapped_place(&mapping, place)?);
                }
            }
            for instruction in &mut block.instructions {
                match &mut instruction.kind {
                    InstructionKind::PlaceInit { place, .. }
                    | InstructionKind::PlaceEnd { place }
                    | InstructionKind::Move { place, .. }
                    | InstructionKind::Borrow { place, .. } => {
                        *place = mapped_place(&mapping, *place)?;
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

fn mapped_place(
    mapping: &HashMap<crate::PlaceId, crate::PlaceId>,
    id: crate::PlaceId,
) -> crate::Result<crate::PlaceId> {
    mapping
        .get(&id)
        .copied()
        .ok_or_else(|| crate::IrError::new(format!("pass lost SSA place {}", id.raw())))
}

fn mapped_block(mapping: &HashMap<BlockId, BlockId>, id: BlockId) -> crate::Result<BlockId> {
    mapping
        .get(&id)
        .copied()
        .ok_or_else(|| crate::IrError::new(format!("pass lost SSA block {}", id.raw())))
}

fn compact_values(program: &mut Program) -> crate::Result<()> {
    for function in &mut program.functions {
        let mut mapping = HashMap::new();
        let mut next = 0_u32;
        for block in &function.blocks {
            for parameter in &block.parameters {
                mapping.insert(parameter.id, ValueId::new(next));
                next = next
                    .checked_add(1)
                    .ok_or_else(|| crate::IrError::new("SSA value count exceeds u32"))?;
            }
            for instruction in &block.instructions {
                mapping.insert(instruction.id, ValueId::new(next));
                next = next
                    .checked_add(1)
                    .ok_or_else(|| crate::IrError::new("SSA value count exceeds u32"))?;
            }
        }
        for block in &mut function.blocks {
            for parameter in &mut block.parameters {
                parameter.id = mapped_value(&mapping, parameter.id)?;
            }
            for instruction in &mut block.instructions {
                instruction.id = mapped_value(&mapping, instruction.id)?;
            }
        }
        rewrite_function_values(function, |value| {
            mapping.get(&value).copied().unwrap_or(value)
        });
    }
    Ok(())
}

fn mapped_value(mapping: &HashMap<ValueId, ValueId>, id: ValueId) -> crate::Result<ValueId> {
    mapping
        .get(&id)
        .copied()
        .ok_or_else(|| crate::IrError::new(format!("pass lost SSA value {}", id.raw())))
}

fn rewrite_function_values(
    function: &mut crate::Function,
    mut rewrite: impl FnMut(ValueId) -> ValueId,
) {
    for block in &mut function.blocks {
        if let Some(frame) = &mut block.metadata.frame_state {
            rewrite_frame(frame, &mut rewrite);
        }
        for instruction in &mut block.instructions {
            match &mut instruction.kind {
                InstructionKind::Constant(_)
                | InstructionKind::PlaceEnd { .. }
                | InstructionKind::FunctionRef(_) => {}
                InstructionKind::Copy(value)
                | InstructionKind::PlaceInit { value, .. }
                | InstructionKind::Move { value, .. }
                | InstructionKind::Borrow { value, .. } => {
                    *value = rewrite(*value);
                }
                InstructionKind::Runtime { arguments, .. }
                | InstructionKind::Call { arguments, .. } => {
                    for argument in arguments {
                        *argument = rewrite(*argument);
                    }
                    if let InstructionKind::Call {
                        target: CallTarget::Indirect(target),
                        ..
                    } = &mut instruction.kind
                    {
                        *target = rewrite(*target);
                    }
                }
                InstructionKind::ProductValue { fields, .. } => {
                    for field in fields {
                        *field = rewrite(*field);
                    }
                }
                InstructionKind::ProductField { value, .. } => *value = rewrite(*value),
                InstructionKind::WithProductField {
                    value, replacement, ..
                } => {
                    *value = rewrite(*value);
                    *replacement = rewrite(*replacement);
                }
            }
            if let Some(frame) = &mut instruction.metadata.frame_state {
                rewrite_frame(frame, &mut rewrite);
            }
        }
        match &mut block.terminator {
            Terminator::Branch { arguments, .. } => {
                for argument in arguments {
                    *argument = rewrite(*argument);
                }
            }
            Terminator::ConditionalBranch {
                condition,
                true_arguments,
                false_arguments,
                ..
            } => {
                *condition = rewrite(*condition);
                for argument in true_arguments.iter_mut().chain(false_arguments) {
                    *argument = rewrite(*argument);
                }
            }
            Terminator::Return(value) => *value = rewrite(*value),
            Terminator::Exit { code } => *code = rewrite(*code),
            Terminator::Outcome { detail, .. } => {
                if let Some(detail) = detail {
                    *detail = rewrite(*detail);
                }
            }
            Terminator::Trap { .. } => {}
        }
    }
}

fn rewrite_frame(frame: &mut FrameState, rewrite: &mut impl FnMut(ValueId) -> ValueId) {
    for local in &mut frame.locals {
        local.value = rewrite(local.value);
    }
    for value in &mut frame.operand_stack {
        *value = rewrite(*value);
    }
}

fn finish(program: Program) -> crate::Result<VerifiedProgram> {
    verify(program)
}
