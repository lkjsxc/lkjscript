//! Complete serialized associated-owner controls built from published source.
use super::*;
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::execution::normalized::tests::associated_serial_artifact_tests::fixture;
use crate::platform::kernel::{DeclarationPayload, EncodedOwnerKey, KernelSnapshot};

/// Re-encode the changed private root owner in its exact runtime inventory,
/// every root unit's source summaries/key, and all enclosing package/map identities.
/// This constructs input bytes only: success still requires a fresh strict load.
fn encoded_source(
    original: &LoadedArtifact,
    source: &KernelSnapshot,
    replacement: OwnerKey,
) -> Vec<u8> {
    assert_eq!(original.manifest.root_package, source.root.package_id);
    let mut loaded = original.clone();
    let index = loaded
        .manifest
        .packages
        .iter()
        .position(|package| package.package == source.root.package_id)
        .unwrap();
    let canonical = &source.owners[&replacement];
    let (digest, bytes) = encode_owner(canonical).unwrap();
    let runtime = loaded.manifest.packages[index]
        .runtime_owners
        .iter_mut()
        .find(|binding| binding.owner == replacement)
        .expect("fixture replacement is retained as real runtime metadata");
    let old = runtime.object;
    assert_eq!(runtime.kind, canonical.kind());
    runtime.object = digest;
    loaded
        .objects
        .remove(&ObjectKey::from_digest(ObjectDomain::Owner, old.bytes()))
        .unwrap();
    loaded.objects.insert(
        ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
        bytes,
    );

    // The private unused implementation and actual Parameter are runtime-only.
    // The reference inventory contains executable declarations/expressions/bindings;
    // do not invent a second Parameter binding that the real linker never emits.
    let root = loaded.manifest.packages[index].reference_owners;
    let references = artifact_map_entries(original, root);
    let encoded = EncodedOwnerKey::new(replacement).bytes();
    assert!(
        references
            .iter()
            .all(|(key, _)| key.as_slice() != encoded.as_slice())
    );
    assert!(
        matches!(canonical, OwnerRecord::Parameter(_))
            || matches!(canonical,
        OwnerRecord::Declaration(declaration)
            if matches!(declaration.payload, DeclarationPayload::OwnedImplementation(_)))
    );

    // Neither private change alters the public interface. Rebind the complete
    // semantic state, revision and compilation envelope to the changed source.
    let interface = loaded.manifest.packages[index].interface;
    owned_closure_tests::rebind(&mut loaded, source);
    assert_eq!(loaded.manifest.packages[index].interface, interface);
    let old_compilation = loaded.manifest.packages[index].compilation;
    let mut compilation = CompilationManifest::decode(
        &loaded.objects[&old_compilation.object_key()],
        old_compilation,
    )
    .unwrap();
    let mut units = artifact_map_entries(&loaded, compilation.units);
    let facts = crate::platform::witness::rebuild_canonical_facts(source).unwrap();
    let mut replaced_implementation = false;
    for (owner, value) in &mut units {
        let owner = EncodedOwnerKey::decode(owner).unwrap();
        let mut binding = CompilationBinding::decode(value, owner).unwrap();
        let old_unit = binding.object.object_key();
        let mut unit = CompilationUnit::decode(&loaded.objects[&old_unit], old_unit).unwrap();
        assert_eq!(unit.source.package, source.root.package_id);
        if owner == replacement {
            let OwnerRecord::Declaration(declaration) = canonical else {
                panic!("parameters do not have standalone compiler units")
            };
            let DeclarationPayload::OwnedImplementation(implementation) = &declaration.payload
            else {
                unreachable!()
            };
            unit.payload = CompilationPayload::OwnedImplementation(implementation.clone());
            for associated in &implementation.associated_types {
                if !unit.tables.types.contains(&associated.ty()) {
                    unit.tables.types.push(associated.ty());
                }
                let object = &source.types[&associated.ty()];
                let (ty, bytes) = encode_type_object(object).unwrap();
                assert_eq!(ty, associated.ty());
                loaded.objects.insert(
                    ObjectKey::from_digest(ObjectDomain::Type, ty.bytes()),
                    bytes,
                );
            }
            replaced_implementation = true;
        }
        let summary = &facts.summaries[&owner];
        unit.source = CompilationSource {
            package: source.root.package_id,
            owner,
            kind: summary.kind,
            semantic_interface: summary.semantic_interface,
            implementation: summary.implementation,
            type_digest: summary.type_digest,
            effect: summary.effect,
            capability: summary.capability,
            test: summary.test,
            validation_dependencies: summary.validation_dependencies,
        };
        unit.key = CompilationUnitKey::derive(&unit.source, unit.optimization).unwrap();
        // Structural validation does not confer semantic artifact admission.
        let (key, bytes) = unit.encode().unwrap();
        assert_eq!(CompilationUnit::decode(&bytes, key).unwrap(), unit);
        loaded.objects.remove(&old_unit).unwrap();
        loaded.objects.insert(key, bytes);
        binding.object = CompilerUnitObjectDigest::from_bytes(key.digest.bytes());
        binding.key = unit.key;
        binding.kind = unit.source.kind;
        *value = binding.encode(owner).unwrap();
    }
    assert_eq!(
        replaced_implementation,
        matches!(canonical, OwnerRecord::Declaration(_))
    );
    compilation.units = replace_artifact_map(&mut loaded.objects, units);
    let (digest, bytes) = compilation.encode().unwrap();
    loaded
        .objects
        .remove(&old_compilation.object_key())
        .unwrap();
    loaded.objects.insert(digest.object_key(), bytes);
    loaded.manifest.packages[index].compilation = digest;
    nominal_owned_record_admission_tests::retain_live_map_pages(&mut loaded);
    let (closure, count, bytes) = super::super::artifact::closure_facts(&loaded.objects).unwrap();
    loaded.manifest.closure = closure;
    loaded.manifest.object_count = count;
    loaded.manifest.object_bytes = bytes;
    nominal_session_tests::hostile_bundle(&loaded.manifest, &loaded.objects)
}

fn scheme(source: &KernelSnapshot) -> OwnerKey {
    OwnerKey::Declaration(declaration_named(source, "HiddenChunked"))
}

fn block_parameter(source: &KernelSnapshot) -> OwnerKey {
    let OwnerRecord::Declaration(declaration) =
        &source.owners[&OwnerKey::Declaration(declaration_named(source, "block-size"))]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &declaration.payload else {
        unreachable!()
    };
    assert_eq!(function.parameters.len(), 1);
    OwnerKey::Parameter(function.parameters[0])
}

#[test]
fn associated_serial_neutral_complete_reencoding_strictly_loads_before_preparation() {
    let fixture = fixture();
    for owner in [scheme(&fixture.source), block_parameter(&fixture.source)] {
        let bytes = encoded_source(&fixture.loaded, &fixture.source, owner);
        let loaded = load_artifact(&bytes).expect("neutral complete serialization admits");
        loaded.require_current_admission(|| Ok(())).unwrap();
        assert_eq!(loaded.manifest.packages, fixture.loaded.manifest.packages);
        crate::platform::execution::normalized::NormalizedProgram::prepare(loaded)
            .expect("fresh loader admission authorizes neutral preparation");
    }
}

#[test]
fn associated_serial_unused_owned_binding_rejects_after_complete_reencoding() {
    let fixture = fixture();
    let mut source = fixture.source.clone();
    let owner = scheme(&source);
    let scalar = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
        .unwrap()
        .0;
    assert!(source.types.contains_key(&scalar));
    let OwnerRecord::Declaration(declaration) = source.owners.get_mut(&owner).unwrap() else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(implementation) = &mut declaration.payload else {
        unreachable!()
    };
    let spare = implementation
        .associated_types
        .iter_mut()
        .find(|binding| binding.member.to_string() == "assoc_type_c7000000000000000000000000000002")
        .unwrap();
    assert_ne!(spare.ty(), scalar);
    spare.value = crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty: scalar };
    let bytes = encoded_source(&fixture.loaded, &source, owner);
    let error =
        load_artifact(&bytes).expect_err("unused associated member still requires its Owned bound");
    assert_eq!(error.class, DiagnosticClass::Corrupt, "{error:?}");
    // Complete associated-unit inventory admission rejects the unused Owned
    // binding before the later body/affine validation phase.
    assert_eq!(error.code, "artifact_associated_type", "{error:?}");
    assert!(error.message.contains("kernel_owned_contract"), "{error:?}");
    fixture.loaded.require_current_admission(|| Ok(())).unwrap();
}

#[test]
fn associated_serial_parameter_owner_requires_current_contract() {
    let fixture = fixture();
    let mut source = fixture.source.clone();
    let owner = block_parameter(&source);
    let record = source.owners.get_mut(&owner).unwrap();
    record.set_encoding_for_edit(crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION ^ 1);
    assert_eq!(
        crate::platform::kernel::encode_owner(record)
            .unwrap_err()
            .code,
        "kernel_owner_header"
    );
    fixture.loaded.require_current_admission(|| Ok(())).unwrap();
}
