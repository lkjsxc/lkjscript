use super::*;

#[test]
fn sqlite_connection_rejects_close_until_statement_finalizes() {
    let mut table = ResourceTable::default();
    let connection = table
        .sqlite_open(":memory:", 0x0001_0006)
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
}
