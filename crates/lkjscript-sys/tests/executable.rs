#![allow(clippy::panic)]

use lkjscript_native::{
    encode, AbiVersions, BackendLimits, EncodingConfig, F64Comparison, FunctionId, I64Comparison,
    InstallableImage, MachinePlanBuilder, NativeValue, RuntimeCallSlot, Signature,
    SourceFunctionId, TrapCode, ValueType,
};
use lkjscript_sys::executable::{
    ExecutableInstaller, ExecutableLimitKind, ExecutableLimits, InstallError, InvocationOutcome,
};

#[derive(Clone, Copy)]
struct Entries {
    multi_block: FunctionId,
    loop_sum: FunctionId,
    checked_add: FunctionId,
    checked_sub: FunctionId,
    checked_mul: FunctionId,
    checked_div: FunctionId,
    f64_arithmetic: FunctionId,
    f64_branch: FunctionId,
    f64_comparisons: [FunctionId; 6],
    bool_not: FunctionId,
    bool_equal: FunctionId,
    direct_call: FunctionId,
    exit: FunctionId,
    unit: FunctionId,
}

fn scalar_image(
    versions: AbiVersions,
) -> Result<(InstallableImage, Entries), Box<dyn std::error::Error>> {
    let mut plan = MachinePlanBuilder::new();
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
    let f64_branch = plan.declare_function(
        SourceFunctionId::new(6),
        Signature::new(vec![ValueType::F64, ValueType::F64], ValueType::F64)?,
    )?;
    let f64_comparisons = [
        F64Comparison::OrderedEqual,
        F64Comparison::OrderedNotEqual,
        F64Comparison::OrderedLessThan,
        F64Comparison::OrderedLessThanOrEqual,
        F64Comparison::OrderedGreaterThan,
        F64Comparison::OrderedGreaterThanOrEqual,
    ];
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

    {
        let mut builder = plan.function_builder(multi_block)?;
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
        let mut builder = plan.function_builder(loop_sum)?;
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
        let mut builder = plan.function_builder(checked_add)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let left = builder.parameter(0)?;
        let right = builder.parameter(1)?;
        let result = builder.i64_add(entry, left, right)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(checked_sub)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let left = builder.parameter(0)?;
        let right = builder.parameter(1)?;
        let result = builder.i64_sub(entry, left, right)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(checked_mul)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let left = builder.parameter(0)?;
        let right = builder.parameter(1)?;
        let result = builder.i64_mul(entry, left, right)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(checked_div)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let left = builder.parameter(0)?;
        let right = builder.parameter(1)?;
        let result = builder.i64_div(entry, left, right)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(f64_arithmetic)?;
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
        let mut builder = plan.function_builder(f64_branch)?;
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

    for (function, comparison) in f64_comparison_functions
        .iter()
        .copied()
        .zip(f64_comparisons)
    {
        let mut builder = plan.function_builder(function)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let left = builder.parameter(0)?;
        let right = builder.parameter(1)?;
        let result = builder.f64_compare(entry, comparison, left, right)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(bool_not)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let input = builder.parameter(0)?;
        let result = builder.bool_not(entry, input)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(bool_equal)?;
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
        let mut builder = plan.function_builder(callee)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let input = builder.parameter(0)?;
        let two = builder.i64_const(entry, 2)?;
        let result = builder.i64_mul(entry, input, two)?;
        builder.return_value(entry, result)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(direct_call)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let input = builder.parameter(0)?;
        let called = builder.call(entry, callee, vec![input])?;
        let returned = builder.runtime_call(entry, RuntimeCallSlot::IdentityI64V1, vec![called])?;
        builder.return_value(entry, returned)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(exit)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let code = builder.i64_const(entry, 17)?;
        builder.exit(entry, code)?;
        plan.define_function(builder.finish())?;
    }

    {
        let mut builder = plan.function_builder(unit)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let value = builder.parameter(0)?;
        builder.return_value(entry, value)?;
        plan.define_function(builder.finish())?;
    }

    let verified = plan.verify(BackendLimits::default())?;
    let image = encode(verified, EncodingConfig::new(versions))?;
    Ok((
        image,
        Entries {
            multi_block,
            loop_sum,
            checked_add,
            checked_sub,
            checked_mul,
            checked_div,
            f64_arithmetic,
            f64_branch,
            f64_comparisons: f64_comparison_functions,
            bool_not,
            bool_equal,
            direct_call,
            exit,
            unit,
        },
    ))
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn calls_multiblock_loop_scalar_and_structured_outcomes() -> Result<(), Box<dyn std::error::Error>>
{
    let (image, entries) = scalar_image(AbiVersions::current())?;
    assert!(!image.bytes().is_empty());
    assert!(!image.source_map().is_empty());
    assert!(!image.trap_map().is_empty());
    assert!(!image.outcome_map().is_empty());
    assert!(image
        .safepoints()
        .iter()
        .all(|point| point.stack_map().roots().is_empty()));
    let installer = ExecutableInstaller::default();
    let installed = installer.install(image)?;
    assert_eq!(
        installed.invoke(entries.multi_block, &[NativeValue::I64(7)])?,
        InvocationOutcome::Returned(NativeValue::I64(17))
    );
    assert_eq!(
        installed.invoke(entries.multi_block, &[NativeValue::I64(-3)])?,
        InvocationOutcome::Returned(NativeValue::I64(13))
    );
    assert_eq!(
        installed.invoke(entries.loop_sum, &[NativeValue::I64(100)])?,
        InvocationOutcome::Returned(NativeValue::I64(5_050))
    );
    assert_eq!(
        installed.invoke(entries.bool_not, &[NativeValue::Bool(true)])?,
        InvocationOutcome::Returned(NativeValue::Bool(false))
    );
    assert_eq!(
        installed.invoke(
            entries.bool_equal,
            &[NativeValue::Bool(true), NativeValue::Bool(true)],
        )?,
        InvocationOutcome::Returned(NativeValue::Bool(true))
    );
    assert_eq!(
        installed.invoke(
            entries.bool_equal,
            &[NativeValue::Bool(true), NativeValue::Bool(false)],
        )?,
        InvocationOutcome::Returned(NativeValue::Bool(false))
    );
    assert_eq!(
        installed.invoke(entries.direct_call, &[NativeValue::I64(21)])?,
        InvocationOutcome::Returned(NativeValue::I64(42))
    );
    assert_eq!(
        installed.invoke(entries.exit, &[])?,
        InvocationOutcome::Exited(17)
    );
    assert_eq!(
        installed.invoke(entries.unit, &[NativeValue::Unit])?,
        InvocationOutcome::Returned(NativeValue::Unit)
    );
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn exact_integer_traps_and_f64_bits_and_ordered_branches() -> Result<(), Box<dyn std::error::Error>>
{
    let (image, entries) = scalar_image(AbiVersions::current())?;
    let installer = ExecutableInstaller::default();
    let installed = installer.install(image)?;
    assert_eq!(
        installed.invoke(
            entries.checked_add,
            &[NativeValue::I64(i64::MAX), NativeValue::I64(1)],
        )?,
        InvocationOutcome::Trapped(TrapCode::I64Overflow)
    );
    assert_eq!(
        installed.invoke(
            entries.checked_sub,
            &[NativeValue::I64(i64::MIN), NativeValue::I64(1)],
        )?,
        InvocationOutcome::Trapped(TrapCode::I64Overflow)
    );
    assert_eq!(
        installed.invoke(
            entries.checked_mul,
            &[NativeValue::I64(i64::MAX), NativeValue::I64(2)],
        )?,
        InvocationOutcome::Trapped(TrapCode::I64Overflow)
    );
    assert_eq!(
        installed.invoke(
            entries.checked_div,
            &[NativeValue::I64(i64::MIN), NativeValue::I64(-1)],
        )?,
        InvocationOutcome::Trapped(TrapCode::I64Overflow)
    );
    assert_eq!(
        installed.invoke(
            entries.checked_div,
            &[NativeValue::I64(7), NativeValue::I64(0)],
        )?,
        InvocationOutcome::Trapped(TrapCode::DivisionByZero)
    );
    assert_eq!(
        installed.invoke(
            entries.checked_div,
            &[NativeValue::I64(-21), NativeValue::I64(3)],
        )?,
        InvocationOutcome::Returned(NativeValue::I64(-7))
    );
    assert_eq!(
        installed.invoke(
            entries.f64_arithmetic,
            &[NativeValue::f64(1.5), NativeValue::f64(2.0)],
        )?,
        InvocationOutcome::Returned(NativeValue::f64(1.5))
    );
    let ordered_cases = [
        (1.0, 1.0, true),
        (1.0, 2.0, true),
        (1.0, 2.0, true),
        (2.0, 2.0, true),
        (2.0, 1.0, true),
        (2.0, 2.0, true),
    ];
    for (function, (left, right, expected)) in
        entries.f64_comparisons.iter().copied().zip(ordered_cases)
    {
        assert_eq!(
            installed.invoke(function, &[NativeValue::f64(left), NativeValue::f64(right)],)?,
            InvocationOutcome::Returned(NativeValue::Bool(expected))
        );
        assert_eq!(
            installed.invoke(
                function,
                &[
                    NativeValue::F64Bits(0x7ff8_0000_0000_0001),
                    NativeValue::f64(right),
                ],
            )?,
            InvocationOutcome::Returned(NativeValue::Bool(false))
        );
    }
    assert_eq!(
        installed.invoke(
            entries.f64_branch,
            &[NativeValue::f64(1.0), NativeValue::f64(2.0)],
        )?,
        InvocationOutcome::Returned(NativeValue::F64Bits((-0.0_f64).to_bits()))
    );
    assert_eq!(
        installed.invoke(
            entries.f64_branch,
            &[
                NativeValue::F64Bits(0x7ff8_0000_0000_0001),
                NativeValue::f64(2.0)
            ],
        )?,
        InvocationOutcome::Returned(NativeValue::F64Bits(0x7ff8_0000_0000_1234))
    );
    Ok(())
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn enforces_versions_limits_wx_and_repeated_drop() -> Result<(), Box<dyn std::error::Error>> {
    let mismatched = AbiVersions::new(1, 1, 1);
    let (image, _) = scalar_image(mismatched)?;
    let installer = ExecutableInstaller::default();
    assert!(matches!(
        installer.install(image),
        Err(InstallError::VersionMismatch { .. })
    ));

    let (image, _) = scalar_image(AbiVersions::current())?;
    let accounting = image.accounting();
    let limits = ExecutableLimits::new(
        accounting.code_bytes() - 1,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
    );
    let installer = ExecutableInstaller::new(limits);
    assert!(matches!(
        installer.install(image),
        Err(InstallError::LimitExceeded(
            ExecutableLimitKind::ObjectCodeBytes
        ))
    ));

    let (first_image, _) = scalar_image(AbiVersions::current())?;
    let (second_image, _) = scalar_image(AbiVersions::current())?;
    let one_object_limits = ExecutableLimits::new(
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        1,
    );
    let one_object_installer = ExecutableInstaller::new(one_object_limits);
    let first_installed = one_object_installer.install(first_image)?;
    assert!(matches!(
        one_object_installer.install(second_image),
        Err(InstallError::LimitExceeded(
            ExecutableLimitKind::ObjectCount
        ))
    ));
    drop(first_installed);
    assert_eq!(one_object_installer.usage().objects(), 0);

    let (image, _) = scalar_image(AbiVersions::current())?;
    let installer = ExecutableInstaller::default();
    {
        let installed = installer.install(image)?;
        assert!(installed.wx_transition_verified());
        let permissions = installed.permissions()?;
        assert!(permissions.readable());
        assert!(!permissions.writable());
        assert!(permissions.executable());
        assert_eq!(installer.usage().objects(), 1);
    }
    assert_eq!(installer.usage().objects(), 0);
    assert_eq!(installer.usage().code_bytes(), 0);

    for _ in 0..32 {
        let (image, entries) = scalar_image(AbiVersions::current())?;
        let installed = installer.install(image)?;
        assert_eq!(
            installed.invoke(entries.direct_call, &[NativeValue::I64(9)])?,
            InvocationOutcome::Returned(NativeValue::I64(18))
        );
        drop(installed);
        assert_eq!(installer.usage().objects(), 0);
    }
    Ok(())
}
