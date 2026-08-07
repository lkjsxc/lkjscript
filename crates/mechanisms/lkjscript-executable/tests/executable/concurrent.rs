use super::*;

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
#[test]
fn one_sealed_generated_image_is_concurrently_callable() -> Result<(), Box<dyn std::error::Error>> {
    let (image, entries) = scalar_image(ImageContracts::current())?;
    let installer = ExecutableInstaller::default();
    let installed = std::sync::Arc::new(installer.install(image)?);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
    std::thread::scope(|scope| -> Result<(), TestInvocationError> {
        let mut handles = Vec::new();
        for worker in 0..8_i64 {
            let installed = std::sync::Arc::clone(&installed);
            let barrier = std::sync::Arc::clone(&barrier);
            handles.push(scope.spawn(move || -> Result<(), TestInvocationError> {
                barrier.wait();
                for value in 0..128_i64 {
                    assert_eq!(
                        installed.invoke(
                            entries.checked_add,
                            &[NativeValue::I64(worker), NativeValue::I64(value)],
                        )?,
                        InvocationOutcome::Returned(NativeValue::I64(worker + value))
                    );
                }
                Ok(())
            }));
        }
        for handle in handles {
            handle.join().map_err(|_| {
                TestInvocationError::Entered(EnteredInvocationError::InvalidNativeStatus(u32::MAX))
            })??;
        }
        Ok(())
    })?;
    assert_eq!(installer.usage().objects(), 1);
    drop(installed);
    assert_eq!(installer.usage().objects(), 0);
    Ok(())
}
