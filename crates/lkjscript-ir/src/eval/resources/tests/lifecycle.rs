use super::*;

#[test]
fn all_resource_kinds_have_exact_fake_bindings() {
    let mut resources = session(10);
    let mut handles = Vec::new();
    handles.push(
        resources
            .standard_input
            .clone()
            .expect("borrowed standard input"),
    );
    handles.push(
        resources
            .standard_output
            .clone()
            .expect("borrowed standard output"),
    );
    let mut connection = None;
    for kind in ResourceKind::ALL {
        if matches!(
            kind,
            ResourceKind::InputStream | ResourceKind::OutputStream | ResourceKind::SqliteStatement
        ) {
            continue;
        }
        let resource = resources
            .acquire_owned(kind, true)
            .expect("deterministic fake acquisition");
        if kind == ResourceKind::SqliteConnection {
            connection = Some(resource.clone());
        }
        handles.push(resource);
    }
    handles.push(
        resources
            .prepare_statement(
                &connection.expect("SQLite connection acquired before statement"),
                true,
            )
            .expect("prepare fake statement"),
    );

    for resource in &handles {
        assert_eq!(resource.provider, provider_for_kind(resource.kind));
        assert_eq!(resource.scope, resources.scope());
        assert_eq!(resource.ownership, ownership_for_kind(resource.kind));
        assert_exact_access(&mut resources, resource);
        assert_eq!(resources.table.stats().active_for(resource.kind), 1);
    }
    assert_eq!(handles.len(), ResourceKind::ALL.len());

    let teardown = resources.teardown();
    assert_eq!(teardown.ordinary_obligations, 9);
    assert_eq!(teardown.emergency_obligations.len(), 9);
    assert_eq!(teardown.cleanup_attempts.len(), 9);
    assert_eq!(resources.metrics.borrowed_installed, 2);
    assert_eq!(resources.metrics.borrowed_removed, 2);
    assert_eq!(teardown.remaining.ordinary_obligations(), 0);
    assert_eq!(teardown.remaining.borrowed_open(), 0);
    assert!(teardown.cleanup_error.is_none());
}

#[test]
fn implicit_drop_glue_invalidates_owned_resource_before_payload_release() {
    let mut resources = session(40);
    let reader = resources
        .acquire_owned(ResourceKind::FileReader, true)
        .expect("acquire fake reader");
    let stale = reader.clone();
    resources
        .drop_owned(reader, ResourceKind::FileReader)
        .expect("exact implicit resource glue");
    assert!(matches!(
        resources.access_binding(
            &stale,
            ResourceKind::FileReader,
            provider_for_kind(ResourceKind::FileReader),
            ResourceOwnership::Owned,
        ),
        Err(ResourceTableError::StaleKey)
    ));
    assert_eq!(resources.metrics.resources_closed, 1);
    let teardown = resources.teardown();
    assert_eq!(teardown.ordinary_obligations, 0);
    assert!(teardown.cleanup_attempts.is_empty());
}

#[test]
fn borrowed_standard_streams_are_not_guest_owned() {
    let mut resources = session(11);
    let input = resources
        .standard_input
        .clone()
        .expect("borrowed standard input");
    let output = resources
        .standard_output
        .clone()
        .expect("borrowed standard output");
    assert_exact_access(&mut resources, &input);
    assert_exact_access(&mut resources, &output);
    assert!(matches!(
        resources.reject_borrowed_close(input),
        Err(ResourceTableError::OwnershipMismatch {
            expected: ResourceOwnership::Owned,
            actual: ResourceOwnership::Borrowed,
        })
    ));
    assert_eq!(resources.table.stats().ordinary_obligations(), 0);

    let teardown = resources.teardown();
    assert_eq!(teardown.ordinary_obligations, 0);
    assert!(teardown.emergency_obligations.is_empty());
    assert!(teardown.cleanup_attempts.is_empty());
    assert_eq!(resources.metrics.borrowed_removed, 2);
}

#[test]
fn failed_acquisition_cancels_its_reservation() {
    let mut resources = session(12);
    assert!(resources
        .acquire_owned(ResourceKind::FileReader, false)
        .expect_err("fake acquisition must fail")
        .contains("deterministic fake acquisition failure"));
    let stats = resources.table.stats();
    assert_eq!(stats.reserved(), 0);
    assert_eq!(stats.ordinary_obligations(), 0);
    assert_eq!(resources.metrics.failed_acquisitions, 1);
    assert_eq!(resources.metrics.resources_opened, 0);

    let reader = resources
        .acquire_owned(ResourceKind::FileReader, true)
        .expect("vacated reservation is reusable");
    assert_exact_access(&mut resources, &reader);
    resources.close(reader).expect("close reader");
    let teardown = resources.teardown();
    assert_eq!(teardown.ordinary_obligations, 0);
    assert!(teardown.cleanup_attempts.is_empty());
}
