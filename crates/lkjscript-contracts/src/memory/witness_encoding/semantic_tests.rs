#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

fn numeric_identity(value: u64) -> [u8; 32] {
    let mut identity = [0_u8; 32];
    identity[..8].copy_from_slice(&value.to_be_bytes());
    identity
}

fn field(identity: u8, order: u64, ty: SemanticType) -> SemanticProductField {
    SemanticProductField {
        identity: [identity; 32],
        source_order: order,
        ty,
    }
}

fn product(identity: u8, fields: Vec<SemanticProductField>) -> SemanticDeclaration {
    SemanticDeclaration::Product(SemanticProductDeclaration {
        identity: [identity; 32],
        fields,
    })
}

fn product_descriptor() -> SemanticDescriptor {
    SemanticDescriptor {
        root: SemanticType::Product([1; 32]),
        declarations: vec![product(
            1,
            vec![
                field(11, 0, SemanticType::Primitive(SemanticPrimitiveKind::I64)),
                field(
                    12,
                    1,
                    SemanticType::List(Box::new(SemanticType::Primitive(
                        SemanticPrimitiveKind::Bool,
                    ))),
                ),
            ],
        )],
    }
}

#[test]
fn reachable_product_fields_and_list_shape_change_contract() {
    let baseline = product_descriptor();
    let digest = semantic_contract_hash(&baseline).expect("baseline");
    let mut changed = baseline.clone();
    let SemanticDeclaration::Product(item) = &mut changed.declarations[0] else {
        unreachable!()
    };
    item.fields[0].ty = SemanticType::Primitive(SemanticPrimitiveKind::F64);
    assert_ne!(
        digest,
        semantic_contract_hash(&changed).expect("field change")
    );
    let mut changed = baseline.clone();
    let SemanticDeclaration::Product(item) = &mut changed.declarations[0] else {
        unreachable!()
    };
    item.fields[1].ty = SemanticType::List(Box::new(SemanticType::Primitive(
        SemanticPrimitiveKind::I64,
    )));
    assert_ne!(
        digest,
        semantic_contract_hash(&changed).expect("list change")
    );
}

#[test]
fn enum_variant_field_and_exact_type_argument_change_contract_and_roles() {
    let declaration = SemanticDeclaration::Enum(SemanticEnumDeclaration {
        identity: [2; 32],
        type_parameters: vec!["t".into()],
        variants: vec![SemanticEnumVariant {
            identity: [3; 32],
            source_order: 0,
            fields: vec![SemanticEnumVariantField {
                identity: [4; 32],
                source_order: 0,
                ty: SemanticType::Parameter("t".into()),
                indirect: false,
            }],
        }],
    });
    let descriptor = SemanticDescriptor {
        root: SemanticType::Enum {
            identity: [2; 32],
            arguments: vec![SemanticType::Primitive(SemanticPrimitiveKind::I64)],
        },
        declarations: vec![declaration],
    };
    let requirements = semantic_dependency_requirements(&descriptor).expect("enum requirements");
    assert!(matches!(
        requirements[0].0,
        ExecutableMemoryWitnessRole::TypeArgument { index: 0, .. }
    ));
    assert!(matches!(
        requirements[1].0,
        ExecutableMemoryWitnessRole::EnumVariantField {
            field_source_order: 0,
            ..
        }
    ));
    let mut changed = descriptor.clone();
    let SemanticType::Enum { arguments, .. } = &mut changed.root else {
        unreachable!()
    };
    arguments[0] = SemanticType::Primitive(SemanticPrimitiveKind::Bool);
    assert_ne!(
        semantic_contract_hash(&descriptor).unwrap(),
        semantic_contract_hash(&changed).unwrap()
    );
    let mut changed = descriptor.clone();
    let SemanticDeclaration::Enum(item) = &mut changed.declarations[0] else {
        unreachable!()
    };
    item.variants[0].fields[0].indirect = true;
    assert_ne!(
        semantic_contract_hash(&descriptor).unwrap(),
        semantic_contract_hash(&changed).unwrap()
    );
}

#[test]
fn field_role_swap_and_forged_local_target_reject() {
    let descriptor = product_descriptor();
    let requirements = semantic_dependency_requirements(&descriptor).expect("requirements");
    let dependency = |index: usize| ExecutableMemoryWitnessDependency {
        role: requirements[index].0.clone(),
        target: ExecutableMemoryWitnessTarget::ExternalMember {
            group: [10 + index as u8; 32],
            member: [20 + index as u8; 32],
        },
    };
    let valid = vec![dependency(0), dependency(1)];
    validate_executable_dependencies(&descriptor, &valid).expect("complete roles");
    let swapped = vec![dependency(1), dependency(0)];
    assert!(validate_executable_dependencies(&descriptor, &swapped).is_err());
    let duplicate = vec![dependency(0), dependency(0)];
    assert!(validate_executable_dependencies(&descriptor, &duplicate).is_err());
}

#[test]
fn semantic_descriptor_scale_is_not_a_validity_quota() {
    const DECLARATIONS: u64 = 16_385;
    const FIELDS: u64 = 65_537;
    const DESCRIPTOR_TEXT_BYTES: usize = 16 * 1024 * 1024 + 1;

    let declarations = (0..DECLARATIONS)
        .map(|index| {
            let identity = numeric_identity(index + 1);
            let fields = (index + 1 < DECLARATIONS)
                .then(|| SemanticProductField {
                    identity: numeric_identity(u64::MAX - index),
                    source_order: 0,
                    ty: SemanticType::Product(numeric_identity(index + 2)),
                })
                .into_iter()
                .collect();
            SemanticDeclaration::Product(SemanticProductDeclaration { identity, fields })
        })
        .collect();
    let deep_graph = SemanticDescriptor {
        root: SemanticType::Product(numeric_identity(1)),
        declarations,
    };
    assert_ne!(
        semantic_contract_hash(&deep_graph).expect("semantic declaration scale"),
        [0; 32]
    );

    let fields = (0..FIELDS)
        .map(|index| SemanticProductField {
            identity: numeric_identity(index + 1),
            source_order: index,
            ty: SemanticType::Primitive(SemanticPrimitiveKind::I64),
        })
        .collect();
    let wide_graph = SemanticDescriptor {
        root: SemanticType::Product(numeric_identity(1)),
        declarations: vec![SemanticDeclaration::Product(SemanticProductDeclaration {
            identity: numeric_identity(1),
            fields,
        })],
    };
    assert_eq!(
        semantic_dependency_requirements(&wide_graph)
            .expect("semantic dependency edge scale")
            .len(),
        usize::try_from(FIELDS).expect("test field count fits usize")
    );
    assert_ne!(
        semantic_contract_hash(&wide_graph).expect("semantic type scale"),
        [0; 32]
    );

    let large_parameter = "t".repeat(DESCRIPTOR_TEXT_BYTES);
    let large_descriptor = SemanticDescriptor {
        root: SemanticType::Enum {
            identity: numeric_identity(1),
            arguments: vec![SemanticType::Primitive(SemanticPrimitiveKind::I64)],
        },
        declarations: vec![SemanticDeclaration::Enum(SemanticEnumDeclaration {
            identity: numeric_identity(1),
            type_parameters: vec![large_parameter],
            variants: vec![SemanticEnumVariant {
                identity: numeric_identity(2),
                source_order: 0,
                fields: Vec::new(),
            }],
        })],
    };
    assert_ne!(
        semantic_contract_hash(&large_descriptor).expect("semantic descriptor byte scale"),
        [0; 32]
    );
}

#[test]
fn recursive_self_and_mutual_product_enum_closure_is_cycle_free() {
    let self_recursive = SemanticDescriptor {
        root: SemanticType::Product([1; 32]),
        declarations: vec![product(
            1,
            vec![field(11, 0, SemanticType::Product([1; 32]))],
        )],
    };
    let first = canonical_semantic_descriptor(&self_recursive).expect("self recursion");
    assert_eq!(
        first,
        canonical_semantic_descriptor(&self_recursive).unwrap()
    );
    let mutual = SemanticDescriptor {
        root: SemanticType::Product([1; 32]),
        declarations: vec![
            product(
                1,
                vec![field(
                    11,
                    0,
                    SemanticType::Enum {
                        identity: [2; 32],
                        arguments: Vec::new(),
                    },
                )],
            ),
            SemanticDeclaration::Enum(SemanticEnumDeclaration {
                identity: [2; 32],
                type_parameters: Vec::new(),
                variants: vec![SemanticEnumVariant {
                    identity: [3; 32],
                    source_order: 0,
                    fields: vec![SemanticEnumVariantField {
                        identity: [4; 32],
                        source_order: 0,
                        ty: SemanticType::Product([1; 32]),
                        indirect: true,
                    }],
                }],
            }),
        ],
    };
    let encoded = canonical_semantic_descriptor(&mutual).expect("mutual recursion");
    assert_eq!(encoded, canonical_semantic_descriptor(&mutual).unwrap());
    let requirement = semantic_dependency_requirements(&mutual).unwrap().remove(0);
    validate_executable_dependencies(
        &mutual,
        &[ExecutableMemoryWitnessDependency {
            role: requirement.0,
            target: ExecutableMemoryWitnessTarget::LocalMember(0),
        }],
    )
    .expect("mutual local edge");
}
