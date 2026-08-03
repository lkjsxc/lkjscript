#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use super::*;
use lkjscript_core::{
    InlineStructuralValue, LayoutIdentity, OwnedValue, SemanticPayload, SemanticTypeIdentity,
    SemanticValue, StaticStructuralLeaf, StructuralKind, StructuralSnapshotLimits, StructuralType,
};

fn semantic(id: u64, kind: StructuralKind, payload: SemanticPayload) -> SemanticValue {
    SemanticValue::new(
        StructuralType::new(
            LayoutIdentity::new(NonZeroU64::new(id).expect("layout identity")),
            SemanticTypeIdentity::new(NonZeroU64::new(id + 1_000).expect("semantic identity")),
            kind,
        ),
        payload,
    )
}

fn option_string() -> SemanticValue {
    semantic(
        100,
        StructuralKind::Enum,
        SemanticPayload::Enum {
            tag: 1,
            active_payload: vec![semantic(
                101,
                StructuralKind::String,
                SemanticPayload::String("daemon value".as_bytes().to_vec()),
            )]
            .into(),
        },
    )
}

fn result_path_system_error() -> SemanticValue {
    semantic(
        200,
        StructuralKind::Enum,
        SemanticPayload::Enum {
            tag: 0,
            active_payload: vec![semantic(
                201,
                StructuralKind::Path,
                SemanticPayload::Path(b"/var/lib/lkjscript/result".to_vec()),
            )]
            .into(),
        },
    )
}

fn result_bytes_system_error() -> SemanticValue {
    semantic(
        300,
        StructuralKind::Enum,
        SemanticPayload::Enum {
            tag: 0,
            active_payload: vec![semantic(
                301,
                StructuralKind::Bytes,
                SemanticPayload::Bytes(vec![0, 1, 254, 255]),
            )]
            .into(),
        },
    )
}

fn nested_deterministic_aggregate() -> SemanticValue {
    semantic(
        400,
        StructuralKind::Product,
        SemanticPayload::Product(
            vec![
                option_string(),
                result_path_system_error(),
                result_bytes_system_error(),
                semantic(
                    401,
                    StructuralKind::ByteVector,
                    SemanticPayload::ByteVector(vec![9, 8, 7]),
                ),
                semantic(
                    402,
                    StructuralKind::I64,
                    SemanticPayload::Inline(InlineStructuralValue::I64(-1)),
                ),
                semantic(
                    403,
                    StructuralKind::Static,
                    SemanticPayload::Static(StaticStructuralLeaf::Function(11)),
                ),
            ]
            .into(),
        ),
    )
}

fn assert_process_round_trip(value: SemanticValue) {
    let response = ProcessResponse::Outcome {
        provenance: super::tests::provenance(),
        cell: 77,
        outcome: ExecutionOutcome::Returned(
            OwnedValue::from_structural(value, StructuralSnapshotLimits::DEFAULT)
                .expect("owned structural outcome"),
        ),
        output: b"framed daemon output".to_vec(),
        flushes: 2,
    };
    let mut frame = Vec::new();
    write_response(&mut frame, &response).expect("process response encode");
    assert_eq!(
        read_response(&mut frame.as_slice()).expect("process response decode"),
        response
    );
}

#[test]
fn process_protocol_round_trips_structural_daemon_fixtures() {
    for fixture in [
        option_string(),
        result_path_system_error(),
        result_bytes_system_error(),
        nested_deterministic_aggregate(),
    ] {
        assert_process_round_trip(fixture);
    }
}

#[test]
fn process_protocol_rejects_structural_field_bound_before_publication() {
    let product = semantic(
        500,
        StructuralKind::Product,
        SemanticPayload::Product(Vec::new().into()),
    );
    let outcome = ExecutionOutcome::Returned(
        OwnedValue::from_structural(product, StructuralSnapshotLimits::DEFAULT)
            .expect("empty structural product"),
    );
    let mut encoded =
        lkjscript_core::encode_execution_outcome(&outcome, PROCESS_OUTCOME_CODEC_LIMITS)
            .expect("outcome encode");
    encoded[20..24]
        .copy_from_slice(&(lkjscript_core::MAX_STRUCTURAL_SNAPSHOT_FIELDS + 1).to_le_bytes());

    let mut body = Writer::new();
    body.u8(5).expect("outcome tag");
    body.u64(1).expect("cell");
    body.bytes(&encoded).expect("outcome");
    body.bytes(&[]).expect("output");
    body.u64(0).expect("flushes");
    let mut frame = Vec::new();
    write_frame(&mut frame, body.finish()).expect("process frame");
    assert!(read_response(&mut frame.as_slice()).is_err());
}
