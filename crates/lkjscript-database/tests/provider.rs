use std::error::Error;
use std::sync::Arc;

use lkjscript_database::{Database, DatabaseLimits, DatabaseTenantService, MAX_VALUE_BYTES};
use lkjscript_host::{DatabaseTenantFactory, DatabaseTransactionId, FakeDurableStorage, HostError};

#[test]
fn tenant_provider_preserves_snapshots_isolation_and_incarnation_identity(
) -> Result<(), Box<dyn Error>> {
    let storage = Arc::new(FakeDurableStorage::new());
    let database = Database::create(storage, "provider", DatabaseLimits::default())?;
    let service = DatabaseTenantService::new(database.clone());
    let alpha = service.attach("application-1", 7)?;
    let beta = service.attach("application-2", 7)?;

    let write = alpha.begin_write()?;
    alpha.put(write, b"key".to_vec(), b"first".to_vec())?;
    let old_snapshot = alpha.begin_read()?;
    alpha.commit(write)?;
    service.checkpoint()?;
    assert_eq!(alpha.get(old_snapshot, b"key")?, None);
    alpha.commit(old_snapshot)?;

    let current = alpha.begin_read()?;
    assert_eq!(alpha.get(current, b"key")?, Some(b"first".to_vec()));
    assert!(matches!(
        beta.get(current, b"key"),
        Err(lkjscript_host::HostError::PermissionDenied(_))
    ));
    alpha.commit(current)?;
    let beta_read = beta.begin_read()?;
    assert_eq!(beta.get(beta_read, b"key")?, None);
    beta.abort(beta_read)?;

    let aborted = alpha.begin_write()?;
    alpha.put(aborted, b"discarded".to_vec(), b"value".to_vec())?;
    assert_eq!(alpha.abort_all()?, 1);
    let replacement = alpha.begin_write()?;
    alpha.abort(replacement)?;
    let verify = alpha.begin_read()?;
    assert_eq!(alpha.get(verify, b"discarded")?, None);
    alpha.abort(verify)?;

    drop((alpha, beta, service));
    database.close()?;
    Ok(())
}

#[test]
fn tenant_provider_range_zero_returns_no_rows() -> Result<(), Box<dyn Error>> {
    let storage = Arc::new(FakeDurableStorage::new());
    let database = Database::create(storage, "provider-zero-range", DatabaseLimits::default())?;
    let service = DatabaseTenantService::new(database.clone());
    let provider = service.attach("application-1", 1)?;
    let write = provider.begin_write()?;
    provider.put(write, b"key".to_vec(), b"value".to_vec())?;
    provider.commit(write)?;

    let read = provider.begin_read()?;
    assert!(provider.range(read, b"", b"", 0)?.is_empty());
    assert_eq!(provider.get(read, b"key")?, Some(b"value".to_vec()));
    provider.abort(read)?;

    drop((provider, service));
    database.close()?;
    Ok(())
}

#[test]
fn tenant_provider_range_enforces_aggregate_returned_bytes() -> Result<(), Box<dyn Error>> {
    let storage = Arc::new(FakeDurableStorage::new());
    let limits = DatabaseLimits {
        logical_write_buffer_bytes: 16 * 1024 * 1024,
    };
    let database = Database::create(storage, "provider-range-bytes", limits)?;
    let service = DatabaseTenantService::new(database.clone());
    let provider = service.attach("application-1", 1)?;
    let write = provider.begin_write()?;
    for index in 0..8 {
        let key = format!("key-{index}").into_bytes();
        let value = vec![index; MAX_VALUE_BYTES - key.len()];
        provider.put(write, key, value)?;
    }
    provider.put(write, b"key-8".to_vec(), vec![8])?;
    provider.commit(write)?;

    let read = provider.begin_read()?;
    let exact = provider.range(read, b"", b"", 8)?;
    assert_eq!(exact.len(), 8);
    assert!(exact
        .iter()
        .all(|(key, value)| key.len() + value.len() == MAX_VALUE_BYTES));
    assert!(matches!(
        provider.range(read, b"", b"", 9),
        Err(HostError::Io { operation, message })
            if operation == "range database transaction"
                && message == "provider returned byte bound reached"
    ));
    assert_eq!(
        provider.get(read, b"key-0")?.map(|value| value.len()),
        Some(MAX_VALUE_BYTES - b"key-0".len())
    );
    provider.abort(read)?;

    drop((provider, service));
    database.close()?;
    Ok(())
}

#[test]
fn tenant_provider_rejects_stale_incarnation_before_lookup() -> Result<(), Box<dyn Error>> {
    let storage = Arc::new(FakeDurableStorage::new());
    let database = Database::create(storage, "provider-stale", DatabaseLimits::default())?;
    let service = DatabaseTenantService::new(database.clone());
    let provider = service.attach("application-1", 7)?;
    let current = provider.begin_read()?;
    let stale = DatabaseTransactionId::new(current.provider(), current.slot(), 8)
        .ok_or_else(|| std::io::Error::other("nonzero stale transaction identity"))?;

    assert!(matches!(
        provider.get(stale, b"key"),
        Err(HostError::PermissionDenied(message))
            if message == "stale or foreign database transaction"
    ));
    assert_eq!(provider.get(current, b"key")?, None);
    provider.abort(current)?;

    drop((provider, service));
    database.close()?;
    Ok(())
}
