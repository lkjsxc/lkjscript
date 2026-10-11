//! Ordinary storage items and owner custody retain independent, exact obligations.
use super::*;

const SOURCE: &str = r#"declarations.begin
(units (module create mixed-storage
  (record create Phantom (visibility public)
    (type-parameter create X) (field create marker (type I64)))
  (owned-contract create Indexed (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Item (constraint transferable))
    (type-parameter create PhantomItem (constraint capture-safe transferable))
    (type-parameter create Owner (constraint owned))
    (method method_33000000000000000000000000000001 get
      (parameters (I64 unrestricted) (Self borrow)) (returns Item))
    (method method_33000000000000000000000000000002 replace
      (parameters (I64 unrestricted) (Item unrestricted) (Self consume))
      (returns (owned-product (field rest Self) (field value Item)))))
  (function create get (visibility public) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create index (type I64))
    (parameter create values (type (owned-sequence T)) (use borrow))
    (returns T)
    (body (sequence-get (type (owned-sequence T)) (local values) (index (local index)))))
  (function create replace (visibility public) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create index (type I64)) (parameter create item (type T))
    (parameter create values (type (owned-sequence T)) (use consume))
    (returns (owned-product (field rest (owned-sequence T)) (field value T)))
    (body (sequence-replace (type (owned-sequence T)) (index (local index))
      (local item) (local values))))
  (owned-implementation create Flat (visibility public)
    (type-parameter create T (constraint transferable))
    (type-parameter create P (constraint capture-safe transferable))
    (type-parameter create O (constraint owned transferable))
    (contract Indexed) (self (owned-sequence T)) (types T P O)
    (method method_33000000000000000000000000000001 get (types T))
    (method method_33000000000000000000000000000002 replace (types T)))
  (function create forward-get (visibility public) (effect pure)
    (type-parameter create W (constraint owned))
    (type-parameter create T (constraint transferable))
    (type-parameter create P (constraint capture-safe transferable))
    (type-parameter create O (constraint owned transferable))
    (implementation-parameter implparam_33000000000000000000000000000001 reader Indexed W (types T P O))
    (parameter create index (type I64)) (parameter create values (type W) (use borrow))
    (returns T)
    (body (method-call parameter@forward-get@implparam_33000000000000000000000000000001
      Indexed method_33000000000000000000000000000001 (local index) (local values))))))
declarations.end"#;

fn fixture() -> KernelSnapshot {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
        .expect("mixed contracts author through the public product")
}

fn named(snapshot: &KernelSnapshot, name: &str) -> DeclarationReference {
    let declaration = snapshot
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Declaration(owner) if owner.name.as_str() == name => {
                let OwnerKey::Declaration(id) = owner.header.owner else {
                    unreachable!()
                };
                Some(id)
            }
            _ => None,
        })
        .unwrap();
    DeclarationReference {
        package: snapshot.root.package_id,
        declaration,
    }
}

fn ty(snapshot: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    snapshot.types.insert(digest, object);
    digest
}

fn applied(
    snapshot: &mut KernelSnapshot,
    item: TypeForm,
    phantom: TypeForm,
) -> ImplementationOperand {
    let item = ty(snapshot, item);
    let phantom = ty(snapshot, phantom);
    let owner = ty(snapshot, TypeForm::ByteBuffer);
    ImplementationOperand::Concrete {
        implementation: named(snapshot, "Flat"),
        type_arguments: vec![item, phantom, owner],
        implementations: vec![],
    }
}

fn contract(snapshot: &KernelSnapshot) -> OwnedContract {
    contract_record(snapshot, named(snapshot, "Indexed")).unwrap()
}

fn implementation(snapshot: &KernelSnapshot) -> OwnedImplementation {
    implementation_record(snapshot, named(snapshot, "Flat")).unwrap()
}

#[test]
fn mixed_contracts_admit_ordinary_results_products_and_forwarded_prerequisites() {
    let mut snapshot = fixture();
    let scalar = ty(&mut snapshot, TypeForm::I64);
    let nested = ty(&mut snapshot, TypeForm::Option { item: scalar });
    let operand = applied(
        &mut snapshot,
        TypeForm::List { item: nested },
        TypeForm::Bool,
    );
    let result = method_signature(
        &snapshot,
        &operand,
        named(&snapshot, "Indexed"),
        "method_33000000000000000000000000000001".parse().unwrap(),
        None,
    )
    .unwrap();
    assert!(result.method.result_borrow.is_none());
    assert_eq!(result.method.parameters[1].use_mode, ParameterUse::Borrow);
    let overlay = AppliedTypeRead {
        read: &snapshot,
        types: &result.types,
    };
    assert!(ordinary_closed(&overlay, result.method.result).unwrap());
    assert!(!super::super::memory::direct(&overlay, result.method.result).unwrap());
    let replacement = method_signature(
        &snapshot,
        &operand,
        named(&snapshot, "Indexed"),
        "method_33000000000000000000000000000002".parse().unwrap(),
        None,
    )
    .unwrap();
    assert_eq!(
        replacement.method.parameters[1].use_mode,
        ParameterUse::Unrestricted
    );
    assert_eq!(
        replacement.method.parameters[2].use_mode,
        ParameterUse::Consume
    );
    let overlay = AppliedTypeRead {
        read: &snapshot,
        types: &replacement.types,
    };
    validate_owned_type(&overlay, replacement.method.result, None).unwrap();
}

#[test]
fn mixed_contracts_reject_owned_or_callable_ordinary_actuals_including_phantoms() {
    let mut snapshot = fixture();
    for form in [TypeForm::ByteBuffer, TypeForm::OwnedI64Cell] {
        let operand = applied(&mut snapshot, TypeForm::I64, form.clone());
        assert!(witness_contract(&snapshot, &operand, None).is_err());
        let operand = applied(&mut snapshot, form, TypeForm::Bool);
        assert!(witness_contract(&snapshot, &operand, None).is_err());
    }
    let scalar = ty(&mut snapshot, TypeForm::I64);
    let callback = ty(
        &mut snapshot,
        TypeForm::Function {
            parameters: vec![scalar],
            result: scalar,
        },
    );
    let hidden = TypeForm::Applied {
        declaration: named(&snapshot, "Phantom"),
        arguments: vec![callback],
    };
    let operand = applied(&mut snapshot, TypeForm::I64, hidden);
    assert!(witness_contract(&snapshot, &operand, None).is_err());
}

#[test]
fn mixed_contracts_enforce_exact_self_bounds_modes_and_borrowed_result_sources() {
    let baseline = fixture();
    let reference = named(&baseline, "Indexed");
    let original = contract(&baseline);
    for constraints in [
        TypeParameterConstraints::Transferable,
        TypeParameterConstraints::OwnedTransferable,
    ] {
        let mut snapshot = baseline.clone();
        let Some(OwnerRecord::TypeParameter(parameter)) = snapshot
            .owners
            .get_mut(&OwnerKey::TypeParameter(original.self_parameter))
        else {
            unreachable!()
        };
        parameter.constraints = constraints;
        assert!(validate_contract_at(&snapshot, reference, &original).is_err());
    }
    for constraints in [
        TypeParameterConstraints::None,
        TypeParameterConstraints::CaptureSafe,
        TypeParameterConstraints::OwnedTransferable,
    ] {
        let mut snapshot = baseline.clone();
        let Some(OwnerRecord::TypeParameter(parameter)) = snapshot
            .owners
            .get_mut(&OwnerKey::TypeParameter(original.type_parameters[0]))
        else {
            unreachable!()
        };
        parameter.constraints = constraints;
        assert!(validate_contract_at(&snapshot, reference, &original).is_err());
    }
    let mut changed = original.clone();
    changed.methods[1].parameters[1].use_mode = ParameterUse::Consume;
    assert!(validate_contract_at(&baseline, reference, &changed).is_err());
    let mut changed = original.clone();
    changed.methods[1].parameters.swap(1, 2);
    assert!(validate_contract_at(&baseline, reference, &changed).is_err());
    let mut changed = original;
    changed.methods[0].result_borrow = Some(1);
    assert!(validate_contract_at(&baseline, reference, &changed).is_err());
}

#[test]
fn mixed_implementation_maps_preserve_formal_categories_scope_and_unused_obligations() {
    let baseline = fixture();
    let reference = named(&baseline, "Flat");
    for constraints in [
        TypeParameterConstraints::None,
        TypeParameterConstraints::CaptureSafe,
        TypeParameterConstraints::Owned,
    ] {
        let mut snapshot = baseline.clone();
        let scheme = implementation(&snapshot);
        let Some(OwnerRecord::TypeParameter(owner)) = snapshot
            .owners
            .get_mut(&OwnerKey::TypeParameter(scheme.type_parameters[0]))
        else {
            unreachable!()
        };
        owner.constraints = constraints;
        assert!(validate_implementation_at(&snapshot, reference, &scheme).is_err());
    }
    for constraints in [
        TypeParameterConstraints::None,
        TypeParameterConstraints::CaptureSafe,
        TypeParameterConstraints::OwnedTransferable,
    ] {
        let mut snapshot = baseline.clone();
        let target = function_contract(&snapshot, named(&snapshot, "get")).unwrap();
        let Some(OwnerRecord::TypeParameter(owner)) = snapshot
            .owners
            .get_mut(&OwnerKey::TypeParameter(target.type_parameters[0]))
        else {
            unreachable!()
        };
        owner.constraints = constraints;
        assert!(
            validate_implementation_at(&snapshot, reference, &implementation(&snapshot)).is_err()
        );
    }
    let mut scheme = implementation(&baseline);
    scheme.type_arguments.swap(0, 2);
    assert!(validate_implementation_at(&baseline, reference, &scheme).is_err());
    let source = SOURCE.replace(
        "(contract Indexed) (self (owned-sequence T))",
        "(type-parameter create Unused (constraint transferable))\n    (contract Indexed) (self (owned-sequence T))",
    );
    let mut snapshot =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(&source)
            .unwrap();
    let mut operand = applied(&mut snapshot, TypeForm::I64, TypeForm::Bool);
    let invalid = ty(&mut snapshot, TypeForm::ByteBuffer);
    let ImplementationOperand::Concrete { type_arguments, .. } = &mut operand else {
        unreachable!()
    };
    type_arguments.push(invalid);
    assert!(witness_contract(&snapshot, &operand, None).is_err());
    let valid = ty(&mut snapshot, TypeForm::Text);
    let ImplementationOperand::Concrete { type_arguments, .. } = &mut operand else {
        unreachable!()
    };
    type_arguments[3] = valid;
    witness_contract(&snapshot, &operand, None).unwrap();
}

#[test]
fn mixed_contracts_admit_only_exact_scope_and_independent_graph33_headers() {
    let baseline = fixture();
    for name in ["Indexed", "Flat", "forward-get"] {
        let mut snapshot = baseline.clone();
        let reference = named(&snapshot, name);
        let Some(OwnerRecord::Declaration(owner)) = snapshot
            .owners
            .get_mut(&OwnerKey::Declaration(reference.declaration))
        else {
            unreachable!()
        };
        owner.header.contract_version = 32;
        let payload = owner.payload.clone();
        let rejected = match payload {
            DeclarationPayload::OwnedContract(_) => contract_record(&snapshot, reference).is_err(),
            DeclarationPayload::OwnedImplementation(_) => {
                implementation_record(&snapshot, reference).is_err()
            }
            DeclarationPayload::Function(_) => {
                let signature = function_contract(&snapshot, reference).unwrap();
                validate_witness_parameters(
                    &snapshot,
                    &signature.implementation_parameters,
                    reference,
                    &OwnedAssociatedAdmission::new(),
                )
                .is_err()
            }
            _ => unreachable!(),
        };
        assert!(
            rejected,
            "{name} declaration must retain Graph 33 authority"
        );
    }
    for name in ["Indexed", "Flat"] {
        let mut snapshot = baseline.clone();
        let reference = named(&snapshot, name);
        let parameter = match name {
            "Indexed" => contract(&snapshot).type_parameters[1],
            _ => implementation(&snapshot).type_parameters[1],
        };
        let Some(OwnerRecord::TypeParameter(owner)) =
            snapshot.owners.get_mut(&OwnerKey::TypeParameter(parameter))
        else {
            unreachable!()
        };
        owner.header.contract_version = 32;
        let rejected = if name == "Indexed" {
            validate_contract_at(&snapshot, reference, &contract(&snapshot)).is_err()
        } else {
            validate_implementation_at(&snapshot, reference, &implementation(&snapshot)).is_err()
        };
        assert!(
            rejected,
            "{name} unused ordinary formal retains its own generation"
        );
    }
    let mut calls = 0;
    for owner in baseline.owners.values() {
        if let OwnerRecord::Expression(expression) = owner
            && matches!(expression.operation, ExpressionOperation::MethodCall { .. })
        {
            calls += 1;
            let mut downgraded = expression.clone();
            downgraded.contract_version = 32;
            assert!(require_expression_generation(&baseline, &downgraded).is_err());
        }
    }
    assert!(calls > 0);
    let mut snapshot = baseline.clone();
    let indexed = named(&snapshot, "Indexed");
    let parameter = contract(&snapshot).type_parameters[0];
    let foreign = named(&snapshot, "get");
    let Some(OwnerRecord::TypeParameter(owner)) =
        snapshot.owners.get_mut(&OwnerKey::TypeParameter(parameter))
    else {
        unreachable!()
    };
    owner.declaration = foreign.declaration;
    assert!(super::super::transfer::ordinary_assumptions_in(&snapshot, Some(indexed)).is_err());
}
