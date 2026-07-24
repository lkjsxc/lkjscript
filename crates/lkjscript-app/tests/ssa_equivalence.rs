#![allow(clippy::expect_used)]

use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Limits, Op, OwnedValue};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue, RuntimeOp};
use lkjscript_vm::run_chunk;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScalarOutcome {
    Unit,
    Bool(bool),
    I64(i64),
    F64(u64),
    Str(String),
    Exited(i64),
    Trapped,
    Other(String),
}

fn evaluator_outcome(outcome: EvalOutcome) -> ScalarOutcome {
    match outcome {
        EvalOutcome::Returned(EvalValue::Unit) => ScalarOutcome::Unit,
        EvalOutcome::Returned(EvalValue::Bool(value)) => ScalarOutcome::Bool(value),
        EvalOutcome::Returned(EvalValue::I64(value)) => ScalarOutcome::I64(value),
        EvalOutcome::Returned(EvalValue::F64(value)) => ScalarOutcome::F64(value.to_bits()),
        EvalOutcome::Returned(EvalValue::Str(value)) => ScalarOutcome::Str(value),
        EvalOutcome::Exited(code) => ScalarOutcome::Exited(code),
        EvalOutcome::Trapped(_) => ScalarOutcome::Trapped,
        other => ScalarOutcome::Other(format!("{other:?}")),
    }
}

fn vm_value(value: &OwnedValue) -> ScalarOutcome {
    if value.is_unit() {
        ScalarOutcome::Unit
    } else if let Some(value) = value.as_bool() {
        ScalarOutcome::Bool(value)
    } else if let Some(value) = value.as_i64() {
        ScalarOutcome::I64(value)
    } else if let Some(value) = value.as_f64() {
        ScalarOutcome::F64(value.to_bits())
    } else if let Some(value) = value.as_str() {
        ScalarOutcome::Str(value.to_owned())
    } else {
        ScalarOutcome::Other(format!("{value:?}"))
    }
}

fn vm_outcome(outcome: ExecutionOutcome) -> ScalarOutcome {
    match outcome {
        ExecutionOutcome::Returned(value) => vm_value(&value),
        ExecutionOutcome::Exited(code) => ScalarOutcome::Exited(i64::from(code)),
        ExecutionOutcome::Trapped(_) => ScalarOutcome::Trapped,
        other => ScalarOutcome::Other(other.summary()),
    }
}

fn compare_source(source: &str, name: &str) -> ScalarOutcome {
    let program = compile_source(source, name, &Limits::default()).expect("compile SSA fixture");
    let evaluated = evaluator_outcome(evaluate(program.ssa(), &EvalConfig::default()));
    let executed = vm_outcome(run_chunk(program.bytecode(), &ExecutionConfig::default()));
    assert_eq!(evaluated, executed, "SSA/VM mismatch for {name}");
    assert_eq!(
        program.bytecode_links().functions.len(),
        program.ssa().program().functions.len()
    );
    evaluated
}

fn main_source(return_type: &str, expression: &str) -> String {
    format!("main/\nsig/\n->\n{return_type}\n/sig\n{expression}\n/main\n")
}

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

struct Generator(u64);

impl Generator {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn choose(&mut self, count: u64) -> u64 {
        self.next() % count
    }

    fn i64_expression(&mut self, depth: u8) -> String {
        if depth == 0 || self.choose(4) == 0 {
            let value = i64::try_from(self.choose(21)).unwrap_or(0) - 10;
            return value.to_string();
        }
        match self.choose(4) {
            0..=2 => {
                let operation = ["+", "-", "*"]
                    .get(usize::try_from(self.choose(3)).unwrap_or(0))
                    .copied()
                    .unwrap_or("+");
                format!(
                    "{operation}/\n{}\n{}\n/{operation}",
                    self.i64_expression(depth - 1),
                    self.i64_expression(depth - 1)
                )
            }
            _ => format!(
                "if/\n{}\n{}\n{}\n/if",
                self.bool_expression(depth - 1),
                self.i64_expression(depth - 1),
                self.i64_expression(depth - 1)
            ),
        }
    }

    fn bool_expression(&mut self, depth: u8) -> String {
        if depth == 0 || self.choose(4) == 0 {
            return if self.choose(2) == 0 {
                "false".into()
            } else {
                "true".into()
            };
        }
        match self.choose(4) {
            0 => format!(
                "lt/\n{}\n{}\n/lt",
                self.i64_expression(depth - 1),
                self.i64_expression(depth - 1)
            ),
            1 => format!(
                "equal-value/\n{}\n{}\n/equal-value",
                self.i64_expression(depth - 1),
                self.i64_expression(depth - 1)
            ),
            2 => format!("not/\n{}\n/not", self.bool_expression(depth - 1)),
            _ => format!(
                "if/\n{}\n{}\n{}\n/if",
                self.bool_expression(depth - 1),
                self.bool_expression(depth - 1),
                self.bool_expression(depth - 1)
            ),
        }
    }
}

#[test]
fn owned_buf_borrows_moves_and_mutation_match_evaluator_and_vm() {
    let source = "def/\nname/\npass-owned\n/name\nfn/\nsig/\nOwned\nBuf\n->\nOwned\nBuf\n/sig\nparams/\nb\nOwned/\nBuf\n/Owned\n/params\nmove/\nb\n/move\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nlet/\nbind/\nb\nowned-buf-new/\n2\n/owned-buf-new\n/bind\ndo/\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\nowned-buf-set/\nm\n1\n77\n/owned-buf-set\n/let\nlet/\nbind/\nc\npass-owned/\nmove/\nb\n/move\n/pass-owned\n/bind\nlet/\nbind/\nr\nborrow/\nc\n/borrow\n/bind\nowned-buf-ref/\nr\n1\n/owned-buf-ref\n/let\n/let\n/do\n/let\n/main\n";
    assert_eq!(
        compare_source(source, "owned-buffer.lkjscript"),
        ScalarOutcome::I64(77)
    );

    let var_source = "main/\nsig/\n->\nI64\n/sig\nvar/\nname/\nb\n/name\ntype/\nOwned\nBuf\n/type\nowned-buf-new/\n2\n/owned-buf-new\ndo/\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\nowned-buf-set/\nm\n0\n91\n/owned-buf-set\n/let\nmove/\nb\n/move\nset/\nb\nowned-buf-new/\n3\n/owned-buf-new\n/set\nlet/\nbind/\nfresh\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nmove/\nb\n/move\nset/\nb\nmove/\nfresh\n/move\n/set\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/do\n/let\n/do\n/var\n/main\n";
    assert_eq!(
        compare_source(var_source, "owned-var-reinit.lkjscript"),
        ScalarOutcome::I64(1)
    );

    let program = compile_source(source, "owned-buffer-limits.lkjscript", &Limits::default())
        .expect("compile owned buffer limits fixture");
    let eval_limits = EvalConfig {
        max_allocations: 0,
        ..EvalConfig::default()
    };
    assert!(matches!(
        evaluate(program.ssa(), &eval_limits),
        EvalOutcome::ResourceLimitExceeded(_)
    ));
    let vm_limits = ExecutionConfig {
        max_allocations: 0,
        ..ExecutionConfig::default()
    };
    assert!(matches!(
        run_chunk(program.bytecode(), &vm_limits),
        ExecutionOutcome::ResourceLimitExceeded(_)
    ));
}

#[test]
fn evaluator_reports_host_operations_as_explicitly_unsupported() {
    let source = main_source("Unit", "print/\nstr/\nnot emitted\n/str\n/print");
    let program = compile_source(&source, "unsupported-host.lkjscript", &Limits::default())
        .expect("compile host operation");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::UnsupportedOperation(RuntimeOp::Print)
    );
}

#[test]
fn bounded_randomized_type_correct_scalar_programs_match() {
    let mut generator = Generator(0x5eed_cafe_d00d_f00d);
    for index in 0..64 {
        let (return_type, expression) = if generator.choose(2) == 0 {
            ("I64", generator.i64_expression(3))
        } else {
            ("Bool", generator.bool_expression(3))
        };
        let source = main_source(return_type, &expression);
        let name = format!("random-{index}.lkjscript");
        let _outcome = compare_source(&source, &name);
    }
}
