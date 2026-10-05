//! Bounded, disposable liveness of the current value in each ordinary local.
//! Stores kill the previous value; switch payload writes kill only their edge.
//! A fixed point handles cycles. An unfinished proof never authorizes a move.

use super::super::prepare::{
    NormalizedBorrowedOwnedChoiceJump, NormalizedOwnedChoiceJump, NormalizedVariantJump,
};
use super::{Budget, Diagnostic, I, NormalizedCode, ParameterUse, corrupt};
use std::sync::Arc;

const MAXIMUM_BYTES: usize = 8 * 1024 * 1024;
const MAXIMUM_STEPS: usize = 1_000_000;

#[derive(Clone, Copy)]
enum Edges<'a> {
    Next,
    Jump(u32),
    Branch(u32),
    Switch(&'a [NormalizedVariantJump]),
    Choice(&'a [NormalizedOwnedChoiceJump]),
    BorrowedChoice(&'a [NormalizedBorrowedOwnedChoiceJump]),
    Exit,
}

struct Flow<'a> {
    read: Option<u32>,
    read_extra: Option<u32>,
    write: Option<u32>,
    write_extra: Option<u32>,
    writes: &'a [u32],
    edges: Edges<'a>,
}

fn flow(instruction: &I) -> Flow<'_> {
    // Exhaustive on purpose: both implicit local accesses and successors must
    // be classified when an instruction is added. A derived move is still a read.
    let (read, write, edges) = match instruction {
        I::LoadLocal { local, .. } | I::MoveLocal(local) => (Some(*local), None, Edges::Next),
        I::SequenceLength { source_local, .. } | I::BeginBorrowCall { source_local, .. } => {
            (Some(*source_local), None, Edges::Next)
        }
        I::SequencePush { source_local, .. } | I::SequencePop { source_local, .. } => {
            (Some(*source_local), Some(*source_local), Edges::Next)
        }
        I::StoreLocal(local) => (None, Some(*local), Edges::Next),
        // Begin observes the old slot's emptiness before defining its token.
        // Moving an earlier value must not turn that guard into acceptance.
        I::BeginTransaction { binding: local, .. }
        | I::BeginParameterTransaction { binding: local, .. }
        | I::BeginTransactionOutcome { binding: local, .. } => {
            (Some(*local), Some(*local), Edges::Next)
        }
        I::CommitTransaction { binding, .. }
        | I::CommitParameterTransaction { binding, .. }
        | I::CommitTransactionOutcome { binding, .. } => (Some(*binding), None, Edges::Next),
        I::Jump(target) => (None, None, Edges::Jump(*target)),
        I::JumpIfFalse(target) => (None, None, Edges::Branch(*target)),
        I::SwitchVariant(jumps) => (None, None, Edges::Switch(jumps)),
        I::MatchOwned { cases, .. } => (None, None, Edges::Choice(cases)),
        I::BorrowOwnedField {
            source_local,
            binding_local,
            ..
        }
        | I::BorrowOwnedItem {
            source_local,
            binding_local,
            ..
        }
        | I::AdoptBorrowResult {
            source_local,
            binding_local,
            ..
        } => (Some(*source_local), Some(*binding_local), Edges::Next),
        I::MatchBorrowedOwned {
            source_local,
            cases,
            ..
        } => (Some(*source_local), None, Edges::BorrowedChoice(cases)),
        I::EndOwnedBorrow { binding_local } => (None, Some(*binding_local), Edges::Next),
        I::Return | I::ReturnBorrowed | I::TailCall { .. } => (None, None, Edges::Exit),
        // Dynamic external callees return to this frame; graph callees transfer.
        // Without callee proof, preserve the possible continuation's live values.
        I::TailInvoke { .. } => (None, None, Edges::Next),
        I::Unit
        | I::SequenceEmpty { .. }
        | I::ChooseOwned { .. }
        | I::PackOwned { .. }
        | I::UnpackOwned { .. }
        | I::Bool(_)
        | I::I64(_)
        | I::F64(_)
        | I::Text(_)
        | I::StaticText(_)
        | I::Drop
        | I::ImplementationCall { .. }
        | I::MethodCall { .. }
        | I::Call { .. }
        | I::Parallel { .. }
        | I::FunctionValue { .. }
        | I::Invoke { .. }
        | I::Bind { .. }
        | I::BeginBind { .. }
        | I::Capture { .. }
        | I::Record { .. }
        | I::Variant { .. }
        | I::Field(_)
        | I::List { .. }
        | I::Map { .. }
        | I::Perform { .. }
        | I::PerformParameter { .. } => (None, None, Edges::Next),
    };
    let writes = match instruction {
        I::UnpackOwned { locals, .. } => locals.as_ref(),
        _ => &[],
    };
    Flow {
        read,
        read_extra: match instruction {
            I::SequencePush { value_local, .. } => Some(*value_local),
            I::BorrowOwnedField { binding_local, .. }
            | I::BorrowOwnedItem { binding_local, .. }
            | I::AdoptBorrowResult { binding_local, .. } => Some(*binding_local),
            _ => None,
        },
        write,
        write_extra: match instruction {
            I::SequencePush { value_local, .. } => Some(*value_local),
            _ => None,
        },
        writes,
        edges,
    }
}

fn local(code: &NormalizedCode, local: u32) -> Result<(), Diagnostic> {
    if local >= code.local_count {
        return Err(corrupt("local liveness encountered a foreign local"));
    }
    Ok(())
}

fn validate(code: &NormalizedCode, work: &mut Budget<'_>) -> Result<bool, Diagnostic> {
    let mut backward = false;
    for (pc, instruction) in code.instructions.iter().enumerate() {
        work.step()?;
        let access = flow(instruction);
        for operand in [
            access.read,
            access.read_extra,
            access.write,
            access.write_extra,
        ]
        .into_iter()
        .flatten()
        {
            local(code, operand)?;
        }
        for operand in access.writes {
            work.step()?;
            local(code, *operand)?;
        }
        let mut target = |destination: u32| -> Result<(), Diagnostic> {
            work.step()?;
            if destination as usize >= code.instructions.len() {
                return Err(corrupt(
                    "local liveness encountered a foreign jump destination",
                ));
            }
            backward |= destination as usize <= pc;
            Ok(())
        };
        match access.edges {
            Edges::Jump(to) | Edges::Branch(to) => target(to)?,
            Edges::Switch(jumps) => {
                for jump in jumps {
                    target(jump.target)?;
                    if let Some(binding) = jump.binding_local {
                        local(code, binding)?;
                    }
                }
            }
            Edges::Choice(cases) => {
                for case in cases {
                    target(case.target)?;
                    local(code, case.binding_local)?;
                }
            }
            Edges::BorrowedChoice(cases) => {
                for case in cases {
                    target(case.target)?;
                    local(code, case.binding_local)?;
                }
            }
            Edges::Next | Edges::Exit => {}
        }
    }
    Ok(backward)
}

struct Meter(usize);

impl Meter {
    fn step(&mut self, work: &mut Budget<'_>) -> Result<bool, Diagnostic> {
        if self.0 == 0 {
            return Ok(false);
        }
        work.step()?;
        self.0 -= 1;
        Ok(true)
    }
}

fn mask(local: Option<u32>, word: usize) -> u64 {
    local
        .filter(|local| *local as usize / 64 == word)
        .map_or(0, |local| 1_u64 << (local % 64))
}

struct Live {
    words: usize,
    before: Vec<u64>,
}

impl Live {
    fn outgoing(
        &self,
        edges: Edges<'_>,
        pc: usize,
        word: usize,
        meter: &mut Meter,
        work: &mut Budget<'_>,
    ) -> Result<Option<u64>, Diagnostic> {
        if !meter.step(work)? {
            return Ok(None);
        }
        let at = |pc: usize| {
            self.before
                .get(pc * self.words + word)
                .copied()
                .unwrap_or(0)
        };
        Ok(Some(match edges {
            Edges::Next => at(pc + 1),
            Edges::Jump(target) => at(target as usize),
            Edges::Branch(target) => at(pc + 1) | at(target as usize),
            Edges::Switch(jumps) => {
                let mut out = 0;
                for jump in jumps {
                    if !meter.step(work)? {
                        return Ok(None);
                    }
                    out |= at(jump.target as usize) & !mask(jump.binding_local, word);
                }
                out
            }
            Edges::Choice(cases) => {
                let mut out = 0;
                for case in cases {
                    if !meter.step(work)? {
                        return Ok(None);
                    }
                    out |= at(case.target as usize) & !mask(Some(case.binding_local), word);
                }
                out
            }
            Edges::BorrowedChoice(cases) => {
                let mut out = 0;
                for case in cases {
                    if !meter.step(work)? {
                        return Ok(None);
                    }
                    out |= at(case.target as usize) & !mask(Some(case.binding_local), word);
                }
                out
            }
            Edges::Exit => 0,
        }))
    }
}

pub(super) fn derive(code: &mut NormalizedCode, work: &mut Budget<'_>) -> Result<bool, Diagnostic> {
    derive_with_limits(code, work, MAXIMUM_BYTES, MAXIMUM_STEPS)
}

pub(super) fn derive_with_limits(
    code: &mut NormalizedCode,
    work: &mut Budget<'_>,
    maximum_bytes: usize,
    maximum_steps: usize,
) -> Result<bool, Diagnostic> {
    let backward = validate(code, work)?;
    let words = (code.local_count as usize).div_ceil(64);
    let Some(cells) = code.instructions.len().checked_mul(words) else {
        return Ok(false);
    };
    // Both the matrix and the complete rewrite plan are reserved before growth.
    let Some(bytes) = cells
        .checked_mul(std::mem::size_of::<u64>())
        .and_then(|bytes| bytes.checked_add(code.instructions.len()))
    else {
        return Ok(false);
    };
    if maximum_steps == 0 || bytes > maximum_bytes || !work.try_reserve::<u8>(bytes) {
        return Ok(false);
    }
    let mut live = Live {
        words,
        before: vec![0; cells],
    };
    let mut moves = vec![0_u8; code.instructions.len()];
    let mut meter = Meter(maximum_steps);
    loop {
        let mut changed = false;
        for (pc, instruction) in code.instructions.iter().enumerate().rev() {
            if !meter.step(work)? {
                return Ok(false);
            }
            let access = flow(instruction);
            for word in 0..words {
                let Some(out) = live.outgoing(access.edges, pc, word, &mut meter, work)? else {
                    return Ok(false);
                };
                let mut kills = mask(access.write, word) | mask(access.write_extra, word);
                for local in access.writes {
                    if !meter.step(work)? {
                        return Ok(false);
                    }
                    kills |= mask(Some(*local), word);
                }
                let mut reads = mask(access.read, word) | mask(access.read_extra, word);
                // A borrowed choice checks the selected destination's prior occupancy
                // before replacing it. Preserve every possible selected guard.
                if let Edges::BorrowedChoice(cases) = access.edges {
                    for case in cases {
                        if !meter.step(work)? {
                            return Ok(false);
                        }
                        reads |= mask(Some(case.binding_local), word);
                    }
                }
                let before = (out & !kills) | reads;
                let slot = &mut live.before[pc * words + word];
                changed |= *slot != before;
                *slot = before;
            }
        }
        // Forward-only code is solved in one reverse pass; cycles need a fixed point.
        if !backward || !changed {
            break;
        }
    }
    for (pc, instruction) in code.instructions.iter().enumerate() {
        if !meter.step(work)? {
            return Ok(false);
        }
        if let I::LoadLocal {
            local,
            use_mode: ParameterUse::Unrestricted,
        }
        | I::MoveLocal(local) = instruction
        {
            let Some(out) = live.outgoing(
                flow(instruction).edges,
                pc,
                *local as usize / 64,
                &mut meter,
                work,
            )?
            else {
                return Ok(false);
            };
            moves[pc] = u8::from(out & (1_u64 << (*local % 64)) == 0);
        }
    }
    // Never publish partial fixed-point/plan results, including on advisory exhaustion.
    for (pc, terminal) in moves.into_iter().enumerate() {
        work.step()?;
        let local = match code.instructions[pc] {
            I::LoadLocal {
                local,
                use_mode: ParameterUse::Unrestricted,
            }
            | I::MoveLocal(local) => local,
            _ => continue,
        };
        let replacement = if terminal != 0 {
            I::MoveLocal(local)
        } else {
            I::LoadLocal {
                local,
                use_mode: ParameterUse::Unrestricted,
            }
        };
        if replacement != code.instructions[pc] {
            if Arc::get_mut(&mut code.instructions).is_none() {
                work.reserve::<I>(code.instructions.len())?;
            }
            Arc::make_mut(&mut code.instructions)[pc] = replacement;
        }
    }
    Ok(true)
}
