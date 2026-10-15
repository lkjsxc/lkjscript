//! Current indexed sequence syntax retains canonical authored roles and type roots.
use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::ExpressionId;

#[test]
fn indexed_sequence_operations_round_trip_with_ordered_children() {
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
    for (operation, expected_children) in [
        (
            ExpressionOperation::SequenceGet {
                sequence_type,
                source,
                index,
            },
            vec![index, source],
        ),
        (
            ExpressionOperation::SequenceReplace {
                sequence_type,
                result_type: sequence_type,
                index,
                value,
                source,
            },
            vec![index, value, source],
        ),
        (
            ExpressionOperation::SequencePush {
                sequence_type,
                value,
                source,
            },
            vec![value, source],
        ),
    ] {
        let expression = ExpressionRecord::new(id, operation).unwrap();
        assert_eq!(
            expression
                .children()
                .iter()
                .map(|c| c.expression)
                .collect::<Vec<_>>(),
            expected_children
        );
        assert!(expression.type_roots().contains(&sequence_type));
        let record = OwnerRecord::Expression(expression);
        let (digest, bytes) = encode_owner(&record).unwrap();
        assert_eq!(&bytes[..8], &OWNER_MAGIC);
        assert_eq!(
            decode_owner(&bytes, record.owner(), record.kind(), digest).unwrap(),
            record
        );
    }
}
