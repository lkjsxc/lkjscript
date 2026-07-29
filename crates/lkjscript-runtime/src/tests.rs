use std::error::Error;
use std::num::{NonZeroU64, NonZeroUsize};
use std::sync::Arc;

use lkjscript_core::{validate_chunk, Chunk, Constant, ExecutionOutcome, Op, ValidationLimits};

use super::*;

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
        capabilities: Vec::new(),
        quota: ResourceQuota {
            max_concurrent_invocations: NonZeroUsize::new(concurrent).unwrap_or(NonZeroUsize::MIN),
            max_total_invocations: NonZeroU64::new(total).unwrap_or(NonZeroU64::MIN),
            execution: lkjscript_core::ExecutionConfig::default(),
        },
        restart: RestartPolicy::Never,
    }
}

fn node(cache_entries: usize) -> Result<Node, Box<dyn Error>> {
    let identity = NodeIdentity::new(1).ok_or("node identity must be nonzero")?;
    let entries = NonZeroUsize::new(cache_entries).ok_or("cache bound must be nonzero")?;
    Ok(Node::new(identity, entries))
}

#[test]
fn identities_and_lifecycle_reject_zero_and_illegal_steps() -> Result<(), Box<dyn Error>> {
    assert!(NodeIdentity::new(0).is_none());
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
    let node = node(2)?;
    let good = node.install(
        manifest(ApplicationKind::Command, 1, 4),
        package(1)?,
        chunk(false)?,
    )?;
    let bad = node.install(
        manifest(ApplicationKind::Service, 1, 4),
        package(2)?,
        chunk(true)?,
    )?;
    let good_generation = node.start(good)?;
    let bad_generation = node.start(bad)?;
    let results = node.invoke_concurrent(vec![
        InvocationRequest {
            generation: good_generation,
            arguments: Vec::new(),
        },
        InvocationRequest {
            generation: bad_generation,
            arguments: Vec::new(),
        },
    ])?;
    let outcomes = results.into_iter().collect::<Result<Vec<_>, _>>()?;
    assert!(matches!(outcomes[0].outcome, ExecutionOutcome::Returned(_)));
    assert!(matches!(outcomes[1].outcome, ExecutionOutcome::Trapped(_)));
    assert_eq!(node.status(good)?.metrics.trapped, 0);
    assert_eq!(node.status(bad)?.metrics.trapped, 1);
    assert!(matches!(
        node.invoke(good_generation, Vec::new())?.outcome,
        ExecutionOutcome::Returned(_)
    ));
    Ok(())
}

#[test]
fn restart_changes_generation_and_rejects_stale_handles() -> Result<(), Box<dyn Error>> {
    let node = node(1)?;
    let app = node.install(
        manifest(ApplicationKind::Command, 1, 4),
        package(3)?,
        chunk(false)?,
    )?;
    let first = node.start(app)?;
    let second = node.restart(first)?;
    assert_ne!(first, second);
    assert_eq!(second.generation(), first.generation() + 1);
    assert!(matches!(
        node.invoke(first, Vec::new()),
        Err(RuntimeError::StaleGeneration { .. })
    ));
    assert!(matches!(
        node.invoke(second, Vec::new())?.outcome,
        ExecutionOutcome::Returned(_)
    ));
    Ok(())
}

#[test]
fn ticket_admission_has_no_starvation_and_enforces_total_quota() -> Result<(), Box<dyn Error>> {
    let node = node(1)?;
    let app = node.install(
        manifest(ApplicationKind::Service, 1, 8),
        package(4)?,
        chunk(false)?,
    )?;
    let generation = node.start(app)?;
    let requests = (0..8)
        .map(|index| InvocationRequest {
            generation,
            arguments: vec![index.to_string()],
        })
        .collect();
    let results = node.invoke_concurrent(requests)?;
    assert_eq!(results.len(), 8);
    assert!(results.into_iter().all(|result| result.is_ok()));
    let status = node.status(app)?;
    assert_eq!(status.metrics.admitted, 8);
    assert_eq!(status.metrics.completed, 8);
    assert_eq!(status.metrics.peak_concurrent, 1);
    assert_eq!(
        node.invoke(generation, Vec::new()),
        Err(RuntimeError::QuotaExceeded(QuotaKind::TotalInvocations))
    );
    Ok(())
}

#[test]
fn cache_never_evicts_a_live_application_lease() -> Result<(), Box<dyn Error>> {
    let node = node(1)?;
    let first_package = package(5)?;
    let second_package = package(6)?;
    let first_chunk = chunk(false)?;
    let app = node.install(
        manifest(ApplicationKind::Command, 1, 1),
        first_package,
        Arc::clone(&first_chunk),
    )?;
    drop(first_chunk);
    let second_chunk = chunk(false)?;
    assert_eq!(
        node.install(
            manifest(ApplicationKind::Command, 1, 1),
            second_package,
            Arc::clone(&second_chunk),
        ),
        Err(RuntimeError::PackageCacheFull)
    );
    node.remove(app)?;
    let _second = node.install(
        manifest(ApplicationKind::Command, 1, 1),
        second_package,
        second_chunk,
    )?;
    assert!(!node.cache_contains(first_package)?);
    assert!(node.cache_contains(second_package)?);
    assert_eq!(node.cache_len()?, 1);
    Ok(())
}

#[test]
fn capability_bearing_manifests_are_rejected() -> Result<(), Box<dyn Error>> {
    let node = node(1)?;
    let mut unsafe_manifest = manifest(ApplicationKind::Command, 1, 1);
    unsafe_manifest
        .capabilities
        .push(lkjscript_core::CapabilityKind::Arguments);
    assert_eq!(
        node.install(unsafe_manifest, package(7)?, chunk(false)?),
        Err(RuntimeError::UnsafeCapabilities)
    );
    Ok(())
}
