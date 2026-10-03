//! Coherent canonical and compiled double consumption must fail affine artifact admission.
use super::*;
use crate::platform::DiagnosticClass;
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

#[test]
fn owned_task_artifact_rejects_consistently_rehashed_untaken_double_transfer() {
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(r#"
declarations.begin
(units (module create task-artifact
  (external create empty (visibility private) (implementation core.buffer.empty) (returns ByteBuffer))
  (function create transfer (visibility private) (effect (task))
    (parameter create owner (type ByteBuffer) (use consume))
    (returns Unit) (body (unit)))
  (function create attack (visibility private) (effect (task)) (returns Unit)
    (body (if (bool true) (unit)
      (let
        (binding a (type ByteBuffer) (call empty))
        (binding b (type ByteBuffer) (call empty))
        (in (sequence (call transfer (local a)) (call transfer (local b))))))))))
declarations.end
"#).unwrap();
    reject_source(source, "attack", TypeForm::ByteBuffer, "transfer");
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
    reject_source(snapshot, name, TypeForm::ByteBuffer, "discard");
}

#[test]
fn owned_witness_artifact_rejects_consistently_rehashed_untaken_consumption() {
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        &[
            include_str!("../../../tests/fixtures/owned-witness-library.lkjc"),
            include_str!("../../../tests/fixtures/owned-witness-cell.lkjc"),
            include_str!("../../../tests/fixtures/owned-witness-buffer.lkjc"),
            include_str!("../../../tests/fixtures/owned-witness-artifact.lkjc"),
        ]
        .join("\n"),
    )
    .unwrap();
    reject_source(
        source.clone(),
        "hostile-cell",
        TypeForm::OwnedI64Cell,
        "retire",
    );
    reject_source(source, "hostile-buffer", TypeForm::ByteBuffer, "retire");
}

#[test]
fn owned_choice_artifact_rejects_rehashed_case_targets_bindings_tags_and_loan_modes() {
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        include_str!("../../../tests/fixtures/owned-choices.lkjc"),
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&dir.path().join("choices"), &source, None)
        .unwrap()
        .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let artifact = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    let loaded = load_artifact(&artifact.artifact.bytes).unwrap();
    for name in ["restore", "route"] {
        let owner = OwnerKey::Declaration(declaration_named(&source, name));
        let (key, unit) = loaded
            .objects
            .iter()
            .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
            .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
            .find(|(_, unit)| unit.source.owner == owner)
            .unwrap();
        load_artifact(&effect_tests::replace_unit(&loaded, key, &unit, vec![])).unwrap();
        let attacks = if name == "restore" { 6 } else { 2 };
        for attack in 0..attacks {
            let mut changed = unit.clone();
            let CompilationPayload::Function { code, .. } = &mut changed.payload else {
                panic!("choice function");
            };
            if name == "restore" {
                let index = code
                    .instructions
                    .iter()
                    .position(|i| matches!(i, CompiledInstruction::MatchOwned { .. }))
                    .unwrap();
                if attack < 4 {
                    let CompiledInstruction::MatchOwned { cases, .. } =
                        &mut code.instructions[index]
                    else {
                        unreachable!();
                    };
                    assert_eq!(cases.len(), 2);
                    match attack {
                        0 => cases.swap(0, 1),
                        1 => cases[1].binding_local = cases[0].binding_local,
                        2 => cases[0].target = cases[1].target,
                        3 => {
                            cases.pop();
                        }
                        _ => unreachable!(),
                    }
                } else {
                    let CompiledInstruction::LoadLocal { use_mode, .. } =
                        &mut code.instructions[index - 1]
                    else {
                        panic!("direct owning match source");
                    };
                    assert_eq!(*use_mode, ParameterUse::Consume);
                    *use_mode = if attack == 4 {
                        ParameterUse::Borrow
                    } else {
                        ParameterUse::Unrestricted
                    };
                }
            } else {
                let instruction = code
                    .instructions
                    .iter_mut()
                    .find(|i| matches!(i, CompiledInstruction::ChooseOwned { .. }))
                    .unwrap();
                let CompiledInstruction::ChooseOwned { case, .. } = instruction else {
                    unreachable!();
                };
                *case = if attack == 0 { 1 - *case } else { 2 };
            }
            let bytes = if name == "restore" && (1..=3).contains(&attack) {
                effect_tests::replace_rejected_unit(
                    &loaded,
                    key,
                    &changed,
                    if attack == 1 {
                        "compiler_unit_choice_local"
                    } else {
                        "compiler_unit_unreachable_instruction"
                    },
                )
            } else {
                effect_tests::replace_unit(&loaded, key, &changed, vec![])
            };
            let error = load_artifact(&bytes)
                .expect_err("consistent rehash cannot change checked choice meaning");
            println!("owned choice {name} attack {attack}: {}", error.code);
        }
    }
}

#[test]
fn owned_choice_artifact_rejects_coherent_untaken_double_consumption() {
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(r#"declarations.begin
(units (module create hostile
  (external create empty (visibility private) (implementation core.buffer.empty) (returns ByteBuffer))
  (function create factory (visibility private) (effect pure) (returns (owned-choice (case accepted Unit) (case rejected ByteBuffer)))
    (body (let (binding b (type ByteBuffer) (call empty))
      (in (choose-owned (type (owned-choice (case accepted Unit) (case rejected ByteBuffer))) (case rejected) (local b))))))
  (function create retire (visibility private) (effect pure)
    (parameter create p (type (owned-choice (case accepted Unit) (case rejected ByteBuffer))) (use consume))
    (returns Unit) (body (unit)))
  (function create attack (visibility private) (effect pure) (returns Unit)
    (body (if (bool true) (unit)
      (let
        (binding a (type (owned-choice (case accepted Unit) (case rejected ByteBuffer))) (call factory))
        (binding b (type (owned-choice (case accepted Unit) (case rejected ByteBuffer))) (call factory))
        (in (sequence (call retire (local a)) (call retire (local b))))))))))
declarations.end"#).unwrap();
    let carrier = source
        .types
        .values()
        .find(|t| matches!(t.form, TypeForm::OwnedChoice { .. }))
        .unwrap()
        .form
        .clone();
    reject_source(source, "attack", carrier, "retire");
}

#[test]
fn owned_products_artifact_rejects_rehashed_source_code_and_metadata_double_consume() {
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(r#"declarations.begin
(units (module create hostile
  (external create empty (visibility private) (implementation core.buffer.empty) (returns ByteBuffer))
  (function create factory (visibility private) (effect pure) (returns (owned-product (field data ByteBuffer)))
    (body (let (binding b (type ByteBuffer) (call empty))
      (in (pack-owned (type (owned-product (field data ByteBuffer))) (field data (local b)))))))
  (function create retire (visibility private) (effect pure)
    (parameter create p (type (owned-product (field data ByteBuffer))) (use consume))
    (returns Unit) (body (unit)))
  (function create attack (visibility private) (effect pure) (returns Unit)
    (body (if (bool true) (unit)
      (let
        (binding a (type (owned-product (field data ByteBuffer))) (call factory))
        (binding b (type (owned-product (field data ByteBuffer))) (call factory))
        (in (sequence (call retire (local a)) (call retire (local b))))))))))
declarations.end"#).unwrap();
    let carrier = source
        .types
        .values()
        .find(|t| matches!(t.form, TypeForm::OwnedProduct { .. }))
        .unwrap()
        .form
        .clone();
    reject_source(source, "attack", carrier, "retire");
}

#[test]
fn owned_products_metadata_artifact_rejects_rehashed_loan_mode_changes() {
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        include_str!("../../../tests/fixtures/owned-products-read.lkjc"),
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&dir.path().join("metadata"), &source, None)
        .unwrap()
        .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let artifact = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    let loaded = load_artifact(&artifact.artifact.bytes).unwrap();
    let owner = OwnerKey::Declaration(declaration_named(&source, "tag"));
    let (key, unit) = loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .find(|(_, unit)| unit.source.owner == owner)
        .unwrap();
    // Rehashing a neutral replacement must preserve admission: rejection below
    // is about instruction meaning, not a stale digest or an invalid container.
    load_artifact(&effect_tests::replace_unit(&loaded, key, &unit, vec![])).unwrap();
    let CompilationPayload::Function { code, .. } = &unit.payload else {
        panic!("metadata helper function");
    };
    let indices = code
        .instructions
        .windows(2)
        .enumerate()
        .filter_map(|(i, pair)| {
            matches!(
                pair,
                [
                    CompiledInstruction::LoadLocal {
                        use_mode: ParameterUse::Borrow,
                        ..
                    },
                    CompiledInstruction::Field(_),
                ]
            )
            .then_some(i)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        indices.len(),
        1,
        "the read borrows exactly one whole product"
    );
    for mode in [ParameterUse::Unrestricted, ParameterUse::Consume] {
        let mut changed = unit.clone();
        let CompilationPayload::Function { code, .. } = &mut changed.payload else {
            unreachable!();
        };
        let CompiledInstruction::LoadLocal { use_mode, .. } = &mut code.instructions[indices[0]]
        else {
            unreachable!();
        };
        *use_mode = mode;
        let bytes = effect_tests::replace_unit(&loaded, key, &changed, vec![]);
        let error = load_artifact(&bytes)
            .expect_err("rehashed metadata read must not copy or move its source");
        println!("metadata read {mode:?} forgery: {}", error.code);
    }
}

#[test]
fn owned_products_signature_only_artifact_rejects_rehashed_graph_18_package() {
    // A private Local-only relay gives the loader no public type or new operation
    // tag on which to rely. The complete compiled type closure still binds Graph 19.
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create signatures
  (function create relay (visibility private) (effect pure)
    (parameter create p (type (owned-product (field data ByteBuffer))) (use consume))
    (returns (owned-product (field data ByteBuffer))) (body (local p)))))
declarations.end"#,
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&dir.path().join("signatures"), &source, None)
        .unwrap()
        .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let artifact = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    let loaded = load_artifact(&artifact.artifact.bytes).unwrap();
    let mut objects = loaded.objects.clone();
    let mut manifest = loaded.manifest.clone();
    assert_eq!(manifest.packages.len(), 1);
    let package = &mut manifest.packages[0];
    let old = package.package_revision;
    let mut revision = crate::platform::package_transport::PackageRevision::decode(
        &objects
            .remove(&ObjectKey::from_digest(
                ObjectDomain::PackageRevision,
                old.bytes(),
            ))
            .unwrap(),
        old,
    )
    .unwrap();
    revision.graph_contract_version = 18;
    revision.revision.graph_contract_version = 18;
    package.semantic_revision = revision.revision.revision_id().unwrap();
    revision.interface = crate::platform::package_interface::package_interface_digest_for_graph(
        package.package,
        package.interface_owners.content_root(),
        18,
    )
    .unwrap();
    package.interface = revision.interface;
    let (digest, bytes) = revision.encode().unwrap();
    package.package_revision = digest;
    objects.insert(
        ObjectKey::from_digest(ObjectDomain::PackageRevision, digest.bytes()),
        bytes,
    );
    let mut compilation = CompilationManifest::decode(
        &objects.remove(&package.compilation.object_key()).unwrap(),
        package.compilation,
    )
    .unwrap();
    compilation.revision = package.semantic_revision;
    compilation.package_revision = package.package_revision;
    compilation.package_interface = package.interface;
    let (compilation_digest, compilation_bytes) = compilation.encode().unwrap();
    package.compilation = compilation_digest;
    objects.insert(compilation_digest.object_key(), compilation_bytes);
    let (closure, count, length) = super::super::artifact::closure_facts(&objects).unwrap();
    manifest.closure = closure;
    manifest.object_count = count;
    manifest.object_bytes = length;
    let bytes = nominal_session_tests::hostile_bundle(&manifest, &objects);
    let error = load_artifact(&bytes).unwrap_err();
    assert_eq!(error.code, "kernel_product_generation", "{error:?}");
    assert_eq!(error.class, DiagnosticClass::Semantic);
}

fn reject_source(snapshot: KernelSnapshot, name: &str, carrier: TypeForm, callee: &str) {
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
    let buffer = encode_type_object(&TypeObject::new(carrier).unwrap())
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
            let (function, arguments) = match expression(&snapshot, *call) {
                ExpressionOperation::Call {
                    function,
                    arguments,
                    ..
                }
                | ExpressionOperation::ImplementationCall {
                    function,
                    arguments,
                    ..
                } => (function, arguments),
                _ => panic!("consuming call"),
            };
            assert_eq!(function.package, snapshot.root.package_id);
            assert_eq!(function.declaration, declaration_named(&snapshot, callee));
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
    let mappings: usize = snapshot
        .owners
        .values()
        .map(|record| match record {
            OwnerRecord::Declaration(d) => match &d.payload {
                DeclarationPayload::OwnedImplementation(i) => i.methods.len(),
                _ => 0,
            },
            _ => 0,
        })
        .sum();
    assert_eq!(
        loaded.work.implementation_inventory_steps,
        linked.work.compiler_units + mappings as u64
    );
    assert_eq!(
        linked.work.implementation_inventory_steps,
        loaded.work.implementation_inventory_steps
    );
    let units = loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| {
            let unit = CompilationUnit::decode(bytes, *key).unwrap();
            ((unit.source.package, unit.source.owner), unit)
        })
        .collect();
    let mut exhausted = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK as u64;
    let failure = super::super::artifact::runtime_owner_expectations(
        &units,
        &mut exhausted,
        |_| panic!("work exhaustion must precede type reads"),
        || Ok(()),
    )
    .unwrap_err();
    assert_eq!(failure.class, DiagnosticClass::Resource);
    assert_eq!(failure.code, "artifact_owned_contract_work");
    let mut cancelled = 0;
    let failure = super::super::artifact::runtime_owner_expectations(
        &units,
        &mut cancelled,
        |_| panic!("cancellation must precede type reads"),
        || {
            Err(crate::platform::diagnostic::Diagnostic::new(
                DiagnosticClass::Resource,
                "owned_inventory_cancelled",
                "cancel before inventory growth",
            ))
        },
    )
    .unwrap_err();
    assert_eq!(failure.code, "owned_inventory_cancelled");
    assert_eq!(cancelled, 0);
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
        super::super::lower::canonical_code(
            &forged,
            unit.source.package,
            None,
            &unit.tables,
            root,
            &[],
        )
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
