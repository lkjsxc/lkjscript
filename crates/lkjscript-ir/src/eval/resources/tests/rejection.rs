use super::*;

#[test]
fn explicit_close_invalidates_and_reuse_does_not_revive_stale_keys() {
    let mut resources = session(20);
    let reader = resources
        .acquire_owned(ResourceKind::FileReader, true)
        .expect("acquire reader");
    let stale = reader.clone();
    resources.close(reader).expect("explicit close reader");
    assert!(matches!(
        resources.access_binding(
            &stale,
            ResourceKind::FileReader,
            provider_for_kind(ResourceKind::FileReader),
            ResourceOwnership::Owned,
        ),
        Err(ResourceTableError::StaleKey)
    ));

    let writer = resources
        .acquire_owned(ResourceKind::FileWriter, true)
        .expect("reuse closed slot");
    assert_eq!(
        stale.key.token_parts().slot(),
        writer.key.token_parts().slot()
    );
    assert_ne!(
        stale.key.token_parts().generation(),
        writer.key.token_parts().generation()
    );
    assert!(matches!(
        resources.access_binding(
            &stale,
            ResourceKind::FileReader,
            provider_for_kind(ResourceKind::FileReader),
            ResourceOwnership::Owned,
        ),
        Err(ResourceTableError::StaleKey)
    ));
    assert_exact_access(&mut resources, &writer);
    resources.close(writer).expect("close writer");
    assert_eq!(resources.metrics.resources_opened, 2);
    assert_eq!(resources.metrics.resources_closed, 2);
    assert_eq!(resources.metrics.slots_reused, 1);
    assert_eq!(resources.metrics.stale_key_failures, 2);
}

#[test]
fn kind_provider_and_scope_mismatches_reject_before_payload_access() {
    let mut first = session(21);
    let reader = first
        .acquire_owned(ResourceKind::FileReader, true)
        .expect("acquire reader");
    assert!(matches!(
        first.access_binding(
            &reader,
            ResourceKind::FileWriter,
            provider_for_kind(ResourceKind::FileReader),
            ResourceOwnership::Owned,
        ),
        Err(ResourceTableError::WrongKind {
            expected: ResourceKind::FileWriter,
            actual: ResourceKind::FileReader,
        })
    ));
    let wrong_provider = ProviderId::for_capability(CapabilityKind::Network);
    assert!(matches!(
        first.access_binding(
            &reader,
            ResourceKind::FileReader,
            wrong_provider,
            ResourceOwnership::Owned,
        ),
        Err(ResourceTableError::ProviderMismatch { .. })
    ));

    let mut second = session(22);
    assert!(matches!(
        second.access_binding(
            &reader,
            ResourceKind::FileReader,
            provider_for_kind(ResourceKind::FileReader),
            ResourceOwnership::Owned,
        ),
        Err(ResourceTableError::ScopeMismatch {
            expected,
            actual,
        }) if expected == second.scope() && actual == first.scope()
    ));
    first.close(reader).expect("close reader in owning scope");
    assert_eq!(first.teardown().ordinary_obligations, 0);
    assert_eq!(second.teardown().ordinary_obligations, 0);
}

#[test]
fn sqlite_connection_cannot_close_before_statement_finalize() {
    let mut resources = session(23);
    let connection = resources
        .acquire_owned(ResourceKind::SqliteConnection, true)
        .expect("open fake SQLite connection");
    let statement = resources
        .prepare_statement(&connection, true)
        .expect("prepare fake SQLite statement");
    assert!(resources
        .close_sqlite_connection(connection.clone())
        .expect_err("live statement protects connection")
        .contains("live children"));
    assert_exact_access(&mut resources, &connection);

    let stale_statement = statement.clone();
    resources
        .finalize_statement(statement)
        .expect("finalize statement");
    assert!(matches!(
        resources.access_binding(
            &stale_statement,
            ResourceKind::SqliteStatement,
            provider_for_kind(ResourceKind::SqliteStatement),
            ResourceOwnership::Owned,
        ),
        Err(ResourceTableError::StaleKey)
    ));
    resources
        .close_sqlite_connection(connection)
        .expect("close unprotected connection");
    assert_eq!(resources.teardown().ordinary_obligations, 0);
}

#[test]
fn failed_statement_acquisition_releases_parent_child_count() {
    let mut resources = session(24);
    let connection = resources
        .acquire_owned(ResourceKind::SqliteConnection, true)
        .expect("open fake SQLite connection");
    assert!(resources
        .prepare_statement(&connection, false)
        .expect_err("statement acquisition must fail")
        .contains("deterministic fake acquisition failure"));
    resources
        .close_sqlite_connection(connection)
        .expect("failed child reservation must not protect parent");
    assert_eq!(resources.metrics.failed_acquisitions, 1);
    assert_eq!(resources.teardown().ordinary_obligations, 0);
}
