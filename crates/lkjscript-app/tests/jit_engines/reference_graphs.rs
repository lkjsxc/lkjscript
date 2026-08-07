use crate::canonical::{compile, evaluator, execution, Scalar};
use lkjscript_core::ExecutionPolicy;
use lkjscript_ir::{evaluate, EvalConfig};
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig};
use lkjscript_vm::run_chunk;

#[path = "reference_graphs/regions.rs"]
mod regions;

#[test]
fn nested_copy_product_projection_and_update_are_collector_free() {
    let source = concat!(
        "product/\nname/\ninner\n/name\nfields/\nfield/\nname/\nx\n/name\ntype/\ni64\n/type\n/field\n",
        "field/\nname/\ny\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\n",
        "product/\nname/\nouter\n/name\nfields/\nfield/\nname/\nvalue\n/name\n",
        "type/\nproduct\ninner\n/type\n/field\n/fields\n/product\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nfield/\nfield/\nwith-field/\n",
        "product-value/\nouter\nfield/\nvalue\nproduct-value/\ninner\nfield/\nx\n1\n/field\n",
        "field/\ny\n2\n/field\n/product-value\n/field\n/product-value\nvalue\n",
        "product-value/\ninner\nfield/\nx\n9\n/field\nfield/\ny\n2\n/field\n/product-value\n",
        "/with-field\nvalue\n/field\nx\n/field\n/main\n",
    );
    let program = compile(source, "copy-product-cutover.lkjscript");
    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    assert_eq!(expected, Scalar::I64(9));
    assert_eq!(
        execution(run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
        )),
        expected
    );
    let config = JitConfig::default();
    for result in [
        execute_forced(program.ssa(), &ExecutionPolicy::unrestricted(), config)
            .expect("baseline copy product"),
        execute_optimizing(program.ssa(), &ExecutionPolicy::unrestricted(), config)
            .expect("proof copy product"),
    ] {
        assert_eq!(execution(result.outcome), expected);
        assert_eq!(result.stats.vm_fallbacks, 0);
        assert_eq!(result.stats.native_structural.live_roots, 0);
        assert_eq!(result.stats.native_structural.live_loans, 0);
        assert_eq!(result.stats.native_structural.live_views, 0);
        assert_eq!(result.stats.native_structural.live_destinations, 0);
        assert_eq!(result.stats.native_structural.release_backlog, 0);
        assert_eq!(result.stats.native_structural.teardown_failures, 0);
    }
}

#[test]
fn structural_string_length_uses_generated_calls_without_collector_metadata() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "string-byte-length/\nempty-string/\n/empty-string\n/string-byte-length\n/main\n",
    );
    let program = compile(source, "structural-string-length.lkjscript");
    let vm = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    let native = execute_forced(
        program.ssa(),
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect("generated structural string execution");
    let optimized = execute_optimizing(
        program.ssa(),
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect("optimizing structural string execution");
    assert_eq!(execution(native.outcome), execution(vm.clone()));
    assert_eq!(execution(optimized.outcome), execution(vm));
    for stats in [&native.stats, &optimized.stats] {
        assert_eq!(stats.vm_fallbacks, 0);
        assert_eq!(stats.runtime_heap_successes, 0);
        assert!(stats.structural_runtime_calls > 0);
        assert_eq!(stats.native_structural.live_roots, 0);
        assert_eq!(stats.native_structural.live_loans, 0);
        assert_eq!(stats.native_structural.live_destinations, 0);
        assert_eq!(stats.native_structural.teardown_failures, 0);
        assert!(stats.code_objects.iter().all(|object| {
            !object
                .runtime_calls
                .contains(&lkjscript_native::RuntimeCallSlot::HeapDispatch)
        }));
    }
    assert!(native.stats.native_entries > 0);
    assert!(optimized.stats.optimizing_native_entries > 0);
}

#[test]
fn recursive_structural_string_calls_teardown_every_owner() {
    let source = concat!(
        "def/\nname/\nwalk\n/name\nfn/\nsig/\ninputs/\nstring\ni64\n/inputs\n",
        "output/\ni64\n/output\n/sig\nparams/\ntext\nstring\ndepth\ni64\n/params\n",
        "if/\nless-than-or-equal/\ndepth\n0\n/less-than-or-equal\n",
        "string-byte-length/\ntext\n/string-byte-length\nwalk/\ntext\n",
        "subtract/\ndepth\n1\n/subtract\n/walk\n/if\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "walk/\nempty-string/\n/empty-string\n4\n/walk\n/main\n",
    );
    let program = compile(source, "recursive-structural-string.lkjscript");
    let vm = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    let native = execute_forced(
        program.ssa(),
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect("recursive generated structural execution");
    let optimized = execute_optimizing(
        program.ssa(),
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect("recursive optimized structural execution");
    assert_eq!(execution(native.outcome), execution(vm.clone()));
    assert_eq!(execution(optimized.outcome), execution(vm));
    assert!(native.stats.direct_native_calls >= 4);
    assert!(optimized.stats.direct_native_calls >= 4);
    assert!(native.stats.peak_native_frame_depth >= 5);
    assert!(optimized.stats.peak_native_frame_depth >= 5);
    for stats in [&native.stats, &optimized.stats] {
        assert_eq!(stats.vm_fallbacks, 0);
        assert_eq!(stats.runtime_heap_successes, 0);
        assert_eq!(stats.native_structural.live_roots, 0);
        assert_eq!(stats.native_structural.live_loans, 0);
        assert_eq!(stats.native_structural.live_destinations, 0);
        assert_eq!(stats.native_structural.teardown_failures, 0);
        assert!(stats.native_structural.roots_dropped >= 5);
    }
}
