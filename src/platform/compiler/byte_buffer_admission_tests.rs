//! Coherent canonical and compiled double consumption must fail affine artifact admission.
use super::*;
use crate::platform::kernel::{
    DeclarationPayload, EncodedOwnerKey, ExpressionValidationLimits, KernelSnapshot, ParameterUse,
    decode_owner_binding, encode_owner_binding, validate_affine_roots_with_limits,
    validate_expression_roots_with_limits,
};

// Both owners are authored and accepted through the native change surface. The true arm is
// ordinary Unit; the two distinct buffer owners are consumed only in the untaken arm.
const EXTRA: &str = r#"
declarations.begin
(units (module create admission
  (function create affine-function (visibility private) (returns Unit) (effect pure)
    (body (if (bool true) (unit)
      (let
        (binding a (type ByteBuffer) (call memory::empty))
        (binding b (type ByteBuffer) (call memory::empty))
        (in (sequence
          (call memory::discard (local a))
          (call memory::discard (local b))))))))
  (constant create affine-constant (visibility private) (type Unit)
    (value (if (bool true) (unit)
      (let
        (binding a (type ByteBuffer) (call memory::empty))
        (binding b (type ByteBuffer) (call memory::empty))
        (in (sequence
          (call memory::discard (local a))
          (call memory::discard (local b))))))))))
declarations.end
"#;

#[test]
fn byte_buffer_function_artifact_rejects_rehashed_untaken_double_consume() {
    reject_untaken_double_consume("affine-function");
}

#[test]
fn byte_buffer_constant_artifact_rejects_rehashed_untaken_double_consume() {
    reject_untaken_double_consume("affine-constant");
}

fn expression(snapshot: &KernelSnapshot, id: ExpressionId) -> &ExpressionOperation {
    let OwnerRecord::Expression(record) = &snapshot.owners[&OwnerKey::Expression(id)] else {
        panic!("fixture expression");
    };
    &record.operation
}

fn reject_untaken_double_consume(name: &str) {
    let snapshot =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author(EXTRA).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(
        &snapshot
    ));
    let owner = OwnerKey::Declaration(declaration_named(&snapshot, name));
    let OwnerRecord::Declaration(declaration) = &snapshot.owners[&owner] else {
        panic!("fixture declaration");
    };
    let root = match &declaration.payload {
        DeclarationPayload::Function(function) => {
            assert!(function.parameters.is_empty());
            function.body
        }
        DeclarationPayload::Constant { value, .. } => *value,
        _ => panic!("function or constant root"),
    };
    let ExpressionOperation::If {
        condition,
        when_true,
        when_false,
    } = expression(&snapshot, root)
    else {
        panic!("conditional root");
    };
    assert_eq!(
        expression(&snapshot, *condition),
        &ExpressionOperation::Bool { value: true }
    );
    assert_eq!(
        expression(&snapshot, *when_true),
        &ExpressionOperation::Unit {}
    );
    let ExpressionOperation::Let { bindings, body } = expression(&snapshot, *when_false) else {
        panic!("untaken lexical buffer scope");
    };
    assert_eq!(bindings.len(), 2);
    let buffer = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
        .unwrap()
        .0;
    for (binding, expected) in bindings.iter().zip(["a", "b"]) {
        let OwnerRecord::Binding(record) = &snapshot.owners[&OwnerKey::Binding(*binding)] else {
            panic!("buffer binding");
        };
        assert_eq!(record.name.as_str(), expected);
        assert_eq!(record.declared_type, Some(buffer));
    }
    let ExpressionOperation::Sequence { items } = expression(&snapshot, *body) else {
        panic!("untaken consume sequence");
    };
    assert_eq!(items.len(), 2);
    let locals = items
        .iter()
        .zip(bindings)
        .map(|(call, binding)| {
            let ExpressionOperation::Call {
                function,
                arguments,
                ..
            } = expression(&snapshot, *call)
            else {
                panic!("discard call");
            };
            assert_eq!(function.package, snapshot.root.package_id);
            assert_eq!(
                function.declaration,
                declaration_named(&snapshot, "discard")
            );
            assert_eq!(arguments.len(), 1);
            assert_eq!(
                expression(&snapshot, arguments[0]),
                &ExpressionOperation::Local {
                    value: LocalValueReference::LexicalBinding(*binding),
                }
            );
            arguments[0]
        })
        .collect::<Vec<_>>();

    let dir = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&dir.path().join("admission"), &snapshot, None)
        .unwrap()
        .repository;
    let compiled = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let linked = link_artifact(&repository, compiled.manifest_digest, &[]).unwrap();
    let loaded = load_artifact(&linked.artifact.bytes).expect("accepted native control loads");
    let (old_unit, mut unit) = loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .find(|(_, unit)| unit.source.owner == owner)
        .unwrap();
    let local_owner = OwnerKey::Expression(locals[1]);
    let control =
        replace_reference_expression(&loaded, local_owner, &snapshot.owners[&local_owner]);
    load_artifact(&effect_tests::replace_unit(
        &control,
        old_unit,
        &unit,
        vec![],
    ))
    .expect("neutral reference-map and compilation-map reconstruction preserves valid control");

    // Change exactly the second local from b to a. Both let bindings and their initializers
    // remain in the reference closure, including the now-unused b, through the let inventory.
    let mut forged = snapshot.clone();
    let OwnerRecord::Expression(record) = forged.owners.get_mut(&local_owner).unwrap() else {
        unreachable!();
    };
    record.operation = ExpressionOperation::Local {
        value: LocalValueReference::LexicalBinding(bindings[0]),
    };
    assert!(!crate::platform::kernel::memory_reference::accepts(&forged));
    let limits = ExpressionValidationLimits {
        maximum_steps: crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK,
        maximum_diagnostics: 1,
    };
    let mut diagnostics = Vec::new();
    validate_expression_roots_with_limits(&forged, [owner], &mut diagnostics, &mut 0, limits)
        .unwrap();
    assert!(
        diagnostics.is_empty(),
        "type-valid forgery: {diagnostics:?}"
    );
    validate_affine_roots_with_limits(&forged, [owner], &mut diagnostics, &mut 0, limits).unwrap();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "kernel_buffer_ownership");

    let code = match &mut unit.payload {
        CompilationPayload::Function { code, .. } | CompilationPayload::Constant { code, .. } => {
            code
        }
        _ => panic!("executable fixture code"),
    };
    let loads = code
        .instructions
        .iter()
        .enumerate()
        .filter_map(|(index, instruction)| match instruction {
            CompiledInstruction::LoadLocal { local, use_mode } => {
                assert_eq!(*use_mode, ParameterUse::Consume);
                Some((index, *local))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(loads.len(), 2);
    assert_ne!(loads[0].1, loads[1].1);
    code.instructions[loads[1].0] = CompiledInstruction::LoadLocal {
        local: loads[0].1,
        use_mode: ParameterUse::Consume,
    };
    assert_eq!(
        *code,
        super::super::lower::canonical_code(&forged, unit.source.package, &unit.tables, root, &[],)
            .unwrap(),
        "forged instructions must exactly correspond to forged canonical meaning"
    );

    // Recompute the actual source summary dimensions without minting a validation witness.
    // The semantic violation is intentional; stale producer summary/key assertions are not.
    let facts = crate::platform::witness::rebuild_canonical_facts(&forged).unwrap();
    let summary = &facts.summaries[&owner];
    assert_ne!(summary.implementation, unit.source.implementation);
    unit.source = CompilationSource {
        package: forged.root.package_id,
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
    unit.key = CompilationUnitKey::derive_generation(
        &unit.source,
        unit.optimization,
        unit.contract_version,
        unit.bytecode_contract_version,
        unit.graph_contract_version,
    )
    .unwrap();
    unit.validate()
        .expect("structurally valid compiled forgery");
    let altered = replace_reference_expression(&loaded, local_owner, &forged.owners[&local_owner]);
    // This helper repairs the unit object, compilation map/manifest, exact closure inventory,
    // artifact manifest, pack identities and complete-envelope checksum without self-admission.
    let bytes = effect_tests::replace_unit(&altered, old_unit, &unit, vec![]);
    let failure = match load_artifact(&bytes) {
        Ok(_) => panic!("untaken double consumption was incorrectly admitted"),
        Err(failure) => failure,
    };
    assert_eq!(
        failure.code, "artifact_affine_meaning",
        "{name}: {failure:?}"
    );
    assert!(
        failure.message.contains("kernel_buffer_ownership"),
        "{name}: {failure:?}"
    );
}

fn replace_reference_expression(
    loaded: &LoadedArtifact,
    owner: OwnerKey,
    record: &OwnerRecord,
) -> LoadedArtifact {
    assert!(matches!(owner, OwnerKey::Expression(_)));
    assert!(matches!(record, OwnerRecord::Expression(_)));
    let mut altered = loaded.clone();
    let package = altered
        .manifest
        .packages
        .iter_mut()
        .find(|package| package.package == loaded.manifest.root_package)
        .unwrap();
    let old_root = package.reference_owners;
    let mut entries = artifact_map_entries(loaded, old_root);
    let encoded = EncodedOwnerKey::new(owner).bytes().to_vec();
    let (_, value) = entries
        .iter_mut()
        .find(|(key, _)| key.as_slice() == encoded.as_slice())
        .unwrap();
    let mut binding = decode_owner_binding(value, owner).unwrap();
    let old_owner = ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
    let (digest, bytes) = encode_owner(record).unwrap();
    assert!(
        package
            .runtime_owners
            .iter()
            .all(|entry| entry.object != binding.object)
    );
    binding.object = digest;
    *value = encode_owner_binding(&binding);
    altered.objects.remove(&old_owner).unwrap();
    altered.objects.insert(
        ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
        bytes,
    );
    package.reference_owners = replace_artifact_map(&mut altered.objects, entries);
    assert_eq!(package.reference_owners.entries(), old_root.entries());

    let mut old_pages = MemoryPageStore::default();
    PersistentMap::from_root(old_root)
        .copy_reachable(
            &ObjectPageReader::new(loaded),
            &mut old_pages,
            &mut MapWork::default(),
        )
        .unwrap();
    // Retain shared pages still reached by any interface, reference or compilation map.
    let mut live_pages = MemoryPageStore::default();
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
                    &mut live_pages,
                    &mut MapWork::default(),
                )
                .unwrap();
        }
    }
    let live = live_pages
        .objects()
        .map(|(digest, _)| digest)
        .collect::<BTreeSet<_>>();
    for (digest, _) in old_pages.objects() {
        if !live.contains(&digest) {
            altered.objects.remove(&ObjectKey::from_digest(
                ObjectDomain::MapPage,
                digest.bytes(),
            ));
        }
    }
    altered
}
