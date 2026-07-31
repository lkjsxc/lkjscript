use crate::oracle::{compare_source, evaluator_outcome, main_source, vm_outcome, ScalarOutcome};
use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, Limits, Op};
use lkjscript_ir::{evaluate, EvalConfig};
use lkjscript_vm::run_chunk;

#[test]
fn focused_ssa_evaluator_and_reference_vm_equivalence() {
    let cases = [
        ("unit.lkjscript", main_source("unit", "unit")),
        (
            "f64.lkjscript",
            main_source(
                "f64",
                "add/\n1.5\nconvert-i64-to-f64-rounded/\n2\n/convert-i64-to-f64-rounded\n/add",
            ),
        ),
        (
            "loop.lkjscript",
            main_source(
                "i64",
                "var/\nname/\ni\n/name\ntype/\ni64\n/type\n0\ndo/\nwhile/\nless-than/\ni\n5\n/less-than\nset/\ni\nadd/\ni\n1\n/add\n/set\n/while\ni\n/do\n/var",
            ),
        ),
        (
            "short-circuit.lkjscript",
            main_source(
                "i64",
                "var/\nname/\nx\n/name\ntype/\ni64\n/type\n0\ndo/\nand/\nfalse\ndo/\nset/\nx\n1\n/set\ntrue\n/do\n/and\nx\n/do\n/var",
            ),
        ),
        (
            "option.lkjscript",
            main_source("i64", "unwrap-some/\nsome/\n42\n/some\n/unwrap-some"),
        ),
        (
            "byte-vector.lkjscript",
            main_source(
                "i64",
                "let/\nbind/\nb\nnew-byte-vector/\n4\n/new-byte-vector\n/bind\ndo/\nbyte-slice-mut-set-byte/\nborrow-mut/\nb\n/borrow-mut\n0\n255\n/byte-slice-mut-set-byte\nbyte-slice-byte-at/\nborrow/\nb\n/borrow\n0\n/byte-slice-byte-at\n/do\n/let",
            ),
        ),
        (
            "trap.lkjscript",
            main_source("i64", "divide/\n1\n0\n/divide"),
        ),
        (
            "exit.lkjscript",
            main_source("unit", "exit/\n7\n/exit"),
        ),
    ];
    for (name, source) in cases {
        let _outcome = compare_source(&source, name);
    }

    let recursion = "def/\nname/\nfactorial\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nn\ni64\n/params\nif/\nless-than-or-equal/\nn\n1\n/less-than-or-equal\n1\nmultiply/\nn\nfactorial/\nsubtract/\nn\n1\n/subtract\n/factorial\n/multiply\n/if\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nfactorial/\n8\n/factorial\n/main\n";
    assert_eq!(
        compare_source(recursion, "recursion.lkjscript"),
        ScalarOutcome::I64(40_320)
    );

    let tail_recursion = "def/\nname/\ncount-down\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nn\ni64\n/params\nif/\nless-than-or-equal/\nn\n0\n/less-than-or-equal\nn\ncount-down/\nsubtract/\nn\n1\n/subtract\n/count-down\n/if\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\ncount-down/\n100\n/count-down\n/main\n";
    let tail_recursion = tail_recursion.to_string();
    let tail_program = compile_source(
        &tail_recursion,
        "tail-recursion.lkjscript",
        &Limits::default(),
    )
    .expect("compile tail recursion");
    assert_eq!(
        evaluator_outcome(evaluate(tail_program.ssa(), &EvalConfig::default())),
        vm_outcome(run_chunk(
            tail_program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default(),
        ))
    );
    assert!(tail_program.bytecode().protos()[0]
        .code
        .windows(3)
        .any(|bytes| bytes == [Op::Call as u8, 1, Op::Return as u8]));

    let product = "product/\nname/\npair-state\n/name\nfields/\nfield/\nname/\nleft\n/name\ntype/\ni64\n/type\n/field\nfield/\nname/\nright\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nfield/\nwith-field/\nproduct-value/\npair-state\nfield/\nleft\n3\n/field\nfield/\nright\n4\n/field\n/product-value\nleft\n9\n/with-field\nleft\n/field\n/main\n";
    assert_eq!(
        compare_source(product, "product.lkjscript"),
        ScalarOutcome::I64(9)
    );
}

#[test]
fn explicit_path_construction_is_evaluator_vm_equivalent() {
    let source = main_source(
        "path",
        "unwrap-ok/\nconvert-string-to-path/\nstring-literal/\n/tmp/exact-path\n/string-literal\n/convert-string-to-path\n/unwrap-ok",
    );
    assert_eq!(
        compare_source(&source, "path-equivalence.lkjscript"),
        ScalarOutcome::Path(b"/tmp/exact-path".to_vec())
    );
}

#[test]
fn invalid_path_construction_traps_equally() {
    for (name, text) in [
        ("empty-path.lkjscript", ""),
        ("relative-path.lkjscript", "relative"),
    ] {
        let expression =
            format!("unwrap-ok/\nconvert-string-to-path/\nstring-literal/\n{text}\n/string-literal\n/convert-string-to-path\n/unwrap-ok");
        let source = main_source("path", &expression);
        assert_eq!(compare_source(&source, name), ScalarOutcome::Trapped);
    }
}

#[test]
fn bounded_marker_generic_is_erased_after_ssa_verification_with_vm_equivalence() {
    let source = "trait/\nname/\nmarked\n/name\n/trait\nproduct/\nname/\nboxed\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\nimpl/\ntrait/\nmarked\n/trait\nfor/\nproduct\nboxed\n/for\n/impl\ndef/\nname/\nkeep-marked\n/name\nfn/\nforall/\nt\n/forall\nbounds/\nbound/\nt\nmarked\n/bound\n/bounds\nsig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\nparams/\nvalue\nt\n/params\nvalue\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nfield/\nkeep-marked/\nproduct-value/\nboxed\nfield/\nvalue\n42\n/field\n/product-value\n/keep-marked\nvalue\n/field\n/main\n";
    assert_eq!(
        compare_source(source, "bounded-marker.lkjscript"),
        ScalarOutcome::I64(42)
    );
}
