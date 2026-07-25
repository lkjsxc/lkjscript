use super::*;

pub(super) fn successor_reference_live(
    block: &crate::plan::Block,
    live_in: &[BTreeSet<LiveHome>],
    work: &mut u64,
    maximum_work: u64,
) -> Result<BTreeSet<LiveHome>, VerificationError> {
    let mut live = BTreeSet::new();
    let terminator = block
        .terminator
        .as_ref()
        .ok_or(VerificationError::MissingTerminator(block.id))?;
    let successors: Vec<BlockId> = match terminator {
        Terminator::Branch(target) => vec![*target],
        Terminator::BranchIf {
            when_true,
            when_false,
            ..
        } => vec![*when_true, *when_false],
        Terminator::Return(_)
        | Terminator::Trap { .. }
        | Terminator::Exit(_)
        | Terminator::Outcome(_) => Vec::new(),
    };
    for successor in successors {
        let successor_live = live_in
            .get(successor.index as usize)
            .ok_or(VerificationError::InvalidTarget(successor))?;
        for home in successor_live {
            charge_liveness(work, maximum_work)?;
            live.insert(*home);
        }
    }
    Ok(live)
}

pub(super) fn add_terminator_references(
    function: &FunctionPlan,
    block: &crate::plan::Block,
    live: &mut BTreeSet<LiveHome>,
    work: &mut u64,
    maximum_work: u64,
) -> Result<(), VerificationError> {
    let terminator = block
        .terminator
        .as_ref()
        .ok_or(VerificationError::MissingTerminator(block.id))?;
    for operand in terminator.operands() {
        if value_reference_type(function, operand).is_some() {
            charge_liveness(work, maximum_work)?;
            live.insert(LiveHome::Value(operand.index));
        }
    }
    Ok(())
}

pub(super) fn transfer_reference_liveness(
    function: &FunctionPlan,
    instruction: &crate::plan::Instruction,
    live: &mut BTreeSet<LiveHome>,
    work: &mut u64,
    maximum_work: u64,
) -> Result<(), VerificationError> {
    let output_live = instruction.output_type.reference_type().is_some()
        && live.remove(&LiveHome::Value(instruction.output.index));
    match &instruction.operation {
        Operation::WriteLocal(local, value) => {
            if function.locals[local.index as usize]
                .value_type
                .reference_type()
                .is_some()
            {
                live.remove(&LiveHome::Local(local.index));
            }
            insert_reference_value(function, *value, live, work, maximum_work)?;
        }
        Operation::ReadLocal(local) if output_live => {
            if function.locals[local.index as usize]
                .value_type
                .reference_type()
                .is_some()
            {
                charge_liveness(work, maximum_work)?;
                live.insert(LiveHome::Local(local.index));
            }
        }
        Operation::Call(_, arguments)
        | Operation::RuntimeCall(_, arguments)
        | Operation::HeapCall(_, arguments) => {
            for operand in arguments {
                insert_reference_value(function, *operand, live, work, maximum_work)?;
            }
        }
        _ if output_live => {
            for operand in instruction.operation.operands() {
                insert_reference_value(function, operand, live, work, maximum_work)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn insert_reference_value(
    function: &FunctionPlan,
    value: ValueId,
    live: &mut BTreeSet<LiveHome>,
    work: &mut u64,
    maximum_work: u64,
) -> Result<(), VerificationError> {
    if value_reference_type(function, value).is_some() {
        charge_liveness(work, maximum_work)?;
        live.insert(LiveHome::Value(value.index));
    }
    Ok(())
}

pub(super) fn value_reference_type(
    function: &FunctionPlan,
    value: ValueId,
) -> Option<ReferenceType> {
    function
        .values
        .get(value.index as usize)
        .and_then(|fact| fact.value_type.reference_type())
}

pub(super) fn push_certified_root(
    function: &FunctionPlan,
    home: LiveHome,
    roots: &mut Vec<CertifiedRoot>,
    work: &mut u64,
    maximum_work: u64,
    root_records: &mut u64,
    maximum_root_records: u64,
) -> Result<(), VerificationError> {
    charge_liveness(work, maximum_work)?;
    *root_records = root_records
        .checked_add(1)
        .ok_or(VerificationError::LimitExceeded("stack-map root metadata"))?;
    if *root_records > maximum_root_records {
        return Err(VerificationError::LimitExceeded("stack-map root metadata"));
    }
    roots
        .try_reserve(1)
        .map_err(|_| VerificationError::LimitExceeded("stack-map root metadata"))?;
    let (kind, reference_type) = match home {
        LiveHome::Local(index) => {
            let reference_type = function
                .locals
                .get(index as usize)
                .and_then(|fact| fact.value_type.reference_type())
                .ok_or(VerificationError::InvalidLocal(LocalId {
                    function: function.id,
                    index,
                }))?;
            (crate::FrameHomeKind::Local(index), reference_type)
        }
        LiveHome::Value(index) => {
            let value = ValueId {
                function: function.id,
                index,
            };
            let reference_type = value_reference_type(function, value)
                .ok_or(VerificationError::InvalidValue(value))?;
            (crate::FrameHomeKind::Value(index), reference_type)
        }
    };
    roots.push(CertifiedRoot {
        kind,
        reference_type,
    });
    Ok(())
}

pub(super) fn charge_liveness(work: &mut u64, maximum: u64) -> Result<(), VerificationError> {
    *work = work
        .checked_add(1)
        .ok_or(VerificationError::LimitExceeded("stack-map liveness work"))?;
    if *work > maximum {
        return Err(VerificationError::LimitExceeded("stack-map liveness work"));
    }
    Ok(())
}
