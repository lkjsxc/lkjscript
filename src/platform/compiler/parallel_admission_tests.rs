//! Rehashed derived code cannot invent, erase or retarget structured children.
use super::*;
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::kernel::decode_type_object;
use bincode::{Encode, enc::Encoder, error::EncodeError};

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
fn rehash_logical_generation(
    loaded: &LoadedArtifact,
    package_generation: u16,
    source_generation: u16,
) -> Vec<u8> {
    use crate::platform::package_transport::PackageRevision;

    let mut objects = loaded.objects.clone();
    let mut manifest = loaded.manifest.clone();
    assert_eq!(manifest.packages.len(), 1);
    let package = &mut manifest.packages[0];
    let old_revision = ObjectKey::from_digest(
        ObjectDomain::PackageRevision,
        package.package_revision.bytes(),
    );
    let mut revision =
        PackageRevision::decode(&objects[&old_revision], package.package_revision).unwrap();
    assert!(revision.dependencies.is_empty());
    revision.graph_contract_version = package_generation;
    revision.revision.graph_contract_version = source_generation;
    revision.interface = crate::platform::package_interface::package_interface_digest_for_graph(
        package.package,
        package.interface_owners.content_root(),
        package_generation,
    )
    .unwrap();
    let (revision_digest, revision_bytes) = revision.encode().unwrap();
    objects.remove(&old_revision);
    objects.insert(
        ObjectKey::from_digest(ObjectDomain::PackageRevision, revision_digest.bytes()),
        revision_bytes,
    );
    let old_compilation = package.compilation;
    let mut compilation =
        CompilationManifest::decode(&objects[&old_compilation.object_key()], old_compilation)
            .unwrap();
    compilation.revision = revision.revision.revision_id().unwrap();
    compilation.package_revision = revision_digest;
    compilation.package_interface = revision.interface;
    let (compilation_digest, compilation_bytes) = compilation.encode().unwrap();
    objects.remove(&old_compilation.object_key());
    objects.insert(compilation_digest.object_key(), compilation_bytes);
    package.package_revision = revision_digest;
    package.semantic_revision = compilation.revision;
    package.interface = revision.interface;
    package.compilation = compilation_digest;
    let (closure, count, bytes) = super::super::artifact::closure_facts(&objects).unwrap();
    manifest.closure = closure;
    manifest.object_count = count;
    manifest.object_bytes = bytes;
    nominal_session_tests::hostile_bundle(&manifest, &objects)
}

#[test]
fn parallel_artifact_rejects_rehashed_predecessor_logical_source() {
    let mut source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .unwrap();
    // Only Parallel needs Graph 21. Unchanged surrounding owners retain supported
    // Graph 20 encodings, so the attack cannot fail on an unrelated newer owner.
    for owner in source.owners.values_mut() {
        if !matches!(owner, OwnerRecord::Expression(expression)
            if matches!(expression.operation, ExpressionOperation::Parallel { .. }))
        {
            owner.set_encoding_for_edit(20);
        }
    }
    crate::platform::kernel::validate_full(&source).unwrap();
    let loaded = artifact_for_source(&source);
    load_artifact(&rehash_logical_generation(&loaded, 21, 21)).unwrap();
    source.root.graph_contract_version = 20;
    assert!(
        crate::platform::kernel::validate_full(&source)
            .unwrap_err()
            .iter()
            .any(|error| error.code == "kernel_owner_graph_generation")
    );
    assert!(!crate::platform::kernel::memory_reference::accepts(&source));
    for package_generation in [20, 21] {
        let error = load_artifact(&rehash_logical_generation(&loaded, package_generation, 20))
            .map(|_| ())
            .expect_err("current derived contracts cannot authorize newer canonical meaning");
        assert_eq!(error.code, "kernel_owner_graph_generation", "{error:?}");
    }
}

#[test]
fn parallel_successor_rebuilds_supported_graph_20_meaning() {
    let input = SOURCE.replace(
        "(parallel (call first) (call second))",
        "(record structural (field left (call first)) (field right (call second)))",
    );
    let mut source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(&input)
            .unwrap();
    source.root.graph_contract_version = 20;
    for owner in source.owners.values_mut() {
        owner.set_encoding_for_edit(20);
    }
    crate::platform::kernel::validate_full(&source).unwrap();
    let loaded = artifact_for_source(&source);
    assert_eq!(loaded.manifest.graph_contract_version, 21);
    // The transport producer can also remain at Graph 20 around old source.
    load_artifact(&rehash_logical_generation(&loaded, 20, 20)).unwrap();
}

#[test]
fn parallel_artifact_logical_source_generation_also_bounds_signature_types() {
    for (ty, use_mode, generation, expected) in [
        ("F64", "", 16, "artifact_f64_graph_generation"),
        (
            "(owned-product (field data ByteBuffer))",
            "(use consume)",
            18,
            "kernel_product_generation",
        ),
        (
            "(owned-choice (case data ByteBuffer))",
            "(use consume)",
            19,
            "kernel_choice_generation",
        ),
    ] {
        // A private, uncalled Local-only body has no new operation or exported
        // signature to provide an accidental substitute for complete type closure.
        let input = format!(
            "declarations.begin\n(units (module create signatures
              (function create relay (visibility private) (effect pure)
                (parameter create value (type {ty}) {use_mode})
                (returns {ty}) (body (local value)))))\ndeclarations.end"
        );
        let source =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(&input)
                .unwrap();
        let loaded = artifact_for_source(&source);
        load_artifact(&rehash_logical_generation(&loaded, 21, 21)).unwrap();
        let error = load_artifact(&rehash_logical_generation(&loaded, 21, generation))
            .map(|_| ())
            .expect_err("current package/derived envelopes cannot upgrade source type permission");
        assert_eq!(error.code, expected, "{error:?}");
        assert_eq!(
            error.class,
            if generation == 16 {
                DiagnosticClass::Corrupt
            } else {
                DiagnosticClass::Semantic
            },
        );
    }
}

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
fn parallel_artifact_rejects_predecessor_unit_and_mislabeled_manifest() {
    let (loaded, key, mut unit) = fixture();
    unit.contract_version = 18;
    unit.bytecode_contract_version = 14;
    unit.graph_contract_version = 20;
    unit.key =
        CompilationUnitKey::derive_generation(&unit.source, unit.optimization, 18, 14, 20).unwrap();
    let error =
        load_artifact(&effect_tests::replace_unit(&loaded, key, &unit, vec![])).unwrap_err();
    assert_eq!(error.code, "compiler_unit_contract");
    let mut manifest = loaded.manifest.clone();
    manifest.contract_version = 25;
    assert_eq!(
        manifest.encode().unwrap_err().code,
        "artifact_manifest_contract"
    );
    manifest.contract_version = 26;
    assert_eq!(
        manifest.encode().unwrap_err().code,
        "artifact_manifest_contract"
    );
}

// Frozen v19 function-unit encoding. The two enums retain their historical ordinals;
// Parallel contains exactly four u32 operands and has no successor result operand.
#[derive(Encode)]
struct FunctionUnit19<'a> {
    contract_version: u16,
    graph_contract_version: u16,
    bytecode_contract_version: u16,
    key: CompilationUnitKey,
    source: &'a CompilationSource,
    optimization: OptimizationPolicy,
    tables: &'a super::super::unit::CompilationTables,
    payload: Function19<'a>,
}
struct Function19<'a> {
    signature: &'a super::super::unit::CompiledSignature,
    code: Code19<'a>,
}
impl Encode for Function19<'_> {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        6_u32.encode(encoder)?; // Function payload in compiler-unit 19.
        self.signature.encode(encoder)?;
        self.code.encode(encoder)
    }
}
#[derive(Encode)]
struct Code19<'a> {
    parameter_count: u32,
    local_count: u32,
    instructions: Vec<Instruction19<'a>>,
}
struct Instruction19<'a>(&'a CompiledInstruction);
impl Encode for Instruction19<'_> {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
        if let CompiledInstruction::Parallel {
            left,
            left_arguments,
            right,
            right_arguments,
            ..
        } = self.0
        {
            38_u32.encode(encoder)?; // Parallel ordinal in bytecode 15.
            left.encode(encoder)?;
            left_arguments.encode(encoder)?;
            right.encode(encoder)?;
            right_arguments.encode(encoder)
        } else {
            assert!(
                matches!(self.0, CompiledInstruction::Return),
                "frozen fixture contains another instruction"
            );
            27_u32.encode(encoder) // Return ordinal in bytecode 15.
        }
    }
}

#[test]
fn parallel_genuine_predecessor_wire_requires_rebuild_even_in_rehashed_current_artifact() {
    let (loaded, key, mut unit) = fixture();
    unit.contract_version = 19;
    unit.bytecode_contract_version = 15;
    unit.key =
        CompilationUnitKey::derive_generation(&unit.source, unit.optimization, 19, 15, 21).unwrap();
    let CompilationPayload::Function { signature, code } = &unit.payload else {
        panic!("function");
    };
    let predecessor = FunctionUnit19 {
        contract_version: 19,
        graph_contract_version: 21,
        bytecode_contract_version: 15,
        key: unit.key,
        source: &unit.source,
        optimization: unit.optimization,
        tables: &unit.tables,
        payload: Function19 {
            signature,
            code: Code19 {
                parameter_count: code.parameter_count,
                local_count: code.local_count,
                instructions: code.instructions.iter().map(Instruction19).collect(),
            },
        },
    };
    let bytes = crate::platform::packed::encode(
        *b"LKJCUN19",
        "lkjscript.compiler-unit-envelope.v19",
        &predecessor,
        super::super::unit::MAXIMUM_COMPILER_UNIT_BYTES,
    )
    .unwrap();
    let predecessor_key = ObjectKey::for_bytes(ObjectDomain::CompilerUnit, &bytes);
    assert_eq!(
        CompilationUnit::decode(&bytes, predecessor_key)
            .unwrap_err()
            .code,
        "compiler_unit_contract"
    );
    let error = load_artifact(&effect_tests::replace_unit_encoded(
        &loaded,
        key,
        &unit,
        bytes,
        vec![],
    ))
    .unwrap_err();
    assert_eq!(error.code, "compiler_unit_contract", "{error:?}");
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

#[path = "parallel_predecessor20_tests.rs"]
mod predecessor20;
