use std::os::unix::ffi::OsStrExt;

use super::*;

#[test]
fn sqlite_connection_rejects_close_until_statement_finalizes() -> std::io::Result<()> {
    let path = std::env::temp_dir().join(format!(
        "lkjscript-vm-sqlite-{}-{}.sqlite",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_file(&path);
    let mut table = ResourceTable::default();
    let connection = table
        .sqlite_open(path.as_os_str().as_bytes(), 0x0001_0006)
        .expect("open SQLite connection");
    let statement = table
        .sqlite_prepare(connection, "SELECT 1")
        .expect("prepare SQLite statement");
    assert!(table.sqlite_close(connection).is_err());
    assert!(table.close(statement).is_err());
    table
        .sqlite_finalize(statement)
        .expect("finalize statement");
    assert!(table.sqlite_step(statement).is_err());
    table.sqlite_close(connection).expect("close connection");
    assert!(table.sqlite_close(connection).is_err());
    std::fs::remove_file(path)?;
    Ok(())
}
