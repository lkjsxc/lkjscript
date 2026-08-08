use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionOutcome, ExecutionPolicy, OwnedValue};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_vm::run_chunk;

#[derive(Debug, Clone, PartialEq)]
pub enum ScalarOutcome {
    Unit,
    Bool(bool),
    I64(i64),
    F64(u64),
    Str(String),
    Path(Vec<u8>),
    Exited(i64),
    Trapped,
    Owned(OwnedValue),
    Other(String),
}

pub fn evaluator_outcome(outcome: EvalOutcome) -> ScalarOutcome {
    match outcome {
        EvalOutcome::Returned(EvalValue::Unit) => ScalarOutcome::Unit,
        EvalOutcome::Returned(EvalValue::Bool(value)) => ScalarOutcome::Bool(value),
        EvalOutcome::Returned(EvalValue::I64(value)) => ScalarOutcome::I64(value),
        EvalOutcome::Returned(EvalValue::F64(value)) => ScalarOutcome::F64(value.to_bits()),
        EvalOutcome::Returned(EvalValue::Str(value)) => ScalarOutcome::Str(value),
        EvalOutcome::Returned(EvalValue::ReturnedOwned(value)) => owned_value(value),
        EvalOutcome::Exited(code) => ScalarOutcome::Exited(code),
        EvalOutcome::Trapped(_) => ScalarOutcome::Trapped,
        other => ScalarOutcome::Other(format!("{other:?}")),
    }
}

fn owned_value(value: OwnedValue) -> ScalarOutcome {
    if value.is_unit() {
        ScalarOutcome::Unit
    } else if let Some(result) = value.as_bool() {
        ScalarOutcome::Bool(result)
    } else if let Some(result) = value.as_i64() {
        ScalarOutcome::I64(result)
    } else if let Some(result) = value.as_f64() {
        ScalarOutcome::F64(result.to_bits())
    } else if let Some(result) = value.as_str() {
        ScalarOutcome::Str(result.to_owned())
    } else if let Some(result) = value.as_path_bytes() {
        ScalarOutcome::Path(result.to_vec())
    } else {
        ScalarOutcome::Owned(value)
    }
}

pub fn vm_outcome(outcome: ExecutionOutcome) -> ScalarOutcome {
    match outcome {
        ExecutionOutcome::Returned(value) => owned_value(value),
        ExecutionOutcome::Exited(code) => ScalarOutcome::Exited(i64::from(code)),
        ExecutionOutcome::Trapped(_) => ScalarOutcome::Trapped,
        other => ScalarOutcome::Other(other.summary()),
    }
}

pub fn compare_source(source: &str, name: &str) -> ScalarOutcome {
    let marked;
    let source = if source.starts_with("") {
        source
    } else {
        marked = source.to_string();
        &marked
    };
    let program = compile_source(source, name).expect("compile SSA fixture");
    let evaluated = evaluator_outcome(evaluate(program.ssa(), &EvalConfig::default()));
    let executed = vm_outcome(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    ));
    assert_eq!(evaluated, executed, "SSA/VM mismatch for {name}");
    assert_eq!(
        program.bytecode_links().functions.len(),
        program.ssa().program().functions.len()
    );
    evaluated
}

pub fn main_source(return_type: &str, expression: &str) -> String {
    format!("main/\nsig/\ninputs/\n/inputs\noutput/\n{return_type}\n/output\n/sig\n{expression}\n/main\n")
}

#[test]
fn structural_oracle_compares_values_instead_of_bounded_debug_text() {
    use std::num::NonZeroU64;

    use lkjscript_core::{
        InlineStructuralValue, LayoutIdentity, SemanticPayload, SemanticTypeIdentity,
        SemanticValue, StructuralKind, StructuralType,
    };

    let product_type = StructuralType::new(
        LayoutIdentity::new(NonZeroU64::new(1).expect("layout")),
        SemanticTypeIdentity::new(NonZeroU64::new(2).expect("semantic type")),
        StructuralKind::Product,
    );
    let integer_type = StructuralType::new(
        LayoutIdentity::new(NonZeroU64::new(3).expect("layout")),
        SemanticTypeIdentity::new(NonZeroU64::new(4).expect("semantic type")),
        StructuralKind::I64,
    );
    let value = |integer| {
        OwnedValue::from_structural(SemanticValue::new(
            product_type,
            SemanticPayload::Product(
                vec![SemanticValue::new(
                    integer_type,
                    SemanticPayload::Inline(InlineStructuralValue::I64(integer)),
                )]
                .into(),
            ),
        ))
        .expect("owned structural oracle value")
    };
    let left = value(1);
    let right = value(2);
    assert_eq!(format!("{left:?}"), format!("{right:?}"));
    assert_ne!(owned_value(left), owned_value(right));
}
