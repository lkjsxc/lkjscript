use super::*;

pub(super) fn define(
    plan: &mut MachinePlanBuilder,
    entries: Entries,
) -> Result<(), Box<dyn std::error::Error>> {
    {
        let mut builder = plan.function_builder(entries.multi_block)?;
        let entry = builder.create_block()?;
        let positive = builder.create_block()?;
        let non_positive = builder.create_block()?;
        builder.set_entry(entry)?;
        let input = builder.parameter(0)?;
        let zero = builder.i64_const(entry, 0)?;
        let condition = builder.i64_compare(entry, I64Comparison::GreaterThan, input, zero)?;
        builder.branch_if(entry, condition, positive, non_positive)?;
        let ten = builder.i64_const(positive, 10)?;
        let added = builder.i64_add(positive, input, ten)?;
        builder.return_value(positive, added)?;
        let ten = builder.i64_const(non_positive, 10)?;
        let subtracted = builder.i64_sub(non_positive, ten, input)?;
        builder.return_value(non_positive, subtracted)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(entries.loop_sum)?;
        let entry = builder.create_block()?;
        let condition_block = builder.create_block()?;
        let body = builder.create_block()?;
        let done = builder.create_block()?;
        builder.set_entry(entry)?;
        let limit = builder.parameter(0)?;
        let index_local = builder.create_local(ValueType::I64)?;
        let sum_local = builder.create_local(ValueType::I64)?;
        let zero = builder.i64_const(entry, 0)?;
        let _write_index = builder.write_local(entry, index_local, zero)?;
        let _write_sum = builder.write_local(entry, sum_local, zero)?;
        builder.branch(entry, condition_block)?;
        let index = builder.read_local(condition_block, index_local)?;
        let condition =
            builder.i64_compare(condition_block, I64Comparison::LessThan, index, limit)?;
        builder.branch_if(condition_block, condition, body, done)?;
        let old_index = builder.read_local(body, index_local)?;
        let old_sum = builder.read_local(body, sum_local)?;
        let one = builder.i64_const(body, 1)?;
        let next_index = builder.i64_add(body, old_index, one)?;
        let next_sum = builder.i64_add(body, old_sum, next_index)?;
        let _write_index = builder.write_local(body, index_local, next_index)?;
        let _write_sum = builder.write_local(body, sum_local, next_sum)?;
        builder.branch(body, condition_block)?;
        let sum = builder.read_local(done, sum_local)?;
        builder.return_value(done, sum)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(entries.checked_add)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let left = builder.parameter(0)?;
        let right = builder.parameter(1)?;
        let result = builder.i64_add(entry, left, right)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(entries.checked_sub)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let left = builder.parameter(0)?;
        let right = builder.parameter(1)?;
        let result = builder.i64_sub(entry, left, right)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(entries.checked_mul)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let left = builder.parameter(0)?;
        let right = builder.parameter(1)?;
        let result = builder.i64_mul(entry, left, right)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(entries.checked_div)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let left = builder.parameter(0)?;
        let right = builder.parameter(1)?;
        let result = builder.i64_div(entry, left, right)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    Ok(())
}
