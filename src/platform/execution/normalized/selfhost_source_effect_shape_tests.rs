//! Structural controls retain invalid unused descriptors in hash-consistent carriers.
use super::*;

// Invalid local encodings retain their literal bytes and receive exact directory
// state plus explicitly synthetic fixture provenance. One production writer owns
// the carrier; this helper is not a second serialization implementation.
fn raw_packet(
    source: &KernelSnapshot,
    owner_override: Option<(OwnerKey, Vec<u8>)>,
    extra_type: Option<Vec<u8>>,
) -> Vec<u8> {
    source_fixture::raw_object_packet(
        source,
        owner_override
            .as_ref()
            .map(|(key, bytes)| (*key, bytes.as_slice())),
        extra_type.as_deref(),
    )
}

fn raw_owner(source: &KernelSnapshot, owner: &OwnerRecord) {
    let bytes = object_envelope(
        b"LKJOWNER",
        "lkjscript.kernel.owner-envelope",
        &encode(owner),
    );
    assert!(
        kernel::decode_owner(
            &bytes,
            owner.owner(),
            owner.kind(),
            kernel::OwnerObjectDigest::of(&bytes)
        )
        .is_err()
    );
    let carrier = raw_packet(source, Some((owner.owner(), bytes)), None);
    packet_result("read-carrier", &carrier, 12);
}

#[test]
fn selfhost_source_effect_shapes_reject_noncanonical_owner_rows_and_limits() {
    let source = fixture();
    assert_eq!(raw_packet(&source, None, None), graph::packet(&source));
    for effect in [false, true] {
        for duplicate in [false, true] {
            let mut changed = source.clone();
            let function = function_mut(&mut changed, "first");
            let kernel::FunctionEffect::Task {
                requirements,
                effect_parameters,
            } = &mut function.effect
            else {
                panic!("task")
            };
            if effect {
                if duplicate {
                    effect_parameters[1] = effect_parameters[0];
                } else {
                    effect_parameters.swap(0, 1);
                }
            } else if duplicate {
                requirements[1] = requirements[0];
            } else {
                requirements.swap(0, 1);
            }
            let key = OwnerKey::Declaration(declaration(&changed, "first"));
            raw_owner(&source, &changed.owners[&key]);
        }
    }
    for parameter in [false, true] {
        for duplicate in [false, true] {
            let key = if parameter {
                OwnerKey::RequirementParameter(function(&source, "first").requirement_parameters[0])
            } else {
                owner_key(&source, |owner| {
                    matches!(owner, OwnerRecord::Requirement(record)
                    if record.name.as_str() == "clock")
                })
            };
            let mut owner = source.owners[&key].clone();
            let operations = match &mut owner {
                OwnerRecord::Requirement(record) => &mut record.operations,
                OwnerRecord::RequirementParameter(record) => &mut record.constraint.operations,
                _ => panic!("requirement"),
            };
            if duplicate {
                operations[1] = operations[0];
            } else {
                operations.swap(0, 1);
            }
            raw_owner(&source, &owner);
        }
    }
    let key = owner_key(&source, |owner| {
        matches!(owner, OwnerRecord::Requirement(record)
        if record.name.as_str() == "clock")
    });
    let mut owner = source.owners[&key].clone();
    let OwnerRecord::Requirement(record) = &mut owner else {
        panic!("requirement")
    };
    record.limits[0].maximum = 0;
    raw_owner(&source, &owner);
    let mut owner = source.owners[&key].clone();
    let OwnerRecord::Requirement(record) = &mut owner else {
        panic!("requirement")
    };
    record.limits.push(record.limits[0].clone());
    raw_owner(&source, &owner);
}

#[test]
fn selfhost_source_effect_shapes_check_unused_task_rows_without_context_rejection() {
    let source = fixture();
    let first = declaration(&source, "first");
    let key = parameter_key(&source, kernel::ParameterParent::Function(first), "ordered");
    let original = parameter_type(&source, key);
    for duplicate in [false, true] {
        let mut object = source.types[&original].clone();
        let kernel::TypeForm::TaskFunction {
            parameters, effect, ..
        } = &mut object.form
        else {
            panic!("task type")
        };
        // A different signature ensures this raw object is unused by every owner.
        parameters.clear();
        if duplicate {
            effect.parameters[1] = effect.parameters[0];
        } else {
            effect.parameters.swap(0, 1);
        }
        let payload = encode(&object);
        let bytes = envelope(&payload);
        assert!(kernel::decode_type_object(&bytes, TypeObjectDigest::of(&bytes)).is_err());
        assert_eq!(result("read-envelope", &bytes), (0, payload));
        let carrier = raw_packet(&source, None, Some(bytes));
        packet_result("read-carrier", &carrier, 12);
    }
}

#[test]
fn selfhost_source_effect_links_check_unused_task_row_endpoints() {
    use crate::platform::semantic_id::{EffectParameterId, RequirementParameterId};
    for requirement in [false, true] {
        let mut source = fixture();
        let scalar = source
            .types
            .iter()
            .find_map(|(digest, object)| {
                matches!(object.form, kernel::TypeForm::I64).then_some(*digest)
            })
            .unwrap();
        let package = source.root.package_id;
        let effect = if requirement {
            kernel::EffectRow {
                requirements: vec![kernel::RequirementOperand::Parameter(
                    kernel::RequirementParameterReference {
                        package,
                        parameter: RequirementParameterId::from_bytes([249; 16]).unwrap(),
                    },
                )],
                parameters: vec![],
            }
        } else {
            kernel::EffectRow {
                requirements: vec![],
                parameters: vec![kernel::EffectParameterReference {
                    package,
                    parameter: EffectParameterId::from_bytes([250; 16]).unwrap(),
                }],
            }
        };
        intern(
            &mut source,
            kernel::TypeForm::TaskFunction {
                parameters: vec![],
                result: scalar,
                effect,
            },
        );
        // No contextual owner uses this descriptor; endpoint closure still applies.
        let bytes = graph::packet(&source);
        packet_result("read-carrier", &bytes, 0);
        packet_result("read-graph", &bytes, 14);
    }
}
