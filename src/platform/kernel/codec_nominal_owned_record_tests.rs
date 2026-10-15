//! Canonical nominal custody records and independently framed hostile metadata.
use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::*;

#[test]
fn nominal_owner_round_trips_and_foreign_header_rejects() {
    let seed = b"nominal-record-owner-codec";
    let mut record = OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(
            OwnerKey::Declaration(DeclarationId::migrate(seed, 0)),
            OwnerKind::OwnedRecord,
        ),
        module: ModuleId::migrate(seed, 0),
        name: Name::new("Packet").unwrap(),
        visibility: DeclarationVisibility::Public,
        payload: DeclarationPayload::OwnedRecord {
            type_parameters: vec![],
            fields: vec![],
        },
    });
    let (digest, bytes) = encode_owner(&record).unwrap();
    assert_eq!(&bytes[..8], &OWNER_MAGIC);
    assert_eq!(
        decode_owner(&bytes, record.owner(), record.kind(), digest).unwrap(),
        record
    );
    record.set_encoding_for_edit(0);
    assert!(encode_owner(&record).is_err());
    let forged = packed::encode_canonical(
        OWNER_MAGIC,
        OWNER_ENVELOPE_DOMAIN,
        &record,
        MAXIMUM_OWNER_OBJECT_BYTES,
    )
    .unwrap();
    assert_eq!(
        decode_owner(
            &forged,
            record.owner(),
            record.kind(),
            OwnerObjectDigest::of(&forged)
        )
        .unwrap_err()
        .code,
        "kernel_owner_header"
    );
}

#[test]
fn nominal_custody_operations_round_trip_in_current_schema() {
    let seed = b"nominal-record-operation-codec";
    let id = ExpressionId::migrate(seed, 0);
    let source = ExpressionId::migrate(seed, 1);
    let body = ExpressionId::migrate(seed, 2);
    let record_type = encode_type_object(
        &TypeObject::new(TypeForm::Named {
            declaration: DeclarationReference {
                package: PackageId::migrate(seed, 0),
                declaration: DeclarationId::migrate(seed, 0),
            },
        })
        .unwrap(),
    )
    .unwrap()
    .0;
    for operation in [
        ExpressionOperation::PackOwnedRecord {
            record_type,
            fields: Vec::<OwnedRecordExpressionField>::new(),
        },
        ExpressionOperation::UnpackOwnedRecord {
            record_type,
            source,
            fields: Vec::<OwnedRecordBinding>::new(),
            body,
        },
        ExpressionOperation::BorrowOwnedRecordField {
            record_type,
            source,
            field: FieldReference {
                package: PackageId::migrate(seed, 0),
                field: FieldId::migrate(seed, 0),
            },
            binding: BindingId::migrate(seed, 0),
            body,
        },
    ] {
        let record = OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap());
        let (digest, bytes) = encode_owner(&record).unwrap();
        assert_eq!(
            decode_owner(&bytes, record.owner(), record.kind(), digest).unwrap(),
            record
        );
        assert!(record.type_roots().contains(&record_type));
    }
}
