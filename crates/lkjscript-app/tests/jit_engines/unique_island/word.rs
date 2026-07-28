use super::*;

#[test]
fn little_endian_u32_access_matches_evaluator_vm_and_forced_native() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "let/\nbind/\nb\nnew-byte-vector/\n8\n/new-byte-vector\n/bind\ndo/\n",
        "let/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\n",
        "byte-slice-mut-write-u32-little-endian/\nm\n2\n2018915346\n",
        "/byte-slice-mut-write-u32-little-endian\n/let\n",
        "byte-slice-read-u32-little-endian/\nborrow/\nb\n/borrow\n2\n",
        "/byte-slice-read-u32-little-endian\n/do\n/let\n/main\n",
    );
    assert_i64_all_engines(
        source,
        "native-unique-little-endian-word.lkjscript",
        2_018_915_346,
        1,
        false,
    );
}

#[test]
fn little_endian_u32_bounds_and_value_fail_before_mutation() {
    let invalid_value = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "let/\nbind/\nb\nnew-byte-vector/\n4\n/new-byte-vector\n/bind\n",
        "let/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\ndo/\n",
        "byte-slice-mut-write-u32-little-endian/\nm\n0\n4294967296\n",
        "/byte-slice-mut-write-u32-little-endian\n0\n/do\n/let\n/let\n/main\n",
    );
    let program = compile(invalid_value, "native-unique-word-value-trap.lkjscript");
    assert!(matches!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Trapped(_)
    ));
    assert!(matches!(
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default()
        ),
        ExecutionOutcome::Trapped(_)
    ));
    for (proof, execution) in forced_pair(&program, &ExecutionConfig::default()) {
        assert!(matches!(execution.outcome, ExecutionOutcome::Trapped(_)));
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.cleanup_attempts, 0);
        assert_eq!(execution.stats.native_unique.cleanup_releases, 0);
        assert_eq!(execution.stats.native_unique.byte_writes, 0);
    }
}
