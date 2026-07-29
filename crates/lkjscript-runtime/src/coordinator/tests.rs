use std::error::Error;
use std::num::NonZeroUsize;

use lkjscript_host::{FakeDurableStorage, PortableClock};

use super::*;

#[test]
fn coordinator_recovers_clean_and_unclean_boots_without_database() -> Result<(), Box<dyn Error>> {
    let storage = FakeDurableStorage::new();
    let identity = CoordinatorIdentity::new(7).ok_or("identity")?;
    let mut first = start(identity, storage.clone())?;
    assert!(first.status()?.previous_shutdown_clean);
    first.shutdown()?;
    storage.crash();
    let second = start(identity, storage.clone())?;
    assert!(second.status()?.previous_shutdown_clean);
    drop(second);
    storage.crash();
    let third = start(identity, storage)?;
    assert!(!third.status()?.previous_shutdown_clean);
    Ok(())
}

fn start(
    identity: CoordinatorIdentity,
    storage: FakeDurableStorage,
) -> Result<MachineCoordinator<FakeDurableStorage>, CoordinatorError> {
    MachineCoordinator::start(
        identity,
        1000,
        storage,
        NonZeroUsize::MIN,
        Arc::new(PortableClock::new()),
        None,
    )
}
