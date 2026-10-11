#![allow(clippy::unwrap_used, clippy::expect_used)]

use super::super::prepare::NormalizedVariantJump;
use super::super::value::{ValueOrigin, VariantLayoutIndex};
use super::*;
use crate::platform::execution::ExecutionControl;

pub(super) fn load(local: u32) -> I {
    I::LoadLocal {
        local,
        use_mode: ParameterUse::Unrestricted,
    }
}

pub(super) fn code(instructions: Vec<I>) -> NormalizedCode {
    NormalizedCode {
        parameter_count: 2,
        local_count: 2,
        instructions: instructions.into(),
    }
}

pub(super) fn optimize(code: &mut NormalizedCode) {
    derive(code, &mut Budget::new(&ExecutionControl::uncancelled())).unwrap();
}

pub(super) fn switch(a: u32, b: u32) -> I {
    I::SwitchVariant(
        [a, b]
            .into_iter()
            .enumerate()
            .map(|(case, target)| NormalizedVariantJump {
                layout: VariantLayoutIndex(0, ValueOrigin::default()),
                case: case as u32,
                target,
                binding_local: None,
            })
            .collect::<Vec<_>>()
            .into(),
    )
}

// Independent per-read reachability, not the production bitset fixed point or
// linear fallback. A write ends this value's lifetime only on its actual path.
pub(super) fn future_read(instructions: &[I], at: usize, local: u32) -> bool {
    let mut pending = vec![at + 1];
    let mut seen = vec![false; instructions.len()];
    while let Some(pc) = pending.pop() {
        if pc >= instructions.len() || std::mem::replace(&mut seen[pc], true) {
            continue;
        }
        match &instructions[pc] {
            I::SequenceLength { source_local, .. }
            | I::SequenceGet { source_local, .. }
            | I::SequencePush { source_local, .. }
            | I::SequencePop { source_local, .. }
            | I::SequenceReplace { source_local, .. }
            | I::BorrowOwnedItem { source_local, .. }
                if *source_local == local =>
            {
                return true;
            }
            I::BorrowOwnedItem { binding_local, .. }
            | I::BorrowOwnedField { binding_local, .. }
                if *binding_local == local =>
            {
                return true;
            }
            I::BorrowOwnedField { source_local, .. } if *source_local == local => return true,
            I::MatchBorrowedOwned {
                source_local,
                cases,
                ..
            } if *source_local == local || cases.iter().any(|case| case.binding_local == local) => {
                return true;
            }
            I::EndOwnedBorrow { binding_local } if *binding_local == local => continue,
            I::LoadLocal { local: read, .. } | I::MoveLocal(read) if *read == local => return true,
            I::BeginTransaction { binding, .. }
            | I::BeginParameterTransaction { binding, .. }
            | I::BeginTransactionOutcome { binding, .. }
            | I::CommitTransaction { binding, .. }
            | I::CommitParameterTransaction { binding, .. }
            | I::CommitTransactionOutcome { binding, .. }
                if *binding == local =>
            {
                return true;
            }
            I::StoreLocal(write) if *write == local => {
                continue;
            }
            I::Jump(target) => {
                pending.push(*target as usize);
                continue;
            }
            I::JumpIfFalse(target) => pending.push(*target as usize),
            I::SwitchVariant(jumps) => {
                pending.extend(
                    jumps
                        .iter()
                        .filter(|jump| jump.binding_local != Some(local))
                        .map(|jump| jump.target as usize),
                );
                continue;
            }
            I::Return | I::TailCall { .. } => continue,
            // A dynamic external call can return even at a tail-position invoke.
            I::TailInvoke { .. } => {}
            _ => {}
        }
        pending.push(pc + 1);
    }
    false
}

#[test]
fn terminal_reads_preserve_reuse_affinity_and_shared_preparation() {
    let mut input = code(vec![
        load(0),
        I::Drop,
        load(0),
        I::Drop,
        I::LoadLocal {
            local: 1,
            use_mode: ParameterUse::Borrow,
        },
        I::Drop,
        I::LoadLocal {
            local: 1,
            use_mode: ParameterUse::Consume,
        },
        I::Return,
    ]);
    let original = Arc::clone(&input.instructions);
    optimize(&mut input);
    assert_eq!(input.instructions[0], load(0));
    assert_eq!(input.instructions[2], I::MoveLocal(0));
    assert_eq!(&input.instructions[4..], &original[4..]);
    assert_eq!(original[2], load(0));
    let once = input.instructions.clone();
    optimize(&mut input);
    assert!(Arc::ptr_eq(&once, &input.instructions));
    let mut instructions = input.instructions.to_vec();
    instructions.pop(); // A reachable later read, not dead code after Return.
    instructions.push(load(0));
    instructions.push(I::Return);
    input.instructions = instructions.into();
    optimize(&mut input);
    assert_eq!(input.instructions[2], load(0));
    assert_eq!(input.instructions[7], I::MoveLocal(0));
}

#[test]
fn exclusive_branch_reads_are_both_terminal() {
    let mut input = code(vec![
        load(1),
        I::JumpIfFalse(4),
        load(0),
        I::Jump(5),
        load(0),
        I::Return,
    ]);
    optimize(&mut input);
    assert_eq!(input.instructions[2], I::MoveLocal(0));
    assert_eq!(input.instructions[4], I::MoveLocal(0));
}

#[test]
fn weak_shared_storage_is_reserved_before_detaching_and_rewriting() {
    let mut input = code(vec![load(0), I::Return]);
    let weak = Arc::downgrade(&input.instructions);
    let control = ExecutionControl::uncancelled();
    let mut work = Budget::new(&control);
    // Leave room only for the linear fallback's scratch arrays. Optional
    // liveness cannot authorize an unreserved copy when a weak observer exists.
    let scratch = 2 * std::mem::size_of::<Option<usize>>() + 2 * std::mem::size_of::<usize>();
    work.reserve::<u8>(256 * 1024 * 1024 - scratch).unwrap();
    assert_eq!(
        derive(&mut input, &mut work).unwrap_err().class,
        DiagnosticClass::Resource
    );
    assert!(weak.upgrade().is_some());
    assert_eq!(input.instructions[0], load(0));
    optimize(&mut input);
    assert!(weak.upgrade().is_none());
    assert_eq!(input.instructions[0], I::MoveLocal(0));
}

#[test]
fn backward_edges_and_implicit_reads_do_not_become_moves() {
    for branch in [I::Jump(0), I::JumpIfFalse(0), switch(0, 2)] {
        let mut input = code(vec![load(0), branch, load(1), I::Return]);
        optimize(&mut input);
        assert_eq!(input.instructions[0], load(0));
        assert_eq!(input.instructions[2], I::MoveLocal(1));
    }
    let mut input = code(vec![
        load(0),
        I::CommitTransaction {
            requirement: super::super::value::RequirementIndex(0),
            binding: 0,
        },
        I::Return,
    ]);
    optimize(&mut input);
    assert_eq!(input.instructions[0], load(0));
    let mut input = code(vec![I::MoveLocal(0), I::Jump(0)]);
    optimize(&mut input);
    assert_eq!(input.instructions[0], load(0));
}

#[test]
fn exhaustive_small_control_flow_has_no_read_after_a_derived_move() {
    assert!(future_read(&[load(0), load(0), I::Return], 0, 0));
    assert!(future_read(&[load(0), I::Jump(0)], 0, 0));
    let mut alphabet = vec![load(0), load(1), I::Unit, I::Return];
    for target in 0..4 {
        alphabet.extend([
            I::Jump(target),
            I::JumpIfFalse(target),
            switch(target, (target + 1) % 4),
        ]);
    }
    let mut moves = 0;
    for mut encoding in 0..alphabet.len().pow(4) {
        let mut instructions = Vec::new();
        for _ in 0..4 {
            instructions.push(alphabet[encoding % alphabet.len()].clone());
            encoding /= alphabet.len();
        }
        instructions.push(I::Return);
        let original = instructions.clone();
        let mut input = code(instructions);
        optimize(&mut input);
        for (pc, instruction) in input.instructions.iter().enumerate() {
            if let I::MoveLocal(local) = instruction {
                assert!(!future_read(&original, pc, *local), "{original:?} at {pc}");
                moves += 1;
            }
        }
    }
    assert!(moves > 0);
}

#[test]
fn analysis_is_bounded_cancelled_and_rejects_foreign_operands() {
    for instructions in [vec![load(2)], vec![I::Jump(1)], vec![switch(0, 1)]] {
        let mut input = code(instructions);
        assert_eq!(
            derive(
                &mut input,
                &mut Budget::new(&ExecutionControl::uncancelled())
            )
            .unwrap_err()
            .class,
            DiagnosticClass::Corrupt
        );
    }
    let mut input = code(vec![I::Return]);
    input.local_count = u32::MAX;
    assert_eq!(
        derive(
            &mut input,
            &mut Budget::new(&ExecutionControl::uncancelled())
        )
        .unwrap_err()
        .class,
        DiagnosticClass::Resource
    );
    for size in [32, 4096] {
        let mut input = code(vec![load(0); size]);
        let control = ExecutionControl::cancel_after_checks((size * 12 + 64) as u64);
        derive(&mut input, &mut Budget::new(&control)).unwrap();
        assert_eq!(input.instructions[size - 1], I::MoveLocal(0));
        let control = ExecutionControl::cancel_after_checks(1);
        assert_eq!(
            derive(&mut input, &mut Budget::new(&control))
                .unwrap_err()
                .class,
            DiagnosticClass::Cancelled
        );
    }
}
