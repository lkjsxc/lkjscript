use super::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn exact_integer_traps_and_f64_bits_and_ordered_branches() -> Result<(), Box<dyn std::error::Error>>
{
    let (image, entries) = scalar_image(ImageContracts::current())?;
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
