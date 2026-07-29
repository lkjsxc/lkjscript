use super::*;

#[test]
fn cache_never_evicts_a_live_application_lease() -> Result<(), Box<dyn Error>> {
    let system = system(1, 1)?;
    let first_package = package(5)?;
    let second_package = package(6)?;
    let app = system.install(
        manifest(ApplicationKind::Command, 1, 1),
        first_package,
        chunk(false)?,
    )?;
    assert_eq!(
        system.install(
            manifest(ApplicationKind::Command, 1, 1),
            second_package,
            chunk(false)?
        ),
        Err(RuntimeError::PackageCacheFull)
    );
    system.remove(app)?;
    let _second = system.install(
        manifest(ApplicationKind::Command, 1, 1),
        second_package,
        chunk(false)?,
    )?;
    assert!(!system.cache_contains(first_package)?);
    assert!(system.cache_contains(second_package)?);
    assert_eq!(system.cache_len()?, 1);
    Ok(())
}

#[test]
fn capability_bearing_manifests_still_fail_closed() -> Result<(), Box<dyn Error>> {
    let system = system(1, 1)?;
    let mut manifest = manifest(ApplicationKind::Command, 1, 1);
    manifest
        .capabilities
        .push(lkjscript_core::CapabilityKind::Arguments);
    assert_eq!(
        system.install(manifest, package(7)?, chunk(false)?),
        Err(RuntimeError::UnsafeCapabilities)
    );
    Ok(())
}
