use std::error::Error;
use std::sync::Arc;

use lkjscript_database::{Database, DatabaseLimits, DatabaseTenantService};
use lkjscript_host::{DatabaseTenantFactory, FakeDurableStorage};

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
