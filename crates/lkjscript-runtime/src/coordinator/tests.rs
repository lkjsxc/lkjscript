use std::error::Error;
use std::num::NonZeroUsize;

use lkjscript_host::FakeDurableStorage;

use super::*;

#[test]
fn coordinator_recovers_clean_and_unclean_boots_without_database() -> Result<(), Box<dyn Error>> {
    let storage = FakeDurableStorage::new();
    let identity = CoordinatorIdentity::new(7).ok_or("identity")?;
    let mut first =
        MachineCoordinator::start(identity, 1000, storage.clone(), NonZeroUsize::MIN, None)?;
    assert!(first.status()?.previous_shutdown_clean);
    first.shutdown()?;
    storage.crash();
    let second =
        MachineCoordinator::start(identity, 1000, storage.clone(), NonZeroUsize::MIN, None)?;
    assert!(second.status()?.previous_shutdown_clean);
    drop(second);
    storage.crash();
    let third = MachineCoordinator::start(identity, 1000, storage, NonZeroUsize::MIN, None)?;
    assert!(!third.status()?.previous_shutdown_clean);
    Ok(())
}
