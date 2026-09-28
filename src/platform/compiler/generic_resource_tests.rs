use super::*;
use crate::platform::kernel::DeclarationPayload;

#[test]
fn type_generic_resource_artifact_preserves_exact_signature_and_authority() {
    let snapshot = crate::platform::execution::normalized::tests::iteration_resource_tests::type_generic_borrowed_snapshot(true);
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
