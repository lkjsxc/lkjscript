//! Unsupported namespace owner forms stop at shape admission, before uniqueness.
use super::*;
use crate::platform::semantic_id::{
    EffectParameterId, OperationId, RequirementId, RequirementParameterId,
};

#[test]
fn selfhost_source_namespace_does_not_relabel_unsupported_classes_as_semantic_proof() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-namespace.lkjc"
    ))
    .unwrap();
    let declaration = source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.name.as_str() == "Pair" =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap();
    let result_type = source
        .types
        .iter()
        .find_map(|(digest, object)| {
            matches!(object.form, kernel::TypeForm::I64).then_some(*digest)
        })
        .unwrap();
    let effect = EffectParameterId::from_bytes([241; 16]).unwrap();
    let requirement_parameter = RequirementParameterId::from_bytes([242; 16]).unwrap();
    let operation = OperationId::from_bytes([243; 16]).unwrap();
    let requirement = RequirementId::from_bytes([244; 16]).unwrap();
    let interface = DeclarationReference {
        package: source.root.package_id,
        declaration,
    };
    let operation_reference = kernel::OperationReference {
        package: source.root.package_id,
        operation,
    };
    let name = kernel::Name::new("unsupported").unwrap();
    let records = [
        OwnerRecord::EffectParameter(kernel::EffectParameterRecord {
            header: kernel::OwnerHeader::new(
                OwnerKey::EffectParameter(effect),
                kernel::OwnerKind::EffectParameter,
            ),
            declaration,
            name: name.clone(),
        }),
        OwnerRecord::RequirementParameter(kernel::RequirementParameterRecord {
            header: kernel::OwnerHeader::new(
                OwnerKey::RequirementParameter(requirement_parameter),
                kernel::OwnerKind::RequirementParameter,
            ),
            declaration,
            name: name.clone(),
            constraint: kernel::RequirementConstraint {
                interface,
                operations: vec![operation_reference],
            },
        }),
        OwnerRecord::Operation(kernel::OperationRecord {
            header: kernel::OwnerHeader::new(
                OwnerKey::Operation(operation),
                kernel::OwnerKind::Operation,
            ),
            declaration,
            name: name.clone(),
            parameters: vec![],
            result: result_type,
            idempotency: kernel::Idempotency::Idempotent,
            external_visibility: kernel::ExternalVisibility::None,
        }),
        OwnerRecord::Requirement(kernel::RequirementRecord {
            header: kernel::OwnerHeader::new(
                OwnerKey::Requirement(requirement),
                kernel::OwnerKind::Requirement,
            ),
            declaration,
            name,
            interface,
            operations: vec![operation_reference],
            limits: vec![],
        }),
    ];
    for record in records {
        let mut changed = source.clone();
        changed.owners.insert(record.owner(), record);
        let bytes = graph::packet(&changed);
        // These controls establish the unsupported shape boundary only. They
        // deliberately do not claim complete later reference/scope admission.
        assert_eq!(result("read-namespace", &bytes), (13, vec![]));
        emitted_result("read-namespace", &bytes, 13);
    }
}
