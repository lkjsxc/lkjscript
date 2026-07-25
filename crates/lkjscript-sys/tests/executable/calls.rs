use super::*;

pub(super) fn define(
    plan: &mut MachinePlanBuilder,
    entries: Entries,
) -> Result<(), Box<dyn std::error::Error>> {
    {
        let mut builder = plan.function_builder(entries.bool_not)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let input = builder.parameter(0)?;
        let result = builder.bool_not(entry, input)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(entries.bool_equal)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let left = builder.parameter(0)?;
        let right = builder.parameter(1)?;
        let result =
            builder.bool_compare(entry, lkjscript_native::BoolComparison::Equal, left, right)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(entries.callee)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let input = builder.parameter(0)?;
        let two = builder.i64_const(entry, 2)?;
        let result = builder.i64_mul(entry, input, two)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(entries.direct_call)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let input = builder.parameter(0)?;
        let called = builder.call(entry, entries.callee, vec![input])?;
        let returned = builder.runtime_call(entry, RuntimeCallSlot::IdentityI64V1, vec![called])?;
        builder.return_value(entry, returned)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(entries.exit)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let code = builder.i64_const(entry, 17)?;
        builder.exit(entry, code)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(entries.unit)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let value = builder.parameter(0)?;
        builder.return_value(entry, value)?;
        plan.define_function(builder.finish())?;
    }

    Ok(())
}
