//! Independent path witnesses for value lifetimes, fixed points and bounded fallback.
#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::tests::{code, future_read, load, optimize, switch};
use super::*;
use crate::platform::execution::ExecutionControl;

fn binding_switch(a: u32, b: u32, first: Option<u32>, second: Option<u32>) -> I {
    let I::SwitchVariant(mut jumps) = switch(a, b) else {
        unreachable!()
    };
    Arc::make_mut(&mut jumps)[0].binding_local = first;
    Arc::make_mut(&mut jumps)[1].binding_local = second;
    I::SwitchVariant(jumps)
}

#[test]
fn branches_keep_live_join_values_and_stores_end_only_the_replaced_lifetime() {
    let mut joined = code(vec![
        load(0),
        I::JumpIfFalse(4),
        load(0),
        I::Jump(5),
        load(0),
        load(0),
        I::Return,
    ]);
    optimize(&mut joined);
    for at in [0, 2, 4] {
        assert_eq!(joined.instructions[at], load(0));
    }
    assert_eq!(joined.instructions[5], I::MoveLocal(0));
    for written in [0, 1] {
        let mut looped = code(vec![load(0), I::StoreLocal(written), I::Jump(0)]);
        optimize(&mut looped);
        assert_eq!(
            looped.instructions[0],
            if written == 0 {
                I::MoveLocal(0)
            } else {
                load(0)
            }
        );
    }
    let mut rebound = code(vec![load(0), I::StoreLocal(0), load(0), I::Return]);
    optimize(&mut rebound);
    assert_eq!(rebound.instructions[0], I::MoveLocal(0));
    assert_eq!(rebound.instructions[2], I::MoveLocal(0));
}

#[test]
fn switch_payload_redefinition_is_edge_specific_including_rederivation() {
    let mut input = code(vec![
        load(0),
        binding_switch(2, 4, Some(0), Some(0)),
        load(0),
        I::Return,
        load(0),
        I::Return,
    ]);
    optimize(&mut input);
    assert_eq!(input.instructions[0], I::MoveLocal(0));
    Arc::make_mut(&mut input.instructions)[1] = binding_switch(2, 4, Some(0), Some(1));
    optimize(&mut input);
    assert_eq!(input.instructions[0], load(0));
    for at in [2, 4] {
        assert_eq!(input.instructions[at], I::MoveLocal(0));
    }
}

#[test]
fn local_bitset_words_do_not_alias_and_exact_tail_exits_do_not_fall_through() {
    let locals = [0, 63, 64, 127, 128, 191];
    let mut instructions: Vec<_> = locals.into_iter().map(load).collect();
    instructions.extend(locals.into_iter().map(load));
    instructions.push(I::Return);
    let mut input = code(instructions);
    input.local_count = 192;
    optimize(&mut input);
    for (index, local) in locals.into_iter().enumerate() {
        assert_eq!(input.instructions[index], load(local));
        assert_eq!(
            input.instructions[index + locals.len()],
            I::MoveLocal(local)
        );
    }
    for exit in [
        I::Return,
        I::TailCall {
            function: super::super::value::FunctionIndex(
                0,
                super::super::value::ValueOrigin::default(),
            ),
            type_arguments: Arc::from([]),
            effect_arguments: Arc::from([]),
            requirement_arguments: Arc::from([]),
            arguments: 1,
        },
    ] {
        let mut input = code(vec![load(0), exit, load(0), I::Return]);
        optimize(&mut input);
        assert_eq!(input.instructions[0], I::MoveLocal(0));
    }
}

#[test]
fn implicit_transaction_definition_and_commit_preserve_value_order() {
    let requirement = super::super::value::RequirementIndex(0);
    let mut input = code(vec![
        load(0),
        I::BeginTransaction {
            requirement,
            binding: 0,
        },
        load(0),
        I::CommitTransaction {
            requirement,
            binding: 0,
        },
        I::Return,
    ]);
    optimize(&mut input);
    assert_eq!(input.instructions[0], I::MoveLocal(0));
    assert_eq!(input.instructions[2], load(0));
    let mut bad = code(vec![
        I::Return,
        I::BeginTransaction {
            requirement,
            binding: 2,
        },
    ]);
    assert_eq!(
        derive(&mut bad, &mut Budget::new(&ExecutionControl::uncancelled()))
            .unwrap_err()
            .class,
        DiagnosticClass::Corrupt
    );
}

#[test]
fn dynamic_tail_invoke_preserves_a_possible_external_continuation() {
    let mut input = code(vec![
        load(0),
        I::TailInvoke { arguments: 1 },
        load(0),
        I::Return,
    ]);
    optimize(&mut input);
    assert_eq!(input.instructions[0], load(0));
    assert_eq!(input.instructions[2], I::MoveLocal(0));
}

#[test]
fn exhausted_precise_analysis_never_rewrites_and_linear_fallback_remains_safe() {
    let original = code(vec![
        load(1),
        I::Drop,
        I::MoveLocal(0),
        I::JumpIfFalse(2),
        load(0),
        I::Return,
    ]);
    let mut completed = 0;
    let mut declined = 0;
    for steps in 0..256 {
        let mut input = original.clone();
        let control = ExecutionControl::uncancelled();
        let mut work = Budget::new(&control);
        if liveness::derive_with_limits(&mut input, &mut work, 1024, steps).unwrap() {
            completed += 1;
        } else {
            declined += 1;
            assert!(Arc::ptr_eq(&original.instructions, &input.instructions));
            derive_linear(&mut input, &mut work).unwrap();
        }
        assert_eq!(input.instructions[2], load(0));
        assert_eq!(input.instructions[4], I::MoveLocal(0));
        for (at, instruction) in input.instructions.iter().enumerate() {
            if let I::MoveLocal(local) = instruction {
                assert!(!future_read(&original.instructions, at, *local));
            }
        }
    }
    assert!(completed > 0 && declined > 0);
    let control = ExecutionControl::uncancelled();
    let mut work = Budget::new(&control);
    let mut input = original.clone();
    assert!(!liveness::derive_with_limits(&mut input, &mut work, 0, 1024).unwrap());
    assert!(Arc::ptr_eq(&original.instructions, &input.instructions));
    assert!(!work.try_reserve::<u64>(usize::MAX));
    work.reserve::<u8>(256 * 1024 * 1024).unwrap();
    assert!(!work.try_reserve::<u8>(1));
}

#[test]
fn cancellation_preserves_shared_code_and_dead_foreign_writes_still_reject() {
    let original = code(vec![
        load(0),
        I::StoreLocal(0),
        I::JumpIfFalse(0),
        load(1),
        I::Return,
    ]);
    let mut cancelled = 0;
    let mut finished = 0;
    for checks in 0..128 {
        let mut input = original.clone();
        let control = ExecutionControl::cancel_after_checks(checks);
        match derive(&mut input, &mut Budget::new(&control)) {
            Ok(()) => finished += 1,
            Err(error) => {
                assert_eq!(error.class, DiagnosticClass::Cancelled);
                cancelled += 1;
            }
        }
        assert_eq!(original.instructions[0], load(0));
        for (at, instruction) in input.instructions.iter().enumerate() {
            if let I::MoveLocal(local) = instruction {
                assert!(!future_read(&original.instructions, at, *local));
            }
        }
    }
    assert!(cancelled > 0 && finished > 0);
    for bad in [I::StoreLocal(2), binding_switch(0, 0, Some(2), None)] {
        let mut input = code(vec![I::Return, bad]);
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
}

#[test]
fn exhaustive_redefinitions_and_payload_edges_match_independent_reachability() {
    let mut alphabet = vec![
        load(0),
        load(1),
        I::StoreLocal(0),
        I::StoreLocal(1),
        I::Unit,
        I::Return,
    ];
    for target in 0..4 {
        alphabet.extend([
            I::Jump(target),
            I::JumpIfFalse(target),
            switch(target, (target + 1) % 4),
            binding_switch(target, (target + 1) % 4, Some(0), Some(1)),
        ]);
    }
    assert_eq!(alphabet.len().pow(4), 234_256);
    let mut checked_reads = 0;
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
        for (at, instruction) in original.iter().enumerate() {
            if let I::LoadLocal { local, .. } = instruction {
                let expected = if future_read(&original, at, *local) {
                    load(*local)
                } else {
                    I::MoveLocal(*local)
                };
                assert_eq!(input.instructions[at], expected, "{original:?} at {at}");
                checked_reads += 1;
            }
        }
    }
    assert_eq!(checked_reads, 2 * 4 * 22_usize.pow(3));
}
