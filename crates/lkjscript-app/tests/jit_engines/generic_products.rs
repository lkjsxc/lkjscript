use crate::canonical::{compile, evaluator, execution, Scalar};
use lkjscript_core::ExecutionConfig;
use lkjscript_ir::{evaluate, EvalConfig};
use lkjscript_jit::{execute_forced, execute_optimizing, FailureCode, JitConfig};
use lkjscript_vm::run_chunk;

#[test]
fn copy_product_vm_transport_does_not_claim_native_witness_abi() {
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
    let prototype = &program.bytecode().protos()[0];
    assert_eq!(prototype.parameter_type_variables, vec![Some(0)]);
    assert_eq!(prototype.return_type_variable, Some(0));
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
    let config = JitConfig::default();
    for error in [
        execute_forced(program.ssa(), &ExecutionConfig::default(), config)
            .expect_err("baseline rejects generic signature without witness ABI"),
        execute_optimizing(program.ssa(), &ExecutionConfig::default(), config)
            .expect_err("proof rejects generic signature without witness ABI"),
    ] {
        assert_eq!(error.code(), FailureCode::UnsupportedSignature);
    }
}
