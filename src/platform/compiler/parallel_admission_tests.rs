//! Rehashed derived code cannot invent, erase or retarget structured children.
use super::*;
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::kernel::decode_type_object;

const SOURCE: &str = r#"declarations.begin
(units (module create parallel-artifact
  (function create first (visibility private) (effect (task)) (returns I64)
    (body (i64 17)))
  (function create second (visibility private) (effect (task)) (returns I64)
    (body (i64 -29)))
  (function create joined (visibility public) (effect (task))
    (returns (record (left I64) (right I64)))
    (body (parallel (call first) (call second))))))
declarations.end
"#;

const OWNED_SOURCE: &str = r#"declarations.begin
(units (module create owned-parallel-artifact
  (external create empty (visibility private) (implementation core.buffer.empty)
    (returns ByteBuffer))
  (function create child (visibility private) (effect (task))
    (parameter create payload (type ByteBuffer) (use consume))
    (returns ByteBuffer) (body (local payload)))
  (function create scalar (visibility private) (effect (task))
    (returns I64) (body (i64 19)))
  (function create joined (visibility public) (effect (task))
    (returns I64)
    (body (let
      (binding a (type ByteBuffer) (call empty))
      (binding b (type ByteBuffer) (call empty))
      (in (sequence
        (parallel (call child (local a)) (call child (local b)))
        (i64 41))))))))
declarations.end
"#;

fn fixture() -> (LoadedArtifact, ObjectKey, CompilationUnit) {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .unwrap();
    let loaded = artifact_for_source(&source);
    let owner = OwnerKey::Declaration(declaration_named(&source, "joined"));
    let (key, unit) = loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .find(|(_, unit)| unit.source.owner == owner)
        .unwrap();
    (loaded, key, unit)
}

fn artifact_for_source(source: &crate::platform::kernel::KernelSnapshot) -> LoadedArtifact {
    let directory = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&directory.path().join("parallel"), source, None)
        .unwrap()
        .repository;
    let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let artifact = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
    load_artifact(&artifact.artifact.bytes).unwrap()
}

fn parallel_result(
    loaded: &LoadedArtifact,
    source: &crate::platform::kernel::KernelSnapshot,
) -> (ObjectKey, CompilationUnit, TypeObjectDigest) {
    let joined = OwnerKey::Declaration(declaration_named(source, "joined"));
    let (key, unit) = loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .find(|(_, unit)| unit.source.owner == joined)
        .unwrap();
    let CompilationPayload::Function { code, .. } = &unit.payload else {
        panic!("function");
    };
    let result = code
        .instructions
        .iter()
        .find_map(|instruction| match instruction {
            CompiledInstruction::Parallel { result_type, .. } => {
                Some(unit.tables.types[*result_type as usize])
            }
            _ => None,
        })
        .unwrap();
    (key, unit, result)
}

#[test]
fn parallel_inferred_pair_types_are_persisted_even_when_unannotated_and_unused() {
    for (left, right, body, owned) in [
        (
            "ByteBuffer",
            "ByteBuffer",
            "(parallel (call child (local a)) (call child (local b)))",
            true,
        ),
        (
            "ByteBuffer",
            "I64",
            "(parallel (call child (local a)) (call scalar))",
            true,
        ),
        (
            "I64",
            "ByteBuffer",
            "(parallel (call scalar) (call child (local b)))",
            true,
        ),
        (
            "I64",
            "I64",
            "(parallel (call scalar) (call scalar))",
            false,
        ),
    ] {
        let literal = OWNED_SOURCE.replace(
            "(parallel (call child (local a)) (call child (local b)))",
            body,
        );
        let mut source =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(&literal)
                .unwrap();
        let scalar = |name| {
            source
                .types
                .iter()
                .find_map(|(digest, object)| {
                    matches!(
                        (&object.form, name),
                        (TypeForm::ByteBuffer, "ByteBuffer") | (TypeForm::I64, "I64")
                    )
                    .then_some(*digest)
                })
                .unwrap()
        };
        let fields = vec![
            crate::platform::kernel::StructuralTypeField {
                name: Name::new("left").unwrap(),
                ty: scalar(left),
            },
            crate::platform::kernel::StructuralTypeField {
                name: Name::new("right").unwrap(),
                ty: scalar(right),
            },
        ];
        let expected = TypeObject::new(if owned {
            TypeForm::OwnedProduct { fields }
        } else {
            TypeForm::StructuralRecord { fields }
        })
        .unwrap();
        let digest = encode_type_object(&expected).unwrap().0;
        // The inferred pair has no canonical annotation or owner type reference.
        source.types.remove(&digest);
        crate::platform::kernel::validate_full(&source).unwrap();
        let receipt = compile_memory(
            &source,
            OwnerKey::Declaration(declaration_named(&source, "joined")),
        );
        let key = ObjectKey::from_digest(ObjectDomain::Type, digest.bytes());
        let CompilationPayload::Function { code, .. } = &receipt.unit.payload else {
            panic!("function");
        };
        let result_type = code
            .instructions
            .iter()
            .find_map(|instruction| match instruction {
                CompiledInstruction::Parallel { result_type, .. } => Some(*result_type),
                _ => None,
            })
            .unwrap();
        assert_eq!(receipt.unit.tables.types[result_type as usize], digest);
        let directory = tempfile::tempdir().unwrap();
        let repository =
            GraphRepository::create(&directory.path().join("inferred-pair"), &source, None)
                .unwrap()
                .repository;
        let accepted = repository.view_current().unwrap().current().head;
        let compilation =
            build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
        let store = repository.object_store().unwrap();
        assert!(
            store
                .read(
                    key,
                    ObjectDomain::Type.maximum_bytes(),
                    &mut StoreWork::default()
                )
                .unwrap()
                .is_none(),
            "compiler staging must not write inferred type objects into semantic storage"
        );
        let linked = link_artifact(&repository, compilation.manifest_digest, &[]).unwrap();
        assert_eq!(repository.view_current().unwrap().current().head, accepted);
        let loaded = load_artifact(&linked.artifact.bytes).unwrap();
        let (_, _, actual) = parallel_result(&loaded, &source);
        assert_eq!(actual, digest);
        assert_eq!(
            decode_type_object(&loaded.objects[&key], digest).unwrap(),
            expected
        );
        use crate::platform::execution::normalized::{
            NormalizedProgram, NormalizedReferenceSchema,
        };
        NormalizedProgram::prepare(loaded).unwrap();
        let reference = NormalizedReferenceSchema::reconstruct([&source]).unwrap();
        assert_eq!(reference.types.get(&digest), Some(&expected));
    }
}

#[test]
fn parallel_result_operand_rejects_rehashed_wrong_shape_field_type_and_names() {
    for literal in [SOURCE, OWNED_SOURCE] {
        let source =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(literal)
                .unwrap();
        let loaded = artifact_for_source(&source);
        let (key, unit, digest) = parallel_result(&loaded, &source);
        let object = decode_type_object(
            &loaded.objects[&ObjectKey::from_digest(ObjectDomain::Type, digest.bytes())],
            digest,
        )
        .unwrap();
        let (owned, fields) = match object.form {
            TypeForm::OwnedProduct { fields } => (true, fields),
            TypeForm::StructuralRecord { fields } => (false, fields),
            _ => panic!("pair result"),
        };
        let (unit_digest, unit_bytes) =
            encode_type_object(&TypeObject::new(TypeForm::Unit).unwrap()).unwrap();
        let (buffer_digest, buffer_bytes) =
            encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap()).unwrap();
        let i64_type = source
            .types
            .iter()
            .find_map(|(digest, object)| matches!(object.form, TypeForm::I64).then_some(*digest))
            .unwrap();
        for attack in 0..3 {
            let mut forged_fields = fields.clone();
            let mut changed = unit.clone();
            let forged = match attack {
                0 => {
                    // Both forged alternatives are valid closed shapes, so correspondence
                    // is the rejection boundary rather than unrelated type admission.
                    forged_fields[0].ty = if owned { i64_type } else { buffer_digest };
                    forged_fields[1].ty = i64_type;
                    TypeObject::new(if owned {
                        TypeForm::StructuralRecord {
                            fields: forged_fields,
                        }
                    } else {
                        TypeForm::OwnedProduct {
                            fields: forged_fields,
                        }
                    })
                    .unwrap()
                }
                1 => {
                    forged_fields[0].ty = if owned { i64_type } else { unit_digest };
                    TypeObject::new(if owned {
                        TypeForm::OwnedProduct {
                            fields: forged_fields,
                        }
                    } else {
                        TypeForm::StructuralRecord {
                            fields: forged_fields,
                        }
                    })
                    .unwrap()
                }
                _ => {
                    forged_fields[0].name = Name::new("before").unwrap();
                    TypeObject::new(if owned {
                        TypeForm::OwnedProduct {
                            fields: forged_fields,
                        }
                    } else {
                        TypeForm::StructuralRecord {
                            fields: forged_fields,
                        }
                    })
                    .unwrap()
                }
            };
            let (forged_digest, bytes) = encode_type_object(&forged).unwrap();
            changed.tables.types.push(forged_digest);
            let CompilationPayload::Function { code, .. } = &mut changed.payload else {
                panic!("function");
            };
            let instruction = code
                .instructions
                .iter_mut()
                .find(|instruction| matches!(instruction, CompiledInstruction::Parallel { .. }))
                .unwrap();
            let CompiledInstruction::Parallel { result_type, .. } = instruction else {
                unreachable!();
            };
            *result_type = (changed.tables.types.len() - 1) as u32;
            let error = load_artifact(&effect_tests::replace_unit(
                &loaded,
                key,
                &changed,
                vec![
                    (
                        ObjectKey::from_digest(ObjectDomain::Type, forged_digest.bytes()),
                        bytes,
                    ),
                    (
                        ObjectKey::from_digest(ObjectDomain::Type, unit_digest.bytes()),
                        unit_bytes.clone(),
                    ),
                    (
                        ObjectKey::from_digest(ObjectDomain::Type, buffer_digest.bytes()),
                        buffer_bytes.clone(),
                    ),
                ],
            ))
            .expect_err("rehashing cannot replace a parallel result contract");
            assert_eq!(error.code, "artifact_compiled_control_meaning", "{error:?}");
        }
    }
}

#[test]
fn parallel_result_operand_requires_an_existing_type_table_entry() {
    let (loaded, key, mut unit) = fixture();
    let CompilationPayload::Function { code, .. } = &mut unit.payload else {
        panic!("function");
    };
    let CompiledInstruction::Parallel { result_type, .. } = code
        .instructions
        .iter_mut()
        .find(|instruction| matches!(instruction, CompiledInstruction::Parallel { .. }))
        .unwrap()
    else {
        unreachable!();
    };
    *result_type = u32::MAX;
    let error = load_artifact(&effect_tests::replace_rejected_unit(
        &loaded,
        key,
        &unit,
        "compiler_unit_index",
    ))
    .unwrap_err();
    assert_eq!(error.code, "compiler_unit_index", "{error:?}");
}

// Rebind logical source generations without changing canonical owners, compiler
// units or the current artifact generation. The neutral bundle writer does not
// invoke the strict loader, and every enclosing identity is recomputed.
#[test]
fn parallel_artifact_rejects_rehashed_retargeting_and_erasure() {
    let (loaded, key, unit) = fixture();
    load_artifact(&effect_tests::replace_unit(&loaded, key, &unit, vec![])).unwrap();
    for attack in 0..3 {
        let mut changed = unit.clone();
        let CompilationPayload::Function { code, .. } = &mut changed.payload else {
            panic!("function");
        };
        let at = code
            .instructions
            .iter()
            .position(|i| matches!(i, CompiledInstruction::Parallel { .. }))
            .unwrap();
        let CompiledInstruction::Parallel {
            left,
            right,
            left_arguments: 0,
            right_arguments: 0,
            result_type,
            ..
        } = code.instructions[at]
        else {
            panic!("zero-argument pair");
        };
        code.instructions[at] = match attack {
            0 => CompiledInstruction::Parallel {
                left_types: vec![],
                left_implementations: vec![],
                right_types: vec![],
                right_implementations: vec![],
                left_read_scope_arguments: vec![],
                right_read_scope_arguments: vec![],
                left: right,
                right: left,
                left_arguments: 0,
                right_arguments: 0,
                result_type,
            },
            1 => CompiledInstruction::Parallel {
                left_types: vec![],
                left_implementations: vec![],
                right_types: vec![],
                right_implementations: vec![],
                left_read_scope_arguments: vec![],
                right_read_scope_arguments: vec![],
                left,
                right: left,
                left_arguments: 0,
                right_arguments: 0,
                result_type,
            },
            _ => CompiledInstruction::Unit,
        };
        let error = load_artifact(&effect_tests::replace_unit(&loaded, key, &changed, vec![]))
            .expect_err("rehashing cannot change canonical parallel control");
        assert!(
            matches!(
                error.code.as_str(),
                "artifact_compiled_control_meaning" | "artifact_nominal_instruction_meaning"
            ),
            "{error:?}"
        );
    }
}

#[test]
fn parallel_artifact_binds_current_unit_and_manifest_contracts() {
    let (loaded, key, mut unit) = fixture();
    unit.contract_version ^= 1;
    let error =
        load_artifact(&effect_tests::replace_unit(&loaded, key, &unit, vec![])).unwrap_err();
    assert_eq!(error.code, "compiler_unit_contract");
    let mut manifest = loaded.manifest.clone();
    manifest.contract_version ^= 1;
    assert_eq!(
        manifest.encode().unwrap_err().code,
        "artifact_manifest_contract"
    );
}

#[test]
fn parallel_generic_artifact_rejects_rehashed_type_and_implementation_substitution() {
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        &[
            include_str!("../../../tests/fixtures/owned-witness-library.lkjc"),
            include_str!("../../../tests/fixtures/parallel-result-library.lkjc"),
            include_str!("../../../tests/fixtures/parallel-generic-workers.lkjc"),
            include_str!("../../../tests/fixtures/parallel-generic-witness.lkjc"),
        ]
        .join("\n"),
    )
    .unwrap();
    let loaded = artifact_for_source(&source);
    let owner = OwnerKey::Declaration(declaration_named(&source, "owned"));
    let (key, unit) = loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .find(|(_, unit)| unit.source.owner == owner)
        .unwrap();
    load_artifact(&effect_tests::replace_unit(&loaded, key, &unit, vec![])).unwrap();
    for attack in 0..4 {
        let mut changed = unit.clone();
        let CompilationPayload::Function { code, .. } = &mut changed.payload else {
            panic!("function");
        };
        let instruction = code
            .instructions
            .iter_mut()
            .find(|i| matches!(i, CompiledInstruction::Parallel { .. }))
            .unwrap();
        let CompiledInstruction::Parallel {
            left_types,
            left_implementations,
            right_implementations,
            ..
        } = instruction
        else {
            unreachable!()
        };
        match attack {
            0 => left_types.clear(),
            1 => {
                left_types[0] = u32::try_from(
                    changed
                        .tables
                        .types
                        .iter()
                        .position(|ty| *ty != changed.tables.types[left_types[0] as usize])
                        .unwrap(),
                )
                .unwrap();
            }
            2 => *left_implementations = right_implementations.clone(),
            _ => left_implementations.clear(),
        }
        let error = load_artifact(&effect_tests::replace_unit(&loaded, key, &changed, vec![]))
            .expect_err("rehashed application is not canonical meaning");
        assert!(
            matches!(
                error.class,
                DiagnosticClass::Corrupt | DiagnosticClass::Source | DiagnosticClass::Semantic
            ),
            "{attack}: {error:?}"
        );
    }
}

#[path = "transfer_admission_tests.rs"]
mod transfer;
