//! Complete imported declarations retain category and generation even for phantom formals.
use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::*;

fn fixture(
    implementation: bool,
    bound: TypeParameterConstraints,
) -> (
    PackageId,
    BTreeMap<OwnerKey, PackageInterfaceOwner>,
    BTreeMap<TypeObjectDigest, TypeObject>,
) {
    let seed = b"mixed-storage-interface-phantom";
    let package = PackageId::migrate(seed, 0);
    let declaration = DeclarationId::migrate(seed, 0);
    let self_parameter = TypeParameterId::migrate(seed, 0);
    let phantom = TypeParameterId::migrate(seed, 1);
    let object = TypeObject::new(TypeForm::I64).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    let storage = TypeObject::new(TypeForm::OwnedI64Cell).unwrap();
    let storage_type = encode_type_object(&storage).unwrap().0;
    let reference = DeclarationReference {
        package: PackageId::migrate(seed, 1),
        declaration,
    };
    let payload = if implementation {
        PackageInterfaceDeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: vec![phantom],
            implementation_parameters: vec![],
            contract: reference,
            self_type: storage_type,
            type_arguments: vec![],
            methods: vec![OwnedMethodImplementation {
                read_scope_arguments: vec![],
                method: MethodId::migrate(seed, 0),
                function: reference,
                type_arguments: vec![],
                implementations: vec![],
            }],
            associated_types: Vec::new(),
        })
    } else {
        PackageInterfaceDeclarationPayload::OwnedContract(OwnedContract {
            self_parameter,
            type_parameters: vec![phantom],
            methods: vec![OwnedMethod {
                id: MethodId::migrate(seed, 0),
                name: Name::new("seven").unwrap(),
                parameters: vec![],
                result: CallableResult {
                    ty,
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: vec![],
                read_scope_sources: vec![],
                effect: FunctionEffect::Pure,
            }],
            associated_types: Vec::new(),
        })
    };
    let mut owners = BTreeMap::from([
        (
            OwnerKey::Declaration(declaration),
            PackageInterfaceOwner {
                contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                record: PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                    header: OwnerHeader::new(
                        OwnerKey::Declaration(declaration),
                        if implementation {
                            OwnerKind::OwnedImplementation
                        } else {
                            OwnerKind::OwnedContract
                        },
                    ),
                    name: Name::new("PhantomStorage").unwrap(),
                    payload,
                }),
            },
        ),
        (
            OwnerKey::TypeParameter(phantom),
            PackageInterfaceOwner {
                contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                record: PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
                    header: OwnerHeader::new(
                        OwnerKey::TypeParameter(phantom),
                        OwnerKind::TypeParameter,
                    ),
                    declaration,
                    name: Name::new("Phantom").unwrap(),
                    constraints: bound,
                }),
            },
        ),
    ]);
    if !implementation {
        owners.insert(
            OwnerKey::TypeParameter(self_parameter),
            PackageInterfaceOwner {
                contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                record: PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
                    header: OwnerHeader::new(
                        OwnerKey::TypeParameter(self_parameter),
                        OwnerKind::TypeParameter,
                    ),
                    declaration,
                    name: Name::new("Self").unwrap(),
                    constraints: TypeParameterConstraints::Owned,
                }),
            },
        );
    }
    (
        package,
        owners,
        BTreeMap::from([(ty, object), (storage_type, storage)]),
    )
}

fn rehash(owners: &mut BTreeMap<OwnerKey, PackageInterfaceOwner>) {
    for (owner, value) in owners.iter_mut() {
        let (digest, bytes) = value.encode().unwrap();
        *value = PackageInterfaceOwner::decode(&bytes, *owner, digest).unwrap();
    }
}

#[test]
fn mixed_storage_interface_admits_exact_supported_phantom_bounds() {
    use TypeParameterConstraints as C;
    for implementation in [false, true] {
        for bound in [
            C::Owned,
            C::Transferable,
            C::CaptureSafeTransferable,
            C::OwnedTransferable,
            C::OwnedShareable,
            C::OwnedTransferableShareable,
            C::None,
            C::CaptureSafe,
        ] {
            let (package, mut owners, types) = fixture(implementation, bound);
            rehash(&mut owners);
            let admitted = validate_owner_closure(package, &owners, &types).is_ok();
            let expected = if implementation {
                bound.has_owned() || matches!(bound, C::Transferable | C::CaptureSafeTransferable)
            } else {
                matches!(
                    bound,
                    C::Owned | C::Transferable | C::CaptureSafeTransferable
                )
            };
            assert_eq!(
                admitted, expected,
                "implementation={implementation}, bound={bound:?}"
            );
        }
    }
}

#[test]
fn contract_self_retains_exact_owned_bound_even_with_ordinary_extras() {
    use TypeParameterConstraints as C;
    let (package, owners, types) = fixture(false, C::Transferable);
    for bound in [
        C::Transferable,
        C::CaptureSafeTransferable,
        C::OwnedTransferable,
    ] {
        let mut forged = owners.clone();
        for value in forged.values_mut() {
            if let PackageInterfaceRecord::TypeParameter(record) = &mut value.record
                && record.name.as_str() == "Self"
            {
                record.constraints = bound;
            }
        }
        rehash(&mut forged);
        assert!(
            validate_owner_closure(package, &forged, &types).is_err(),
            "{bound:?}"
        );
    }
}
