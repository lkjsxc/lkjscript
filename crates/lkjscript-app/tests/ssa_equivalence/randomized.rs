use crate::oracle::{compare_source, main_source};
use lkjscript_compiler::compile_source;
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, RuntimeOp};

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
                let operation = ["add", "subtract", "multiply"]
                    .get(usize::try_from(self.choose(3)).unwrap_or(0))
                    .copied()
                    .unwrap_or("add");
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
                "less-than/\n{}\n{}\n/less-than",
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
    let source = concat!(
        "main/\nsig/\ninputs/\ncapability/\nstdio\n/capability\n/inputs\noutput/\nunit\n/output\n/sig\n",
        "params/\nstdio\ncapability/\nstdio\n/capability\n/params\n",
        "print/\nstdio\nstring-literal/\nnot emitted\n/string-literal\n/print\n/main\n"
    );
    let program =
        compile_source(source, "unsupported-host.lkjscript").expect("compile host operation");
    assert_eq!(
        evaluate(
            program.ssa(),
            &EvalConfig {
                capabilities: vec![lkjscript_core::CapabilityKind::Stdio],
                ..EvalConfig::default()
            },
        ),
        EvalOutcome::UnsupportedOperation(RuntimeOp::Print)
    );
}

#[test]
fn bounded_randomized_type_correct_scalar_programs_match() {
    let mut generator = Generator(0x5eed_cafe_d00d_f00d);
    for index in 0..64 {
        let (return_type, expression) = if generator.choose(2) == 0 {
            ("i64", generator.i64_expression(3))
        } else {
            ("bool", generator.bool_expression(3))
        };
        let source = main_source(return_type, &expression);
        let name = format!("random-{index}.lkjscript");
        compare_source(&source, &name);
    }
}
