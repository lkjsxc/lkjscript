use crate::oracle::{compare_source, evaluator_outcome, main_source};
use lkjscript_compiler::compile_source;
use lkjscript_core::Limits;
use lkjscript_ir::{evaluate, optimize, EvalConfig, EvalOutcome, OptimizationLimits, RuntimeOp};

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
        let expected = compare_source(&source, &name);
        let program = compile_source(&source, &name, &Limits::default())
            .expect("compile randomized optimization input");
        let optimized = optimize(program.ssa(), OptimizationLimits::default())
            .expect("proof-optimize randomized scalar program");
        assert_eq!(
            evaluator_outcome(evaluate(
                optimized.verified_program(),
                &EvalConfig::default()
            )),
            expected,
            "optimized randomized evaluator case {index}"
        );
    }
}
