#![allow(clippy::expect_used)]

use super::*;

#[test]
fn structural_codec_round_trips_every_semantic_payload_without_keys() {
    let nested = value(
        1,
        StructuralKind::Product,
        SemanticPayload::Product(
            vec![
                value(
                    2,
                    StructuralKind::Unit,
                    SemanticPayload::Inline(InlineStructuralValue::Unit),
                ),
                value(
                    3,
                    StructuralKind::Bool,
                    SemanticPayload::Inline(InlineStructuralValue::Bool(true)),
                ),
                value(
                    4,
                    StructuralKind::I64,
                    SemanticPayload::Inline(InlineStructuralValue::I64(i64::MIN)),
                ),
                value(
                    5,
                    StructuralKind::F64,
                    SemanticPayload::Inline(InlineStructuralValue::F64Bits(u64::MAX)),
                ),
                value(
                    6,
                    StructuralKind::Static,
                    SemanticPayload::Static(StaticStructuralLeaf::Function(7)),
                ),
                value(
                    60,
                    StructuralKind::Static,
                    SemanticPayload::Static(StaticStructuralLeaf::Symbol(8)),
                ),
                value(
                    7,
                    StructuralKind::String,
                    SemanticPayload::String("semantic".as_bytes().to_vec()),
                ),
                value(
                    8,
                    StructuralKind::Path,
                    SemanticPayload::Path(b"/tmp/result".to_vec()),
                ),
                value(
                    9,
                    StructuralKind::Bytes,
                    SemanticPayload::Bytes(vec![0, 255]),
                ),
                value(
                    10,
                    StructuralKind::ByteVector,
                    SemanticPayload::ByteVector(vec![1, 2, 3]),
                ),
                value(
                    11,
                    StructuralKind::Enum,
                    SemanticPayload::Enum {
                        tag: 4,
                        active_payload: vec![value(
                            12,
                            StructuralKind::Static,
                            SemanticPayload::Static(StaticStructuralLeaf::Bytes(9)),
                        )]
                        .into(),
                    },
                ),
            ]
            .into(),
        ),
    );
    let owned = OwnedValue::from_structural(nested.clone()).expect("owned structural value");
    let metrics = owned
        .structural_snapshot_metrics()
        .expect("structural metrics");
    assert_eq!(metrics.nodes, 13);
    assert_eq!(metrics.fields, 12);
    assert_eq!(metrics.aggregate_bytes, 24);
    assert_eq!(metrics.string_bytes, 8);
    assert_eq!(metrics.path_bytes, 11);
    assert_eq!(metrics.encode_work, 49);
    assert_eq!(metrics.decode_work, 49);
    assert_eq!(owned.as_structural(), Some(&nested));
    let mut resolved = Vec::new();
    let owned = owned
        .retain_symbols(|index| {
            resolved.push(index);
            assert_eq!(index, 8);
            Ok("semantic-symbol")
        })
        .expect("retained structural symbols");
    assert_eq!(resolved, vec![8]);

    let outcome = ExecutionOutcome::Returned(owned);
    let bytes = encode_execution_outcome(&outcome, 2 * 1024 * 1024).expect("encode outcome");
    let decoded = decode_execution_outcome(&bytes, 2 * 1024 * 1024).expect("decode outcome");
    assert_eq!(decoded, outcome);
}

#[test]
fn structural_codec_round_trips_deep_values_without_native_recursion() {
    const DEPTH: usize = 12_000;
    let product_type = structural_type(70, StructuralKind::Product);
    let integer_type = structural_type(71, StructuralKind::I64);
    let mut nested = SemanticValue::new(
        integer_type,
        SemanticPayload::Inline(InlineStructuralValue::I64(7)),
    );
    for _ in 0..DEPTH {
        nested = SemanticValue::new(product_type, SemanticPayload::Product(vec![nested].into()));
    }
    let outcome = ExecutionOutcome::Returned(
        OwnedValue::from_structural(nested).expect("deep owned structural value"),
    );
    let bytes = encode_execution_outcome(&outcome, 4 * 1024 * 1024).expect("deep encode");
    let decoded = decode_execution_outcome(&bytes, 4 * 1024 * 1024).expect("deep decode");
    let metrics = decoded
        .returned()
        .and_then(OwnedValue::structural_snapshot_metrics)
        .expect("deep structural metrics");
    assert_eq!(
        metrics.nodes,
        u64::try_from(DEPTH + 1).expect("test depth fits u64")
    );
    assert_eq!(
        metrics.fields,
        u64::try_from(DEPTH).expect("test depth fits u64")
    );
}
