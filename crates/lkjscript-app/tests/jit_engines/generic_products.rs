use crate::canonical::{compile, evaluator, execution, Scalar};
use lkjscript_core::{ExecutionConfig, Limits};
use lkjscript_ir::{evaluate, EvalConfig, InstructionKind};
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig};
use lkjscript_vm::run_chunk;

#[test]
fn copy_product_hidden_witness_executes_in_all_four_tiers() {
    let source = concat!(
        "trait/\nname/\nmarked\n/name\n/trait\nproduct/\nname/\nboxed\n/name\nfields/\n",
        "field/\nname/\nvalue\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\n",
        "impl/\ntrait/\nmarked\n/trait\nfor/\nproduct\nboxed\n/for\n/impl\n",
        "def/\nname/\nkeep-marked\n/name\nfn/\nforall/\nt\n/forall\nbounds/\nbound/\n",
        "t\nmarked\n/bound\n/bounds\nsig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\n",
        "params/\nvalue\nt\n/params\nvalue\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\n",
        "output/\ni64\n/output\n/sig\nfield/\nkeep-marked/\nproduct-value/\nboxed\n",
        "field/\nvalue\n42\n/field\n/product-value\n/keep-marked\nvalue\n/field\n/main\n",
    );
    let program = compile(source, "generic-copy-product.lkjscript");
    let generic = &program.ssa().program().functions[0];
    assert_eq!(generic.signature.memory_witness_parameters.len(), 1);
    assert_eq!(
        generic.signature.memory_witness_parameters[0].parameter,
        "t"
    );
    let call = program.ssa().program().functions[1].blocks[0]
        .instructions
        .iter()
        .find_map(|instruction| match &instruction.kind {
            InstructionKind::Call {
                instantiation: Some(instantiation),
                ..
            } => Some(instantiation),
            _ => None,
        })
        .expect("generic direct call retains hidden witness binding");
    assert_eq!(call.memory_witnesses.len(), 1);
    assert!(program
        .ssa()
        .program()
        .memory
        .witness(call.memory_witnesses[0].witness)
        .is_some());

    let prototype = &program.bytecode().protos()[0];
    assert_eq!(prototype.parameter_type_variables, vec![Some(0)]);
    assert_eq!(prototype.return_type_variable, Some(0));
    assert_eq!(prototype.memory_witness_parameters.len(), 1);
    assert_eq!(program.bytecode().main().call_witnesses.len(), 1);
    assert!(!program.bytecode().memory_witnesses().is_empty());

    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    assert_eq!(expected, Scalar::I64(42));
    let vm = execution(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ));
    assert_eq!(vm, expected);

    let config = JitConfig::default();
    let baseline = execute_forced(program.ssa(), &ExecutionConfig::default(), config)
        .expect("bounded transport specialization enters baseline native code");
    assert_eq!(execution(baseline.outcome), expected);
    assert!(baseline.stats.native_entries > 0);
    assert!(baseline.stats.direct_native_calls > 0);
    assert_eq!(baseline.stats.vm_fallbacks, 0);
    assert!(baseline
        .stats
        .functions
        .iter()
        .all(|function| function.native_entries() > 0));

    let proof = execute_optimizing(program.ssa(), &ExecutionConfig::default(), config)
        .expect("bounded transport specialization enters proof native code");
    assert_eq!(execution(proof.outcome), expected);
    assert!(proof.stats.optimizing_native_entries > 0);
    assert_eq!(proof.stats.baseline_native_entries, 0);
    assert!(proof.stats.direct_native_calls > 0);
    assert_eq!(proof.stats.vm_fallbacks, 0);
}

#[test]
fn native_transport_specialization_fails_closed_for_residual_body() {
    let source = concat!(
        "def/\nname/\nkeep\n/name\nfn/\nforall/\nt\n/forall\nsig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\n",
        "params/\nvalue\nt\n/params\nif/\ntrue\nvalue\nvalue\n/if\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nkeep/\n42\n/keep\n/main\n",
    );
    let program = compile(source, "residual-native-transport.lkjscript");
    let error = lkjscript_ir::specialize_native_transport(program.ssa())
        .expect_err("residual transport body must not enter native code");
    assert!(error
        .to_string()
        .starts_with("native transport specialization"));
    assert!(execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .is_err());
    assert!(execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .is_err());
}

#[test]
fn cross_package_transport_witness_executes_in_all_four_tiers() {
    let entry = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../src/examples/polymorphic-transport/main.lkjscript");
    let program = lkjscript_compiler::compile_path(&entry, &Limits::default())
        .expect("compile locked cross-package generic consumer");
    assert_eq!(program.ssa().program().sources.len(), 2);
    let call = program
        .memory_plan()
        .calls
        .iter()
        .find(|call| !call.witness_arguments.is_empty())
        .expect("cross-package call has HIR witness arguments");
    assert_eq!(call.witness_arguments.len(), 1);
    assert_eq!(call.witness_arguments[0].parameter, "t");

    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    assert_eq!(expected, Scalar::I64(42));
    assert_eq!(
        execution(run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default(),
        )),
        expected
    );
    let baseline = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("cross-package generic baseline specialization");
    assert_eq!(execution(baseline.outcome), expected);
    assert!(baseline.stats.direct_native_calls > 0);
    assert_eq!(baseline.stats.vm_fallbacks, 0);
    let proof = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("cross-package generic proof specialization");
    assert_eq!(execution(proof.outcome), expected);
    assert!(proof.stats.optimizing_native_entries > 0);
    assert_eq!(proof.stats.vm_fallbacks, 0);
}
