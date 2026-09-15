//! Independent closed-signature vectors and complete canonical binary64 admission.

use super::*;
use crate::platform::binary64::Binary64;
use crate::platform::intrinsic_contract::{validate_intrinsic, validate_kernel_intrinsic};
use crate::platform::semantic::{
    FunctionSignature, OwnerId, ResolvedField, ResolvedTaskCapability, ResolvedType,
};

const SEED: &[u8] = b"binary64-independent-admission";

fn result_record(value: ResolvedType) -> ResolvedType {
    ResolvedType::Record(vec![
        ResolvedField {
            name: "valid".into(),
            ty: ResolvedType::Bool,
        },
        ResolvedField {
            name: "value".into(),
            ty: value,
        },
    ])
}

// This table is the test's declared contract. It is not queried from discovery, standard source,
// the validator, compiler, runtime dispatch, or a generated signature table.
fn signatures() -> Vec<(&'static str, Vec<ResolvedType>, ResolvedType)> {
    use ResolvedType::{Bool, F64, I64, Text};
    vec![
        ("core.f64.add", vec![F64, F64], F64),
        ("core.f64.subtract", vec![F64, F64], F64),
        ("core.f64.multiply", vec![F64, F64], F64),
        ("core.f64.divide", vec![F64, F64], F64),
        ("core.f64.negate", vec![F64], F64),
        ("core.f64.abs", vec![F64], F64),
        ("core.f64.sqrt", vec![F64], F64),
        ("core.f64.less", vec![F64, F64], Bool),
        ("core.f64.less-equal", vec![F64, F64], Bool),
        ("core.f64.is-finite", vec![F64], Bool),
        ("core.f64.is-nan", vec![F64], Bool),
        ("core.f64.from-i64", vec![I64], F64),
        ("core.f64.to-i64-result", vec![F64], result_record(I64)),
        ("core.f64.parse-result", vec![Text], result_record(F64)),
        ("core.f64.to-text", vec![F64], Text),
    ]
}

fn legacy_signature(parameters: Vec<ResolvedType>, result: ResolvedType) -> FunctionSignature {
    FunctionSignature {
        owner: OwnerId {
            package: crate::platform::package::PackageId::parse("10000000000000000000000000000001")
                .unwrap(),
            module_id: ModuleId::migrate(SEED, 0),
            declaration_id: DeclarationId::migrate(SEED, 0),
            module: "numeric".into(),
            declaration: "external".into(),
        },
        type_parameters: Vec::new(),
        parameters,
        result,
        task_capabilities: Vec::new(),
        external_implementation: None,
    }
}

fn intern(snapshot: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    snapshot.types.insert(digest, object);
    digest
}

fn canonical_type(snapshot: &mut KernelSnapshot, ty: &ResolvedType) -> TypeObjectDigest {
    let form = match ty {
        ResolvedType::Unit => TypeForm::Unit,
        ResolvedType::Bool => TypeForm::Bool,
        ResolvedType::I64 => TypeForm::I64,
        ResolvedType::F64 => TypeForm::F64,
        ResolvedType::Text => TypeForm::Text,
        ResolvedType::Secret => TypeForm::Secret,
        ResolvedType::List(item) => TypeForm::List {
            item: canonical_type(snapshot, item),
        },
        ResolvedType::Record(fields) => {
            let mut fields = fields
                .iter()
                .map(|field| StructuralTypeField {
                    name: name(&field.name),
                    ty: canonical_type(snapshot, &field.ty),
                })
                .collect::<Vec<_>>();
            fields.sort_by(|left, right| left.name.cmp(&right.name));
            TypeForm::StructuralRecord { fields }
        }
        _ => panic!("bounded test signature form"),
    };
    intern(snapshot, form)
}

fn canonical_signature(
    implementation: &str,
    signature: &FunctionSignature,
) -> (KernelSnapshot, ExternalDeclaration) {
    let mut snapshot = transport_snapshot();
    let parameters = signature
        .parameters
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let id = ParameterId::migrate(SEED, index as u64);
            let ty = canonical_type(&mut snapshot, ty);
            snapshot.owners.insert(
                OwnerKey::Parameter(id),
                OwnerRecord::Parameter(ParameterRecord {
                    header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
                    parent: ParameterParent::Function(signature.owner.declaration_id),
                    name: name(&format!("argument-{index}")),
                    ty,
                    use_mode: ParameterUse::Unrestricted,
                    resource_requirement: None,
                }),
            );
            id
        })
        .collect();
    let result = canonical_type(&mut snapshot, &signature.result);
    (
        snapshot,
        ExternalDeclaration {
            type_parameters: Vec::new(),
            parameters,
            result,
            implementation: ImplementationName::new(implementation).unwrap(),
        },
    )
}

fn assert_foreign_signature(implementation: &str, signature: &FunctionSignature) {
    assert_eq!(
        validate_intrinsic(implementation, signature)
            .unwrap_err()
            .code,
        "intrinsic_signature",
        "legacy adapter: {implementation}: {signature:?}"
    );
    let (snapshot, external) = canonical_signature(implementation, signature);
    assert_eq!(
        validate_kernel_intrinsic(&snapshot, &external, &mut 0, 10_000)
            .unwrap_err()
            .code,
        "intrinsic_signature",
        "canonical adapter: {implementation}: {signature:?}"
    );
}

#[test]
fn f64_independent_intrinsic_signatures_reject_wrong_arity_types_and_legacy_effects() {
    for (implementation, parameters, result) in signatures() {
        let valid = legacy_signature(parameters, result);
        validate_intrinsic(implementation, &valid).unwrap();
        let (snapshot, external) = canonical_signature(implementation, &valid);
        validate_kernel_intrinsic(&snapshot, &external, &mut 0, 10_000).unwrap();

        for index in 0..valid.parameters.len() {
            for wrong in [
                ResolvedType::Unit,
                ResolvedType::Bool,
                ResolvedType::I64,
                ResolvedType::F64,
                ResolvedType::Text,
                ResolvedType::Secret,
                ResolvedType::List(Box::new(ResolvedType::F64)),
            ] {
                if wrong == valid.parameters[index] {
                    continue;
                }
                let mut malformed = valid.clone();
                malformed.parameters[index] = wrong;
                assert_foreign_signature(implementation, &malformed);
            }
        }
        let mut missing = valid.clone();
        missing.parameters.pop();
        assert_foreign_signature(implementation, &missing);
        let mut extra = valid.clone();
        extra.parameters.push(ResolvedType::F64);
        assert_foreign_signature(implementation, &extra);
        for wrong in [ResolvedType::Unit, ResolvedType::I64, ResolvedType::F64] {
            if wrong != valid.result {
                let mut malformed = valid.clone();
                malformed.result = wrong;
                assert_foreign_signature(implementation, &malformed);
            }
        }
        let mut effectful = valid.clone();
        effectful.task_capabilities.push(ResolvedTaskCapability {
            alias: "data".into(),
            interface: effectful.owner.clone(),
        });
        assert_eq!(
            validate_intrinsic(implementation, &effectful)
                .unwrap_err()
                .code,
            "intrinsic_signature"
        );
        for use_mode in [ParameterUse::Borrow, ParameterUse::Consume] {
            let mut resource = snapshot.clone();
            let OwnerRecord::Parameter(parameter) = resource
                .owners
                .get_mut(&OwnerKey::Parameter(external.parameters[0]))
                .unwrap()
            else {
                panic!("external parameter");
            };
            parameter.use_mode = use_mode;
            assert_eq!(
                validate_kernel_intrinsic(&resource, &external, &mut 0, 10_000)
                    .unwrap_err()
                    .code,
                "intrinsic_signature"
            );
        }
    }
}

#[test]
fn f64_fallible_intrinsics_require_exact_structural_result_fields() {
    for (implementation, parameters, value) in [
        (
            "core.f64.parse-result",
            vec![ResolvedType::Text],
            ResolvedType::F64,
        ),
        (
            "core.f64.to-i64-result",
            vec![ResolvedType::F64],
            ResolvedType::I64,
        ),
    ] {
        let valid = legacy_signature(parameters, result_record(value.clone()));
        for fields in [
            vec![ResolvedField {
                name: "valid".into(),
                ty: ResolvedType::Bool,
            }],
            vec![ResolvedField {
                name: "value".into(),
                ty: value.clone(),
            }],
            vec![
                ResolvedField {
                    name: "valid".into(),
                    ty: ResolvedType::I64,
                },
                ResolvedField {
                    name: "value".into(),
                    ty: value.clone(),
                },
            ],
            vec![
                ResolvedField {
                    name: "ok".into(),
                    ty: ResolvedType::Bool,
                },
                ResolvedField {
                    name: "value".into(),
                    ty: value.clone(),
                },
            ],
            vec![
                ResolvedField {
                    name: "error".into(),
                    ty: ResolvedType::Text,
                },
                ResolvedField {
                    name: "valid".into(),
                    ty: ResolvedType::Bool,
                },
                ResolvedField {
                    name: "value".into(),
                    ty: value.clone(),
                },
            ],
        ] {
            let mut malformed = valid.clone();
            malformed.result = ResolvedType::Record(fields);
            assert_foreign_signature(implementation, &malformed);
        }
        let (snapshot, external) = canonical_signature(implementation, &valid);
        assert_eq!(
            validate_kernel_intrinsic(&snapshot, &external, &mut 0, 1)
                .unwrap_err()
                .code,
            "kernel_full_work"
        );
    }
}

#[derive(Clone, Copy)]
pub(crate) struct AdmissionIds {
    pub(crate) unchosen: ExpressionId,
    pub(crate) unused: ParameterId,
    pub(crate) external: DeclarationId,
}

/// Small source fixture shared only by kernel/source-loader tests. The false arm and parameter
/// are present in accepted meaning even though the constant true condition never visits them.
pub(crate) fn admission_snapshot(
    floating: bool,
    generation: u16,
) -> (KernelSnapshot, AdmissionIds) {
    let mut snapshot = transport_snapshot();
    snapshot.owners.clear();
    snapshot.types.clear();
    snapshot.root.graph_contract_version = generation;
    let module = ModuleId::migrate(SEED, 1);
    let function = DeclarationId::migrate(SEED, 1);
    let external = DeclarationId::migrate(SEED, 2);
    let unused = ParameterId::migrate(SEED, 10);
    let condition = ExpressionId::migrate(SEED, 0);
    let chosen = ExpressionId::migrate(SEED, 1);
    let unchosen = ExpressionId::migrate(SEED, 2);
    let body = ExpressionId::migrate(SEED, 3);
    let unit = intern(&mut snapshot, TypeForm::Unit);
    let item = if floating {
        intern(&mut snapshot, TypeForm::F64)
    } else {
        unit
    };
    let list = intern(&mut snapshot, TypeForm::List { item });
    let option = intern(&mut snapshot, TypeForm::Option { item: unit });
    let header = |owner, kind| {
        let mut header = OwnerHeader::new(owner, kind);
        header.contract_version = generation;
        header
    };
    for record in [
        OwnerRecord::Module(ModuleRecord {
            header: header(OwnerKey::Module(module), OwnerKind::Module),
            name: name("numeric-admission"),
        }),
        OwnerRecord::Declaration(DeclarationRecord {
            header: header(OwnerKey::Declaration(function), OwnerKind::PureFunction),
            module,
            name: name("ignore"),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                requirement_parameters: Vec::new(),
                effect_parameters: Vec::new(),
                type_parameters: Vec::new(),
                parameters: vec![unused],
                result: item,
                effect: FunctionEffect::Pure,
                body,
            }),
        }),
        OwnerRecord::Parameter(ParameterRecord {
            header: header(OwnerKey::Parameter(unused), OwnerKind::Parameter),
            parent: ParameterParent::Function(function),
            name: name("unused"),
            ty: list,
            use_mode: ParameterUse::Unrestricted,
            resource_requirement: None,
        }),
        OwnerRecord::Declaration(DeclarationRecord {
            header: header(OwnerKey::Declaration(external), OwnerKind::External),
            module,
            name: name("unused-external"),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::External(ExternalDeclaration {
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: option,
                implementation: ImplementationName::new("core.option.none").unwrap(),
            }),
        }),
    ] {
        snapshot.owners.insert(record.owner(), record);
    }
    for (id, operation) in [
        (condition, ExpressionOperation::Bool { value: true }),
        (
            chosen,
            if floating {
                ExpressionOperation::F64 {
                    value: Binary64::from_bits(0x3ff0_0000_0000_0000).unwrap(),
                }
            } else {
                ExpressionOperation::Unit {}
            },
        ),
        (
            unchosen,
            if floating {
                ExpressionOperation::F64 {
                    value: Binary64::from_bits(0x7ff8_0000_0000_0000).unwrap(),
                }
            } else {
                ExpressionOperation::Unit {}
            },
        ),
        (
            body,
            ExpressionOperation::If {
                condition,
                when_true: chosen,
                when_false: unchosen,
            },
        ),
    ] {
        snapshot.owners.insert(
            OwnerKey::Expression(id),
            OwnerRecord::Expression(ExpressionRecord {
                contract_version: generation,
                id,
                operation,
            }),
        );
    }
    snapshot.root.owners = map_root(snapshot.owners.len(), 1);
    validate_full(&snapshot).unwrap();
    (
        snapshot,
        AdmissionIds {
            unchosen,
            unused,
            external,
        },
    )
}

#[test]
fn f64_complete_graph_admission_checks_unused_types_and_unreachable_branch_meaning() {
    let (mut snapshot, ids) = admission_snapshot(false, 16);
    let f64_type = intern(&mut snapshot, TypeForm::F64);
    let list = intern(&mut snapshot, TypeForm::List { item: f64_type });
    let OwnerRecord::Parameter(parameter) = snapshot
        .owners
        .get_mut(&OwnerKey::Parameter(ids.unused))
        .unwrap()
    else {
        panic!("unused parameter");
    };
    let old_list = std::mem::replace(&mut parameter.ty, list);
    snapshot.types.remove(&old_list);
    let errors = validate_full(&snapshot).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "kernel_type_graph_generation"),
        "{errors:?}"
    );

    snapshot.root.graph_contract_version = 17;
    validate_full(&snapshot).unwrap();
    snapshot.types.get_mut(&f64_type).unwrap().contract_version = 10;
    let errors = validate_full(&snapshot).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "kernel_type_contract"),
        "{errors:?}"
    );

    let (mut snapshot, ids) = admission_snapshot(false, 17);
    let OwnerRecord::Expression(expression) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(ids.unchosen))
        .unwrap()
    else {
        panic!("false arm");
    };
    expression.operation = ExpressionOperation::F64 {
        value: Binary64::from_bits(0x7ff8_0000_0000_0000).unwrap(),
    };
    let errors = validate_full(&snapshot).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "kernel_type_if_branches"),
        "{errors:?}"
    );
}

#[test]
fn f64_canonical_owner_decoder_rejects_rehashed_nan_bits_in_an_unreachable_arm() {
    let (snapshot, ids) = admission_snapshot(true, 17);
    let owner = &snapshot.owners[&OwnerKey::Expression(ids.unchosen)];
    let (_, bytes) = encode_owner(owner).unwrap();
    let expected = [0, 0, 0, 0, 0, 0, 0xf8, 0x7f];
    let positions = bytes
        .windows(8)
        .enumerate()
        .filter_map(|(index, actual)| (actual == expected).then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(positions.len(), 1);
    for malformed in [
        [1, 0, 0, 0, 0, 0, 0xf8, 0x7f],
        [1, 0, 0, 0, 0, 0, 0xf0, 0x7f],
        [0, 0, 0, 0, 0, 0, 0xf8, 0xff],
    ] {
        let mut hostile = bytes.clone();
        hostile[positions[0]..positions[0] + 8].copy_from_slice(&malformed);
        let end = hostile.len() - 32;
        let mut checksum = blake3::Hasher::new_derive_key("lkjscript.kernel.owner-envelope.v17");
        checksum.update(&(end as u64).to_be_bytes());
        checksum.update(&hostile[..end]);
        hostile[end..].copy_from_slice(checksum.finalize().as_bytes());
        assert_eq!(
            decode_owner(
                &hostile,
                owner.owner(),
                owner.kind(),
                OwnerObjectDigest::of(&hostile)
            )
            .unwrap_err()
            .code,
            "packed_decode"
        );
    }
}
