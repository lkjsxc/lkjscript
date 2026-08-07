use std::os::unix::ffi::OsStrExt;

use super::*;

#[test]
fn integer_and_borrowed_handles_cannot_be_closed() {
    let mut table = ResourceTable::default();
    assert_eq!(table.table.stats().borrowed_open(), 1);
    assert_eq!(table.metrics().resources_opened(), 0);
    let integer = Value::from_i64(16);
    assert!(table.close(integer).is_err());
    assert!(table.close(ResourceTable::stdin_handle()).is_err());
    assert_eq!(table.metrics().resources_closed(), 0);
    let teardown = table.teardown();
    assert_eq!(teardown.ordinary_obligations(), 0);
    assert_eq!(teardown.emergency_obligations(), 0);
    assert_eq!(teardown.cleanup_attempts(), 0);
}

#[test]
fn closed_slots_reuse_with_a_new_generation_and_reject_stale_tokens() -> std::io::Result<()> {
    let file = TempFile::new()?;
    let path = file.0.as_os_str().as_bytes();
    let mut table = ResourceTable::default();
    let first = table
        .sys_open_read(path)
        .expect("open first temporary file");
    assert_ne!(first, ResourceTable::stdin_handle());
    assert_eq!(table.allocated_handle_slots(), 1);
    table.close(first).expect("close first file");

    let second = table
        .sys_open_read(path)
        .expect("open second temporary file");
    assert_ne!(first, second);
    assert_eq!(table.allocated_handle_slots(), 1);
    assert!(table.close(first).is_err());
    assert!(table.read_byte(first).is_err());
    table.close(second).expect("close second file");

    let metrics = table.metrics();
    assert_eq!(metrics.resources_opened(), 2);
    assert_eq!(metrics.resources_closed(), 2);
    assert_eq!(metrics.slots_reused(), 1);
    assert_eq!(metrics.stale_key_failures(), 2);
    Ok(())
}

#[test]
fn failed_acquisition_publishes_no_key_or_open_obligation() {
    let mut table = ResourceTable::default();
    let missing = b"/lkjscript-test-path-that-must-not-exist";
    assert!(table.sys_open_read(missing).is_err());
    assert_eq!(table.metrics().resources_opened(), 0);
    assert_eq!(table.table.stats().reserved(), 0);
    assert_eq!(table.table.stats().owned_open(), 0);
    assert_eq!(table.table.stats().ordinary_obligations(), 0);
}

#[test]
fn opaque_resource_tokens_cross_u32_without_packing_slot_or_generation(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let file = TempFile::new()?;
    let mut table = ResourceTable::default();
    let high = u64::from(u32::MAX) + 23;
    table.next_token = std::num::NonZeroU64::new(high);
    let handle = table.sys_open_read(file.0.as_os_str().as_bytes())?;
    assert_eq!(handle.as_resource(), Some(high));
    table.close(handle)?;
    Ok(())
}

#[test]
fn guest_token_resolution_checks_provider_and_scope(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let file = TempFile::new()?;
    let mut table = ResourceTable::default();
    let handle = table.sys_open_read(file.0.as_os_str().as_bytes())?;
    let parts = table.decode_handle_for_test(handle)?;
    let scope = table.scope_id();

    assert!(matches!(
        table.table.resolve_token_parts(
            parts,
            lkjscript_core::ResourceKind::FileReader,
            crate::host_ext::NETWORK_PROVIDER,
            scope,
            lkjscript_core::ResourceOwnership::Owned,
        ),
        Err(lkjscript_core::ResourceTableError::ProviderMismatch { .. })
    ));
    let other_scope = lkjscript_core::ScopeId::new(scope.get() + 1).expect("next test scope");
    assert!(matches!(
        table.table.resolve_token_parts(
            parts,
            lkjscript_core::ResourceKind::FileReader,
            crate::host_ext::FILESYSTEM_PROVIDER,
            other_scope,
            lkjscript_core::ResourceOwnership::Owned,
        ),
        Err(lkjscript_core::ResourceTableError::ScopeMismatch { .. })
    ));
    table.close(handle)?;
    Ok(())
}
#[test]
fn durable_file_capabilities_check_kind_staleness_and_effects(
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let file = TempFile::new()?;
    let appended = std::env::temp_dir().join(format!(
        "lkjscript-durable-new-{}-{}",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let renamed = std::env::temp_dir().join(format!(
        "lkjscript-durable-rename-{}-{}",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = fs::remove_file(&appended);
    let _ = fs::remove_file(&renamed);
    let directory = std::env::temp_dir().join(format!(
        "lkjscript-durable-dir-{}-{}",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&directory)?;

    let mut table = ResourceTable::default();
    let reader = table.sys_open_read(file.0.as_os_str().as_bytes())?;
    assert!(table.write_byte(reader, b'z'.into()).is_err());
    assert!(table.sys_fsync(reader).is_err());
    assert!(table.sys_truncate(reader, 0).is_err());
    table.close(reader)?;

    let append = table.sys_open_append(file.0.as_os_str().as_bytes())?;
    table.write_byte(append, b'y'.into())?;
    table.sys_fsync(append)?;
    table.sys_truncate(append, 1)?;
    table.close(append)?;
    assert_eq!(fs::read(&file.0)?, b"x");
    assert!(table.sys_fsync(append).is_err());

    let created = table.sys_open_create_new(appended.as_os_str().as_bytes())?;
    assert!(table
        .sys_open_create_new(appended.as_os_str().as_bytes())
        .is_err());
    table.close(created)?;
    ResourceTable::sys_rename(
        file.0.as_os_str().as_bytes(),
        renamed.as_os_str().as_bytes(),
    )?;
    assert!(renamed.is_file());

    let dir = table.sys_open_dir(directory.as_os_str().as_bytes())?;
    table.sys_fsync(dir)?;
    assert!(table.sys_truncate(dir, 0).is_err());
    assert!(table.write_byte(dir, 0).is_err());
    table.close(dir)?;
    let _ = fs::remove_file(&appended);
    let _ = fs::remove_file(&renamed);
    fs::remove_dir(&directory)?;
    Ok(())
}
