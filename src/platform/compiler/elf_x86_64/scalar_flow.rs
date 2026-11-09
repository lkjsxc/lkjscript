//! Disposable representation flow for native scalar lowering.
//!
//! The accepted normalized program owns semantic validity. This analysis only
//! proves where emitted words are raw I64 values, boxed pointers, or empty local
//! slots. A missing proof declines the optimization for the complete function.
//! No finite work/count allowance is imposed; allocation failure is a Resource
//! diagnostic. States are flat and retained only at basic-block entries.

use super::{I, NormalizedCode, NormalizedProgram, Result, reserve};
use crate::platform::kernel::{ParameterUse, TypeForm, TypeObjectDigest};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Representation {
    Boxed,
    I64,
}

impl Representation {
    /// Exact declared I64 signatures use raw words. A type parameter or any
    /// other declared form retains the boxed ABI; no speculative substitution.
    pub(super) fn of_type(program: &NormalizedProgram, ty: TypeObjectDigest) -> Option<Self> {
        Some(if matches!(program.types.get(&ty)?.form, TypeForm::I64) {
            Self::I64
        } else {
            Self::Boxed
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Local {
    Empty,
    Value(Representation),
    Maybe(Representation),
}

impl Local {
    fn joined(self, other: Self) -> Option<Self> {
        use Local::{Empty, Maybe, Value};
        match (self, other) {
            (Empty, Empty) => Some(Empty),
            (Empty, Value(value) | Maybe(value)) | (Value(value) | Maybe(value), Empty) => {
                Some(Maybe(value))
            }
            (Value(left), Value(right)) if left == right => Some(Value(left)),
            (Value(left) | Maybe(left), Value(right) | Maybe(right)) if left == right => {
                Some(Maybe(left))
            }
            _ => None,
        }
    }

    fn loaded(self) -> Option<Representation> {
        match self {
            Self::Value(value) => Some(value),
            Self::Empty | Self::Maybe(_) => None,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct State {
    operands: Vec<Representation>,
    locals: Vec<Local>,
}

#[derive(Clone, Copy)]
struct Signature {
    parameters: usize,
    result: Representation,
}

fn signature(program: &NormalizedProgram, index: usize) -> Option<Signature> {
    let function = program.functions.get(index)?;
    let parameters = usize::try_from(function.parameter_count).ok()?;
    if function.parameters.len() != parameters {
        return None;
    }
    Some(Signature {
        parameters,
        result: Representation::of_type(program, function.result.ty)?,
    })
}

impl State {
    fn initial(parameters: &[Representation], local_count: u32) -> Result<Option<Self>> {
        let Ok(count) = usize::try_from(local_count) else {
            return Ok(None);
        };
        if parameters.len() > count {
            return Ok(None);
        }
        let mut locals = Vec::new();
        reserve(&mut locals, count)?;
        locals.resize(count, Local::Empty);
        for (slot, representation) in locals.iter_mut().zip(parameters) {
            *slot = Local::Value(*representation);
        }
        Ok(Some(Self {
            operands: Vec::new(),
            locals,
        }))
    }

    pub(super) fn try_clone(&self) -> Result<Self> {
        let mut operands = Vec::new();
        reserve(&mut operands, self.operands.len())?;
        operands.extend_from_slice(&self.operands);
        let mut locals = Vec::new();
        reserve(&mut locals, self.locals.len())?;
        locals.extend_from_slice(&self.locals);
        Ok(Self { operands, locals })
    }

    pub(super) fn operands(&self) -> &[Representation] {
        &self.operands
    }

    pub(super) fn local(&self, index: u32) -> Option<Local> {
        self.locals.get(index as usize).copied()
    }

    /// Only boxed words can be probed as pointers by the existing owner scan.
    /// Unknown indices fail closed; derived flow never grants custody authority.
    pub(super) fn may_own(&self, index: u32) -> bool {
        !matches!(
            self.local(index),
            Some(
                Local::Empty
                    | Local::Value(Representation::I64)
                    | Local::Maybe(Representation::I64)
            )
        )
    }

    fn push(&mut self, representation: Representation) -> Result<()> {
        reserve(&mut self.operands, 1)?;
        self.operands.push(representation);
        Ok(())
    }

    fn replace_tail(&mut self, consumed: usize, produced: Option<Representation>) -> Result<bool> {
        let Some(start) = self.operands.len().checked_sub(consumed) else {
            return Ok(false);
        };
        self.operands.truncate(start);
        if let Some(produced) = produced {
            self.push(produced)?;
        }
        Ok(true)
    }

    fn boxed_top(&self) -> bool {
        self.operands.last() == Some(&Representation::Boxed)
    }

    fn callable_position(&self, arguments: u32) -> Option<usize> {
        let count = usize::try_from(arguments).ok()?.checked_add(1)?;
        let position = self.operands.len().checked_sub(count)?;
        (self.operands.get(position) == Some(&Representation::Boxed)).then_some(position)
    }

    /// Replay the same transfer used by derive, after the emitter has inspected
    /// its input representations. False means no representation proof.
    pub(super) fn advance(&mut self, program: &NormalizedProgram, instruction: &I) -> Result<bool> {
        self.advance_with(instruction, &|index| signature(program, index))
    }

    fn advance_with(
        &mut self,
        instruction: &I,
        call: &impl Fn(usize) -> Option<Signature>,
    ) -> Result<bool> {
        use Representation::{Boxed, I64};
        match instruction {
            I::Unit | I::Bool(_) | I::Text(_) | I::StaticText(_) | I::FunctionValue { .. } => {
                self.push(Boxed)?;
            }
            I::I64(_) => self.push(I64)?,
            I::LoadLocal { local, use_mode } => {
                let Some(value) = self.local(*local).and_then(Local::loaded) else {
                    return Ok(false);
                };
                if *use_mode == ParameterUse::Consume {
                    self.locals[*local as usize] = Local::Empty;
                }
                self.push(value)?;
            }
            I::MoveLocal(local) => {
                let Some(value) = self.local(*local).and_then(Local::loaded) else {
                    return Ok(false);
                };
                self.locals[*local as usize] = Local::Empty;
                self.push(value)?;
            }
            I::StoreLocal(local) => {
                let Some(slot) = self.locals.get_mut(*local as usize) else {
                    return Ok(false);
                };
                let Some(value) = self.operands.pop() else {
                    return Ok(false);
                };
                *slot = Local::Value(value);
            }
            I::Drop => {
                if self.operands.pop().is_none() {
                    return Ok(false);
                }
            }
            I::Jump(_) => {}
            I::JumpIfFalse(_) | I::SwitchVariant(_) => {
                if !self.boxed_top() {
                    return Ok(false);
                }
                self.operands.pop();
            }
            I::Call {
                function,
                arguments,
                ..
            }
            | I::TailCall {
                function,
                arguments,
                ..
            } => {
                let Some(target) = call(function.0 as usize) else {
                    return Ok(false);
                };
                let Ok(arguments) = usize::try_from(*arguments) else {
                    return Ok(false);
                };
                if arguments != target.parameters {
                    return Ok(false);
                }
                return self.replace_tail(arguments, Some(target.result));
            }
            I::BeginBind { .. } => {
                if !self.boxed_top() {
                    return Ok(false);
                }
            }
            I::Capture { .. } => {
                if self.operands.is_empty() {
                    return Ok(false);
                }
            }
            I::Bind { arguments } | I::Invoke { arguments } | I::TailInvoke { arguments } => {
                let Some(position) = self.callable_position(*arguments) else {
                    return Ok(false);
                };
                self.operands.truncate(position);
                self.push(Boxed)?;
            }
            I::List { items } => {
                let Ok(items) = usize::try_from(*items) else {
                    return Ok(false);
                };
                return self.replace_tail(items, Some(Boxed));
            }
            I::Record { fields, .. } => {
                return self.replace_tail(fields.len(), Some(Boxed));
            }
            I::Variant { has_payload, .. } => {
                return self.replace_tail(usize::from(*has_payload), Some(Boxed));
            }
            I::Field(_) => {
                if !self.boxed_top() {
                    return Ok(false);
                }
                self.operands.pop();
                self.push(Boxed)?;
            }
            I::Return => {
                if self.operands.len() != 1 {
                    return Ok(false);
                }
                self.operands.pop();
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    // The local lattice has finite height. A raw/boxed disagreement cannot be
    // represented without an edge conversion, so decline the whole function.
    fn join(&mut self, incoming: &Self) -> Option<bool> {
        if self.operands != incoming.operands || self.locals.len() != incoming.locals.len() {
            return None;
        }
        let mut changed = false;
        for (previous, next) in self.locals.iter_mut().zip(&incoming.locals) {
            let joined = previous.joined(*next)?;
            changed |= *previous != joined;
            *previous = joined;
        }
        Some(changed)
    }
}

struct Block {
    start: usize,
    end: usize,
    state: Option<State>,
}

pub(super) enum Entry<'a> {
    Continue,
    Reachable(&'a State),
    Unreachable,
}

pub(super) struct Plan {
    blocks: Vec<Block>,
}

impl Plan {
    /// The code must already contain the exact expanded/remapped native rewrite
    /// stream. The plan is private to this exact function/application, not to a
    /// shared instruction Arc or a source declaration alone.
    pub(super) fn derive(
        program: &NormalizedProgram,
        function: usize,
        code: &NormalizedCode,
    ) -> Result<Option<Self>> {
        let Some(function) = program.functions.get(function) else {
            return Ok(None);
        };
        if function.parameter_count != code.parameter_count
            || usize::try_from(code.parameter_count).ok() != Some(function.parameters.len())
        {
            return Ok(None);
        }
        let mut parameters = Vec::new();
        reserve(&mut parameters, function.parameters.len())?;
        for parameter in function.parameters.iter() {
            let Some(representation) = Representation::of_type(program, parameter.ty) else {
                return Ok(None);
            };
            parameters.push(representation);
        }
        let Some(initial) = State::initial(&parameters, code.local_count)? else {
            return Ok(None);
        };
        Self::from_state(code, initial, &|index| signature(program, index))
    }

    /// None of an unreachable block's representation facts are consumed by
    /// reachable code. The emitter can retain boxed lowering for those bytes.
    pub(super) fn entry(&self, instruction: usize) -> Entry<'_> {
        match self
            .blocks
            .binary_search_by_key(&instruction, |block| block.start)
        {
            Ok(index) => match &self.blocks[index].state {
                Some(state) => Entry::Reachable(state),
                None => Entry::Unreachable,
            },
            Err(_) => Entry::Continue,
        }
    }

    fn from_state(
        code: &NormalizedCode,
        initial: State,
        call: &impl Fn(usize) -> Option<Signature>,
    ) -> Result<Option<Self>> {
        let count = code.instructions.len();
        if count == 0 || usize::try_from(code.local_count).ok() != Some(initial.locals.len()) {
            return Ok(None);
        }
        let mut leaders = Vec::new();
        reserve(&mut leaders, count)?;
        leaders.resize(count, false);
        leaders[0] = true;
        for (ip, instruction) in code.instructions.iter().enumerate() {
            let end = match instruction {
                I::Jump(target) | I::JumpIfFalse(target) => {
                    if !mark_leader(&mut leaders, *target) {
                        return Ok(None);
                    }
                    true
                }
                I::SwitchVariant(arms) => {
                    if arms.is_empty() {
                        return Ok(None);
                    }
                    for arm in arms.iter() {
                        if !mark_leader(&mut leaders, arm.target) {
                            return Ok(None);
                        }
                    }
                    true
                }
                I::Return => true,
                _ => false,
            };
            if end && ip + 1 < count {
                leaders[ip + 1] = true;
            }
        }
        let mut blocks = Vec::new();
        reserve(
            &mut blocks,
            leaders.iter().filter(|leader| **leader).count(),
        )?;
        for (start, leader) in leaders.into_iter().enumerate() {
            if leader {
                blocks.push(Block {
                    start,
                    end: count,
                    state: None,
                });
            }
        }
        for index in 0..blocks.len() {
            blocks[index].end = blocks.get(index + 1).map_or(count, |block| block.start);
        }
        blocks[0].state = Some(initial);
        let mut queued = Vec::new();
        reserve(&mut queued, blocks.len())?;
        queued.resize(blocks.len(), false);
        let mut pending = Vec::new();
        reserve(&mut pending, blocks.len())?;
        pending.push(0);
        queued[0] = true;
        while let Some(index) = pending.pop() {
            queued[index] = false;
            let Some(entry) = &blocks[index].state else {
                return Ok(None);
            };
            let mut state = entry.try_clone()?;
            let start = blocks[index].start;
            let end = blocks[index].end;
            for instruction in &code.instructions[start..end] {
                if !state.advance_with(instruction, call)? {
                    return Ok(None);
                }
            }
            match &code.instructions[end - 1] {
                I::Return => {}
                I::Jump(target) => {
                    if !merge(
                        &mut blocks,
                        *target as usize,
                        &state,
                        &mut pending,
                        &mut queued,
                    )? {
                        return Ok(None);
                    }
                }
                I::JumpIfFalse(target) => {
                    for target in [*target as usize, end] {
                        if !merge(&mut blocks, target, &state, &mut pending, &mut queued)? {
                            return Ok(None);
                        }
                    }
                }
                I::SwitchVariant(arms) => {
                    for arm in arms.iter() {
                        let previous = if let Some(local) = arm.binding_local {
                            let Some(slot) = state.locals.get_mut(local as usize) else {
                                return Ok(None);
                            };
                            let previous = *slot;
                            *slot = Local::Value(Representation::Boxed);
                            Some((local, previous))
                        } else {
                            None
                        };
                        let merged = merge(
                            &mut blocks,
                            arm.target as usize,
                            &state,
                            &mut pending,
                            &mut queued,
                        )?;
                        if let Some((local, value)) = previous {
                            state.locals[local as usize] = value;
                        }
                        if !merged {
                            return Ok(None);
                        }
                    }
                }
                _ => {
                    // Tail calls retain this edge: a live owner or external
                    // indirect target can return through lexical cleanup.
                    if !merge(&mut blocks, end, &state, &mut pending, &mut queued)? {
                        return Ok(None);
                    }
                }
            }
        }
        Ok(Some(Self { blocks }))
    }
}

fn mark_leader(leaders: &mut [bool], target: u32) -> bool {
    let Some(leader) = leaders.get_mut(target as usize) else {
        return false;
    };
    *leader = true;
    true
}

fn merge(
    blocks: &mut [Block],
    target: usize,
    incoming: &State,
    pending: &mut Vec<usize>,
    queued: &mut [bool],
) -> Result<bool> {
    let Ok(index) = blocks.binary_search_by_key(&target, |block| block.start) else {
        return Ok(false);
    };
    let changed = if let Some(previous) = &mut blocks[index].state {
        let Some(changed) = previous.join(incoming) else {
            return Ok(false);
        };
        changed
    } else {
        blocks[index].state = Some(incoming.try_clone()?);
        true
    };
    if changed && !queued[index] {
        reserve(pending, 1)?;
        pending.push(index);
        queued[index] = true;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(parameters: u32, locals: u32, instructions: Vec<I>) -> NormalizedCode {
        NormalizedCode {
            parameter_count: parameters,
            local_count: locals,
            instructions: instructions.into(),
        }
    }

    fn derive(code: &NormalizedCode, parameters: &[Representation]) -> Option<Plan> {
        let state = State::initial(parameters, code.local_count)
            .unwrap()
            .unwrap();
        Plan::from_state(code, state, &|_| None).unwrap()
    }

    fn entry(plan: &Plan, ip: usize) -> &State {
        let Entry::Reachable(state) = plan.entry(ip) else {
            panic!("expected a reachable block");
        };
        state
    }

    #[test]
    fn loop_join_keeps_empty_or_raw_local_out_of_pointer_scan() {
        let code = code(
            1,
            2,
            vec![
                I::Bool(true),
                I::JumpIfFalse(5),
                I::I64(1234),
                I::StoreLocal(1),
                I::Jump(0),
                I::LoadLocal {
                    local: 0,
                    use_mode: ParameterUse::Unrestricted,
                },
                I::Return,
            ],
        );
        let plan = derive(&code, &[Representation::I64]).unwrap();
        assert_eq!(
            entry(&plan, 0).local(1),
            Some(Local::Maybe(Representation::I64))
        );
        assert!(!entry(&plan, 0).may_own(1));
        assert!(!entry(&plan, 5).may_own(1));
    }

    #[test]
    fn redefinition_kills_maybe_empty_state_before_the_load() {
        let mut code = code(
            0,
            1,
            vec![
                I::Bool(true),
                I::JumpIfFalse(5),
                I::I64(1),
                I::StoreLocal(0),
                I::Jump(5),
                I::I64(17),
                I::StoreLocal(0),
                I::LoadLocal {
                    local: 0,
                    use_mode: ParameterUse::Unrestricted,
                },
                I::Return,
            ],
        );
        assert!(derive(&code, &[]).is_some());
        // The same join cannot authorize reading zero as a language I64 on
        // the branch which never initialized this slot.
        code.instructions = vec![
            I::Bool(true),
            I::JumpIfFalse(5),
            I::I64(1),
            I::StoreLocal(0),
            I::Jump(5),
            I::LoadLocal {
                local: 0,
                use_mode: ParameterUse::Unrestricted,
            },
            I::Return,
        ]
        .into();
        assert!(derive(&code, &[]).is_none());
    }

    #[test]
    fn incompatible_local_and_operand_joins_decline_the_complete_plan() {
        let local = code(
            0,
            1,
            vec![
                I::Bool(true),
                I::JumpIfFalse(5),
                I::I64(1),
                I::StoreLocal(0),
                I::Jump(7),
                I::Unit,
                I::StoreLocal(0),
                I::I64(2),
                I::Return,
            ],
        );
        let operand = code(
            0,
            0,
            vec![
                I::Bool(true),
                I::JumpIfFalse(4),
                I::I64(1),
                I::Jump(5),
                I::Unit,
                I::Return,
            ],
        );
        assert!(derive(&local, &[]).is_none());
        assert!(derive(&operand, &[]).is_none());
    }

    #[test]
    fn unreachable_expanded_continuation_has_no_reachable_state() {
        let code = code(0, 0, vec![I::I64(7), I::Return, I::Unit, I::Return]);
        let plan = derive(&code, &[]).unwrap();
        assert!(matches!(plan.entry(0), Entry::Reachable(_)));
        assert!(matches!(plan.entry(1), Entry::Continue));
        assert!(matches!(plan.entry(2), Entry::Unreachable));
    }
}
