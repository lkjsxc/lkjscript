use super::*;

fn principal(process: u32) -> LocalPrincipal {
    LocalPrincipal {
        process,
        user: 1000,
        group: 100,
    }
}

fn record(success: ControlSuccess) -> Result<ControlledSession, ControlFailure> {
    match success {
        ControlSuccess::Session(session) => Ok(session),
        _ => Err(ControlFailure::Internal),
    }
}

#[test]
fn session_identity_ownership_lease_and_reaping_are_exact() -> Result<(), ControlFailure> {
    let mut registry = SessionRegistry::new();
    let first = record(registry.register(
        [1; 32],
        SessionBackend::None,
        principal(10),
        MonotonicTime(5),
    )?)?;
    assert_eq!(first.session, 1);
    assert_eq!(first.lease_deadline, 10_000_000_005);
    assert!(matches!(
        registry.register(
            [1; 32],
            SessionBackend::None,
            principal(10),
            MonotonicTime(6)
        ),
        Err(ControlFailure::Rejected(_))
    ));
    assert_eq!(
        registry.heartbeat(first.session, principal(11), MonotonicTime(7)),
        Err(ControlFailure::Unauthorized)
    );
    let renewed = record(registry.heartbeat(first.session, principal(10), MonotonicTime(8))?)?;
    assert_eq!(renewed.lease_deadline, 10_000_000_008);
    assert_eq!(registry.live_count(MonotonicTime(10_000_000_007)), 1);
    assert!(matches!(
        registry.list(MonotonicTime(10_000_000_008)),
        ControlSuccess::Sessions(sessions) if sessions.is_empty()
    ));

    let second = record(registry.register(
        [2; 32],
        SessionBackend::None,
        principal(12),
        MonotonicTime(10_000_000_009),
    )?)?;
    assert_eq!(second.session, 2);
    assert_eq!(
        registry.unregister(second.session, principal(12), MonotonicTime(10_000_000_010)),
        Ok(ControlSuccess::SessionUnregistered { session: 2 })
    );
    assert_eq!(registry.live_count(MonotonicTime(10_000_000_010)), 0);
    Ok(())
}

#[test]
fn session_registry_enforces_its_aggregate_bound() -> Result<(), ControlFailure> {
    let mut registry = SessionRegistry::new();
    for value in 1..=MAX_SESSIONS {
        let mut instance = [0_u8; 32];
        let identity = u64::try_from(value).map_err(|_| ControlFailure::Internal)?;
        instance[..8].copy_from_slice(&identity.to_le_bytes());
        registry.register(
            instance,
            SessionBackend::None,
            principal(u32::try_from(value).map_err(|_| ControlFailure::Internal)? + 1),
            MonotonicTime(0),
        )?;
    }
    assert!(matches!(
        registry.register(
            [255; 32],
            SessionBackend::None,
            principal(100),
            MonotonicTime(1)
        ),
        Err(ControlFailure::Rejected(_))
    ));
    registry.clear();
    assert_eq!(registry.live_count(MonotonicTime(1)), 0);
    Ok(())
}
