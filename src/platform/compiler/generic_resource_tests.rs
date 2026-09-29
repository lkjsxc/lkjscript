use super::*;
use crate::platform::kernel::{DeclarationPayload, KernelSnapshot};

#[test]
fn resource_suffix_artifact_checks_every_parameter_after_rehashing() {
    let snapshot = crate::platform::execution::normalized::tests::iteration_resource_tests::resource_suffix_tests::snapshot(true);
    check_resource_artifact(snapshot);
}

#[test]
fn type_generic_resource_artifact_preserves_exact_signature_and_authority() {
    let snapshot = crate::platform::execution::normalized::tests::iteration_resource_tests::type_generic_borrowed_snapshot(true);
    check_resource_artifact(snapshot);
}

#[test]
fn recursive_generic_resource_artifact_keeps_exact_metadata_after_rehashing() {
    for tail in [false, true] {
        let snapshot = crate::platform::execution::normalized::tests::iteration_resource_tests::recursive_resource_tests::snapshot("mutual", tail);
        check_resource_artifact(snapshot);
    }
}

#[test]
fn public_generic_resource_artifact_preserves_exact_parameter_contracts() {
    let mut snapshot = crate::platform::execution::normalized::tests::iteration_resource_tests::type_generic_borrowed_snapshot(true);
    let functions = snapshot
        .owners
        .iter()
        .filter_map(|(key, owner)| {
            let OwnerRecord::Declaration(record) = owner else {
                return None;
            };
            let DeclarationPayload::Function(function) = &record.payload else {
                return None;
            };
            function.parameters.iter().any(|parameter| matches!(
            snapshot.owners.get(&OwnerKey::Parameter(*parameter)),
            Some(OwnerRecord::Parameter(parameter)) if parameter.resource_requirement.is_some()
        )).then_some(*key)
        })
        .collect::<Vec<_>>();
    for function in functions {
        let Some(OwnerRecord::Declaration(record)) = snapshot.owners.get_mut(&function) else {
            unreachable!();
        };
        record.visibility = crate::platform::kernel::DeclarationVisibility::Public;
    }
    check_resource_artifact(snapshot);
}

fn check_resource_artifact(snapshot: KernelSnapshot) {
    let temporary = tempfile::tempdir().unwrap();
    let created =
        GraphRepository::create(&temporary.path().join("generic-resource"), &snapshot, None)
            .unwrap();
    let compilation = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    let linked = link_artifact(&created.repository, compilation.manifest_digest, &[]).unwrap();
    let loaded = load_artifact(&linked.artifact.bytes).unwrap();
    assert_resource_function_metadata(&loaded);
    reject_parameter_rebinding(&loaded);
    let functions = loaded
        .manifest
        .packages
        .iter()
        .enumerate()
        .flat_map(|(package, record)| {
            record
                .runtime_owners
                .iter()
                .enumerate()
                .filter_map(move |(index, binding)| {
                    (binding.kind == OwnerKind::TaskFunction).then_some((package, index))
                })
        })
        .collect::<Vec<_>>();
    assert_eq!(functions.len(), 2);
    for (package, index) in functions {
        for fault in ["erased", "foreign"] {
            let mut manifest = loaded.manifest.clone();
            let mut objects = loaded.objects.clone();
            let binding = &mut manifest.packages[package].runtime_owners[index];
            let old = ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
            let original = objects.remove(&old).unwrap();
            let mut owner =
                decode_owner(&original, binding.owner, binding.kind, binding.object).unwrap();
            let OwnerRecord::Declaration(record) = &mut owner else {
                panic!("resource declaration")
            };
            let DeclarationPayload::Function(function) = &mut record.payload else {
                panic!("resource function")
            };
            assert_eq!(function.type_parameters.len(), 1);
            if fault == "erased" {
                function.type_parameters.clear();
            } else {
                let own = function.type_parameters[0];
                let foreign = loaded.manifest.packages[package]
                    .runtime_owners
                    .iter()
                    .find_map(|candidate| match candidate.owner {
                        OwnerKey::TypeParameter(parameter) if parameter != own => Some(parameter),
                        _ => None,
                    })
                    .unwrap();
                function.type_parameters = vec![foreign];
            }
            let (digest, bytes) = encode_owner(&owner).unwrap();
            binding.object = digest;
            assert!(
                objects
                    .insert(
                        ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
                        bytes
                    )
                    .is_none()
            );
            let (closure, count, bytes) = super::super::artifact::closure_facts(&objects).unwrap();
            manifest.closure = closure;
            manifest.object_count = count;
            manifest.object_bytes = bytes;
            // Rehash a neutral container without consulting the strict writer.
            let forged = super::nominal_session_tests::hostile_bundle(&manifest, &objects);
            let error = load_artifact(&forged).unwrap_err();
            assert_eq!(
                error.code, "artifact_runtime_owner_semantics",
                "{fault}: {error:?}"
            );
        }
    }
}

fn reject_parameter_rebinding(loaded: &super::super::artifact::LoadedArtifact) {
    let mut checked = 0;
    let mut parameters = 0;
    for (package, record) in loaded.manifest.packages.iter().enumerate() {
        for (index, original) in record.runtime_owners.iter().enumerate() {
            if !matches!(original.owner, OwnerKey::Parameter(_)) {
                continue;
            }
            let key = ObjectKey::from_digest(ObjectDomain::Owner, original.object.bytes());
            let owner = decode_owner(
                &loaded.objects[&key],
                original.owner,
                original.kind,
                original.object,
            )
            .unwrap();
            if !matches!(&owner, OwnerRecord::Parameter(parameter) if parameter.resource_requirement.is_some())
            {
                continue;
            }
            parameters += 1;
            for fault in ["missing-binding", "foreign-binding", "use-mode", "parent"] {
                let mut changed = owner.clone();
                let OwnerRecord::Parameter(parameter) = &mut changed else {
                    unreachable!();
                };
                match fault {
                    "missing-binding" => parameter.resource_requirement = None,
                    "foreign-binding" => {
                        parameter.resource_requirement.as_mut().unwrap().package =
                            crate::platform::kernel::PackageId::migrate(
                                b"hostile-resource-parameter",
                                0,
                            )
                    }
                    "use-mode" => {
                        parameter.use_mode = match parameter.use_mode {
                            crate::platform::kernel::ParameterUse::Borrow => {
                                crate::platform::kernel::ParameterUse::Consume
                            }
                            _ => crate::platform::kernel::ParameterUse::Borrow,
                        };
                    }
                    "parent" => {
                        parameter.parent = crate::platform::kernel::ParameterParent::Function(
                            crate::platform::semantic_id::DeclarationId::migrate(
                                b"hostile-resource-parameter",
                                1,
                            ),
                        )
                    }
                    _ => unreachable!(),
                }
                let mut manifest = loaded.manifest.clone();
                let mut objects = loaded.objects.clone();
                let (digest, bytes) = encode_owner(&changed).unwrap();
                manifest.packages[package].runtime_owners[index].object = digest;
                objects.remove(&key).unwrap();
                objects.insert(
                    ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
                    bytes,
                );
                let (closure, count, bytes) =
                    super::super::artifact::closure_facts(&objects).unwrap();
                manifest.closure = closure;
                manifest.object_count = count;
                manifest.object_bytes = bytes;
                let forged = super::nominal_session_tests::hostile_bundle(&manifest, &objects);
                let error = load_artifact(&forged).unwrap_err();
                assert_eq!(
                    error.code, "artifact_runtime_owner_semantics",
                    "{fault}: {error:?}"
                );
                checked += 1;
            }
        }
    }
    assert!(parameters >= 2);
    assert_eq!(checked, parameters * 4);
}
