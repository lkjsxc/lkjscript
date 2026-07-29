use std::error::Error;
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::Arc;

use lkjscript_core::{validate_chunk, Chunk, Constant, ExecutionOutcome, Op, ValidationLimits};

use super::*;

mod cache;

fn package(tag: u8) -> Result<PackageContentId, Box<dyn Error>> {
    PackageContentId::new([tag; 32]).ok_or_else(|| "zero package digest".into())
}

fn chunk(trap: bool) -> Result<Arc<lkjscript_core::ValidatedChunk>, Box<dyn Error>> {
    let mut chunk = Chunk::new();
    if trap {
        chunk.constants.push(Constant::Str("isolated trap".into()));
        chunk.main.emit_op_u16(Op::LoadConst, 0);
        chunk.main.emit(Op::Trap);
    } else {
        chunk.main.emit(Op::Unit);
        chunk.main.emit(Op::Return);
    }
    Ok(Arc::new(validate_chunk(
        chunk,
        &ValidationLimits::default(),
    )?))
}

fn manifest(kind: ApplicationKind, concurrent: usize, total: u64) -> ApplicationManifest {
    ApplicationManifest {
        name: format!("test-{kind:?}"),
        kind,
        scope: DeploymentScope::Standalone,
        cell: ExecutionCellClass::TrustedInProcess,
        capabilities: Vec::new(),
        quota: ResourceQuota {
            max_concurrent_invocations: NonZeroUsize::new(concurrent).unwrap_or(NonZeroUsize::MIN),
            max_total_invocations: NonZeroU64::new(total).unwrap_or(NonZeroU64::MIN),
            execution: lkjscript_core::ExecutionConfig::default(),
        },
        restart: RestartPolicy::Never,
    }
}

fn system(identity: u64, cache_entries: usize) -> Result<RuntimeSystem, Box<dyn Error>> {
    let identity =
        CoordinatorIdentity::new(identity).ok_or("coordinator identity must be nonzero")?;
    let entries = NonZeroUsize::new(cache_entries).ok_or("cache bound must be nonzero")?;
    Ok(RuntimeSystem::new(identity, entries))
}

#[test]
fn identities_and_lifecycle_reject_zero_and_illegal_steps() -> Result<(), Box<dyn Error>> {
    assert!(CoordinatorIdentity::new(0).is_none());
    assert!(ApplicationId::new(0).is_none());
    assert!(PackageContentId::new([0; 32]).is_none());
    assert_eq!(
        Lifecycle::Installed.transition(Lifecycle::Running),
        Err(RuntimeError::IllegalTransition {
            from: Lifecycle::Installed,
            to: Lifecycle::Running,
        })
    );
    Ok(())
}

#[test]
fn two_apps_invoke_concurrently_and_a_trap_is_isolated() -> Result<(), Box<dyn Error>> {
    let system = system(1, 2)?;
    let good = system.install(
        manifest(ApplicationKind::Command, 1, 4),
        package(1)?,
        chunk(false)?,
        lkjscript_host::HostEnvironment::default(),
    )?;
    let bad = system.install(
        manifest(ApplicationKind::Service, 1, 4),
        package(2)?,
        chunk(true)?,
        lkjscript_host::HostEnvironment::default(),
    )?;
    let good_incarnation = system.start(good)?;
    let bad_incarnation = system.start(bad)?;
    let results = system.invoke_concurrent(vec![
        InvocationRequest {
            incarnation: good_incarnation,
            arguments: Vec::new(),
        },
        InvocationRequest {
            incarnation: bad_incarnation,
            arguments: Vec::new(),
        },
    ])?;
    let outcomes = results.into_iter().collect::<Result<Vec<_>, _>>()?;
    assert!(matches!(outcomes[0].outcome, ExecutionOutcome::Returned(_)));
    assert!(matches!(outcomes[1].outcome, ExecutionOutcome::Trapped(_)));
    assert_eq!(system.status(good)?.metrics.trapped, 0);
    assert_eq!(system.status(bad)?.metrics.trapped, 1);
    assert!(matches!(
        system.invoke(good_incarnation, Vec::new())?.outcome,
        ExecutionOutcome::Returned(_)
    ));
    Ok(())
}

#[test]
fn restart_changes_incarnation_and_rejects_stale_identity() -> Result<(), Box<dyn Error>> {
    let system = system(1, 1)?;
    let app = system.install(
        manifest(ApplicationKind::Command, 1, 4),
        package(3)?,
        chunk(false)?,
        lkjscript_host::HostEnvironment::default(),
    )?;
    let first = system.start(app)?;
    let second = system.restart(first)?;
    assert_ne!(first, second);
    assert_eq!(second.incarnation(), first.incarnation() + 1);
    assert!(matches!(
        system.invoke(first, Vec::new()),
        Err(RuntimeError::StaleIncarnation { .. })
    ));
    assert!(matches!(
        system.invoke(second, Vec::new())?.outcome,
        ExecutionOutcome::Returned(_)
    ));
    let foreign = RuntimeSystem::new(
        CoordinatorIdentity::new(2).ok_or("foreign coordinator identity must be nonzero")?,
        NonZeroUsize::MIN,
    );
    let foreign_app = foreign.install(
        manifest(ApplicationKind::Command, 1, 1),
        package(9)?,
        chunk(false)?,
        lkjscript_host::HostEnvironment::default(),
    )?;
    let foreign_incarnation = foreign.start(foreign_app)?;
    assert!(matches!(
        system.invoke(foreign_incarnation, Vec::new()),
        Err(RuntimeError::StaleIncarnation { .. })
    ));
    Ok(())
}

#[test]
fn ticket_admission_has_no_starvation_and_enforces_total_quota() -> Result<(), Box<dyn Error>> {
    let system = system(1, 1)?;
    let app = system.install(
        manifest(ApplicationKind::Service, 1, 8),
        package(4)?,
        chunk(false)?,
        lkjscript_host::HostEnvironment::default(),
    )?;
    let incarnation = system.start(app)?;
    let requests = (0..8)
        .map(|index| InvocationRequest {
            incarnation,
            arguments: vec![index.to_string()],
        })
        .collect();
    let results = system.invoke_concurrent(requests)?;
    assert_eq!(results.len(), 8);
    assert!(results.into_iter().all(|result| result.is_ok()));
    let status = system.status(app)?;
    assert_eq!(status.metrics.admitted, 8);
    assert_eq!(status.metrics.completed, 8);
    assert_eq!(status.metrics.peak_concurrent, 1);
    assert_eq!(
        system.invoke(incarnation, Vec::new()),
        Err(RuntimeError::QuotaExceeded(QuotaKind::TotalInvocations))
    );
    Ok(())
}
