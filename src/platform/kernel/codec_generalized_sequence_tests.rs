//! Independent envelopes retain predecessor meaning without widened eligibility.
use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::ExpressionId;

#[test]
fn indexed_sequence_operations_require_graph31_in_independently_encoded_envelopes() {
    let seed = b"generalized-sequence-codec";
    let item = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
        .unwrap()
        .0;
    let sequence_type =
        encode_type_object(&TypeObject::new(TypeForm::OwnedSequence { item }).unwrap())
            .unwrap()
            .0;
    let id = ExpressionId::migrate(seed, 0);
    let index = ExpressionId::migrate(seed, 1);
    let source = ExpressionId::migrate(seed, 2);
    let value = ExpressionId::migrate(seed, 3);
    for operation in [
        ExpressionOperation::SequenceGet {
            sequence_type,
            source,
            index,
        },
        ExpressionOperation::SequenceReplace {
            sequence_type,
            result_type: sequence_type,
            index,
            value,
            source,
        },
    ] {
        let record = OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap());
        let (digest, bytes) = encode_owner(&record).unwrap();
        assert_eq!(&bytes[..8], b"LKJOWN31");
        assert_eq!(
            decode_owner(&bytes, record.owner(), record.kind(), digest).unwrap(),
            record
        );
        for generation in 14..=30 {
            let mut predecessor = record.clone();
            predecessor.set_encoding_for_edit(generation);
            let error = encode_owner(&predecessor).unwrap_err();
            assert_eq!(error.code, "kernel_sequence_generation");
            assert!(error.message.contains("Graph 31"));
            // Bypass the production converter. An appended tag in every old
            // producer envelope must fail the independent hostile decoder.
            let magic: [u8; 8] = format!("LKJOWN{generation}").as_bytes().try_into().unwrap();
            let domain = format!("lkjscript.kernel.owner-envelope.v{generation}");
            let raw =
                packed::encode(magic, &domain, &predecessor, MAXIMUM_OWNER_OBJECT_BYTES).unwrap();
            assert!(
                decode_owner(
                    &raw,
                    predecessor.owner(),
                    predecessor.kind(),
                    OwnerObjectDigest::of(&raw)
                )
                .is_err(),
                "generation {generation}"
            );
        }
    }
}

#[test]
fn frozen_graph29_and_graph30_sequence_bytes_are_preserved_exactly() {
    let seed = b"predecessor-sequence-codec";
    let item = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
        .unwrap()
        .0;
    let sequence_type =
        encode_type_object(&TypeObject::new(TypeForm::OwnedSequence { item }).unwrap())
            .unwrap()
            .0;
    for generation in [29, 30] {
        let record = OwnerRecord::Expression(ExpressionRecord {
            contract_version: generation,
            id: ExpressionId::migrate(seed, 0),
            operation: ExpressionOperation::SequencePush {
                sequence_type,
                value: ExpressionId::migrate(seed, 1),
                source: ExpressionId::migrate(seed, 2),
            },
        });
        let magic: [u8; 8] = format!("LKJOWN{generation}").as_bytes().try_into().unwrap();
        let domain = format!("lkjscript.kernel.owner-envelope.v{generation}");
        // Original owner and operation ordinals plus literal field order.
        let original = packed::encode(
            magic,
            &domain,
            &(
                9_u32,
                generation,
                ExpressionId::migrate(seed, 0),
                35_u32,
                sequence_type,
                ExpressionId::migrate(seed, 1),
                ExpressionId::migrate(seed, 2),
            ),
            MAXIMUM_OWNER_OBJECT_BYTES,
        )
        .unwrap();
        let (digest, bytes) = encode_owner(&record).unwrap();
        assert_eq!(bytes, original);
        assert_eq!(
            decode_owner(&original, record.owner(), record.kind(), digest).unwrap(),
            record
        );
    }
}
