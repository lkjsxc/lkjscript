//! Binary64 wire generations and independent admission of transported compiled meaning.

use super::*;
use crate::platform::binary64::Binary64;
use crate::platform::kernel::DeclarationPayload;

fn literal_snapshot(value: Binary64) -> (crate::platform::kernel::KernelSnapshot, OwnerKey) {
    let mut snapshot = crate::platform::kernel::tests::transport_snapshot();
    let owner = OwnerKey::Declaration(declaration_named(&snapshot, "unit_constant"));
    let object = TypeObject::new(TypeForm::F64).unwrap();
    let (digest, _) = encode_type_object(&object).unwrap();
    snapshot.types.insert(digest, object);
    let OwnerRecord::Declaration(record) = snapshot.owners.get_mut(&owner).unwrap() else {
        panic!("constant declaration");
    };
    let DeclarationPayload::Constant { ty, value: body } = &mut record.payload else {
        panic!("constant payload");
    };
    *ty = digest;
    let expression = *body;
    let OwnerRecord::Expression(record) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(expression))
        .unwrap()
    else {
        panic!("constant expression");
    };
    record.operation = ExpressionOperation::F64 { value };
    (snapshot, owner)
}

fn literal_artifact(value: Binary64) -> (LoadedArtifact, OwnerKey) {
    let (snapshot, owner) = literal_snapshot(value);
    let directory = tempfile::tempdir().unwrap();
    let created =
        GraphRepository::create(&directory.path().join("numeric"), &snapshot, None).unwrap();
    let compilation = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    let linked = link_artifact(&created.repository, compilation.manifest_digest, &[]).unwrap();
    (load_artifact(&linked.artifact.bytes).unwrap(), owner)
}

#[test]
fn f64_instruction_has_one_fixed_little_endian_payload_and_strict_nan_admission() {
    let value = Binary64::from_bits(0x0123_4567_89ab_cdef).unwrap();
    // Bytecode 11 assigns tag 33 after its two static-witness operands. The payload is exactly eight little-endian bytes, even under
    // a caller-selected big-endian bincode configuration; host integer encoding is irrelevant.
    let expected = [33, 0xef, 0xcd, 0xab, 0x89, 0x67, 0x45, 0x23, 0x01];
    let instruction = CompiledInstruction::F64(value);
    assert_eq!(
        bincode::encode_to_vec(&instruction, bincode::config::standard()).unwrap(),
        expected
    );
    assert_eq!(
        bincode::encode_to_vec(&instruction, bincode::config::standard().with_big_endian())
            .unwrap(),
        expected
    );
    let (decoded, used): (CompiledInstruction, _) =
        bincode::decode_from_slice(&expected, bincode::config::standard()).unwrap();
    assert_eq!(used, 9);
    assert_eq!(decoded, instruction);

    for malformed in [
        [33, 1, 0, 0, 0, 0, 0, 0xf8, 0x7f], // quiet NaN payload
        [33, 1, 0, 0, 0, 0, 0, 0xf0, 0x7f], // signaling NaN
        [33, 0, 0, 0, 0, 0, 0, 0xf8, 0xff], // negative NaN
    ] {
        assert!(
            bincode::decode_from_slice::<CompiledInstruction, _>(
                &malformed,
                bincode::config::standard()
            )
            .is_err()
        );
    }
    for length in 1..9 {
        assert!(
            bincode::decode_from_slice::<CompiledInstruction, _>(
                &expected[..length],
                bincode::config::standard()
            )
            .is_err()
        );
    }
}

#[test]
fn f64_literals_lower_and_round_trip_all_scalar_classes() {
    for bits in [
        0,
        0x8000_0000_0000_0000,
        1,
        0x0010_0000_0000_0000,
        0x3ff0_0000_0000_0001,
        0x7fef_ffff_ffff_ffff,
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
    ] {
        let value = Binary64::from_bits(bits).unwrap();
        let (snapshot, owner) = literal_snapshot(value);
        let compiled = compile_memory(&snapshot, owner);
        assert_eq!(
            (
                compiled.unit.contract_version,
                compiled.unit.bytecode_contract_version,
                compiled.unit.graph_contract_version
            ),
            (15, 11, 18)
        );
        assert_eq!(&compiled.bytes[..8], b"LKJCUN15");
        let CompilationPayload::Constant { code, .. } = &compiled.unit.payload else {
            panic!("compiled constant");
        };
        assert_eq!(
            code.instructions,
            vec![CompiledInstruction::F64(value), CompiledInstruction::Return]
        );
        assert_eq!(
            CompilationUnit::decode(&compiled.bytes, compiled.object).unwrap(),
            compiled.unit
        );
    }
}

#[test]
fn f64_predecessor_instructions_reject_even_when_unreachable_and_rehashed() {
    let (snapshot, owner) = literal_snapshot(Binary64::from_bits(0).unwrap());
    let original = compile_memory(&snapshot, owner).unit;
    for (compiler, bytecode, graph, magic, domain) in [
        (
            11,
            7,
            15,
            *b"LKJCUN11",
            "lkjscript.compiler-unit-envelope.v11",
        ),
        (
            12,
            8,
            16,
            *b"LKJCUN12",
            "lkjscript.compiler-unit-envelope.v12",
        ),
    ] {
        for unreachable in [false, true] {
            let mut unit = original.clone();
            unit.contract_version = compiler;
            unit.bytecode_contract_version = bytecode;
            unit.graph_contract_version = graph;
            unit.key = CompilationUnitKey::derive_generation(
                &unit.source,
                unit.optimization,
                compiler,
                bytecode,
                graph,
            )
            .unwrap();
            if unreachable {
                let CompilationPayload::Constant { code, .. } = &mut unit.payload else {
                    panic!("constant");
                };
                code.instructions = vec![
                    CompiledInstruction::Unit,
                    CompiledInstruction::Jump(3),
                    CompiledInstruction::F64(Binary64::from_bits(0).unwrap()),
                    CompiledInstruction::Return,
                ];
            }
            assert_eq!(unit.encode().unwrap_err().code, "compiler_unit_contract");
            let bytes = crate::platform::packed::encode(
                magic,
                domain,
                &unit,
                super::super::unit::MAXIMUM_COMPILER_UNIT_BYTES,
            )
            .unwrap();
            let key = ObjectKey::for_bytes(ObjectDomain::CompilerUnit, &bytes);
            assert_eq!(
                CompilationUnit::decode(&bytes, key).unwrap_err().code,
                "compiler_unit_contract"
            );
        }
    }
    assert!(
        super::super::wire10::CompilationUnit10::try_from(original).is_err(),
        "the authentic compiler-unit 10 wire adapter must reject F64"
    );
}

#[test]
fn f64_rehashed_unit_rejects_noncanonical_nan_payload() {
    let (snapshot, owner) = literal_snapshot(Binary64::from_bits(0x7ff8_0000_0000_0000).unwrap());
    let receipt = compile_memory(&snapshot, owner);
    let mut bytes = receipt.bytes;
    let literal = [33, 0, 0, 0, 0, 0, 0, 0xf8, 0x7f];
    let positions = bytes
        .windows(literal.len())
        .enumerate()
        .filter_map(|(index, actual)| (actual == literal).then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(positions.len(), 1);
    bytes[positions[0] + 1] = 1;
    let end = bytes.len() - 32;
    let mut checksum =
        blake3::Hasher::new_derive_key(super::super::unit::COMPILER_UNIT_ENVELOPE_DOMAIN);
    checksum.update(&(end as u64).to_be_bytes());
    checksum.update(&bytes[..end]);
    bytes[end..].copy_from_slice(checksum.finalize().as_bytes());
    let key = ObjectKey::for_bytes(ObjectDomain::CompilerUnit, &bytes);
    assert_eq!(
        CompilationUnit::decode(&bytes, key).unwrap_err().code,
        "packed_decode"
    );
}

#[test]
fn owned_generation_requires_rebuilding_authentic_predecessor_artifacts() {
    for (version, compiler, bytes) in [
        (
            18,
            10,
            include_bytes!("../../../tests/fixtures/graph14-before-task-iteration.lkja").as_slice(),
        ),
        (
            19,
            11,
            include_bytes!(
                "../../../tests/fixtures/transaction-outcome-predecessor/predecessor.lkja"
            )
            .as_slice(),
        ),
        (
            20,
            12,
            include_bytes!("../../../tests/fixtures/graph16-standard.lkja").as_slice(),
        ),
    ] {
        assert_eq!(
            load_artifact(bytes).unwrap_err().code,
            "compiler_unit_contract",
            "artifact {version} / compiler {compiler} is a derived predecessor, not current permission"
        );
    }
}

#[test]
fn f64_artifact_admission_preserves_literal_observation_bits() {
    let (loaded, owner) = literal_artifact(Binary64::from_bits(0).unwrap());
    assert_eq!(loaded.manifest.contract_version, 22);
    let (old, original) = loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .find(|(_, unit)| unit.source.owner == owner)
        .unwrap();
    for bits in [0x8000_0000_0000_0000, 0x7ff8_0000_0000_0000] {
        let mut unit = original.clone();
        let CompilationPayload::Constant { code, .. } = &mut unit.payload else {
            panic!("constant");
        };
        code.instructions[0] = CompiledInstruction::F64(Binary64::from_bits(bits).unwrap());
        let bytes = effect_tests::replace_unit(&loaded, old, &unit, vec![]);
        assert_eq!(
            load_artifact(&bytes).unwrap_err().code,
            "artifact_compiled_control_meaning",
            "numerical zero equality cannot erase literal sign from accepted meaning"
        );
    }
}

#[test]
fn f64_hidden_in_an_unused_predecessor_type_closure_rejects_under_old_or_new_outer_artifact() {
    // Keep the old carrier case while constructing the attack from a current admitted fixture.
    // The declared rebuild cut rejects every old unit, including an unused new scalar closure.
    let (original, _) = literal_artifact(Binary64::from_bits(0).unwrap());
    let (old, original_unit) = original
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .next()
        .unwrap();
    for current_outer in [false, true] {
        let mut loaded = original.clone();
        if !current_outer {
            loaded.manifest.contract_version = 20;
            loaded.manifest.graph_contract_version = 16;
            loaded.manifest.compiler_contract_version = 12;
            loaded.manifest.bytecode_contract_version = 8;
        }
        let (float, float_bytes) =
            encode_type_object(&TypeObject::new(TypeForm::F64).unwrap()).unwrap();
        let (list, list_bytes) =
            encode_type_object(&TypeObject::new(TypeForm::List { item: float }).unwrap()).unwrap();
        let mut unit = original_unit.clone();
        unit.tables.types.push(list);
        unit.contract_version = 11;
        unit.graph_contract_version = 15;
        unit.bytecode_contract_version = 7;
        unit.key =
            CompilationUnitKey::derive_generation(&unit.source, unit.optimization, 11, 7, 15)
                .unwrap();
        let bytes = effect_tests::replace_unit(
            &loaded,
            old,
            &unit,
            vec![
                (
                    ObjectKey::from_digest(ObjectDomain::Type, float.bytes()),
                    float_bytes,
                ),
                (
                    ObjectKey::from_digest(ObjectDomain::Type, list.bytes()),
                    list_bytes,
                ),
            ],
        );
        assert_eq!(
            load_artifact(&bytes).unwrap_err().code,
            if current_outer {
                "compiler_unit_contract"
            } else {
                "artifact_package_compilation_binding"
            },
            "neither an old outer envelope with current compilation nor a current envelope with an old unit grants execution"
        );
    }
}
