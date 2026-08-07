use super::*;

#[test]
fn cache_never_evicts_a_live_application_lease() -> Result<(), Box<dyn Error>> {
    let system = system(1, 1)?;
    let first_package = package(5)?;
    let second_package = package(6)?;
    let app = system.install(
        manifest(ApplicationKind::Command, 1, 1),
        first_package,
        chunk(false)?,
        lkjscript_host::HostEnvironment::default(),
    )?;
    assert_eq!(
        system.install(
            manifest(ApplicationKind::Command, 1, 1),
            second_package,
            chunk(false)?,
            lkjscript_host::HostEnvironment::default(),
        ),
        Err(RuntimeError::PackageCacheFull)
    );
    system.remove(app)?;
    let _second = system.install(
        manifest(ApplicationKind::Command, 1, 1),
        second_package,
        chunk(false)?,
        lkjscript_host::HostEnvironment::default(),
    )?;
    assert!(!system.cache_contains(first_package)?);
    assert!(system.cache_contains(second_package)?);
    assert_eq!(system.cache_len()?, 1);
    Ok(())
}

#[test]
fn manifest_and_installed_execution_cell_class_must_match_before_effects(
) -> Result<(), Box<dyn Error>> {
    let system = system(2, 2)?;
    let mut isolated = manifest(ApplicationKind::Command, 1, 1);
    isolated.cell = ExecutionCellClass::IsolatedProcess {
        entry: lkjscript_host::ApplicationPath::parse("main.lkjscript")?,
    };
    isolated.quota.execution = lkjscript_core::ExecutionPolicy::limited(
        lkjscript_core::LimitedExecutionPolicy::conservative(),
    );
    assert!(matches!(
        system.install(
            isolated,
            package(24)?,
            chunk(false)?,
            lkjscript_host::HostEnvironment::default(),
        ),
        Err(RuntimeError::ExecutionCellClassMismatch)
    ));
    assert!(matches!(
        system.install_isolated(
            manifest(ApplicationKind::Command, 1, 1),
            package(25)?,
            std::path::Path::new("."),
            std::path::Path::new("worker"),
            lkjscript_host::HostEnvironment::default(),
        ),
        Err(RuntimeError::ExecutionCellClassMismatch)
    ));
    assert!(system.list()?.is_empty());
    assert_eq!(system.cache_len()?, 0);
    Ok(())
}

#[test]
fn arguments_and_stdio_providers_execute_in_private_vm_inputs() -> Result<(), Box<dyn Error>> {
    let system = system(1, 2)?;
    let arguments_chunk = capability_chunk(lkjscript_core::CapabilityKind::Arguments, false)?;
    let mut arguments_manifest = manifest(ApplicationKind::Command, 1, 2);
    arguments_manifest.capabilities = vec![lkjscript_core::CapabilityKind::Arguments];
    let arguments_app = system.install(
        arguments_manifest,
        package(7)?,
        arguments_chunk,
        lkjscript_host::HostEnvironment::default(),
    )?;
    let arguments_incarnation = system.start(arguments_app)?;
    assert!(matches!(
        system
            .invoke(arguments_incarnation, vec!["one".into(), "two".into()])?
            .outcome,
        ExecutionOutcome::Returned(_)
    ));

    let stdio = lkjscript_host::BufferedStdio::default();
    let host = lkjscript_host::HostEnvironment {
        stdio: Some(Arc::new(stdio.clone())),
        ..lkjscript_host::HostEnvironment::default()
    };
    let mut stdio_manifest = manifest(ApplicationKind::Command, 1, 1);
    stdio_manifest.capabilities = vec![lkjscript_core::CapabilityKind::Stdio];
    let stdio_app = system.install(
        stdio_manifest,
        package(8)?,
        capability_chunk(lkjscript_core::CapabilityKind::Stdio, true)?,
        host,
    )?;
    let stdio_incarnation = system.start(stdio_app)?;
    assert!(matches!(
        system.invoke(stdio_incarnation, Vec::new())?.outcome,
        ExecutionOutcome::Returned(_)
    ));
    assert_eq!(stdio.output()?, b"provider-output");
    assert_eq!(stdio.flushes()?, 1);
    Ok(())
}

#[test]
fn unsupported_capability_fails_before_installation_effects() -> Result<(), Box<dyn Error>> {
    let system = system(1, 1)?;
    let mut manifest = manifest(ApplicationKind::Command, 1, 1);
    manifest.capabilities = vec![lkjscript_core::CapabilityKind::FileSystem];
    assert_eq!(
        system.install(
            manifest,
            package(10)?,
            capability_chunk(lkjscript_core::CapabilityKind::FileSystem, false)?,
            lkjscript_host::HostEnvironment::default(),
        ),
        Err(RuntimeError::UnsupportedCapability(
            lkjscript_core::CapabilityKind::FileSystem
        ))
    );
    assert!(system.list()?.is_empty());
    Ok(())
}

fn capability_chunk(
    capability: lkjscript_core::CapabilityKind,
    output: bool,
) -> Result<Arc<lkjscript_core::ValidatedChunk>, Box<dyn Error>> {
    let mut chunk = Chunk::new();
    chunk.required_capabilities = vec![capability];
    chunk.main.arity = 1;
    chunk.main.locals = 1;
    if output {
        chunk.main.emit_op_u64(Op::LoadLocal, 0);
        chunk
            .constants
            .push(Constant::Str("provider-output".into()));
        chunk.main.emit_op_u64(Op::LoadConst, 0);
        chunk.main.emit(Op::Print);
    } else if capability == lkjscript_core::CapabilityKind::Arguments {
        chunk.main.emit_op_u64(Op::LoadLocal, 0);
        chunk.main.emit(Op::Argc);
    } else {
        chunk.main.emit(Op::Unit);
    }
    chunk.main.emit(Op::Return);
    let validated = validate_chunk(chunk, ValidationPolicy::Unrestricted)?;
    let prepared = lkjscript_contracts::PreparedProgramIdentity::new(
        [(capability as u8).saturating_add(1); 32],
    )?;
    Ok(Arc::new(validated.bind_prepared_identity(prepared)?))
}
