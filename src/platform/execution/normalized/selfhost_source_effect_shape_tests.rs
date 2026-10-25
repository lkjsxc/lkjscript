//! Structural controls retain invalid unused descriptors in hash-consistent carriers.
use super::*;

fn word(bytes: &mut [u8], at: usize, value: usize) {
    bytes[at..at + 4].copy_from_slice(&u32::try_from(value).unwrap().to_le_bytes());
}

// Serialize neutral inventories with optional raw objects that strict Rust encoders
// deliberately refuse. Commit the actual object bytes, directory keys and roots.
fn raw_packet(
    source: &KernelSnapshot,
    owner_override: Option<(OwnerKey, Vec<u8>)>,
    extra_type: Option<Vec<u8>>,
) -> Vec<u8> {
    let mut owners = Vec::new();
    let mut bindings = Vec::new();
    let mut objects = Vec::new();
    for (key, record) in &source.owners {
        let bytes = owner_override
            .as_ref()
            .filter(|(changed, _)| changed == key)
            .map(|(_, bytes)| bytes.clone())
            .unwrap_or_else(|| kernel::encode_owner(record).unwrap().1);
        let binding = kernel::encode_owner_binding(&kernel::OwnerBinding {
            kind: record.kind(),
            object: kernel::OwnerObjectDigest::of(&bytes),
        });
        let key_bytes = kernel::owner_map_key(*key).to_vec();
        owners.extend_from_slice(&key_bytes);
        owners.extend_from_slice(&binding);
        bindings.push((key_bytes, binding));
        objects.push(bytes);
    }
    let mut types = source
        .types
        .iter()
        .map(|(digest, object)| (*digest, kernel::encode_type_object(object).unwrap().1))
        .collect::<BTreeMap<_, _>>();
    if let Some(bytes) = extra_type {
        types.insert(TypeObjectDigest::of(&bytes), bytes);
    }
    let mut type_keys = Vec::new();
    for (digest, bytes) in &types {
        type_keys.extend_from_slice(&digest.bytes());
        objects.push(bytes.clone());
    }
    let mut retirements = Vec::new();
    for (key, record) in &source.retirements {
        let (digest, bytes) = kernel::encode_retirement(record).unwrap();
        retirements.extend_from_slice(&kernel::retirement_map_key(*key));
        retirements.extend_from_slice(&digest.bytes());
        objects.push(bytes);
    }
    let content = crate::platform::persistent_map::MapContentRoot::from_sorted(bindings).unwrap();
    let mut root = source.root.clone();
    root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        root.owners.page(),
        source.owners.len() as u64,
        content.digest(),
    );
    let name = root.package_name.as_str().as_bytes();
    let mut bytes = vec![0; 128];
    bytes[..8].copy_from_slice(b"LKJSG002");
    word(&mut bytes, 12, source.owners.len());
    word(&mut bytes, 16, types.len());
    word(&mut bytes, 20, source.retirements.len());
    word(&mut bytes, 24, name.len());
    bytes[32..48].copy_from_slice(&root.package_id.bytes());
    bytes[48..80].copy_from_slice(
        &kernel::semantic_state_digest_from_root(&root)
            .unwrap()
            .bytes(),
    );
    bytes.extend_from_slice(name);
    for (at, section) in [(80, owners), (84, type_keys), (88, retirements)] {
        let start = bytes.len();
        word(&mut bytes, at, start);
        bytes.extend(section);
    }
    let spans = bytes.len();
    word(&mut bytes, 92, spans);
    bytes.resize(spans + objects.len() * 8, 0);
    let start = bytes.len();
    word(&mut bytes, 96, start);
    for (index, object) in objects.into_iter().enumerate() {
        let start = bytes.len();
        word(&mut bytes, spans + index * 8, start);
        word(&mut bytes, spans + index * 8 + 4, object.len());
        bytes.extend(object);
    }
    let length = bytes.len();
    word(&mut bytes, 8, length);
    graph::set_allowance(&mut bytes, None);
    bytes
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
