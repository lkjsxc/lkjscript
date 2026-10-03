//! Independently rehashed artifacts must retain one canonical callable meaning.
use super::*;
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::execution::ExecutionControl;
use crate::platform::execution::normalized::{
    NormalizedProgram, NormalizedRunPolicy, run_graph_tests,
};
use crate::platform::kernel::{
    DeclarationPayload, EncodedOwnerKey, ExpressionOperation, ExpressionValidationLimits,
    ImplementationName, KernelSnapshot, LocalValueReference, OwnerKey, OwnerRecord, ParameterUse,
    TypeForm, TypeObject, decode_owner_binding, encode_owner, encode_owner_binding,
    encode_type_object, validate_affine_roots_with_limits, validate_expression_roots_with_limits,
};
use crate::platform::persistent_map::{MapRoot, MapWork, MemoryPageStore, PersistentMap};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::DeclarationId;
use crate::platform::storage::object::{ObjectDomain, ObjectKey};
use crate::platform::storage::pack::PackBuilder;
use crate::platform::storage::page_store::ObjectPageReader;
use std::collections::BTreeSet;

const EXTERNAL_SOURCE: &str = r#"declarations.begin
(units (module create correspondence
  (external create add (visibility private) (implementation core.i64.add)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (test create addition (visibility private)
    (actual (call add (i64 7) (i64 2))) (expected (i64 9)))))
declarations.end
"#;

const AFFINE_SOURCE: &str = r#"declarations.begin
(units (module create correspondence
  (external create empty (visibility private) (implementation core.buffer.empty)
    (returns ByteBuffer))
  (external create discard (visibility private) (implementation core.buffer.discard)
    (parameter create buffer (type ByteBuffer) (use consume)) (returns Unit))
  (function create consume (visibility private) (effect pure)
    (parameter create x (type ByteBuffer) (use consume))
    (returns Unit)
    (body (if (bool true) (unit)
      (let
        (binding a (type ByteBuffer) (call empty))
        (binding b (type ByteBuffer) (call empty))
        (in (sequence (call discard (local a)) (call discard (local b))))))))
  (test create unchanged (visibility private)
    (actual (let (binding input (type ByteBuffer) (call empty))
      (in (call consume (local input))))) (expected (unit)))))
declarations.end
"#;

fn fixture(input: &str) -> (KernelSnapshot, LoadedArtifact) {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(input)
            .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let repository =
        GraphRepository::create(&directory.path().join("correspondence"), &source, None)
            .unwrap()
            .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let linked = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    (source, load_artifact(&linked.artifact.bytes).unwrap())
}

fn declaration_named(source: &KernelSnapshot, name: &str) -> DeclarationId {
    source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.name.as_str() == name =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}

fn unit_for(loaded: &LoadedArtifact, owner: OwnerKey) -> (ObjectKey, CompilationUnit) {
    loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .find(|(_, unit)| unit.source.owner == owner)
        .unwrap()
}

fn assert_unchanged_behavior(source: &KernelSnapshot, bytes: &[u8]) {
    let program = NormalizedProgram::prepare(load_artifact(bytes).unwrap()).unwrap();
    let receipt = run_graph_tests(
        source,
        &program,
        None,
        NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!((receipt.passed, receipt.failed), (1, 0));
    assert_eq!(receipt.differential, "equal");
}

#[test]
fn rehashed_external_implementation_and_signature_must_match_canonical_declaration() {
    let (source, loaded) = fixture(EXTERNAL_SOURCE);
    let (old, unit) = unit_for(
        &loaded,
        OwnerKey::Declaration(declaration_named(&source, "add")),
    );
    let mut control = loaded.clone();
    replace_unit(&mut control, old, &unit);
    assert_unchanged_behavior(&source, &bundle(control));

    for fault in 0..3 {
        let mut forged = loaded.clone();
        let mut changed = unit.clone();
        let CompilationPayload::External {
            signature,
            implementation,
        } = &mut changed.payload
        else {
            panic!("external fixture");
        };
        match fault {
            0 => *implementation = ImplementationName::new("core.i64.subtract").unwrap(),
            1 => signature.parameters.swap(0, 1),
            2 => {
                let (digest, bytes) =
                    encode_type_object(&TypeObject::new(TypeForm::Bool).unwrap()).unwrap();
                signature.result = changed.tables.types.len() as u32;
                changed.tables.types.push(digest);
                forged.objects.insert(
                    ObjectKey::from_digest(ObjectDomain::Type, digest.bytes()),
                    bytes,
                );
            }
            _ => unreachable!(),
        }
        replace_unit(&mut forged, old, &changed);
        let error = load_artifact(&bundle(forged))
            .expect_err("rehashed external mismatch must reject before preparation");
        assert_eq!(
            error.code, "artifact_reference_declaration_payload",
            "fault {fault}: {error:?}"
        );
        assert_eq!(error.class, DiagnosticClass::Corrupt);
    }
}

#[test]
fn rehashed_conflicting_runtime_body_cannot_hide_untaken_double_consumption() {
    let (source, loaded) = fixture(AFFINE_SOURCE);
    let owner = OwnerKey::Declaration(declaration_named(&source, "consume"));
    let OwnerRecord::Declaration(declaration) = &source.owners[&owner] else {
        panic!("function fixture");
    };
    let DeclarationPayload::Function(function) = &declaration.payload else {
        panic!("function fixture");
    };
    let OwnerRecord::Expression(root) = &source.owners[&OwnerKey::Expression(function.body)] else {
        panic!("conditional fixture");
    };
    let ExpressionOperation::If {
        when_true,
        when_false,
        ..
    } = &root.operation
    else {
        panic!("conditional fixture");
    };
    let OwnerRecord::Expression(branch) = &source.owners[&OwnerKey::Expression(*when_false)] else {
        panic!("untaken branch");
    };
    let ExpressionOperation::Let { bindings, body } = &branch.operation else {
        panic!("untaken buffer scope");
    };
    assert_eq!(bindings.len(), 2);
    let OwnerRecord::Expression(sequence) = &source.owners[&OwnerKey::Expression(*body)] else {
        panic!("consume sequence");
    };
    let ExpressionOperation::Sequence { items } = &sequence.operation else {
        panic!("consume sequence");
    };
    let OwnerRecord::Expression(call) = &source.owners[&OwnerKey::Expression(items[1])] else {
        panic!("second consume");
    };
    let ExpressionOperation::Call { arguments, .. } = &call.operation else {
        panic!("second consume");
    };
    let local = OwnerKey::Expression(arguments[0]);
    let mut invalid = source.clone();
    let OwnerRecord::Expression(expression) = invalid.owners.get_mut(&local).unwrap() else {
        panic!("local fixture");
    };
    assert_eq!(
        expression.operation,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(bindings[1]),
        }
    );
    expression.operation = ExpressionOperation::Local {
        value: LocalValueReference::LexicalBinding(bindings[0]),
    };
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    assert!(!crate::platform::kernel::memory_reference::accepts(
        &invalid
    ));

    // The malformed untaken branch remains type-correct, but independently fails ownership.
    let limits = ExpressionValidationLimits {
        maximum_steps: crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK,
        maximum_diagnostics: 1,
    };
    let mut diagnostics = Vec::new();
    validate_expression_roots_with_limits(&invalid, [owner], &mut diagnostics, &mut 0, limits)
        .unwrap();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    validate_affine_roots_with_limits(&invalid, [owner], &mut diagnostics, &mut 0, limits).unwrap();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, "kernel_buffer_ownership");

    let (old, unit) = unit_for(&loaded, owner);
    let mut control = loaded.clone();
    replace_reference_owner(&mut control, local, &source.owners[&local]);
    replace_unit(&mut control, old, &unit);
    assert_unchanged_behavior(&source, &bundle(control));

    // Even two valid bodies cannot be separate canonical meanings for one exact owner.
    let mut conflict = loaded.clone();
    let mut safe = source.owners[&owner].clone();
    let OwnerRecord::Declaration(safe_declaration) = &mut safe else {
        unreachable!()
    };
    let DeclarationPayload::Function(safe_function) = &mut safe_declaration.payload else {
        unreachable!()
    };
    safe_function.body = *when_true;
    replace_runtime_owner(&mut conflict, owner, &safe);
    let error = load_artifact(&bundle(conflict)).unwrap_err();
    assert_eq!(error.code, "artifact_runtime_owner_semantics", "{error:?}");

    let mut changed = unit.clone();
    let CompilationPayload::Function { code, .. } = &mut changed.payload else {
        panic!("compiled function");
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
        super::lower::canonical_code(
            &invalid,
            changed.source.package,
            match changed.source.owner {
                OwnerKey::Declaration(declaration) => Some(declaration),
                _ => None,
            },
            &changed.tables,
            function.body,
            &function.parameters,
        )
        .unwrap()
    );

    // Retain coherent producer summaries and unit keys as well as instruction correspondence.
    // Rebuilding these observations cannot mint a semantic validation witness for the forgery.
    let facts = crate::platform::witness::rebuild_canonical_facts(&invalid).unwrap();
    let summary = &facts.summaries[&owner];
    changed.source = CompilationSource {
        package: invalid.root.package_id,
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
    changed.key = CompilationUnitKey::derive_generation(
        &changed.source,
        changed.optimization,
        changed.contract_version,
        changed.bytecode_contract_version,
        changed.graph_contract_version,
    )
    .unwrap();

    let mut forged = loaded.clone();
    replace_reference_owner(&mut forged, local, &invalid.owners[&local]);
    replace_unit(&mut forged, old, &changed);
    // Without a split declaration, complete affine admission still rejects this coherent code.
    let error = load_artifact(&bundle(forged.clone())).unwrap_err();
    assert_eq!(error.code, "artifact_affine_meaning", "{error:?}");
    assert!(
        error.message.contains("kernel_buffer_ownership"),
        "{error:?}"
    );

    replace_runtime_owner(&mut forged, owner, &safe);
    let error = load_artifact(&bundle(forged))
        .expect_err("a safe runtime body cannot hide the canonical untaken affine violation");
    assert_eq!(error.code, "artifact_runtime_owner_semantics", "{error:?}");
    assert_eq!(error.class, DiagnosticClass::Corrupt);
}

fn map_entries(loaded: &LoadedArtifact, root: MapRoot) -> Vec<(Vec<u8>, Vec<u8>)> {
    let mut entries = Vec::new();
    PersistentMap::from_root(root)
        .for_each(
            &ObjectPageReader::new(loaded),
            &mut MapWork::default(),
            |key, value| {
                entries.push((key.to_vec(), value.to_vec()));
                Ok(())
            },
        )
        .unwrap();
    entries
}

fn replace_map(loaded: &mut LoadedArtifact, entries: Vec<(Vec<u8>, Vec<u8>)>) -> MapRoot {
    let mut pages = MemoryPageStore::default();
    let map = PersistentMap::from_sorted(&mut pages, entries, &mut MapWork::default()).unwrap();
    for (digest, bytes) in pages.objects() {
        loaded.objects.insert(
            ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes()),
            bytes.to_vec(),
        );
    }
    map.root()
}

fn replace_reference_owner(loaded: &mut LoadedArtifact, owner: OwnerKey, record: &OwnerRecord) {
    assert_eq!(loaded.manifest.packages.len(), 1);
    let mut entries = map_entries(loaded, loaded.manifest.packages[0].reference_owners);
    let encoded = EncodedOwnerKey::new(owner).bytes();
    let (_, value) = entries
        .iter_mut()
        .find(|(key, _)| key.as_slice() == encoded.as_slice())
        .unwrap();
    let mut binding = decode_owner_binding(value, owner).unwrap();
    // Only expressions are replaced here; no runtime/interface inventory retains their bytes.
    assert!(matches!(owner, OwnerKey::Expression(_)));
    loaded
        .objects
        .remove(&ObjectKey::from_digest(
            ObjectDomain::Owner,
            binding.object.bytes(),
        ))
        .unwrap();
    let (digest, bytes) = encode_owner(record).unwrap();
    binding.object = digest;
    *value = encode_owner_binding(&binding);
    loaded.objects.insert(
        ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
        bytes,
    );
    loaded.manifest.packages[0].reference_owners = replace_map(loaded, entries);
}

fn replace_runtime_owner(loaded: &mut LoadedArtifact, owner: OwnerKey, record: &OwnerRecord) {
    let binding = loaded.manifest.packages[0]
        .runtime_owners
        .iter_mut()
        .find(|binding| binding.owner == owner)
        .unwrap();
    let (digest, bytes) = encode_owner(record).unwrap();
    // The original owner remains reachable through the reference inventory.
    binding.object = digest;
    loaded.objects.insert(
        ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
        bytes,
    );
}

fn replace_unit(loaded: &mut LoadedArtifact, old: ObjectKey, unit: &CompilationUnit) {
    // Packed encoding, map reconstruction and pack sealing do not perform semantic admission.
    // Structural decoding is a control; the strict artifact loader is the adversarial boundary.
    assert_eq!(unit.contract_version, COMPILER_UNIT_CONTRACT_VERSION);
    let bytes = crate::platform::packed::encode(
        COMPILER_UNIT_MAGIC,
        COMPILER_UNIT_ENVELOPE_DOMAIN,
        unit,
        super::unit::MAXIMUM_COMPILER_UNIT_BYTES,
    )
    .unwrap();
    let key = ObjectKey::for_bytes(ObjectDomain::CompilerUnit, &bytes);
    CompilationUnit::decode(&bytes, key).expect("structurally valid rehashed unit");
    let old_manifest = loaded.manifest.packages[0].compilation;
    let mut compilation =
        CompilationManifest::decode(&loaded.objects[&old_manifest.object_key()], old_manifest)
            .unwrap();
    let mut entries = map_entries(loaded, compilation.units);
    let mut replacements = 0;
    for (owner, value) in &mut entries {
        let owner = EncodedOwnerKey::decode(owner).unwrap();
        let mut binding = CompilationBinding::decode(value, owner).unwrap();
        if binding.object.object_key() == old {
            binding.object = CompilerUnitObjectDigest::from_bytes(key.digest.bytes());
            binding.key = unit.key;
            binding.kind = unit.source.kind;
            *value = binding.encode(owner).unwrap();
            replacements += 1;
        }
    }
    assert_eq!(replacements, 1);
    loaded.objects.remove(&old).unwrap();
    loaded.objects.insert(key, bytes);
    compilation.units = replace_map(loaded, entries);
    let (digest, bytes) = compilation.encode().unwrap();
    loaded.objects.remove(&old_manifest.object_key()).unwrap();
    loaded.objects.insert(digest.object_key(), bytes);
    loaded.manifest.packages[0].compilation = digest;
}

fn bundle(mut loaded: LoadedArtifact) -> Vec<u8> {
    assert_eq!(loaded.manifest.contract_version, ARTIFACT_CONTRACT_VERSION);
    let mut pages = MemoryPageStore::default();
    for package in &loaded.manifest.packages {
        let compilation = CompilationManifest::decode(
            &loaded.objects[&package.compilation.object_key()],
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
                    &ObjectPageReader::new(&loaded),
                    &mut pages,
                    &mut MapWork::default(),
                )
                .unwrap();
        }
    }
    let live_pages = pages
        .objects()
        .map(|(digest, _)| digest.bytes())
        .collect::<BTreeSet<_>>();
    loaded.objects.retain(|key, _| {
        key.domain != ObjectDomain::MapPage || live_pages.contains(&key.digest.bytes())
    });
    let (closure, count, length) = super::artifact::closure_facts(&loaded.objects).unwrap();
    loaded.manifest.closure = closure;
    loaded.manifest.object_count = count;
    loaded.manifest.object_bytes = length;
    let (digest, manifest_bytes) = loaded.manifest.encode().unwrap();
    let mut pack = PackBuilder::default();
    for (key, bytes) in &loaded.objects {
        pack.insert(*key, bytes).unwrap();
    }
    let segments = pack.seal_targeted(4 * 1024 * 1024).unwrap();
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&ARTIFACT_BUNDLE_MAGIC);
    bytes.extend_from_slice(&loaded.manifest.contract_version.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&(manifest_bytes.len() as u64).to_be_bytes());
    bytes.extend_from_slice(&(segments.len() as u64).to_be_bytes());
    bytes.extend_from_slice(&digest.bytes());
    bytes.extend_from_slice(&manifest_bytes);
    for segment in segments {
        bytes.extend_from_slice(&(segment.bytes.len() as u64).to_be_bytes());
        bytes.extend_from_slice(&segment.id.bytes());
        bytes.extend_from_slice(&segment.bytes);
    }
    let mut checksum = blake3::Hasher::new_derive_key(ARTIFACT_BUNDLE_CHECKSUM_DOMAIN);
    checksum.update(&(bytes.len() as u64).to_be_bytes());
    checksum.update(&bytes);
    bytes.extend_from_slice(checksum.finalize().as_bytes());
    bytes.extend_from_slice(&ARTIFACT_BUNDLE_END_MAGIC);
    bytes
}
