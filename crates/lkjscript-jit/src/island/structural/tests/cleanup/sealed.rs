use super::*;
use crate::island::structural::{core_type, NativeOwnerRecord};
use lkjscript_core::SemanticValue;

#[test]
fn sealed_destination_copy_and_disposal_use_exact_route() -> Result<(), Box<dyn std::error::Error>>
{
    let storage = StructuralStorageRoute::Sealed;
    let product = ty(36, StructuralKind::Product);
    let aggregate = StructuralAggregateDescriptor::new(
        303,
        product,
        StructuralAggregateKind::Product,
        Vec::new(),
    );
    let mut runtime = JitStructuralRuntime::new(&ExecutionPolicy::unrestricted())?;
    let destination = service(runtime.create_destination(&aggregate, storage))?;
    let owner = service(runtime.finish_destination(destination, &aggregate, storage))?;
    let owners_before = runtime.owners.clone();
    let metrics_before = runtime.runtime.metrics();
    assert_eq!(
        runtime.require_owner(owner, Some(StructuralStorageRoute::Unique)),
        Err(NativeServiceError::Trap)
    );
    assert_eq!(runtime.owners, owners_before);
    assert_eq!(runtime.runtime.metrics(), metrics_before);
    let copy = service(runtime.copy_owner(owner))?;
    service(runtime.drop_owner(owner))?;
    service(runtime.drop_owner(copy))?;
    let (stats, _) = runtime.finish();
    assert_eq!(stats.sealed_publications, 1);
    assert_eq!(stats.sealed_acquisitions, 1);
    assert_eq!(stats.sealed_releases, 2);
    assert!(stats.sealed_release_work > 0);
    assert!(stats.sealed_nodes_reclaimed > 0);
    assert_eq!(stats.live_sealed_domains, 0);
    assert_eq!(stats.live_sealed_owners, 0);
    assert_eq!(stats.live_roots, 0);
    assert_eq!(stats.teardown_failures, 0);
    Ok(())
}

#[test]
fn sealed_publish_replaces_unique_source_key() -> Result<(), Box<dyn std::error::Error>> {
    let value_type = ty(37, StructuralKind::String);
    let mut runtime = JitStructuralRuntime::new(&ExecutionPolicy::unrestricted())?;
    let unique = service(runtime.publish_static(
        b"sealed",
        value_type,
        StructuralPayloadKind::String,
        StructuralStorageRoute::Unique,
    ))?;
    let sealed = service(runtime.publish_owner(unique, StructuralStorageRoute::Sealed))?;
    assert_eq!(runtime.drop_owner(unique), Err(NativeServiceError::Trap));
    assert!(runtime.owners.contains_key(&sealed.opaque_word()));
    service(runtime.drop_owner(sealed))?;
    let (stats, _) = runtime.finish();
    assert_eq!(stats.sealed_publications, 1);
    assert_eq!(stats.sealed_releases, 1);
    assert!(stats.sealed_nodes_reclaimed > 0);
    assert_eq!(stats.live_sealed_domains, 0);
    assert_eq!(stats.live_sealed_owners, 0);
    assert_eq!(stats.live_roots, 0);
    Ok(())
}

#[test]
fn registry_failures_dispose_new_runtime_owners() -> Result<(), Box<dyn std::error::Error>> {
    let value_type = ty(38, StructuralKind::String);
    let expected = service(core_type(value_type))?;
    let mut runtime = JitStructuralRuntime::new(&ExecutionPolicy::unrestricted())?;
    let key = runtime
        .runtime
        .publish_owned(SemanticValue::new(
            expected,
            SemanticPayload::String(b"collision".to_vec()),
        ))
        .map_err(|failure| std::io::Error::other(format!("publish: {:?}", failure.error)))?;
    runtime.owners.insert(
        key.get(),
        NativeOwnerRecord {
            value_type,
            storage: StructuralStorageRoute::Unique,
        },
    );
    assert_eq!(
        runtime.register_runtime_owner(key, expected, value_type, StructuralStorageRoute::Unique,),
        Err(NativeServiceError::HostFailure),
    );
    assert!(runtime.runtime.verify_empty().is_ok());

    let old = runtime
        .runtime
        .publish_owned(SemanticValue::new(
            expected,
            SemanticPayload::String(b"move".to_vec()),
        ))
        .map_err(|failure| std::io::Error::other(format!("publish: {:?}", failure.error)))?;
    runtime.owners.insert(
        old.get(),
        NativeOwnerRecord {
            value_type,
            storage: StructuralStorageRoute::Unique,
        },
    );
    let new = runtime.runtime.move_owned(old, expected)?;
    runtime.owners.insert(
        new.get(),
        NativeOwnerRecord {
            value_type,
            storage: StructuralStorageRoute::Unique,
        },
    );
    assert_eq!(
        runtime.replace_runtime_owner(
            old,
            new,
            expected,
            value_type,
            StructuralStorageRoute::Unique,
        ),
        Err(NativeServiceError::HostFailure),
    );
    assert!(runtime.runtime.verify_empty().is_ok());
    let (stats, _) = runtime.finish();
    assert_eq!(stats.teardown_failures, 0);
    Ok(())
}

fn service<T>(result: Result<T, NativeServiceError>) -> Result<T, std::io::Error> {
    result.map_err(|error| std::io::Error::other(format!("native service: {error:?}")))
}
