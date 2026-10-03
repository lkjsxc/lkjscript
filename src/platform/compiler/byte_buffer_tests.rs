//! Rehashed artifacts must preserve memory modes, exact results, moves, and lexical cleanup.
use super::*;
use crate::platform::kernel::{DeclarationPayload, ParameterUse};
fn bundle(mut manifest: ArtifactManifest, objects: BTreeMap<ObjectKey, Vec<u8>>) -> Vec<u8> {
    let (closure, count, bytes) = super::super::artifact::closure_facts(&objects).unwrap();
    manifest.closure = closure;
    manifest.object_count = count;
    manifest.object_bytes = bytes;
    super::nominal_session_tests::hostile_bundle(&manifest, &objects)
}
#[test]
fn byte_buffer_affine_admission_retains_all_annotation_roots() {
    use crate::platform::execution::normalized::tests::byte_buffer_tests::author_only;
    let boolean = encode_type_object(&TypeObject::new(TypeForm::Bool).unwrap())
        .unwrap()
        .0;
    let buffer = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
        .unwrap()
        .0;
    for memory in [false, true] {
        let extra = if memory {
            "(external create empty (visibility private) (implementation core.buffer.empty) (returns ByteBuffer))"
        } else {
            ""
        };
        let binding = if memory {
            "(binding storage (type ByteBuffer) (call empty))"
        } else {
            ""
        };
        let input = format!(
            r#"declarations.begin
(units (module create annotations
  {extra}
  (function create answer (visibility public) (returns I64) (effect pure)
    (body (let {binding}
      (binding flag (type Bool) (bool true))
      (in (if (local flag) (i64 1) (i64 0))))))))
declarations.end"#
        );
        let snapshot = author_only(&input).unwrap();
        assert!(snapshot.types.contains_key(&boolean));
        let directory = tempfile::tempdir().unwrap();
        let repository =
            GraphRepository::create(&directory.path().join("annotations"), &snapshot, None)
                .unwrap()
                .repository;
        let built = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
        let linked = link_artifact(&repository, built.manifest_digest, &[]).unwrap();
        let loaded = load_artifact(&linked.artifact.bytes).unwrap();
        assert!(
            loaded
                .objects
                .contains_key(&ObjectKey::from_digest(ObjectDomain::Type, boolean.bytes())),
            "current units retain ordinary annotations too; absent metadata cannot certify unrestricted storage"
        );
        assert_eq!(
            loaded
                .objects
                .contains_key(&ObjectKey::from_digest(ObjectDomain::Type, buffer.bytes())),
            memory
        );
        // A real buffer's missing object is never treated as an ordinary annotation.
        if memory {
            let mut objects = loaded.objects.clone();
            objects
                .remove(&ObjectKey::from_digest(ObjectDomain::Type, buffer.bytes()))
                .unwrap();
            assert!(load_artifact(&bundle(loaded.manifest, objects)).is_err());
        }
    }
}

#[test]
fn byte_buffer_pure_requirement_parameters_keep_exact_artifact_metadata() {
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author(
        r#"declarations.begin
(units (module create metadata
  (interface create Input (visibility private)
    (operation create read (returns I64) (idempotency idempotent) (external-visibility none)))
  (function create producer-with-witness (visibility private)
    (requirement-parameter create R (interface Input) (operations Input::read))
    (returns ByteBuffer) (effect pure) (body (call memory::empty)))))
declarations.end"#,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&directory.path().join("metadata"), &snapshot, None)
        .unwrap()
        .repository;
    let built = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let linked = link_artifact(&repository, built.manifest_digest, &[]).unwrap();
    let loaded = load_artifact(&linked.artifact.bytes).unwrap();
    let declaration = OwnerKey::Declaration(declaration_named(&snapshot, "producer-with-witness"));
    let mut manifest = loaded.manifest.clone();
    let mut objects = loaded.objects.clone();
    let binding = manifest
        .packages
        .iter_mut()
        .flat_map(|p| &mut p.runtime_owners)
        .find(|b| b.owner == declaration)
        .unwrap();
    let old = ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
    let mut record =
        decode_owner(&objects[&old], binding.owner, binding.kind, binding.object).unwrap();
    let OwnerRecord::Declaration(d) = &mut record else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut d.payload else {
        unreachable!()
    };
    assert_eq!(function.requirement_parameters.len(), 1);
    function.requirement_parameters.clear();
    let (digest, bytes) = encode_owner(&record).unwrap();
    objects.remove(&old);
    objects.insert(
        ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
        bytes,
    );
    binding.object = digest;
    let error = load_artifact(&bundle(manifest, objects)).unwrap_err();
    assert_eq!(error.code, "artifact_runtime_owner_semantics");
}

#[test]
fn byte_buffer_forged_artifact_cannot_erase_modes_results_moves_or_cleanup() {
    let snapshot =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author("").unwrap();
    let dir = tempfile::tempdir().unwrap();
    let repo = GraphRepository::create(&dir.path().join("buffer"), &snapshot, None)
        .unwrap()
        .repository;
    let compiled = build_clean(&repo, OptimizationPolicy::DeterministicBaseline).unwrap();
    let linked = link_artifact(&repo, compiled.manifest_digest, &[]).unwrap();
    let loaded = load_artifact(&linked.artifact.bytes).unwrap();
    assert_eq!(
        (
            loaded.manifest.compiler_contract_version,
            loaded.manifest.bytecode_contract_version
        ),
        (22, 17)
    );
    let mut checked = 0;
    for (package, record) in loaded.manifest.packages.iter().enumerate() {
        for (index, binding) in record.runtime_owners.iter().enumerate() {
            let old = ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
            let original = decode_owner(
                &loaded.objects[&old],
                binding.owner,
                binding.kind,
                binding.object,
            )
            .unwrap();
            let mut altered = original.clone();
            let relevant = match &mut altered {
                OwnerRecord::Parameter(p) if p.use_mode != ParameterUse::Unrestricted => {
                    p.use_mode = ParameterUse::Unrestricted;
                    true
                }
                OwnerRecord::Declaration(d) if binding.kind == OwnerKind::PureFunction => {
                    if let DeclarationPayload::Function(f) = &mut d.payload {
                        f.result = crate::platform::kernel::encode_type_object(
                            &crate::platform::kernel::TypeObject::new(TypeForm::Unit).unwrap(),
                        )
                        .unwrap()
                        .0;
                        true
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if !relevant {
                continue;
            }
            let mut manifest = loaded.manifest.clone();
            let mut objects = loaded.objects.clone();
            objects.remove(&old);
            let (digest, bytes) = encode_owner(&altered).unwrap();
            manifest.packages[package].runtime_owners[index].object = digest;
            objects.insert(
                ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
                bytes,
            );
            let failure = load_artifact(&bundle(manifest, objects)).unwrap_err();
            assert_eq!(failure.code, "artifact_runtime_owner_semantics");
            checked += 1;
        }
    }
    assert!(checked >= 5);
    let mut rejected_faults = BTreeSet::new();
    for name in ["main", "abandoned", "producer", "forward"] {
        let declaration = OwnerKey::Declaration(declaration_named(&snapshot, name));
        let (old, unit) = loaded
            .objects
            .iter()
            .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
            .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
            .find(|(_, u)| u.source.owner == declaration)
            .unwrap();
        for fault in [
            "result",
            "parameter",
            "mode",
            "cleanup",
            "cleanup-omitted",
            "cleanup-reordered",
            "generation",
        ] {
            let mut changed = unit.clone();
            let CompilationPayload::Function { signature, code } = &mut changed.payload else {
                unreachable!()
            };
            match fault {
                "result" => {
                    let original = signature.result;
                    let Some(other) = changed
                        .tables
                        .types
                        .iter()
                        .position(|t| *t != changed.tables.types[original as usize])
                    else {
                        continue;
                    };
                    signature.result = other as u32;
                }
                "mode" => {
                    let Some(i) = code.instructions.iter_mut().find(|i| {
                        matches!(
                            i,
                            CompiledInstruction::LoadLocal {
                                use_mode: ParameterUse::Borrow | ParameterUse::Consume,
                                ..
                            }
                        )
                    }) else {
                        continue;
                    };
                    let CompiledInstruction::LoadLocal { use_mode, .. } = i else {
                        unreachable!()
                    };
                    *use_mode = ParameterUse::Unrestricted;
                }
                "parameter" => {
                    let Some(parameter) = signature
                        .parameters
                        .iter_mut()
                        .find(|p| p.use_mode != ParameterUse::Unrestricted)
                    else {
                        continue;
                    };
                    parameter.use_mode = match parameter.use_mode {
                        ParameterUse::Borrow => ParameterUse::Consume,
                        _ => ParameterUse::Borrow,
                    };
                }
                "cleanup" => {
                    let Some(index) = code.instructions.windows(2).position(|w| {
                        matches!(
                            w,
                            [
                                CompiledInstruction::Unit,
                                CompiledInstruction::StoreLocal(_)
                            ]
                        )
                    }) else {
                        continue;
                    };
                    code.instructions[index + 1] = CompiledInstruction::Drop;
                }
                "cleanup-omitted" => {
                    let Some(index) = code.instructions.windows(2).position(|w| {
                        matches!(
                            w,
                            [
                                CompiledInstruction::Unit,
                                CompiledInstruction::StoreLocal(_)
                            ]
                        )
                    }) else {
                        continue;
                    };
                    code.instructions.drain(index..index + 2);
                }
                "cleanup-reordered" => {
                    let pairs = code
                        .instructions
                        .windows(2)
                        .enumerate()
                        .filter_map(|(i, w)| {
                            matches!(
                                w,
                                [
                                    CompiledInstruction::Unit,
                                    CompiledInstruction::StoreLocal(_)
                                ]
                            )
                            .then_some(i + 1)
                        })
                        .collect::<Vec<_>>();
                    if pairs.len() < 2 {
                        continue;
                    }
                    code.instructions.swap(pairs[0], pairs[1]);
                }
                "generation" => {
                    changed.contract_version = 13;
                    changed.bytecode_contract_version = 9;
                    changed.key = CompilationUnitKey::derive_generation(
                        &changed.source,
                        changed.optimization,
                        13,
                        9,
                        changed.graph_contract_version,
                    )
                    .unwrap();
                    assert_eq!(
                        changed.validate().unwrap_err().code,
                        "compiler_unit_contract"
                    );
                    rejected_faults.insert(fault);
                    continue;
                }
                _ => unreachable!(),
            }
            let (key, bytes) = changed.encode().unwrap();
            let mut manifest = loaded.manifest.clone();
            let mut objects = loaded.objects.clone();
            objects.remove(&old);
            objects.insert(key, bytes);
            for package in &mut manifest.packages {
                let previous = package.compilation;
                let mut compilation =
                    CompilationManifest::decode(&objects[&previous.object_key()], previous)
                        .unwrap();
                let mut entries = artifact_map_entries(&loaded, compilation.units);
                for (owner, value) in &mut entries {
                    let owner = crate::platform::kernel::EncodedOwnerKey::decode(owner).unwrap();
                    let mut binding = CompilationBinding::decode(value, owner).unwrap();
                    if binding.object.object_key() == old {
                        binding.object = CompilerUnitObjectDigest::from_bytes(key.digest.bytes());
                        binding.key = changed.key;
                        *value = binding.encode(owner).unwrap();
                    }
                }
                compilation.units = replace_artifact_map(&mut objects, entries);
                let (digest, bytes) = compilation.encode().unwrap();
                objects.remove(&previous.object_key());
                objects.insert(digest.object_key(), bytes);
                package.compilation = digest;
            }
            let failure = load_artifact(&bundle(manifest, objects)).unwrap_err();
            assert_eq!(
                failure.code,
                if fault == "generation" {
                    "artifact_compiler_unit_binding"
                } else if fault == "result" && matches!(name, "producer" | "main" | "abandoned") {
                    // All annotation roots are now retained. Selecting the first
                    // different result can cross the direct-memory boundary even
                    // for a previously ordinary-result local builder.
                    "artifact_runtime_owner_count"
                } else if fault == "result" && name == "forward" {
                    "artifact_runtime_owner_semantics"
                } else if fault == "result" {
                    "artifact_reference_declaration_payload"
                } else if fault == "parameter" {
                    "artifact_runtime_owner_semantics"
                } else {
                    "artifact_compiled_control_meaning"
                },
                "{name} {fault}: {failure:?}"
            );
            rejected_faults.insert(fault);
        }
    }
    assert_eq!(
        rejected_faults,
        [
            "result",
            "parameter",
            "mode",
            "cleanup",
            "cleanup-omitted",
            "cleanup-reordered",
            "generation"
        ]
        .into_iter()
        .collect()
    );
}
