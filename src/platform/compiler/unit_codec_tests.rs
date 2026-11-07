//! Structural unit codec controls; public artifact/execution closure has separate oracles.
use super::*;
use crate::platform::compiler::unit::{CompilationUnitKey, OptimizationPolicy};

fn unit(instructions: Vec<CompiledInstruction>) -> CompilationUnit {
    let digest = SemanticDigest::from_bytes([1; 32]);
    let source = CompilationSource {
        package: "pkg_10000000000000000000000000000001".parse().unwrap(),
        owner: OwnerKey::Declaration("decl_00000000000000000000000000000001".parse().unwrap()),
        kind: OwnerKind::Constant,
        semantic_interface: digest,
        implementation: digest,
        type_digest: digest,
        effect: digest,
        capability: digest,
        test: None,
        validation_dependencies: digest,
    };
    let optimization = OptimizationPolicy::DeterministicBaseline;
    CompilationUnit {
        contract_version: COMPILER_UNIT_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        bytecode_contract_version: BYTECODE_CONTRACT_VERSION,
        key: CompilationUnitKey::derive(&source, optimization).unwrap(),
        source,
        optimization,
        tables: CompilationTables {
            declarations: vec![],
            fields: vec![],
            cases: vec![],
            requirements: vec![],
            operations: vec![],
            ports: vec![],
            types: vec![TypeObjectDigest::from_bytes([2; 32])],
            structural_names: vec![],
            read_view_fields: vec![],
            texts: vec![],
        },
        payload: CompilationPayload::Constant {
            ty: 0,
            code: CompiledCode {
                parameter_count: 0,
                local_count: 0,
                instructions,
            },
        },
    }
}

#[test]
fn compiler_unit_codec_crosses_former_instruction_stack_item_and_payload_limits() {
    const VALUES: u32 = 1_000_001;
    let mut instructions = vec![CompiledInstruction::I64(i64::MAX); VALUES as usize];
    instructions.extend([
        CompiledInstruction::List {
            item_type: 0,
            items: VALUES,
        },
        CompiledInstruction::Drop,
        CompiledInstruction::I64(7),
        CompiledInstruction::Return,
    ]);
    let mut original = unit(instructions);
    let CompilationPayload::Constant { code, .. } = &mut original.payload else {
        unreachable!()
    };
    code.local_count = VALUES;
    let (key, bytes) = original.encode().expect("wide structural unit encodes");
    assert!(bytes.len() > 8 * 1024 * 1024);
    let decoded = CompilationUnit::decode(&bytes, key).expect("wide structural unit decodes");
    assert!(decoded == original);
    drop(decoded);
    drop(original);
}

#[test]
fn compiler_unit_linear_codec_preserves_canonical_wire_and_semantic_refusals() {
    let original = unit(vec![
        CompiledInstruction::I64(7),
        CompiledInstruction::Return,
    ]);
    let (key, bytes) = original.encode().unwrap();
    let previous_encoding = crate::platform::packed::encode_canonical(
        COMPILER_UNIT_MAGIC,
        COMPILER_UNIT_ENVELOPE_DOMAIN,
        &original,
        4096,
    )
    .unwrap();
    assert_eq!(bytes, previous_encoding);
    assert!(CompilationUnit::decode(&bytes, key).unwrap() == original);
    assert_eq!(
        unit(vec![]).encode().unwrap_err().code,
        "compiler_unit_item_count"
    );
    assert_eq!(
        unit(vec![CompiledInstruction::Drop, CompiledInstruction::Return])
            .encode()
            .unwrap_err()
            .code,
        "compiler_unit_stack_underflow"
    );
    let prefix = (
        original.contract_version,
        original.graph_contract_version,
        original.bytecode_contract_version,
        original.key,
        &original.source,
        original.optimization,
        u64::MAX,
    );
    let malformed = crate::platform::packed::linear::encode(
        COMPILER_UNIT_MAGIC,
        COMPILER_UNIT_ENVELOPE_DOMAIN,
        &prefix,
        None,
    )
    .unwrap();
    let key = ObjectKey::for_bytes(ObjectDomain::CompilerUnit, &malformed);
    let error = CompilationUnit::decode(&malformed, key).unwrap_err();
    assert_eq!(error.code, "packed_decode");
    assert_eq!(error.class, DiagnosticClass::Corrupt);
}

#[derive(Debug)]
struct Probe<T>(T);

impl<Context, T: DecodeLinear> Decode<Context> for Probe<T> {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        T::decode_linear(decoder).map(Self)
    }
}

fn rejects_unframed<T: DecodeLinear + std::fmt::Debug>(prefix: impl Encode) {
    let bytes = bincode::encode_to_vec(prefix, bincode::config::standard()).unwrap();
    let error =
        bincode::decode_from_slice::<Probe<T>, _>(&bytes, bincode::config::standard()).unwrap_err();
    assert!(
        matches!(
            error,
            DecodeError::UnexpectedEnd { .. } | DecodeError::OutsideUsizeRange(_)
        ),
        "{error}"
    );
}

#[test]
fn compiler_unit_nested_decoders_reject_unframed_lengths_progressively() {
    rejects_unframed::<CompilationTables>(u64::MAX);
    rejects_unframed::<CompiledCode>((0_u32, 0_u32, u64::MAX));
    rejects_unframed::<CompiledText>((0_u32, u64::MAX));
    // Function payload, before its first implementation-parameter element.
    rejects_unframed::<CompilationPayload>((7_u32, u64::MAX));
    // Call with empty requirement/effect vectors, then an absent type-argument vector.
    rejects_unframed::<CompiledInstruction>((12_u32, 0_usize, 0_usize, 0_u32, u64::MAX));
    // Record with no nominal type, no type arguments, then absent field selectors.
    rejects_unframed::<CompiledInstruction>((15_u32, None::<u32>, 0_usize, u64::MAX));
}

#[test]
fn compiler_unit_counts_keep_the_dense_representation_boundary() {
    require_item_count("instructions", 1_000_001, false).unwrap();
    require_runtime_count("operands", u32::MAX).unwrap();
    assert_eq!(
        require_item_count("instructions", 0, false)
            .unwrap_err()
            .class,
        DiagnosticClass::Corrupt
    );
    #[cfg(target_pointer_width = "64")]
    {
        require_item_count("instructions", u32::MAX as usize, false).unwrap();
        assert_eq!(
            require_item_count("instructions", u32::MAX as usize + 1, false)
                .unwrap_err()
                .class,
            DiagnosticClass::Resource
        );
    }
}
