use crate::canonical::{compile, evaluator, execution, Scalar};
use lkjscript_core::ExecutionConfig;
use lkjscript_ir::{evaluate, EvalConfig};
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig};
use lkjscript_vm::run_chunk;

mod structural_owners;

#[test]
fn list_only_execution_uses_segmented_invocation_storage() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nbool\n/output\n/sig\n",
        "equal-list/\nlist-prepend/\n1\nlist-prepend/\n2\nempty-list/\ni64\n/empty-list\n",
        "/list-prepend\n/list-prepend\nlist-prepend/\n1\nlist-prepend/\n2\n",
        "empty-list/\ni64\n/empty-list\n/list-prepend\n/list-prepend\n/equal-list\n/main\n",
    );
    let program = compile(source, "segmented-list-only.lkjscript");
    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    let vm = execution(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ));
    let config = JitConfig::default();
    for result in [
        execute_forced(program.ssa(), &ExecutionConfig::default(), config)
            .expect("baseline segmented list"),
        execute_optimizing(program.ssa(), &ExecutionConfig::default(), config)
            .expect("proof segmented list"),
    ] {
        assert_eq!(execution(result.outcome), expected);
        assert_eq!(result.stats.vm_fallbacks, 0);
        assert_eq!(result.stats.segmented_lists.prepends, 4);
        assert_eq!(result.stats.segmented_lists.live_entries, 4);
        assert_eq!(result.stats.segmented_lists.segment_allocations, 1);
    }
    assert_eq!(expected, Scalar::Bool(true));
    assert_eq!(vm, expected);
}

#[test]
fn nested_copy_lists_return_without_runtime_keys_in_all_engines() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nlist/\nlist/\ni64\n/list\n/list\n/output\n/sig\n",
        "list-prepend/\nlist-prepend/\n7\nempty-list/\ni64\n/empty-list\n/list-prepend\n",
        "empty-list/\nlist/\ni64\n/list\n/empty-list\n/list-prepend\n/main\n",
    );
    let program = compile(source, "nested-segmented-list-return.lkjscript");
    let evaluated = evaluate(program.ssa(), &EvalConfig::default());
    assert!(matches!(
        evaluated,
        lkjscript_ir::EvalOutcome::Returned(lkjscript_ir::EvalValue::List(_))
    ));
    for outcome in [
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default(),
        ),
        execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("baseline returns nested list")
        .outcome,
        execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("proof returns nested list")
        .outcome,
    ] {
        let lkjscript_core::ExecutionOutcome::Returned(value) = outcome else {
            continue;
        };
        assert_eq!(value.snapshot_object_count(), 2);
        assert_eq!(value.list_len(), Some(1));
    }
}

#[test]
fn copy_list_returns_are_key_free_and_codec_stable_across_engines() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nlist/\ni64\n/list\n/output\n/sig\n",
        "list-prepend/\n1\nlist-prepend/\n2\nempty-list/\ni64\n/empty-list\n/list-prepend\n/list-prepend\n/main\n",
    );
    let program = compile(source, "segmented-list-return.lkjscript");
    let evaluated_outcome = evaluate(program.ssa(), &EvalConfig::default());
    assert!(matches!(
        evaluated_outcome,
        lkjscript_ir::EvalOutcome::Returned(lkjscript_ir::EvalValue::List(_))
    ));
    let lkjscript_ir::EvalOutcome::Returned(lkjscript_ir::EvalValue::List(evaluated)) =
        evaluated_outcome
    else {
        return;
    };
    assert_eq!(evaluated.len(), 2);
    assert!(matches!(
        evaluated.as_slice(),
        [
            lkjscript_ir::EvalValue::I64(1),
            lkjscript_ir::EvalValue::I64(2)
        ]
    ));
    let outcomes = [
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default(),
        ),
        execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("baseline returns segmented list")
        .outcome,
        execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("proof returns segmented list")
        .outcome,
    ];
    for outcome in outcomes {
        assert!(matches!(
            outcome,
            lkjscript_core::ExecutionOutcome::Returned(_)
        ));
        let lkjscript_core::ExecutionOutcome::Returned(value) = &outcome else {
            continue;
        };
        assert_eq!(value.snapshot_object_count(), 2);
        assert_eq!(value.list_len(), Some(2));
        assert_eq!(value.list_i64(0), Some(1));
        assert_eq!(value.list_i64(1), Some(2));
        let wire = lkjscript_core::encode_execution_outcome(&outcome, 64 * 1024)
            .expect("encode list outcome");
        let decoded = lkjscript_core::decode_execution_outcome(&wire, 64 * 1024)
            .expect("decode list outcome");
        assert_eq!(decoded, outcome);
    }
}
