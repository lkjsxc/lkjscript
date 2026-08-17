use crate::core_ir::{
    self, CoreFunction, CoreProgram, CoreTypeId, CoreTypeKind, Instruction, SwitchArgument,
    Terminator, ValueId,
};
use crate::error::{ErrorCode, LkError, Result};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct DiscriminantCondition {
    pub cell: u32,
    pub variant: u32,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ManagedCellPath {
    pub cell: u32,
    pub conditions: Vec<DiscriminantCondition>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ManagedReferenceMap {
    pub paths: Vec<ManagedCellPath>,
}

impl ManagedReferenceMap {
    pub(crate) fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UseAction {
    Immediate,
    Borrow,
    Share,
    Transfer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InstructionOwnership {
    pub uses: Vec<(ValueId, UseAction)>,
    pub drops_after: Vec<ValueId>,
    pub reuse_left: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum EdgeSource {
    Value(ValueId),
    Payload,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EdgeOwnership {
    pub sources: Vec<(EdgeSource, UseAction)>,
    pub drops: Vec<ValueId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TerminatorOwnership {
    Return {
        value: ValueId,
        action: UseAction,
        drops: Vec<ValueId>,
    },
    Branch(EdgeOwnership),
    CondBranch {
        then_edge: EdgeOwnership,
        else_edge: EdgeOwnership,
    },
    SwitchVariant {
        arms: Vec<EdgeOwnership>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BlockOwnership {
    pub entry_drops: Vec<ValueId>,
    pub cleanup_roots: Vec<ValueId>,
    pub instructions: Vec<InstructionOwnership>,
    pub terminator: TerminatorOwnership,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FunctionOwnership {
    pub blocks: Vec<BlockOwnership>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OwnershipPlan {
    pub managed_maps: Vec<ManagedReferenceMap>,
    pub functions: Vec<FunctionOwnership>,
}

pub(crate) fn derive(program: &CoreProgram) -> Result<OwnershipPlan> {
    core_ir::verify(program)?;
    let managed_maps = derive_managed_maps(program)?;
    let functions = program
        .functions
        .iter()
        .map(|function| derive_function(program, function, &managed_maps))
        .collect::<Result<Vec<_>>>()?;
    let plan = OwnershipPlan {
        managed_maps,
        functions,
    };
    verify_after_core(program, &plan)?;
    Ok(plan)
}

#[cfg(test)]
pub(crate) fn verify(program: &CoreProgram, plan: &OwnershipPlan) -> Result<()> {
    core_ir::verify(program)?;
    verify_after_core(program, plan)
}

fn verify_after_core(program: &CoreProgram, plan: &OwnershipPlan) -> Result<()> {
    let maps = derive_managed_maps(program)?;
    if plan.managed_maps != maps {
        return Err(invalid(
            "ownership plan managed-reference maps are not derivable",
        ));
    }
    if plan.functions.len() != program.functions.len() {
        return Err(invalid(
            "ownership plan function table has the wrong length",
        ));
    }
    for (function, actual) in program.functions.iter().zip(&plan.functions) {
        verify_function_plan(program, function, &maps, actual)?;
    }
    Ok(())
}

fn derive_managed_maps(program: &CoreProgram) -> Result<Vec<ManagedReferenceMap>> {
    let mut complete = BTreeMap::<usize, ManagedReferenceMap>::new();
    let mut pending = (0..program.types.len()).collect::<BTreeSet<_>>();
    while !pending.is_empty() {
        let ready = pending.iter().copied().find(|index| {
            type_dependencies(&program.types[*index].kind)
                .iter()
                .all(|dependency| complete.contains_key(dependency))
        });
        let Some(index) = ready else {
            return Err(invalid(
                "managed-reference map derivation found a cyclic type dependency",
            ));
        };
        let map = derive_type_map(program, index, &complete)?;
        complete.insert(index, map);
        pending.remove(&index);
    }
    (0..program.types.len())
        .map(|index| {
            complete
                .remove(&index)
                .ok_or_else(|| invalid("managed-reference map is absent"))
        })
        .collect()
}

fn type_dependencies(kind: &CoreTypeKind) -> Vec<usize> {
    match kind {
        CoreTypeKind::Product { fields } => fields
            .iter()
            .filter_map(|field| usize::try_from(field.ty.0).ok())
            .collect(),
        CoreTypeKind::Sum { variants } => variants
            .iter()
            .filter_map(|variant| variant.payload)
            .filter_map(|payload| usize::try_from(payload.0).ok())
            .collect(),
        CoreTypeKind::Unit | CoreTypeKind::Bool | CoreTypeKind::I64 | CoreTypeKind::Bytes => {
            Vec::new()
        }
    }
}

fn derive_type_map(
    program: &CoreProgram,
    index: usize,
    complete: &BTreeMap<usize, ManagedReferenceMap>,
) -> Result<ManagedReferenceMap> {
    let ty = program
        .types
        .get(index)
        .ok_or_else(|| invalid("managed-reference type index is out of bounds"))?;
    let mut paths = Vec::new();
    match &ty.kind {
        CoreTypeKind::Unit | CoreTypeKind::Bool | CoreTypeKind::I64 => {}
        CoreTypeKind::Bytes => paths.push(ManagedCellPath {
            cell: 0,
            conditions: Vec::new(),
        }),
        CoreTypeKind::Product { fields } => {
            for field in fields {
                let child = complete
                    .get(&usize::try_from(field.ty.0).map_err(|_| {
                        invalid("managed product type index overflows host indexes")
                    })?)
                    .ok_or_else(|| invalid("managed product dependency map is absent"))?;
                let offset = u32::try_from(field.cell_offset)
                    .map_err(|_| invalid("managed product cell offset overflows u32"))?;
                for path in &child.paths {
                    paths.push(offset_path(path, offset, None)?);
                }
            }
        }
        CoreTypeKind::Sum { variants } => {
            for (ordinal, variant) in variants.iter().enumerate() {
                let Some(payload) = variant.payload else {
                    continue;
                };
                let child =
                    complete
                        .get(&usize::try_from(payload.0).map_err(|_| {
                            invalid("managed sum type index overflows host indexes")
                        })?)
                        .ok_or_else(|| invalid("managed sum dependency map is absent"))?;
                let variant = u32::try_from(ordinal)
                    .map_err(|_| invalid("managed sum variant ordinal overflows u32"))?;
                for path in &child.paths {
                    paths.push(offset_path(path, 1, Some(variant))?);
                }
            }
        }
    }
    paths.sort();
    paths.dedup();
    Ok(ManagedReferenceMap { paths })
}

fn offset_path(
    path: &ManagedCellPath,
    offset: u32,
    variant: Option<u32>,
) -> Result<ManagedCellPath> {
    let mut conditions = Vec::with_capacity(path.conditions.len() + usize::from(variant.is_some()));
    if let Some(variant) = variant {
        conditions.push(DiscriminantCondition {
            cell: offset - 1,
            variant,
        });
    }
    for condition in &path.conditions {
        conditions.push(DiscriminantCondition {
            cell: condition
                .cell
                .checked_add(offset)
                .ok_or_else(|| invalid("nested managed discriminant offset overflowed"))?,
            variant: condition.variant,
        });
    }
    Ok(ManagedCellPath {
        cell: path
            .cell
            .checked_add(offset)
            .ok_or_else(|| invalid("nested managed cell offset overflowed"))?,
        conditions,
    })
}

fn derive_function(
    program: &CoreProgram,
    function: &CoreFunction,
    maps: &[ManagedReferenceMap],
) -> Result<FunctionOwnership> {
    Ok(FunctionOwnership {
        blocks: function
            .blocks
            .iter()
            .map(|block| derive_block(program, function, block, maps))
            .collect::<Result<Vec<_>>>()?,
    })
}

fn derive_block(
    program: &CoreProgram,
    function: &CoreFunction,
    block: &crate::core_ir::CoreBlock,
    maps: &[ManagedReferenceMap],
) -> Result<BlockOwnership> {
    let mut last_instruction_use = BTreeMap::<ValueId, usize>::new();
    for (instruction_index, instruction) in block.instructions.iter().enumerate() {
        for value in instruction_operands(instruction) {
            last_instruction_use.insert(value, instruction_index);
        }
    }
    let terminator_values = terminator_values(&block.terminator);
    let terminator_set = terminator_values.iter().copied().collect::<BTreeSet<_>>();
    let mut last_uses_at = vec![Vec::<ValueId>::new(); block.instructions.len()];
    for (value, instruction_index) in &last_instruction_use {
        if !terminator_set.contains(value) {
            last_uses_at[*instruction_index].push(*value);
        }
    }
    for values in &mut last_uses_at {
        values.sort();
    }
    let managed = |value: ValueId| value_is_managed(function, maps, value);
    let mut entry_drops = block
        .parameters
        .iter()
        .copied()
        .filter(|value| {
            managed(*value).unwrap_or(false)
                && !last_instruction_use.contains_key(value)
                && !terminator_set.contains(value)
        })
        .collect::<Vec<_>>();
    entry_drops.sort();

    let mut instructions = Vec::with_capacity(block.instructions.len());
    for (instruction_index, instruction) in block.instructions.iter().enumerate() {
        let operands = instruction_operands(instruction);
        let mut remaining_same = occurrence_counts(&operands);
        let mut uses = Vec::with_capacity(operands.len());
        let copies_ownership = instruction_copies_ownership(instruction);
        for value in operands {
            let count = remaining_same
                .get_mut(&value)
                .ok_or_else(|| invalid("ownership occurrence count is absent"))?;
            *count = count
                .checked_sub(1)
                .ok_or_else(|| invalid("ownership occurrence count underflowed"))?;
            let action = if !managed(value)? {
                UseAction::Immediate
            } else if copies_ownership && !matches!(instruction, Instruction::ProjectField { .. }) {
                let no_later_use = last_instruction_use.get(&value) == Some(&instruction_index)
                    && !terminator_set.contains(&value)
                    && *count == 0;
                if no_later_use {
                    UseAction::Transfer
                } else {
                    UseAction::Share
                }
            } else if matches!(instruction, Instruction::ProjectField { .. }) {
                UseAction::Share
            } else {
                UseAction::Borrow
            };
            uses.push((value, action));
        }

        let transferred = uses
            .iter()
            .filter_map(|(value, action)| (*action == UseAction::Transfer).then_some(*value))
            .collect::<BTreeSet<_>>();
        let mut drops_after = last_uses_at[instruction_index]
            .iter()
            .copied()
            .filter(|value| managed(*value).unwrap_or(false) && !transferred.contains(value))
            .collect::<Vec<_>>();
        let result = instruction_result(instruction);
        if managed(result)?
            && !last_instruction_use.contains_key(&result)
            && !terminator_set.contains(&result)
        {
            drops_after.push(result);
        }
        drops_after.sort();
        drops_after.dedup();
        let reuse_left = match instruction {
            Instruction::BytesConcat { lhs, rhs, .. } => {
                lhs != rhs
                    && last_instruction_use.get(lhs) == Some(&instruction_index)
                    && !terminator_set.contains(lhs)
            }
            _ => false,
        };
        instructions.push(InstructionOwnership {
            uses,
            drops_after,
            reuse_left,
        });
    }

    let available = available_at_terminator(block);
    let cleanup_roots = available
        .iter()
        .copied()
        .filter(|value| managed(*value).unwrap_or(false))
        .collect::<Vec<_>>();
    let terminator = derive_terminator(program, function, maps, &block.terminator, &available)?;
    Ok(BlockOwnership {
        entry_drops,
        cleanup_roots,
        instructions,
        terminator,
    })
}

fn derive_terminator(
    program: &CoreProgram,
    function: &CoreFunction,
    maps: &[ManagedReferenceMap],
    terminator: &Terminator,
    available: &[ValueId],
) -> Result<TerminatorOwnership> {
    match terminator {
        Terminator::Return { value, .. } => {
            let action = if value_is_managed(function, maps, *value)? {
                UseAction::Transfer
            } else {
                UseAction::Immediate
            };
            Ok(TerminatorOwnership::Return {
                value: *value,
                action,
                drops: terminal_drops(function, maps, available, [*value])?,
            })
        }
        Terminator::Branch { arguments, .. } => Ok(TerminatorOwnership::Branch(derive_edge(
            function,
            maps,
            arguments.iter().copied().map(EdgeSource::Value).collect(),
            available,
        )?)),
        Terminator::CondBranch {
            then_arguments,
            else_arguments,
            ..
        } => Ok(TerminatorOwnership::CondBranch {
            then_edge: derive_edge(
                function,
                maps,
                then_arguments
                    .iter()
                    .copied()
                    .map(EdgeSource::Value)
                    .collect(),
                available,
            )?,
            else_edge: derive_edge(
                function,
                maps,
                else_arguments
                    .iter()
                    .copied()
                    .map(EdgeSource::Value)
                    .collect(),
                available,
            )?,
        }),
        Terminator::SwitchVariant {
            scrutinee, arms, ..
        } => {
            let sum = core_ir::value_type(function, *scrutinee)?;
            let CoreTypeKind::Sum { variants } = &core_ir::type_at(program, sum)?.kind else {
                return Err(invalid("ownership switch scrutinee is not a sum"));
            };
            let mut result = Vec::with_capacity(arms.len());
            for (arm, variant) in arms.iter().zip(variants) {
                let sources = arm
                    .arguments
                    .iter()
                    .map(|argument| match argument {
                        SwitchArgument::Value(value) => EdgeSource::Value(*value),
                        SwitchArgument::Payload => EdgeSource::Payload,
                    })
                    .collect::<Vec<_>>();
                let payload_managed = variant
                    .payload
                    .map(|ty| map_at(maps, ty).map(|map| !map.is_empty()))
                    .transpose()?
                    .unwrap_or(false);
                result.push(derive_switch_edge(
                    function,
                    maps,
                    sources,
                    available,
                    *scrutinee,
                    payload_managed,
                )?);
            }
            Ok(TerminatorOwnership::SwitchVariant { arms: result })
        }
    }
}

fn derive_edge(
    function: &CoreFunction,
    maps: &[ManagedReferenceMap],
    sources: Vec<EdgeSource>,
    available: &[ValueId],
) -> Result<EdgeOwnership> {
    let mut remaining = occurrence_counts(
        &sources
            .iter()
            .filter_map(|source| match source {
                EdgeSource::Value(value) => Some(*value),
                EdgeSource::Payload => None,
            })
            .collect::<Vec<_>>(),
    );
    let mut transferred = BTreeSet::new();
    let mut planned = Vec::with_capacity(sources.len());
    for source in sources {
        let action = match source {
            EdgeSource::Value(value) if value_is_managed(function, maps, value)? => {
                let count = remaining
                    .get_mut(&value)
                    .ok_or_else(|| invalid("edge ownership occurrence count is absent"))?;
                *count = count
                    .checked_sub(1)
                    .ok_or_else(|| invalid("edge ownership occurrence count underflowed"))?;
                if *count == 0 {
                    transferred.insert(value);
                    UseAction::Transfer
                } else {
                    UseAction::Share
                }
            }
            EdgeSource::Value(_) => UseAction::Immediate,
            EdgeSource::Payload => UseAction::Immediate,
        };
        planned.push((source, action));
    }
    Ok(EdgeOwnership {
        sources: planned,
        drops: terminal_drops(function, maps, available, transferred)?,
    })
}

fn derive_switch_edge(
    function: &CoreFunction,
    maps: &[ManagedReferenceMap],
    sources: Vec<EdgeSource>,
    available: &[ValueId],
    scrutinee: ValueId,
    payload_managed: bool,
) -> Result<EdgeOwnership> {
    let mut edge = derive_edge(function, maps, sources, available)?;
    for (source, action) in &mut edge.sources {
        if *source == EdgeSource::Payload && payload_managed {
            *action = UseAction::Share;
        }
    }
    if value_is_managed(function, maps, scrutinee)? && !edge.drops.contains(&scrutinee) {
        edge.drops.push(scrutinee);
        edge.drops.sort();
    }
    Ok(edge)
}

fn terminal_drops(
    function: &CoreFunction,
    maps: &[ManagedReferenceMap],
    available: &[ValueId],
    transferred: impl IntoIterator<Item = ValueId>,
) -> Result<Vec<ValueId>> {
    let transferred = transferred.into_iter().collect::<BTreeSet<_>>();
    let mut result = Vec::new();
    for value in available {
        if value_is_managed(function, maps, *value)? && !transferred.contains(value) {
            result.push(*value);
        }
    }
    result.sort();
    result.dedup();
    Ok(result)
}

fn available_at_terminator(block: &crate::core_ir::CoreBlock) -> Vec<ValueId> {
    let mut result = block.parameters.clone();
    result.extend(block.instructions.iter().map(instruction_result));
    result.sort();
    result.dedup();
    result
}

fn instruction_operands(instruction: &Instruction) -> Vec<ValueId> {
    match instruction {
        Instruction::ConstUnit { .. }
        | Instruction::ConstBool { .. }
        | Instruction::ConstI64 { .. }
        | Instruction::ConstBytes { .. } => Vec::new(),
        Instruction::BytesLen { value, .. } => vec![*value],
        Instruction::BytesAt { value, index, .. } => vec![*value, *index],
        Instruction::BytesSlice {
            value,
            start,
            length,
            ..
        } => vec![*value, *start, *length],
        Instruction::AddI64 { lhs, rhs, .. }
        | Instruction::LtI64 { lhs, rhs, .. }
        | Instruction::BytesEqual { lhs, rhs, .. }
        | Instruction::BytesConcat { lhs, rhs, .. } => vec![*lhs, *rhs],
        Instruction::Call { arguments, .. } => arguments.clone(),
        Instruction::ConstructProduct { fields, .. } => fields.clone(),
        Instruction::ProjectField { value, .. } => vec![*value],
        Instruction::ConstructVariant { payload, .. } => payload.iter().copied().collect(),
    }
}

fn terminator_values(terminator: &Terminator) -> Vec<ValueId> {
    match terminator {
        Terminator::Return { value, .. } => vec![*value],
        Terminator::Branch { arguments, .. } => arguments.clone(),
        Terminator::CondBranch {
            condition,
            then_arguments,
            else_arguments,
            ..
        } => {
            let mut result = vec![*condition];
            result.extend(then_arguments);
            result.extend(else_arguments);
            result
        }
        Terminator::SwitchVariant {
            scrutinee, arms, ..
        } => {
            let mut result = vec![*scrutinee];
            result.extend(arms.iter().flat_map(|arm| {
                arm.arguments.iter().filter_map(|argument| match argument {
                    SwitchArgument::Value(value) => Some(*value),
                    SwitchArgument::Payload => None,
                })
            }));
            result
        }
    }
}

fn instruction_copies_ownership(instruction: &Instruction) -> bool {
    matches!(
        instruction,
        Instruction::Call { .. }
            | Instruction::ConstructProduct { .. }
            | Instruction::ProjectField { .. }
            | Instruction::ConstructVariant { .. }
    )
}

fn instruction_result(instruction: &Instruction) -> ValueId {
    match instruction {
        Instruction::ConstUnit { result, .. }
        | Instruction::ConstBool { result, .. }
        | Instruction::ConstI64 { result, .. }
        | Instruction::ConstBytes { result, .. }
        | Instruction::AddI64 { result, .. }
        | Instruction::LtI64 { result, .. }
        | Instruction::BytesLen { result, .. }
        | Instruction::BytesAt { result, .. }
        | Instruction::BytesSlice { result, .. }
        | Instruction::BytesEqual { result, .. }
        | Instruction::BytesConcat { result, .. }
        | Instruction::Call { result, .. }
        | Instruction::ConstructProduct { result, .. }
        | Instruction::ProjectField { result, .. }
        | Instruction::ConstructVariant { result, .. } => *result,
    }
}

fn occurrence_counts(values: &[ValueId]) -> BTreeMap<ValueId, usize> {
    let mut result = BTreeMap::new();
    for value in values {
        *result.entry(*value).or_insert(0) += 1;
    }
    result
}

fn value_is_managed(
    function: &CoreFunction,
    maps: &[ManagedReferenceMap],
    value: ValueId,
) -> Result<bool> {
    Ok(!map_at(maps, core_ir::value_type(function, value)?)?.is_empty())
}

fn map_at(maps: &[ManagedReferenceMap], ty: CoreTypeId) -> Result<&ManagedReferenceMap> {
    maps.get(
        usize::try_from(ty.0)
            .map_err(|_| invalid("managed-reference type index overflows host indexes"))?,
    )
    .ok_or_else(|| invalid("managed-reference type index is out of bounds"))
}

fn verify_function_plan(
    program: &CoreProgram,
    function: &CoreFunction,
    maps: &[ManagedReferenceMap],
    actual: &FunctionOwnership,
) -> Result<()> {
    if actual.blocks.len() != function.blocks.len() {
        return Err(
            invalid("ownership plan block table has the wrong length").for_node(function.origin)
        );
    }
    for (block, actual_block) in function.blocks.iter().zip(&actual.blocks) {
        verify_block_plan(program, function, block, maps, actual_block)?;
    }
    Ok(())
}

fn verify_block_plan(
    program: &CoreProgram,
    function: &CoreFunction,
    block: &crate::core_ir::CoreBlock,
    maps: &[ManagedReferenceMap],
    actual: &BlockOwnership,
) -> Result<()> {
    if actual.instructions.len() != block.instructions.len() {
        return Err(plan_error(
            function,
            block,
            "instruction table has the wrong length",
        ));
    }

    let mut last_instruction_use = BTreeMap::<ValueId, usize>::new();
    for (instruction_index, instruction) in block.instructions.iter().enumerate() {
        for value in instruction_operands(instruction) {
            last_instruction_use.insert(value, instruction_index);
        }
    }
    let terminator_set = terminator_values(&block.terminator)
        .into_iter()
        .collect::<BTreeSet<_>>();
    let managed = |value: ValueId| value_is_managed(function, maps, value);

    let mut expected_entry_drops = block
        .parameters
        .iter()
        .copied()
        .filter(|value| {
            managed(*value).unwrap_or(false)
                && !last_instruction_use.contains_key(value)
                && !terminator_set.contains(value)
        })
        .collect::<Vec<_>>();
    expected_entry_drops.sort();
    if actual.entry_drops != expected_entry_drops {
        return Err(plan_error(
            function,
            block,
            "entry cleanup does not match managed parameter liveness",
        ));
    }

    let available = available_at_terminator(block);
    let expected_cleanup_roots = available
        .iter()
        .copied()
        .filter(|value| managed(*value).unwrap_or(false))
        .collect::<Vec<_>>();
    if actual.cleanup_roots != expected_cleanup_roots {
        return Err(plan_error(
            function,
            block,
            "trap cleanup roots do not match live managed definitions",
        ));
    }

    for (instruction_index, (instruction, actual_instruction)) in block
        .instructions
        .iter()
        .zip(&actual.instructions)
        .enumerate()
    {
        let operands = instruction_operands(instruction);
        if actual_instruction.uses.len() != operands.len() {
            return Err(plan_error(
                function,
                block,
                &format!("instruction {instruction_index} has the wrong ownership-use count"),
            ));
        }
        let mut remaining_same = occurrence_counts(&operands);
        let mut transferred = BTreeSet::new();
        for (operand_index, (value, (planned_value, action))) in
            operands.iter().zip(&actual_instruction.uses).enumerate()
        {
            if value != planned_value {
                return Err(plan_error(
                    function,
                    block,
                    &format!(
                        "instruction {instruction_index} operand {operand_index} names value {} instead of {}",
                        planned_value.0, value.0
                    ),
                ));
            }
            let count = remaining_same
                .get_mut(value)
                .ok_or_else(|| plan_error(function, block, "operand count is absent"))?;
            *count = count
                .checked_sub(1)
                .ok_or_else(|| plan_error(function, block, "operand count underflowed"))?;
            let expected = if !value_is_managed(function, maps, *value)? {
                UseAction::Immediate
            } else if matches!(instruction, Instruction::ProjectField { .. }) {
                UseAction::Share
            } else if instruction_copies_ownership(instruction) {
                if last_instruction_use.get(value) == Some(&instruction_index)
                    && !terminator_set.contains(value)
                    && *count == 0
                {
                    UseAction::Transfer
                } else {
                    UseAction::Share
                }
            } else {
                UseAction::Borrow
            };
            if *action != expected {
                return Err(plan_error(
                    function,
                    block,
                    &format!(
                        "instruction {instruction_index} value {} claims {action:?} instead of {expected:?}",
                        value.0
                    ),
                ));
            }
            if expected == UseAction::Transfer {
                transferred.insert(*value);
            }
        }

        let mut expected_drops = last_instruction_use
            .iter()
            .filter_map(|(value, last_use)| {
                (*last_use == instruction_index
                    && !terminator_set.contains(value)
                    && managed(*value).unwrap_or(false)
                    && !transferred.contains(value))
                .then_some(*value)
            })
            .collect::<Vec<_>>();
        let result = instruction_result(instruction);
        if managed(result)?
            && !last_instruction_use.contains_key(&result)
            && !terminator_set.contains(&result)
        {
            expected_drops.push(result);
        }
        expected_drops.sort();
        expected_drops.dedup();
        if actual_instruction.drops_after != expected_drops {
            return Err(plan_error(
                function,
                block,
                &format!(
                    "instruction {instruction_index} cleanup does not discharge exact last uses"
                ),
            ));
        }

        let expected_reuse = matches!(
            instruction,
            Instruction::BytesConcat { lhs, rhs, .. }
                if lhs != rhs
                    && last_instruction_use.get(lhs) == Some(&instruction_index)
                    && !terminator_set.contains(lhs)
        );
        if actual_instruction.reuse_left != expected_reuse {
            return Err(plan_error(
                function,
                block,
                &format!(
                    "instruction {instruction_index} uniqueness claim is not derivable from last use"
                ),
            ));
        }
    }

    verify_terminator_plan(
        program,
        function,
        block,
        maps,
        &available,
        &actual.terminator,
    )
}

fn verify_terminator_plan(
    program: &CoreProgram,
    function: &CoreFunction,
    block: &crate::core_ir::CoreBlock,
    maps: &[ManagedReferenceMap],
    available: &[ValueId],
    actual: &TerminatorOwnership,
) -> Result<()> {
    match (&block.terminator, actual) {
        (
            Terminator::Return { value, .. },
            TerminatorOwnership::Return {
                value: planned_value,
                action,
                drops,
            },
        ) => {
            let expected_action = if value_is_managed(function, maps, *value)? {
                UseAction::Transfer
            } else {
                UseAction::Immediate
            };
            let expected_drops = terminal_drops(function, maps, available, [*value])?;
            if planned_value != value || *action != expected_action || *drops != expected_drops {
                return Err(plan_error(
                    function,
                    block,
                    "return transfer, escape, or cleanup does not match verified liveness",
                ));
            }
        }
        (Terminator::Branch { arguments, .. }, TerminatorOwnership::Branch(actual_edge)) => {
            let expected = verifier_edge(
                function,
                maps,
                arguments.iter().copied().map(EdgeSource::Value).collect(),
                available,
            )?;
            if *actual_edge != expected {
                return Err(plan_error(
                    function,
                    block,
                    "branch ownership transfer or cleanup is not derivable",
                ));
            }
        }
        (
            Terminator::CondBranch {
                then_arguments,
                else_arguments,
                ..
            },
            TerminatorOwnership::CondBranch {
                then_edge,
                else_edge,
            },
        ) => {
            let expected_then = verifier_edge(
                function,
                maps,
                then_arguments
                    .iter()
                    .copied()
                    .map(EdgeSource::Value)
                    .collect(),
                available,
            )?;
            let expected_else = verifier_edge(
                function,
                maps,
                else_arguments
                    .iter()
                    .copied()
                    .map(EdgeSource::Value)
                    .collect(),
                available,
            )?;
            if *then_edge != expected_then || *else_edge != expected_else {
                return Err(plan_error(
                    function,
                    block,
                    "conditional edge ownership is not derivable",
                ));
            }
        }
        (
            Terminator::SwitchVariant {
                scrutinee, arms, ..
            },
            TerminatorOwnership::SwitchVariant { arms: planned_arms },
        ) => {
            let sum = core_ir::value_type(function, *scrutinee)?;
            let CoreTypeKind::Sum { variants } = &core_ir::type_at(program, sum)?.kind else {
                return Err(plan_error(
                    function,
                    block,
                    "switch scrutinee is not a verified sum",
                ));
            };
            if planned_arms.len() != arms.len() || variants.len() != arms.len() {
                return Err(plan_error(
                    function,
                    block,
                    "switch ownership arm table has the wrong length",
                ));
            }
            for (arm_index, ((arm, variant), actual_edge)) in
                arms.iter().zip(variants).zip(planned_arms).enumerate()
            {
                let sources = arm
                    .arguments
                    .iter()
                    .map(|argument| match argument {
                        SwitchArgument::Value(value) => EdgeSource::Value(*value),
                        SwitchArgument::Payload => EdgeSource::Payload,
                    })
                    .collect::<Vec<_>>();
                let payload_managed = variant
                    .payload
                    .map(|ty| map_at(maps, ty).map(|map| !map.is_empty()))
                    .transpose()?
                    .unwrap_or(false);
                let mut expected = verifier_edge(function, maps, sources, available)?;
                for (source, action) in &mut expected.sources {
                    if *source == EdgeSource::Payload && payload_managed {
                        *action = UseAction::Share;
                    }
                }
                if value_is_managed(function, maps, *scrutinee)?
                    && !expected.drops.contains(scrutinee)
                {
                    expected.drops.push(*scrutinee);
                    expected.drops.sort();
                }
                if *actual_edge != expected {
                    return Err(plan_error(
                        function,
                        block,
                        &format!("switch arm {arm_index} ownership is not derivable"),
                    ));
                }
            }
        }
        _ => {
            return Err(plan_error(
                function,
                block,
                "terminator ownership kind does not match Core IR",
            ));
        }
    }
    Ok(())
}

fn verifier_edge(
    function: &CoreFunction,
    maps: &[ManagedReferenceMap],
    sources: Vec<EdgeSource>,
    available: &[ValueId],
) -> Result<EdgeOwnership> {
    let values = sources
        .iter()
        .filter_map(|source| match source {
            EdgeSource::Value(value) => Some(*value),
            EdgeSource::Payload => None,
        })
        .collect::<Vec<_>>();
    let mut remaining = occurrence_counts(&values);
    let mut transferred = BTreeSet::new();
    let mut planned = Vec::with_capacity(sources.len());
    for source in sources {
        let action = match source {
            EdgeSource::Value(value) if value_is_managed(function, maps, value)? => {
                let count = remaining
                    .get_mut(&value)
                    .ok_or_else(|| invalid("ownership verifier edge count is absent"))?;
                *count = count
                    .checked_sub(1)
                    .ok_or_else(|| invalid("ownership verifier edge count underflowed"))?;
                if *count == 0 {
                    transferred.insert(value);
                    UseAction::Transfer
                } else {
                    UseAction::Share
                }
            }
            EdgeSource::Value(_) | EdgeSource::Payload => UseAction::Immediate,
        };
        planned.push((source, action));
    }
    Ok(EdgeOwnership {
        sources: planned,
        drops: terminal_drops(function, maps, available, transferred)?,
    })
}

fn plan_error(function: &CoreFunction, block: &crate::core_ir::CoreBlock, detail: &str) -> LkError {
    invalid(&format!(
        "function {:?}, block {:?}: {detail}",
        function.origin, block.origin
    ))
    .for_node(block.origin)
}

fn invalid(message: &str) -> LkError {
    LkError::new(ErrorCode::OwnershipPlanInvalid, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core_ir::{
        BYTES_TYPE, BlockId, CoreBlock, CoreField, CoreFunction, CoreProgram, CoreType,
        CoreVariant, FunctionId, I64_TYPE, UNIT_TYPE,
    };
    use crate::ids::{NodeId, WorkspaceId};
    use crate::schema::{ByteString, SemanticType};

    fn node(serial: u64) -> NodeId {
        NodeId::new(WorkspaceId::from_bytes([0x6f; 16]), serial).unwrap()
    }

    fn primitives() -> Vec<crate::core_ir::CoreType> {
        SemanticType::PRIMITIVES
            .into_iter()
            .map(|semantic| crate::core_ir::CoreType {
                origin: None,
                kind: match semantic {
                    SemanticType::Unit => CoreTypeKind::Unit,
                    SemanticType::Bool => CoreTypeKind::Bool,
                    SemanticType::I64 => CoreTypeKind::I64,
                    SemanticType::Bytes => CoreTypeKind::Bytes,
                    SemanticType::Nominal(_) => unreachable!(),
                },
                layout: crate::type_layout::primitive_layout(semantic).unwrap(),
            })
            .collect()
    }

    fn concat_program() -> CoreProgram {
        CoreProgram {
            types: primitives(),
            entry: FunctionId(0),
            functions: vec![CoreFunction {
                origin: node(1),
                parameters: Vec::new(),
                result: BYTES_TYPE,
                value_types: vec![BYTES_TYPE, BYTES_TYPE, BYTES_TYPE, I64_TYPE],
                frame_cells: 4,
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(2),
                    parameters: Vec::new(),
                    instructions: vec![
                        Instruction::ConstBytes {
                            origin: node(3),
                            result: ValueId(0),
                            value: ByteString::from_slice(b"a").unwrap(),
                        },
                        Instruction::ConstBytes {
                            origin: node(4),
                            result: ValueId(1),
                            value: ByteString::from_slice(b"b").unwrap(),
                        },
                        Instruction::BytesConcat {
                            origin: node(5),
                            result: ValueId(2),
                            lhs: ValueId(0),
                            rhs: ValueId(1),
                        },
                        Instruction::BytesLen {
                            origin: node(6),
                            result: ValueId(3),
                            value: ValueId(2),
                        },
                    ],
                    terminator: Terminator::Return {
                        origin: node(7),
                        value: ValueId(2),
                    },
                }],
            }],
        }
    }

    #[test]
    fn maps_and_last_use_reuse_are_derived_and_malformed_plans_reject() {
        let program = concat_program();
        let plan = derive(&program).unwrap();
        assert_eq!(plan.managed_maps[BYTES_TYPE.0 as usize].paths.len(), 1);
        let concat = &plan.functions[0].blocks[0].instructions[2];
        assert!(concat.reuse_left);
        assert_eq!(concat.uses[0].1, UseAction::Borrow);
        assert_eq!(concat.drops_after, vec![ValueId(0), ValueId(1)]);
        assert_eq!(
            plan.functions[0].blocks[0].instructions[3].uses[0].1,
            UseAction::Borrow
        );

        let mut missing_drop = plan.clone();
        missing_drop.functions[0].blocks[0].instructions[2]
            .drops_after
            .pop();
        assert_eq!(
            verify(&program, &missing_drop).unwrap_err().code,
            ErrorCode::OwnershipPlanInvalid
        );

        let mut false_unique = plan.clone();
        false_unique.functions[0].blocks[0].instructions[0].reuse_left = true;
        assert_eq!(
            verify(&program, &false_unique).unwrap_err().code,
            ErrorCode::OwnershipPlanInvalid
        );

        let mut use_after_transfer = plan.clone();
        use_after_transfer.functions[0].blocks[0].instructions[2].uses[0].1 = UseAction::Transfer;
        assert_eq!(
            verify(&program, &use_after_transfer).unwrap_err().code,
            ErrorCode::OwnershipPlanInvalid
        );

        let mut double_drop = plan.clone();
        double_drop.functions[0].blocks[0].instructions[2]
            .drops_after
            .push(ValueId(0));
        assert_eq!(
            verify(&program, &double_drop).unwrap_err().code,
            ErrorCode::OwnershipPlanInvalid
        );

        let mut missing_cleanup_root = plan.clone();
        missing_cleanup_root.functions[0].blocks[0]
            .cleanup_roots
            .pop();
        assert_eq!(
            verify(&program, &missing_cleanup_root).unwrap_err().code,
            ErrorCode::OwnershipPlanInvalid
        );

        let mut dangling_return_borrow = plan.clone();
        let TerminatorOwnership::Return { action, .. } =
            &mut dangling_return_borrow.functions[0].blocks[0].terminator
        else {
            unreachable!()
        };
        *action = UseAction::Borrow;
        assert_eq!(
            verify(&program, &dangling_return_borrow).unwrap_err().code,
            ErrorCode::OwnershipPlanInvalid
        );
    }

    #[test]
    fn product_and_active_variant_maps_are_exact_and_rederived() {
        use crate::type_layout::{FieldLayout, LayoutShape, ValueLayout, VariantLayout};

        let product = CoreTypeId(4);
        let sum = CoreTypeId(5);
        let field = node(20);
        let empty = node(21);
        let payload = node(22);
        let mut types = primitives();
        types.push(CoreType {
            origin: Some(node(10)),
            kind: CoreTypeKind::Product {
                fields: vec![CoreField {
                    origin: field,
                    ty: BYTES_TYPE,
                    cell_offset: 0,
                }],
            },
            layout: ValueLayout {
                size: 8,
                align: 8,
                cells: 1,
                shape: LayoutShape::Product {
                    fields: vec![FieldLayout {
                        field,
                        offset: 0,
                        cells: 1,
                    }],
                },
            },
        });
        types.push(CoreType {
            origin: Some(node(11)),
            kind: CoreTypeKind::Sum {
                variants: vec![
                    CoreVariant {
                        origin: empty,
                        payload: None,
                        discriminant: 0,
                    },
                    CoreVariant {
                        origin: payload,
                        payload: Some(product),
                        discriminant: 1,
                    },
                ],
            },
            layout: ValueLayout {
                size: 16,
                align: 8,
                cells: 2,
                shape: LayoutShape::Sum {
                    discriminant_bytes: 1,
                    payload_offset: 8,
                    variants: vec![
                        VariantLayout {
                            variant: empty,
                            discriminant: 0,
                            payload_size: 0,
                            payload_align: 1,
                            payload_cells: 0,
                        },
                        VariantLayout {
                            variant: payload,
                            discriminant: 1,
                            payload_size: 8,
                            payload_align: 8,
                            payload_cells: 1,
                        },
                    ],
                },
            },
        });
        let program = CoreProgram {
            types,
            entry: FunctionId(0),
            functions: vec![CoreFunction {
                origin: node(30),
                parameters: vec![ValueId(0)],
                result: UNIT_TYPE,
                value_types: vec![sum, UNIT_TYPE],
                frame_cells: 2,
                entry: BlockId(0),
                blocks: vec![CoreBlock {
                    origin: node(31),
                    parameters: vec![ValueId(0)],
                    instructions: vec![Instruction::ConstUnit {
                        origin: node(32),
                        result: ValueId(1),
                    }],
                    terminator: Terminator::Return {
                        origin: node(33),
                        value: ValueId(1),
                    },
                }],
            }],
        };
        let plan = derive(&program).unwrap();
        assert_eq!(
            plan.managed_maps[product.0 as usize].paths,
            vec![ManagedCellPath {
                cell: 0,
                conditions: Vec::new(),
            }]
        );
        assert_eq!(
            plan.managed_maps[sum.0 as usize].paths,
            vec![ManagedCellPath {
                cell: 1,
                conditions: vec![DiscriminantCondition {
                    cell: 0,
                    variant: 1,
                }],
            }]
        );
        assert_eq!(plan.functions[0].blocks[0].entry_drops, vec![ValueId(0)]);

        let mut malformed = plan.clone();
        malformed.managed_maps[sum.0 as usize].paths[0].conditions[0].variant = 0;
        assert_eq!(
            verify(&program, &malformed).unwrap_err().code,
            ErrorCode::OwnershipPlanInvalid
        );
    }
}
