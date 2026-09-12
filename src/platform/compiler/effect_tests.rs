//! Rehashed executable containers must retain canonical effect meaning.
use super::*;
use crate::platform::kernel::{EffectParameterRecord, EffectRow, FunctionEffect};
use crate::platform::semantic_id::EffectParameterId;

#[test]
fn exact_effect_predecessor_artifact_rejects_before_preparation() {
    let bytes = include_bytes!("../../../tests/fixtures/graph13-standard.lkja");
    let error = load_artifact(bytes).unwrap_err();
    assert_eq!(error.code, "artifact_bundle_contract", "{error:?}");
}

fn fixture() -> LoadedArtifact {
    let mut snapshot =
        crate::platform::execution::normalized::tests::effect_tests::library_composition();
    let factory = declaration_named(&snapshot, "configure-task");
    let unused = EffectParameterId::migrate(b"effect-artifact-order", 0);
    snapshot.owners.insert(
        OwnerKey::EffectParameter(unused),
        OwnerRecord::EffectParameter(EffectParameterRecord {
            header: OwnerHeader::new(
                OwnerKey::EffectParameter(unused),
                OwnerKind::EffectParameter,
            ),
            declaration: factory,
            name: Name::new("Unused").unwrap(),
        }),
    );
    let OwnerRecord::Declaration(declaration) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(factory))
        .unwrap()
    else {
        unreachable!()
    };
    let crate::platform::kernel::DeclarationPayload::Function(function) = &mut declaration.payload
    else {
        unreachable!()
    };
    function.effect_parameters.push(unused);
    for owner in snapshot.owners.values_mut() {
        if let OwnerRecord::Expression(expression) = owner
            && let ExpressionOperation::Call {
                function,
                effect_arguments,
                ..
            } = &mut expression.operation
            && function.declaration == factory
        {
            effect_arguments.push(EffectRow::default());
        }
    }
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    let temporary = tempfile::tempdir().unwrap();
    let created =
        GraphRepository::create(&temporary.path().join("fixture"), &snapshot, None).unwrap();
    let compiled = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    let linked = link_artifact(&created.repository, compiled.manifest_digest, &[]).unwrap();
    load_artifact(&linked.artifact.bytes).unwrap()
}

fn replace_unit(
    loaded: &LoadedArtifact,
    old: ObjectKey,
    unit: &CompilationUnit,
    extra: Vec<(ObjectKey, Vec<u8>)>,
) -> Vec<u8> {
    // Neutral envelope construction deliberately bypasses the production compiler and writer's
    // admission. Every changed enclosing digest is recomputed before the strict reader runs.
    let bytes = crate::platform::packed::encode(
        super::super::unit::COMPILER_UNIT_MAGIC,
        super::super::unit::COMPILER_UNIT_ENVELOPE_DOMAIN,
        unit,
        super::super::unit::MAXIMUM_COMPILER_UNIT_BYTES,
    )
    .unwrap();
    let key = ObjectKey::for_bytes(ObjectDomain::CompilerUnit, &bytes);
    CompilationUnit::decode(&bytes, key).expect("structurally valid forged unit");
    let mut objects = loaded.objects.clone();
    objects.remove(&old);
    objects.insert(key, bytes);
    objects.extend(extra);
    let mut manifest = loaded.manifest.clone();
    let package = manifest
        .packages
        .iter_mut()
        .find(|package| package.package == unit.source.package)
        .unwrap();
    let old_compilation = package.compilation;
    let mut compilation =
        CompilationManifest::decode(&objects[&old_compilation.object_key()], old_compilation)
            .unwrap();
    let mut entries = artifact_map_entries(loaded, compilation.units);
    let mut count = 0;
    for (owner, bytes) in &mut entries {
        let owner = crate::platform::kernel::EncodedOwnerKey::decode(owner).unwrap();
        let mut binding = CompilationBinding::decode(bytes, owner).unwrap();
        if binding.object.object_key() == old {
            binding.object = CompilerUnitObjectDigest::from_bytes(key.digest.bytes());
            binding.key = unit.key;
            binding.kind = unit.source.kind;
            *bytes = binding.encode(owner).unwrap();
            count += 1;
        }
    }
    assert_eq!(count, 1);
    let mut old_pages = MemoryPageStore::default();
    PersistentMap::from_root(compilation.units)
        .copy_reachable(
            &ObjectPageReader::new(loaded),
            &mut old_pages,
            &mut MapWork::default(),
        )
        .unwrap();
    for (digest, _) in old_pages.objects() {
        objects.remove(&ObjectKey::from_digest(
            ObjectDomain::MapPage,
            digest.bytes(),
        ));
    }
    compilation.units = replace_artifact_map(&mut objects, entries);
    let (digest, bytes) = compilation.encode().unwrap();
    objects.remove(&old_compilation.object_key());
    objects.insert(digest.object_key(), bytes);
    package.compilation = digest;
    let (closure, count, bytes) = super::super::artifact::closure_facts(&objects).unwrap();
    manifest.closure = closure;
    manifest.object_count = count;
    manifest.object_bytes = bytes;
    nominal_session_tests::hostile_bundle(&manifest, &objects)
}

#[test]
fn strict_effect_artifact_rejects_order_erasure_target_and_nested_callable_forgery() {
    let loaded = fixture();
    let units = loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .collect::<Vec<_>>();
    let factory = units.iter().find(|(_, unit)| matches!(&unit.payload, CompilationPayload::Function { signature, .. } if signature.effect_parameters.len() == 2)).unwrap();
    let task = units.iter().find(|(_, unit)| matches!(&unit.payload, CompilationPayload::Function { signature, .. } if unit.source.kind == OwnerKind::TaskFunction && !signature.effect_parameters.is_empty())).unwrap();
    let caller = units.iter().find(|(_, unit)| matches!(&unit.payload, CompilationPayload::Function { code, .. } if code.instructions.iter().any(|i| matches!(i, CompiledInstruction::FunctionValue { .. })))).unwrap();
    for fault in [
        "parameter-order",
        "parameter-ownership",
        "row-erasure",
        "argument-erasure",
        "forged-target",
        "nested-erasure",
        "task-as-pure",
        "terminal-cycle",
    ] {
        let (old, original) = match fault {
            "row-erasure" | "task-as-pure" | "terminal-cycle" => task,
            "argument-erasure" | "forged-target" => caller,
            _ => factory,
        };
        let mut unit = original.clone();
        let mut extra = Vec::new();
        let CompilationPayload::Function { signature, code } = &mut unit.payload else {
            unreachable!()
        };
        match fault {
            "task-as-pure" => {
                signature.effect = FunctionEffect::Pure;
                unit.source.kind = OwnerKind::PureFunction;
                unit.key =
                    super::super::unit::CompilationUnitKey::derive(&unit.source, unit.optimization)
                        .unwrap();
            }
            "terminal-cycle" => {
                assert!(matches!(
                    code.instructions.last(),
                    Some(CompiledInstruction::Return)
                ));
                let end = code.instructions.len() - 1;
                // Both successors are structurally reachable with the same stack shape. The
                // conditional back edge changes canonical terminal control despite valid hashes.
                code.instructions.splice(
                    end..end,
                    [
                        CompiledInstruction::Bool(true),
                        CompiledInstruction::JumpIfFalse((end + 4) as u32),
                        CompiledInstruction::Drop,
                        CompiledInstruction::Jump(0),
                    ],
                );
            }
            "parameter-order" => signature.effect_parameters.swap(0, 1),
            "parameter-ownership" => {
                signature.effect_parameters[0] =
                    EffectParameterId::migrate(b"foreign-effect-owner", 1)
            }
            "row-erasure" => {
                signature.effect = FunctionEffect::Task {
                    requirements: Vec::new(),
                    effect_parameters: Vec::new(),
                }
            }
            "argument-erasure" => {
                let args = code
                    .instructions
                    .iter_mut()
                    .find_map(|i| match i {
                        CompiledInstruction::Call {
                            effect_arguments, ..
                        } if !effect_arguments.is_empty() => Some(effect_arguments),
                        _ => None,
                    })
                    .unwrap();
                args[0] = EffectRow::default();
            }
            "forged-target" => {
                let target = code
                    .instructions
                    .iter_mut()
                    .find_map(|i| match i {
                        CompiledInstruction::FunctionValue { function, .. } => Some(function),
                        _ => None,
                    })
                    .unwrap();
                *target = (*target + 1) % unit.tables.declarations.len() as u32;
            }
            "nested-erasure" => {
                let old_result = unit.tables.types[signature.result as usize];
                let mut result = crate::platform::kernel::decode_type_object(
                    &loaded.objects
                        [&ObjectKey::from_digest(ObjectDomain::Type, old_result.bytes())],
                    old_result,
                )
                .unwrap();
                let TypeForm::Applied { arguments, .. } = &mut result.form else {
                    unreachable!()
                };
                let old_task = arguments[0];
                let mut task = crate::platform::kernel::decode_type_object(
                    &loaded.objects[&ObjectKey::from_digest(ObjectDomain::Type, old_task.bytes())],
                    old_task,
                )
                .unwrap();
                let TypeForm::TaskFunction { effect, .. } = &mut task.form else {
                    unreachable!()
                };
                *effect = EffectRow::default();
                let (digest, bytes) = encode_type_object(&task).unwrap();
                extra.push((
                    ObjectKey::from_digest(ObjectDomain::Type, digest.bytes()),
                    bytes,
                ));
                arguments[0] = digest;
                let (digest, bytes) = encode_type_object(&result).unwrap();
                extra.push((
                    ObjectKey::from_digest(ObjectDomain::Type, digest.bytes()),
                    bytes,
                ));
                unit.tables.types[signature.result as usize] = digest;
            }
            _ => unreachable!(),
        }
        let bytes = replace_unit(&loaded, *old, &unit, extra);
        let failure = load_artifact(&bytes).unwrap_err();
        assert_eq!(
            failure.class,
            crate::platform::DiagnosticClass::Corrupt,
            "{fault}: {failure:?}"
        );
        let expected = match fault {
            "parameter-ownership" => "artifact_runtime_owner_unexpected",
            "argument-erasure" | "forged-target" => "artifact_nominal_instruction_meaning",
            "terminal-cycle" => "artifact_compiled_control_meaning",
            _ => "artifact_reference_declaration_payload",
        };
        assert_eq!(failure.code, expected, "{fault}: {failure:?}");
        println!("effect-artifact-negative {fault} {}", failure.code);
    }

    let (old, original) = units.iter().find(|(_, unit)| matches!(&unit.payload, CompilationPayload::Component { ports, .. } if !ports.is_empty())).unwrap();
    let mut unit = original.clone();
    let CompilationPayload::Component { ports, .. } = &mut unit.payload else {
        unreachable!()
    };
    let old_type = unit.tables.types[ports[0].function_type as usize];
    let task_type = crate::platform::kernel::decode_type_object(
        &loaded.objects[&ObjectKey::from_digest(ObjectDomain::Type, old_type.bytes())],
        old_type,
    )
    .unwrap();
    let TypeForm::TaskFunction {
        parameters, result, ..
    } = task_type.form
    else {
        panic!("entry must use an explicit task callable");
    };
    let pure_type = TypeObject::new(TypeForm::Function { parameters, result }).unwrap();
    let (digest, bytes) = encode_type_object(&pure_type).unwrap();
    unit.tables.types[ports[0].function_type as usize] = digest;
    let bytes = replace_unit(
        &loaded,
        *old,
        &unit,
        vec![(
            ObjectKey::from_digest(ObjectDomain::Type, digest.bytes()),
            bytes,
        )],
    );
    let error = load_artifact(&bytes).unwrap_err();
    assert_eq!(error.code, "artifact_runtime_owner_semantics");
    println!("effect-artifact-negative task-as-pure-port {}", error.code);
}

#[test]
fn strict_rehashed_artifact_cannot_delete_a_pending_transaction_commit() {
    let (snapshot, _) =
        crate::platform::execution::normalized::tests::iteration_transaction_tests::fixture(false);
    let temporary = tempfile::tempdir().unwrap();
    let created =
        GraphRepository::create(&temporary.path().join("transaction"), &snapshot, None).unwrap();
    let compiled = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    let linked = link_artifact(&created.repository, compiled.manifest_digest, &[]).unwrap();
    let loaded = load_artifact(&linked.artifact.bytes).unwrap();
    let (old, mut unit) = loaded.objects.iter().filter(|(key,_)|key.domain==ObjectDomain::CompilerUnit)
        .map(|(key,bytes)|(*key,CompilationUnit::decode(bytes,*key).unwrap()))
        .find(|(_,unit)|matches!(&unit.payload,CompilationPayload::Function{code,..} if code.instructions.iter().any(|instruction|matches!(instruction,CompiledInstruction::CommitTransaction{..})))).unwrap();
    let CompilationPayload::Function { code, .. } = &mut unit.payload else {
        unreachable!()
    };
    let before = code.instructions.len();
    code.instructions.retain(|instruction| {
        !matches!(instruction, CompiledInstruction::CommitTransaction { .. })
    });
    assert_eq!(code.instructions.len() + 1, before);
    let bytes = replace_unit(&loaded, old, &unit, vec![]);
    let error = load_artifact(&bytes).unwrap_err();
    assert_eq!(error.code, "artifact_compiled_control_meaning", "{error:?}");
}
