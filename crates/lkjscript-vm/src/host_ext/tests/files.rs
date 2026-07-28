use std::os::unix::ffi::OsStrExt;

use super::*;

#[test]
fn integer_and_borrowed_handles_cannot_be_closed() {
    let mut table = ResourceTable::default();
    let integer = Value::from_i64(16);
    assert!(table.close(integer).is_err());
    assert!(table.close(ResourceTable::stdin_handle()).is_err());
}
#[test]
fn closed_tokens_are_never_reused() -> std::io::Result<()> {
    let file = TempFile::new()?;
    let path = file.0.as_os_str().as_bytes();
    let mut table = ResourceTable::default();
    let first = table.sys_open_read(path).ok();
    assert!(first.is_some());
    let first = first.expect("open first temporary file");
    assert_ne!(first, ResourceTable::stdin_handle());
    assert!(table.close(first).is_ok());
    assert!(table.close(first).is_err());
    assert!(table.read_byte(first).is_err());

    let second = table.sys_open_read(path).ok();
    assert!(second.is_some());
    let second = second.expect("open second temporary file");
    assert_ne!(first, second);
    assert!(table.close(second).is_ok());
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
