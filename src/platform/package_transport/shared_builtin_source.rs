//! Reuse only the process-minted complete builtin proof after reading its exact current bytes.
use super::*;

pub(super) fn recheck<S: ImmutableObjectStore + ?Sized>(
    store: &CollectingStore<'_, S>,
    binding: PackageTransportBinding,
    revision: &PackageRevision,
    transport: &PackageTransport,
    work: &mut StoreWork,
    control: &ExecutionControl,
) -> Result<Option<Arc<ImmutablePackage>>, Diagnostic> {
    control.check().map_err(execution_diagnostic)?;
    let Some(standard) =
        crate::platform::builtin_standard::BuiltinStandard::initialized_for(binding)
    else {
        return Ok(None);
    };
    let admitted = standard.admitted_source();
    let package = admitted
        .packages
        .get(&binding.package_revision)
        .ok_or_else(|| {
            corrupt(
                "package_builtin_source",
                "the retained builtin proof lost its exact package",
            )
        })?;
    if package.binding != binding
        || &package.revision != revision
        || &package.transport != transport
    {
        return Err(corrupt(
            "package_builtin_source",
            "current builtin source differs from its complete retained proof",
        ));
    }
    // A prior proof never supplies absent physical input. Read every complete canonical
    // object through the caller's limits, cancellation and integrity boundary. This also
    // retains the exact source inventory required by the enclosing container's completeness
    // check. Readiness and cross-package composition remain with the ordinary callers.
    for (key, expected) in &admitted.container.objects {
        control.check().map_err(execution_diagnostic)?;
        let actual = required(store, key.domain, key.digest.bytes(), work)?;
        if &actual != expected {
            return Err(corrupt(
                "package_builtin_source",
                "current builtin source bytes differ from their complete retained proof",
            ));
        }
    }
    store.charge_visits(1)?;
    control.check().map_err(execution_diagnostic)?;
    Ok(Some(Arc::clone(package)))
}
