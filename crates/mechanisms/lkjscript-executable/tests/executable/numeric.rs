use super::*;

pub(super) fn define(
    plan: &mut MachinePlanBuilder,
    entries: Entries,
) -> Result<(), Box<dyn std::error::Error>> {
    {
        let mut builder = plan.function_builder(entries.f64_arithmetic)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let left = builder.parameter(0)?;
        let right = builder.parameter(1)?;
        let sum = builder.f64_add(entry, left, right)?;
        let difference = builder.f64_sub(entry, sum, right)?;
        let product = builder.f64_mul(entry, difference, right)?;
        let quotient = builder.f64_div(entry, product, right)?;
        builder.return_value(entry, quotient)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(entries.i64_to_f64)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let value = builder.parameter(0)?;
        let converted = builder.i64_to_f64(entry, value)?;
        builder.return_value(entry, converted)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(entries.f64_branch)?;
        let entry = builder.create_block()?;
        let less = builder.create_block()?;
        let otherwise = builder.create_block()?;
        builder.set_entry(entry)?;
        let left = builder.parameter(0)?;
        let right = builder.parameter(1)?;
        let condition = builder.f64_compare(entry, F64Comparison::OrderedLessThan, left, right)?;
        builder.branch_if(entry, condition, less, otherwise)?;
        let negative_zero = builder.f64_const_bits(less, (-0.0_f64).to_bits())?;
        builder.return_value(less, negative_zero)?;
        let payload_nan = builder.f64_const_bits(otherwise, 0x7ff8_0000_0000_1234)?;
        builder.return_value(otherwise, payload_nan)?;
        plan.define_function(builder.finish())?;
    }

    let comparisons = [
        F64Comparison::OrderedEqual,
        F64Comparison::OrderedNotEqual,
        F64Comparison::OrderedLessThan,
        F64Comparison::OrderedLessThanOrEqual,
        F64Comparison::OrderedGreaterThan,
        F64Comparison::OrderedGreaterThanOrEqual,
    ];
    for (function, comparison) in entries.f64_comparisons.iter().copied().zip(comparisons) {
        let mut builder = plan.function_builder(function)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let left = builder.parameter(0)?;
        let right = builder.parameter(1)?;
        let result = builder.f64_compare(entry, comparison, left, right)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    Ok(())
}
