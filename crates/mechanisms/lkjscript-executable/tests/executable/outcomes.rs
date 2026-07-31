use super::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn calls_multiblock_loop_scalar_and_structured_outcomes() -> Result<(), Box<dyn std::error::Error>>
{
    let (image, entries) = scalar_image(ImageContracts::current())?;
    assert!(!image.bytes().is_empty());
    assert!(!image.source_map().is_empty());
    assert!(!image.trap_map().is_empty());
    assert!(!image.outcome_map().is_empty());
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
