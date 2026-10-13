//! Literal predecessor envelopes distinguish readable old meaning from new authority.
use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::*;

#[test]
fn current_nominal_owner_uses_graph32_and_rehashed_graph31_cannot_admit_it() {
    let seed = b"nominal-record-owner-codec";
    let mut header = OwnerHeader::new(
        OwnerKey::Declaration(DeclarationId::migrate(seed, 0)),
        OwnerKind::OwnedRecord,
    );
    let module = ModuleId::migrate(seed, 0);
    let name = Name::new("Packet").unwrap();
    let record = OwnerRecord::Declaration(DeclarationRecord {
        header,
        module,
        name: name.clone(),
        visibility: DeclarationVisibility::Public,
        payload: DeclarationPayload::OwnedRecord {
            type_parameters: vec![],
            fields: vec![],
        },
    });
    let (digest, bytes) = encode_owner(&record).unwrap();
    assert_eq!(&bytes[..8], b"LKJOWN34");
    assert_eq!(
        decode_owner(&bytes, record.owner(), record.kind(), digest).unwrap(),
        record
    );

    header.contract_version = 31;
    // Freeze the owner/declaration ordinals and original field order rather
    // than calling the production owner encoder with a new nominal value.
    let forged = packed::encode(
        *b"LKJOWN31",
        "lkjscript.kernel.owner-envelope.v31",
        &(
            1_u32,
            header,
            module,
            name,
            DeclarationVisibility::Public,
            10_u32,
            Vec::<TypeParameterId>::new(),
            Vec::<FieldId>::new(),
        ),
        MAXIMUM_OWNER_OBJECT_BYTES,
    )
    .unwrap();
    assert_eq!(
        decode_owner(
            &forged,
            header.owner,
            header.kind,
            OwnerObjectDigest::of(&forged)
        )
        .unwrap_err()
        .code,
        "kernel_owned_record_generation"
    );
}

#[test]
fn independently_encoded_graph31_rejects_all_new_nominal_custody_tags() {
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
    let magic = *b"LKJOWN31";
    let domain = "lkjscript.kernel.owner-envelope.v31";
    let forged = [
        packed::encode(
            magic,
            domain,
            &(
                9_u32,
                31_u16,
                id,
                41_u32,
                record_type,
                Vec::<OwnedRecordExpressionField>::new(),
            ),
            MAXIMUM_OWNER_OBJECT_BYTES,
        )
        .unwrap(),
        packed::encode(
            magic,
            domain,
            &(
                9_u32,
                31_u16,
                id,
                42_u32,
                record_type,
                source,
                Vec::<OwnedRecordBinding>::new(),
                body,
            ),
            MAXIMUM_OWNER_OBJECT_BYTES,
        )
        .unwrap(),
        packed::encode(
            magic,
            domain,
            &(
                9_u32,
                31_u16,
                id,
                43_u32,
                record_type,
                source,
                FieldReference {
                    package: PackageId::migrate(seed, 0),
                    field: FieldId::migrate(seed, 0),
                },
                BindingId::migrate(seed, 0),
                body,
            ),
            MAXIMUM_OWNER_OBJECT_BYTES,
        )
        .unwrap(),
    ];
    for bytes in forged {
        assert_eq!(
            decode_owner(
                &bytes,
                OwnerKey::Expression(id),
                OwnerKind::Expression,
                OwnerObjectDigest::of(&bytes)
            )
            .unwrap_err()
            .code,
            "kernel_owned_record_generation"
        );
    }
}

#[test]
fn literal_graph31_indexed_sequence_envelope_roundtrips_without_reencoding_as_graph32() {
    let seed = b"literal-graph31-indexed-sequence";
    let id = ExpressionId::migrate(seed, 0);
    let source = ExpressionId::migrate(seed, 1);
    let index = ExpressionId::migrate(seed, 2);
    let item = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
        .unwrap()
        .0;
    let sequence_type =
        encode_type_object(&TypeObject::new(TypeForm::OwnedSequence { item }).unwrap())
            .unwrap()
            .0;
    let original = packed::encode(
        *b"LKJOWN31",
        "lkjscript.kernel.owner-envelope.v31",
        &(9_u32, 31_u16, id, 39_u32, sequence_type, source, index),
        MAXIMUM_OWNER_OBJECT_BYTES,
    )
    .unwrap();
    let digest = OwnerObjectDigest::of(&original);
    let record = decode_owner(
        &original,
        OwnerKey::Expression(id),
        OwnerKind::Expression,
        digest,
    )
    .unwrap();
    assert_eq!(
        record,
        OwnerRecord::Expression(ExpressionRecord {
            contract_version: 31,
            id,
            operation: ExpressionOperation::SequenceGet {
                sequence_type,
                source,
                index
            },
        })
    );
    assert_eq!(encode_owner(&record).unwrap(), (digest, original));
}
