use super::*;

pub(super) fn declare(
    plan: &mut MachinePlanBuilder,
) -> Result<Entries, Box<dyn std::error::Error>> {
    let multi_block = plan.declare_function(
        SourceFunctionId::new(1),
        Signature::new(vec![ValueType::I64], ValueType::I64)?,
    )?;
    let loop_sum = plan.declare_function(
        SourceFunctionId::new(2),
        Signature::new(vec![ValueType::I64], ValueType::I64)?,
    )?;
    let checked_add = plan.declare_function(
        SourceFunctionId::new(3),
        Signature::new(vec![ValueType::I64, ValueType::I64], ValueType::I64)?,
    )?;
    let checked_div = plan.declare_function(
        SourceFunctionId::new(4),
        Signature::new(vec![ValueType::I64, ValueType::I64], ValueType::I64)?,
    )?;
    let checked_sub = plan.declare_function(
        SourceFunctionId::new(19),
        Signature::new(vec![ValueType::I64, ValueType::I64], ValueType::I64)?,
    )?;
    let checked_mul = plan.declare_function(
        SourceFunctionId::new(20),
        Signature::new(vec![ValueType::I64, ValueType::I64], ValueType::I64)?,
    )?;
    let f64_arithmetic = plan.declare_function(
        SourceFunctionId::new(5),
        Signature::new(vec![ValueType::F64, ValueType::F64], ValueType::F64)?,
    )?;
    let i64_to_f64 = plan.declare_function(
        SourceFunctionId::new(21),
        Signature::new(vec![ValueType::I64], ValueType::F64)?,
    )?;
    let f64_branch = plan.declare_function(
        SourceFunctionId::new(6),
        Signature::new(vec![ValueType::F64, ValueType::F64], ValueType::F64)?,
    )?;
    let f64_comparison_functions = [
        plan.declare_function(
            SourceFunctionId::new(7),
            Signature::new(vec![ValueType::F64, ValueType::F64], ValueType::Bool)?,
        )?,
        plan.declare_function(
            SourceFunctionId::new(8),
            Signature::new(vec![ValueType::F64, ValueType::F64], ValueType::Bool)?,
        )?,
        plan.declare_function(
            SourceFunctionId::new(9),
            Signature::new(vec![ValueType::F64, ValueType::F64], ValueType::Bool)?,
        )?,
        plan.declare_function(
            SourceFunctionId::new(10),
            Signature::new(vec![ValueType::F64, ValueType::F64], ValueType::Bool)?,
        )?,
        plan.declare_function(
            SourceFunctionId::new(11),
            Signature::new(vec![ValueType::F64, ValueType::F64], ValueType::Bool)?,
        )?,
        plan.declare_function(
            SourceFunctionId::new(12),
            Signature::new(vec![ValueType::F64, ValueType::F64], ValueType::Bool)?,
        )?,
    ];
    let bool_not = plan.declare_function(
        SourceFunctionId::new(13),
        Signature::new(vec![ValueType::Bool], ValueType::Bool)?,
    )?;
    let bool_equal = plan.declare_function(
        SourceFunctionId::new(14),
        Signature::new(vec![ValueType::Bool, ValueType::Bool], ValueType::Bool)?,
    )?;
    let callee = plan.declare_function(
        SourceFunctionId::new(15),
        Signature::new(vec![ValueType::I64], ValueType::I64)?,
    )?;
    let direct_call = plan.declare_function(
        SourceFunctionId::new(16),
        Signature::new(vec![ValueType::I64], ValueType::I64)?,
    )?;
    let exit = plan.declare_function(
        SourceFunctionId::new(17),
        Signature::new(vec![], ValueType::Unit)?,
    )?;
    let unit = plan.declare_function(
        SourceFunctionId::new(18),
        Signature::new(vec![ValueType::Unit], ValueType::Unit)?,
    )?;

    Ok(Entries {
        multi_block,
        loop_sum,
        checked_add,
        checked_sub,
        checked_mul,
        checked_div,
        f64_arithmetic,
        i64_to_f64,
        f64_branch,
        f64_comparisons: f64_comparison_functions,
        bool_not,
        bool_equal,
        direct_call,
        exit,
        unit,
        callee,
    })
}
