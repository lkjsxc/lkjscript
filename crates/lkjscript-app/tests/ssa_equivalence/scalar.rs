use crate::oracle::{compare_source, evaluator_outcome, main_source, vm_outcome, ScalarOutcome};
use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, Limits, Op};
use lkjscript_ir::{evaluate, EvalConfig};
use lkjscript_vm::run_chunk;

#[test]
fn focused_ssa_evaluator_and_reference_vm_equivalence() {
    let cases = [
        ("unit.lkjscript", main_source("Unit", "unit")),
        (
            "f64.lkjscript",
            main_source("F64", "+/\n1.5\n2\n/+"),
        ),
        (
            "loop.lkjscript",
            main_source(
                "I64",
                "var/\nname/\ni\n/name\ntype/\nI64\n/type\n0\ndo/\nwhile/\nlt/\ni\n5\n/lt\nset/\ni\n+/\ni\n1\n/+\n/set\n/while\ni\n/do\n/var",
            ),
        ),
        (
            "short-circuit.lkjscript",
            main_source(
                "I64",
                "var/\nname/\nx\n/name\ntype/\nI64\n/type\n0\ndo/\nand/\nfalse\ndo/\nset/\nx\n1\n/set\ntrue\n/do\n/and\nx\n/do\n/var",
            ),
        ),
        (
            "option.lkjscript",
            main_source("I64", "unwrap-some/\nsome/\n42\n/some\n/unwrap-some"),
        ),
        (
            "buffer.lkjscript",
            main_source(
                "I64",
                "var/\nname/\nb\n/name\ntype/\nBuf\n/type\nbuf-new/\n4\n/buf-new\ndo/\nbuf-set/\nb\n0\n255\n/buf-set\nbuf-ref/\nb\n0\n/buf-ref\n/do\n/var",
            ),
        ),
        (
            "trap.lkjscript",
            main_source("I64", "div/\n1\n0\n/div"),
        ),
        (
            "exit.lkjscript",
            main_source("Unit", "exit/\n7\n/exit"),
        ),
    ];
    for (name, source) in cases {
        let _outcome = compare_source(&source, name);
    }

    let recursion = "def/\nname/\nfactorial\n/name\nfn/\nsig/\nI64\n->\nI64\n/sig\nparams/\nn\nI64\n/params\nif/\nlte/\nn\n1\n/lte\n1\n*/\nn\nfactorial/\n-/\nn\n1\n/-\n/factorial\n/*\n/if\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nfactorial/\n8\n/factorial\n/main\n";
    assert_eq!(
        compare_source(recursion, "recursion.lkjscript"),
        ScalarOutcome::I64(40_320)
    );

    let tail_recursion = "def/\nname/\ncount-down\n/name\nfn/\nsig/\nI64\n->\nI64\n/sig\nparams/\nn\nI64\n/params\nif/\nlte/\nn\n0\n/lte\nn\ncount-down/\n-/\nn\n1\n/-\n/count-down\n/if\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\ncount-down/\n100\n/count-down\n/main\n";
    let tail_program = compile_source(
        tail_recursion,
        "tail-recursion.lkjscript",
        &Limits::default(),
    )
    .expect("compile tail recursion");
    assert_eq!(
        evaluator_outcome(evaluate(tail_program.ssa(), &EvalConfig::default())),
        vm_outcome(run_chunk(
            tail_program.bytecode(),
            &ExecutionConfig::default(),
        ))
    );
    assert!(tail_program.bytecode().protos()[0]
        .code
        .windows(3)
        .any(|bytes| bytes == [Op::Call as u8, 1, Op::Return as u8]));

    let product = "product/\nname/\nPairState\n/name\nfields/\nfield/\nname/\nleft\n/name\ntype/\nI64\n/type\n/field\nfield/\nname/\nright\n/name\ntype/\nI64\n/type\n/field\n/fields\n/product\nmain/\nsig/\n->\nI64\n/sig\nfield/\nwith-field/\nproduct-value/\nPairState\nfield/\nleft\n3\n/field\nfield/\nright\n4\n/field\n/product-value\nleft\n9\n/with-field\nleft\n/field\n/main\n";
    assert_eq!(
        compare_source(product, "product.lkjscript"),
        ScalarOutcome::I64(9)
    );
}

#[test]
fn bounded_marker_generic_is_erased_after_ssa_verification_with_vm_equivalence() {
    let source = "trait/\nname/\nMarked\n/name\n/trait\nproduct/\nname/\nBoxed\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\nI64\n/type\n/field\n/fields\n/product\nimpl/\ntrait/\nMarked\n/trait\nfor/\nProduct\nBoxed\n/for\n/impl\ndef/\nname/\nkeep-marked\n/name\nfn/\nforall/\nT\n/forall\nbounds/\nbound/\nT\nMarked\n/bound\n/bounds\nsig/\nT\n->\nT\n/sig\nparams/\nvalue\nT\n/params\nvalue\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nfield/\nkeep-marked/\nproduct-value/\nBoxed\nfield/\nvalue\n42\n/field\n/product-value\n/keep-marked\nvalue\n/field\n/main\n";
    assert_eq!(
        compare_source(source, "bounded-marker.lkjscript"),
        ScalarOutcome::I64(42)
    );
}
