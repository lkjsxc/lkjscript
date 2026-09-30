//! Terminal reads of ordinary locals, derived from admitted control flow.
//! Bounded liveness proves branch-local and redefinition-sensitive last uses.
//! When its advisory capacity is exhausted, the linear last-read/backward-edge
//! proof remains a conservative fallback. Neither proof is a language ownership
//! contract, a borrow proof, or wire authority.

#[path = "local_liveness.rs"]
mod liveness;

use super::prepare::{
    NormalizedCode, NormalizedEntryPoint, NormalizedFunction, NormalizedFunctionBody,
    NormalizedInstruction as I, NormalizedPort, NormalizedProgram,
};
use super::prepared_types::Budget;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::ParameterUse;
use std::sync::Arc;

pub(super) fn derive_program(
    program: &mut NormalizedProgram,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    work.step()?;
    if Arc::get_mut(&mut program.functions).is_none() {
        work.reserve::<NormalizedFunction>(program.functions.len())?;
    }
    for function in Arc::make_mut(&mut program.functions) {
        work.step()?;
        if let NormalizedFunctionBody::Code(code) = &mut function.body {
            derive(code, work)?;
        }
    }
    if Arc::get_mut(&mut program.ports).is_none() {
        work.reserve::<NormalizedPort>(program.ports.len())?;
    }
    for port in Arc::make_mut(&mut program.ports) {
        work.step()?;
        match &mut port.entry {
            NormalizedEntryPoint::Code(code) | NormalizedEntryPoint::PortExpression(code, _) => {
                derive(code, work)?;
            }
            NormalizedEntryPoint::Function(_) => {}
            #[cfg(test)]
            NormalizedEntryPoint::InstantiatedFunction(_, _) => {}
        }
    }
    for test in program.tests.values_mut() {
        work.step()?;
        derive(&mut test.actual, work)?;
        derive(&mut test.expected, work)?;
    }
    Ok(())
}

fn corrupt(message: &'static str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Corrupt,
        "normalized_local_move_code",
        message,
    )
}

fn edge(
    ends: &mut [usize],
    source: usize,
    target: u32,
    work: &mut Budget<'_>,
) -> Result<(), Diagnostic> {
    work.step()?;
    let end = ends
        .get_mut(target as usize)
        .ok_or_else(|| corrupt("local move analysis encountered a foreign jump destination"))?;
    if target as usize <= source {
        // Exclusive end also protects a self-edge at instruction zero.
        *end = (*end).max(source + 1);
    }
    Ok(())
}

pub(super) fn derive(code: &mut NormalizedCode, work: &mut Budget<'_>) -> Result<(), Diagnostic> {
    work.step()?;
    if liveness::derive(code, work)? {
        return Ok(());
    }
    derive_linear(code, work)
}

fn derive_linear(code: &mut NormalizedCode, work: &mut Budget<'_>) -> Result<(), Diagnostic> {
    work.step()?;
    work.reserve::<Option<usize>>(code.local_count as usize)?;
    work.reserve::<usize>(code.instructions.len())?;
    let mut last = vec![None; code.local_count as usize];
    let mut ends = vec![0; code.instructions.len()];
    for (index, instruction) in code.instructions.iter().enumerate() {
        work.step()?;
        // Exhaustive on purpose: new instructions must classify implicit local
        // reads and non-fallthrough successors before they may use this proof.
        let read = match instruction {
            I::LoadLocal { local, .. } | I::MoveLocal(local) => Some(*local),
            // Begin observes whether the slot is occupied before defining its
            // token. The fallback must preserve that guard just like a value read.
            I::BeginTransaction { binding, .. }
            | I::BeginParameterTransaction { binding, .. }
            | I::BeginTransactionOutcome { binding, .. }
            | I::CommitTransaction { binding, .. }
            | I::CommitParameterTransaction { binding, .. }
            | I::CommitTransactionOutcome { binding, .. } => Some(*binding),
            I::Jump(target) | I::JumpIfFalse(target) => {
                edge(&mut ends, index, *target, work)?;
                None
            }
            I::SwitchVariant(jumps) => {
                for jump in jumps.iter() {
                    edge(&mut ends, index, jump.target, work)?;
                }
                None
            }
            I::Unit
            | I::Bool(_)
            | I::I64(_)
            | I::F64(_)
            | I::Text(_)
            | I::StaticText(_)
            | I::StoreLocal(_)
            | I::Drop
            | I::ImplementationCall { .. }
            | I::MethodCall { .. }
            | I::Call { .. }
            | I::TailCall { .. }
            | I::FunctionValue { .. }
            | I::Invoke { .. }
            | I::Bind { .. }
            | I::BeginBind { .. }
            | I::Capture { .. }
            | I::TailInvoke { .. }
            | I::Record { .. }
            | I::Variant { .. }
            | I::Field(_)
            | I::List { .. }
            | I::Map { .. }
            | I::Perform { .. }
            | I::PerformParameter { .. }
            | I::Return => None,
        };
        if let Some(local) = read {
            *last
                .get_mut(local as usize)
                .ok_or_else(|| corrupt("local move analysis encountered a foreign local"))? =
                Some(index);
        }
    }
    let mut protected_until = 0;
    for (index, end) in ends.into_iter().enumerate() {
        work.step()?;
        protected_until = protected_until.max(end);
        let local = match code.instructions[index] {
            I::LoadLocal {
                local,
                use_mode: ParameterUse::Unrestricted,
            }
            | I::MoveLocal(local) => local,
            _ => continue,
        };
        let terminal = index >= protected_until && last[local as usize] == Some(index);
        let replacement = if terminal {
            I::MoveLocal(local)
        } else {
            // Re-derivation must also invalidate an earlier derived move.
            I::LoadLocal {
                local,
                use_mode: ParameterUse::Unrestricted,
            }
        };
        if replacement != code.instructions[index] {
            if Arc::get_mut(&mut code.instructions).is_none() {
                work.reserve::<I>(code.instructions.len())?;
            }
            Arc::make_mut(&mut code.instructions)[index] = replacement;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "local_moves_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "local_moves_flow_tests.rs"]
mod flow_tests;
