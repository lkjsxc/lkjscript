//! Rehashed nominal custody instructions must retain exact representation authority.
use super::*;
use crate::platform::kernel::{
    DeclarationPayload, DeclarationReference, EncodedOwnerKey, KernelSnapshot, StructuralTypeField,
    decode_owner_binding, encode_owner_binding,
};

const SOURCE: &str = r#"
declarations.begin
(units (module create nominal-artifact
  (owned-record create Packet (visibility private)
    (field create payload (type ByteBuffer))
    (field create sibling (type ByteBuffer))
    (field create tag (type I64)))
  (owned-record create Other (visibility private)
    (field create payload (type ByteBuffer))
    (field create sibling (type ByteBuffer))
    (field create tag (type I64)))
  (owned-record create Empty (visibility private))
  (owned-record create Data (visibility private)
    (type-parameter create T (constraint transferable))
    (field create value (type T)))
  (owned-record create Generic (visibility private)
    (type-parameter create Storage (constraint owned transferable shareable))
    (type-parameter create Tag (constraint transferable))
    (field create storage (type Storage))
    (field create tag (type Tag)))
  (owned-record create Carrier (visibility public)
    (type-parameter create Tag (constraint transferable))
    (field create storage (type (owned-sequence (option Tag)))))
  (function create hold (visibility private) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create value (type (Carrier T)) (use consume))
    (returns (Carrier T)) (body (local value)))
  (external create length (visibility private) (implementation core.buffer.length)
    (parameter create value (type ByteBuffer) (use borrow)) (returns I64))
  (function create empty-producer (visibility private) (effect pure) (returns Empty)
    (body (pack-owned (type Empty))))
  (function create data-producer (visibility private) (effect pure)
    (parameter create value (type I64)) (returns (Data I64))
    (body (pack-owned (type (Data I64)) (field value (local value)))))
  (function create generic-data-producer (visibility private) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create value (type T)) (returns (Data T))
    (body (pack-owned (type (Data T)) (field value (local value)))))
  (function create pack (visibility private) (effect pure)
    (parameter create a (type ByteBuffer) (use consume))
    (parameter create b (type ByteBuffer) (use consume))
    (returns Packet)
    (body (pack-owned (type Packet)
      (field tag (i64 7)) (field sibling (local b)) (field payload (local a)))))
  (function create unpack (visibility private) (effect pure)
    (parameter create packet (type Packet) (use consume)) (returns Unit)
    (body (unpack-owned (type Packet) (local packet)
      (field tag (binding tag (type I64)))
      (field sibling (binding b (type ByteBuffer)))
      (field payload (binding a (type ByteBuffer))) (in (unit)))))
  (function create read (visibility private) (effect pure)
    (parameter create packet (type Packet) (use borrow))
    (parameter create other (type Packet) (use borrow)) (returns I64)
    (body (if (bool true) (i64 7)
      (borrow-owned-field (type Packet) (local packet)
        (field payload (binding view (type ByteBuffer)))
        (in (call length (local view)))))))
  (function create empty (visibility private) (effect pure) (returns Unit)
    (body (let (binding value (type Empty) (pack-owned (type Empty)))
      (in (unpack-owned (type Empty) (local value) (in (unit))))))))
  (module create foreign
    (function create marker (visibility private) (effect pure) (returns Unit) (body (unit)))))
declarations.end
"#;

fn fixture() -> (KernelSnapshot, LoadedArtifact) {
    fixture_from_source(SOURCE)
}

fn fixture_from_source(native: &str) -> (KernelSnapshot, LoadedArtifact) {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(native)
            .expect("nominal fixture authors through the native product surface");
    let directory = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&directory.path().join("nominal"), &source, None)
        .unwrap()
        .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let linked = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    let loaded = load_artifact(&linked.artifact.bytes).expect("nominal control loads");
    (source, loaded)
}

#[test]
fn nominal_unused_borrowed_result_retains_exact_reference_binding() {
    let (source, loaded) = fixture_from_source(
        r#"
declarations.begin
(units (module create unused-borrowed-result
  (owned-record create Empty (visibility private))
  (function create view (visibility private) (effect pure)
    (parameter create source (type Empty) (use borrow))
    (returns Empty (borrow-from source)) (body (local source)))
  (function create ignore-view (visibility private) (effect pure)
    (parameter create source (type Empty) (use borrow)) (returns I64)
    (body (borrow-call (call view (local source))
      (binding ignored (type Empty)) (in (i64 7)))))))
declarations.end
"#,
    );
    let binding = source
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Expression(expression) => match expression.operation {
                ExpressionOperation::BorrowCall { binding, .. } => Some(binding),
                _ => None,
            },
            _ => None,
        })
        .expect("unused nominal borrowed-result binding");
    assert!(!source.owners.values().any(|record| {
        matches!(record,
            OwnerRecord::Expression(expression)
            if matches!(expression.operation,
                ExpressionOperation::Local {
                    value: LocalValueReference::LexicalBinding(local),
                } if local == binding
            )
        )
    }));
    assert_eq!(
        loaded
            .reference_owner(
                source.root.package_id,
                OwnerKey::Binding(binding),
                &mut MapWork::default(),
                &mut StoreWork::default(),
            )
            .unwrap()
            .as_ref(),
        source.owners.get(&OwnerKey::Binding(binding)),
        "linking must retain the exact unused binding without a Local reference edge",
    );
}

fn unit_for(
    source: &KernelSnapshot,
    loaded: &LoadedArtifact,
    name: &str,
) -> (ObjectKey, CompilationUnit) {
    let owner = OwnerKey::Declaration(declaration_named(source, name));
    loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .find(|(_, unit)| unit.source.owner == owner)
        .unwrap()
}

fn code(unit: &CompilationUnit) -> &CompiledCode {
    let CompilationPayload::Function { code, .. } = &unit.payload else {
        panic!("function fixture")
    };
    code
}

fn code_mut(unit: &mut CompilationUnit) -> &mut CompiledCode {
    let CompilationPayload::Function { code, .. } = &mut unit.payload else {
        panic!("function fixture")
    };
    code
}

fn reject(
    loaded: &LoadedArtifact,
    key: ObjectKey,
    unit: &CompilationUnit,
    extra: Vec<(ObjectKey, Vec<u8>)>,
) {
    let bytes = effect_tests::replace_unit(loaded, key, unit, extra);
    let error =
        load_artifact(&bytes).expect_err("rehashed nominal instruction forgery must reject");
    assert_eq!(error.code, "artifact_compiled_control_meaning", "{error:?}");
}

#[test]
fn nominal_owned_record_rehashed_pack_unpack_and_borrow_keep_exact_slots_and_sources() {
    let (source, loaded) = fixture();
    for name in ["pack", "unpack", "read"] {
        let (key, original) = unit_for(&source, &loaded, name);
        load_artifact(&effect_tests::replace_unit(&loaded, key, &original, vec![])).unwrap();
        let mut changed = original.clone();
        let instruction = code_mut(&mut changed)
            .instructions
            .iter_mut()
            .find(|instruction| {
                matches!(
                    instruction,
                    CompiledInstruction::PackOwned { .. }
                        | CompiledInstruction::UnpackOwned { .. }
                        | CompiledInstruction::BorrowOwnedField { .. }
                )
            })
            .unwrap();
        match instruction {
            CompiledInstruction::PackOwned { fields, .. } => fields.swap(0, 1),
            CompiledInstruction::UnpackOwned { locals, .. } => locals.swap(0, 1),
            CompiledInstruction::BorrowOwnedField { source_local, .. } => *source_local = 1,
            _ => unreachable!(),
        }
        reject(&loaded, key, &changed, vec![]);
    }
    let (key, original) = unit_for(&source, &loaded, "read");
    let mut changed = original.clone();
    let payload_slot = code(&original)
        .instructions
        .iter()
        .find_map(|instruction| match instruction {
            CompiledInstruction::BorrowOwnedField { field, .. } => Some(*field),
            _ => None,
        })
        .unwrap();
    let (_, packet) = unit_for(&source, &loaded, "Packet");
    let CompilationPayload::OwnedRecord { fields, .. } = &packet.payload else {
        unreachable!()
    };
    let sibling = fields.iter().position(|layout| {
        let reference = packet.tables.fields[layout.field as usize];
        matches!(&source.owners[&OwnerKey::Field(reference.field)], OwnerRecord::Field(field) if field.name.as_str() == "sibling")
    }).unwrap() as u32;
    assert_ne!(payload_slot, sibling);
    for instruction in &mut code_mut(&mut changed).instructions {
        if let CompiledInstruction::BorrowOwnedField { field, .. } = instruction {
            *field = sibling;
        }
    }
    reject(&loaded, key, &changed, vec![]);
}

#[test]
fn nominal_owned_record_rehashed_type_cannot_select_same_shape_nominal_or_structural_layout() {
    let (source, loaded) = fixture();
    let (key, original) = unit_for(&source, &loaded, "pack");
    for form in [
        TypeForm::Named {
            declaration: DeclarationReference {
                package: source.root.package_id,
                declaration: declaration_named(&source, "Other"),
            },
        },
        TypeForm::OwnedProduct {
            fields: ["payload", "sibling", "tag"]
                .into_iter()
                .map(|name| {
                    let ty = source
                        .owners
                        .values()
                        .find_map(|record| match record {
                            OwnerRecord::Field(field)
                                if field.declaration == declaration_named(&source, "Packet")
                                    && field.name.as_str() == name =>
                            {
                                Some(field.ty)
                            }
                            _ => None,
                        })
                        .unwrap();
                    StructuralTypeField {
                        name: Name::new(name).unwrap(),
                        ty,
                    }
                })
                .collect(),
        },
    ] {
        let mut changed = original.clone();
        let (digest, bytes) = encode_type_object(&TypeObject::new(form).unwrap()).unwrap();
        let index = changed.tables.types.len() as u32;
        changed.tables.types.push(digest);
        for instruction in &mut code_mut(&mut changed).instructions {
            if let CompiledInstruction::PackOwned { product_type, .. } = instruction {
                *product_type = index;
            }
        }
        reject(
            &loaded,
            key,
            &changed,
            vec![(
                ObjectKey::from_digest(ObjectDomain::Type, digest.bytes()),
                bytes,
            )],
        );
    }
}

#[test]
fn nominal_owned_record_compilation_keeps_empty_custody_and_exact_generic_constraints() {
    let (source, loaded) = fixture();
    let (_, empty) = unit_for(&source, &loaded, "Empty");
    assert!(
        matches!(&empty.payload, CompilationPayload::OwnedRecord { fields, .. } if fields.is_empty())
    );
    let (_, empty_function) = unit_for(&source, &loaded, "empty");
    assert!(
        code(&empty_function)
            .instructions
            .iter()
            .any(|instruction| matches!(instruction,
        CompiledInstruction::PackOwned { fields, .. } if fields.is_empty()))
    );
    assert!(
        code(&empty_function)
            .instructions
            .iter()
            .any(|instruction| matches!(instruction,
        CompiledInstruction::UnpackOwned { locals, .. } if locals.is_empty()))
    );
    let (key, original) = unit_for(&source, &loaded, "Generic");
    let CompilationPayload::OwnedRecord {
        type_parameter_constraints,
        ..
    } = &original.payload
    else {
        unreachable!()
    };
    assert_eq!(
        type_parameter_constraints,
        &[
            crate::platform::kernel::TypeParameterConstraints::OwnedTransferableShareable,
            crate::platform::kernel::TypeParameterConstraints::Transferable,
        ]
    );
    let mut changed = original.clone();
    let CompilationPayload::OwnedRecord {
        type_parameter_constraints,
        ..
    } = &mut changed.payload
    else {
        unreachable!()
    };
    type_parameter_constraints[0] = crate::platform::kernel::TypeParameterConstraints::Owned;
    let bytes = effect_tests::replace_unit(&loaded, key, &changed, vec![]);
    let error = load_artifact(&bytes)
        .expect_err("generic record constraints cannot be weakened by rehashing");
    assert_eq!(error.code, "artifact_runtime_owner_semantics", "{error:?}");

    let (key, mut changed) = unit_for(&source, &loaded, "Packet");
    let CompilationPayload::OwnedRecord {
        type_parameters,
        type_parameter_constraints,
        fields,
    } = changed.payload
    else {
        unreachable!()
    };
    changed.payload = CompilationPayload::Record {
        type_parameters,
        type_parameter_constraints,
        fields,
    };
    changed.source.kind = OwnerKind::Record;
    changed.key = CompilationUnitKey::derive(&changed.source, changed.optimization).unwrap();
    let bytes = effect_tests::replace_unit(&loaded, key, &changed, vec![]);
    let error = load_artifact(&bytes).expect_err("same field layout cannot erase nominal custody");
    assert_eq!(error.code, "artifact_runtime_owner_binding", "{error:?}");

    let (_, hold) = unit_for(&source, &loaded, "hold");
    let CompilationPayload::Function { signature, .. } = &hold.payload else {
        unreachable!()
    };
    let parameter = signature.type_parameters[0];
    let parameter_type =
        encode_type_object(&TypeObject::new(TypeForm::TypeParameter { parameter }).unwrap())
            .unwrap()
            .0;
    let (option_type, option_bytes) = encode_type_object(
        &TypeObject::new(TypeForm::Option {
            item: parameter_type,
        })
        .unwrap(),
    )
    .unwrap();
    let (sequence_type, sequence_bytes) = encode_type_object(
        &TypeObject::new(TypeForm::OwnedSequence { item: option_type }).unwrap(),
    )
    .unwrap();
    assert!(
        hold.tables.types.contains(&sequence_type),
        "uninspected generic holder retains substituted field root"
    );
    assert_eq!(
        loaded.objects[&ObjectKey::from_digest(ObjectDomain::Type, option_type.bytes())],
        option_bytes
    );
    assert_eq!(
        loaded.objects[&ObjectKey::from_digest(ObjectDomain::Type, sequence_type.bytes())],
        sequence_bytes
    );
}

#[test]
fn nominal_owned_record_rehashed_untaken_representation_requires_exact_defining_module() {
    let (source, loaded) = fixture();
    let (key, unit) = unit_for(&source, &loaded, "read");
    let owner = unit.source.owner;
    let control = replace_shared_owner(&loaded, owner, source.owners[&owner].clone());
    load_artifact(&effect_tests::replace_unit(&control, key, &unit, vec![]))
        .expect("neutral complete owner rehash preserves access");
    let mut record = source.owners[&owner].clone();
    let OwnerRecord::Declaration(declaration) = &mut record else {
        unreachable!()
    };
    declaration.module = module_named(&source, "foreign");
    let altered = replace_shared_owner(&loaded, owner, record);
    let bytes = effect_tests::replace_unit(&altered, key, &unit, vec![]);
    let error = load_artifact(&bytes)
        .expect_err("untaken field access remains bound to the exact defining module");
    assert_eq!(error.code, "artifact_nominal_meaning", "{error:?}");
    assert!(error.message.contains("module"), "{error:?}");
}

#[test]
fn nominal_owned_record_unused_interface_fields_and_constraints_match_canonical_source() {
    use crate::platform::kernel::{PackageInterfaceRecord, TypeParameterConstraints};
    let (source, loaded) = fixture();
    let OwnerRecord::Declaration(carrier) =
        &source.owners[&OwnerKey::Declaration(declaration_named(&source, "Carrier"))]
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedRecord {
        type_parameters,
        fields,
    } = &carrier.payload
    else {
        unreachable!()
    };
    let parameter_owner = OwnerKey::TypeParameter(type_parameters[0]);
    let field_owner = OwnerKey::Field(fields[0]);
    for owner in [parameter_owner, field_owner] {
        let original = interface_owner(&loaded, owner);
        load_artifact(&replace_interface_owner(&loaded, owner, original.clone()))
            .expect("neutral interface rehash preserves full admission");
        let mut forged = original;
        match &mut forged.record {
            PackageInterfaceRecord::TypeParameter(parameter) => {
                assert_eq!(
                    parameter.constraints,
                    TypeParameterConstraints::Transferable
                );
                parameter.constraints = TypeParameterConstraints::CaptureSafeTransferable;
            }
            PackageInterfaceRecord::Field(field) => {
                field.ty = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
                    .unwrap()
                    .0;
            }
            _ => unreachable!(),
        }
        let error = load_artifact(&replace_interface_owner(&loaded, owner, forged))
            .expect_err("unused interface metadata remains bound to retained canonical source");
        assert_eq!(error.code, "artifact_interface_source", "{error:?}");
    }
}

#[test]
fn nominal_empty_and_data_only_factories_retain_exact_callable_runtime_metadata() {
    let (source, loaded) = fixture();
    let package = loaded
        .manifest
        .packages
        .iter()
        .find(|package| package.package == source.root.package_id)
        .unwrap();
    for name in ["empty-producer", "data-producer", "generic-data-producer"] {
        let owner = OwnerKey::Declaration(declaration_named(&source, name));
        let binding = package
            .runtime_owners
            .iter()
            .find(|binding| binding.owner == owner)
            .expect("intrinsic nominal custody requires callable metadata without Owned formals");
        assert_eq!(binding.kind, OwnerKind::PureFunction);
        let (_, unit) = unit_for(&source, &loaded, name);
        let CompilationPayload::Function { signature, .. } = &unit.payload else {
            unreachable!()
        };
        assert!(
            signature
                .type_parameter_constraints
                .iter()
                .all(|constraint| !constraint.has_owned())
        );
        assert!(
            signature
                .parameters
                .iter()
                .all(|parameter| parameter.use_mode
                    == crate::platform::kernel::ParameterUse::Unrestricted)
        );
    }
    crate::platform::execution::normalized::NormalizedProgram::prepare(loaded)
        .expect("strict preparation recognizes exact nominal factory signatures");
}

pub(super) fn interface_owner(
    loaded: &LoadedArtifact,
    owner: OwnerKey,
) -> crate::platform::package_interface::PackageInterfaceOwner {
    use crate::platform::package_interface::{
        PackageInterfaceOwner, decode_package_interface_binding,
    };
    let package = loaded
        .manifest
        .packages
        .iter()
        .find(|package| package.package == loaded.manifest.root_package)
        .unwrap();
    let encoded = EncodedOwnerKey::new(owner).bytes();
    let (_, value) = artifact_map_entries(loaded, package.interface_owners)
        .into_iter()
        .find(|(key, _)| key.as_slice() == encoded.as_slice())
        .unwrap();
    let digest = decode_package_interface_binding(&value).unwrap();
    PackageInterfaceOwner::decode(
        &loaded.objects[&ObjectKey::from_digest(ObjectDomain::PackageInterface, digest.bytes())],
        owner,
        digest,
    )
    .unwrap()
}

/// Repair every interface binding and envelope, retaining the immutable canonical source.
pub(super) fn replace_interface_owner(
    loaded: &LoadedArtifact,
    owner: OwnerKey,
    projected: crate::platform::package_interface::PackageInterfaceOwner,
) -> Vec<u8> {
    use crate::platform::package_interface::{
        decode_package_interface_binding, encode_package_interface_binding,
        package_interface_digest_for_graph,
    };
    use crate::platform::package_transport::PackageRevision;
    let mut altered = loaded.clone();
    let index = altered
        .manifest
        .packages
        .iter()
        .position(|package| package.package == loaded.manifest.root_package)
        .unwrap();
    let package = &mut altered.manifest.packages[index];
    let mut entries = artifact_map_entries(loaded, package.interface_owners);
    let encoded = EncodedOwnerKey::new(owner).bytes();
    let (_, binding) = entries
        .iter_mut()
        .find(|(key, _)| key.as_slice() == encoded.as_slice())
        .unwrap();
    let old = decode_package_interface_binding(binding).unwrap();
    let (digest, bytes) = projected.encode().unwrap();
    *binding = encode_package_interface_binding(digest);
    altered
        .objects
        .remove(&ObjectKey::from_digest(
            ObjectDomain::PackageInterface,
            old.bytes(),
        ))
        .unwrap();
    altered.objects.insert(
        ObjectKey::from_digest(ObjectDomain::PackageInterface, digest.bytes()),
        bytes,
    );
    package.interface_owners = replace_artifact_map(&mut altered.objects, entries);
    let old_revision = package.package_revision;
    let mut revision = PackageRevision::decode(
        &altered
            .objects
            .remove(&ObjectKey::from_digest(
                ObjectDomain::PackageRevision,
                old_revision.bytes(),
            ))
            .unwrap(),
        old_revision,
    )
    .unwrap();
    package.interface = package_interface_digest_for_graph(
        package.package,
        package.interface_owners.content_root(),
        revision.graph_contract_version,
    )
    .unwrap();
    revision.interface = package.interface;
    let (digest, bytes) = revision.encode().unwrap();
    package.package_revision = digest;
    altered.objects.insert(
        ObjectKey::from_digest(ObjectDomain::PackageRevision, digest.bytes()),
        bytes,
    );
    let mut compilation = CompilationManifest::decode(
        &altered
            .objects
            .remove(&package.compilation.object_key())
            .unwrap(),
        package.compilation,
    )
    .unwrap();
    compilation.package_revision = package.package_revision;
    compilation.package_interface = package.interface;
    let (digest, bytes) = compilation.encode().unwrap();
    package.compilation = digest;
    altered.objects.insert(digest.object_key(), bytes);
    retain_live_map_pages(&mut altered);
    let (closure, count, length) = super::super::artifact::closure_facts(&altered.objects).unwrap();
    altered.manifest.closure = closure;
    altered.manifest.object_count = count;
    altered.manifest.object_bytes = length;
    nominal_session_tests::hostile_bundle(&altered.manifest, &altered.objects)
}

/// Replace both inventories consistently, so this attack reaches semantic admission.
fn replace_shared_owner(
    loaded: &LoadedArtifact,
    owner: OwnerKey,
    record: OwnerRecord,
) -> LoadedArtifact {
    let mut altered = loaded.clone();
    let index = altered
        .manifest
        .packages
        .iter()
        .position(|package| package.package == loaded.manifest.root_package)
        .unwrap();
    let root = altered.manifest.packages[index].reference_owners;
    let mut entries = artifact_map_entries(loaded, root);
    let encoded = EncodedOwnerKey::new(owner).bytes();
    let (_, value) = entries
        .iter_mut()
        .find(|(key, _)| key.as_slice() == encoded.as_slice())
        .unwrap();
    let mut binding = decode_owner_binding(value, owner).unwrap();
    let old = binding.object;
    let (digest, bytes) = encode_owner(&record).unwrap();
    binding.object = digest;
    *value = encode_owner_binding(&binding);
    for binding in &mut altered.manifest.packages[index].runtime_owners {
        if binding.owner == owner {
            assert_eq!(binding.object, old);
            binding.object = digest;
        }
    }
    altered
        .objects
        .remove(&ObjectKey::from_digest(ObjectDomain::Owner, old.bytes()))
        .unwrap();
    altered.objects.insert(
        ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
        bytes,
    );
    altered.manifest.packages[index].reference_owners =
        replace_artifact_map(&mut altered.objects, entries);
    retain_live_map_pages(&mut altered);
    altered
}

pub(super) fn retain_live_map_pages(altered: &mut LoadedArtifact) {
    let mut pages = MemoryPageStore::default();
    for package in &altered.manifest.packages {
        let compilation = CompilationManifest::decode(
            &altered.objects[&package.compilation.object_key()],
            package.compilation,
        )
        .unwrap();
        for root in [
            package.interface_owners,
            package.reference_owners,
            compilation.units,
        ] {
            PersistentMap::from_root(root)
                .copy_reachable(
                    &ObjectPageReader::new(&altered),
                    &mut pages,
                    &mut MapWork::default(),
                )
                .unwrap();
        }
    }
    let live = pages
        .objects()
        .map(|(digest, _)| digest.bytes())
        .collect::<BTreeSet<_>>();
    altered
        .objects
        .retain(|key, _| key.domain != ObjectDomain::MapPage || live.contains(&key.digest.bytes()));
}
