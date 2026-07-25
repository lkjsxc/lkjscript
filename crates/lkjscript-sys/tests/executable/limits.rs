use super::*;

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
