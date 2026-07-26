#![allow(clippy::expect_used)]

use std::os::unix::ffi::OsStrExt;

use super::{Connection, Step};

const OPEN_RW_CREATE_FULLMUTEX: i64 = 0x0001_0006;

#[test]
fn sqlite_path_api_rejects_relative_nul_and_oversized_bytes() {
    assert!(Connection::open(b":memory:", OPEN_RW_CREATE_FULLMUTEX).is_err());
    assert!(Connection::open(b"/nul\0byte", OPEN_RW_CREATE_FULLMUTEX).is_err());
    let oversized = vec![b'/'; crate::native_path::MAX_PATH_BYTES + 1];
    assert!(Connection::open(&oversized, OPEN_RW_CREATE_FULLMUTEX).is_err());
}

#[test]
fn database_prepares_binds_and_reads_exact_values() -> std::io::Result<()> {
    let path = std::env::temp_dir().join(format!(
        "lkjscript-sqlite-values-{}-{}.sqlite",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_file(&path);
    let connection =
        Connection::open(path.as_os_str().as_bytes(), OPEN_RW_CREATE_FULLMUTEX).expect("open");
    connection
        .exec("CREATE TABLE sample (number INTEGER, text TEXT, bytes BLOB)")
        .expect("schema");
    let insert = connection
        .prepare("INSERT INTO sample VALUES (?1, ?2, ?3)")
        .expect("prepare insert");
    insert.bind_i64(1, i64::MIN).expect("integer");
    insert.bind_text(2, "日本語").expect("text");
    insert.bind_bytes(3, &[0, 255]).expect("bytes");
    assert_eq!(insert.step().expect("step"), Step::Done);
    let query = connection
        .prepare("SELECT number, text, bytes FROM sample")
        .expect("prepare query");
    assert_eq!(query.step().expect("row"), Step::Row);
    assert_eq!(query.column_i64(0).expect("number"), Some(i64::MIN));
    assert_eq!(
        query.column_text(1, 1_000).expect("text"),
        Some("日本語".into())
    );
    assert_eq!(
        query.column_bytes(2, 1_000).expect("bytes"),
        Some(vec![0, 255])
    );
    assert_eq!(query.step().expect("done"), Step::Done);
    drop(query);
    drop(insert);
    drop(connection);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn file_database_backup_restores_durable_rows() {
    let root = std::env::temp_dir().join(format!(
        "lkjscript-sqlite-{}-{}",
        std::process::id(),
        line!()
    ));
    let source_path = root.with_extension("source.sqlite");
    let backup_path = root.with_extension("backup.sqlite");
    let source = Connection::open(source_path.as_os_str().as_bytes(), OPEN_RW_CREATE_FULLMUTEX)
        .expect("open source");
    source.busy_timeout(100).expect("busy timeout");
    source
        .exec("CREATE TABLE sample (number INTEGER); INSERT INTO sample VALUES (9)")
        .expect("write source");
    source
        .backup_to(backup_path.as_os_str().as_bytes(), OPEN_RW_CREATE_FULLMUTEX)
        .expect("backup");
    drop(source);
    let restored = Connection::open(backup_path.as_os_str().as_bytes(), OPEN_RW_CREATE_FULLMUTEX)
        .expect("open backup");
    let query = restored
        .prepare("SELECT number FROM sample")
        .expect("prepare restored query");
    assert_eq!(query.step().expect("restored row"), Step::Row);
    assert_eq!(query.column_i64(0).expect("restored number"), Some(9));
    drop(query);
    drop(restored);
    let _ = std::fs::remove_file(source_path);
    let _ = std::fs::remove_file(backup_path);
}
