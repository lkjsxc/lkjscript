//! Production scheme admission is universal; favorable concrete arguments cannot repair it.
use super::*;
use crate::platform::semantic_id::MethodId;
use std::collections::BTreeMap;

const METHOD: &str = "method_b9000000000000000000000000000001";
const SOURCE: &str = r#"declarations.begin
(units (module create scheme
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Item (constraint owned))
    (method method_b9000000000000000000000000000001 at
      (parameters (I64 unrestricted) (Self borrow))
      (returns Item (borrow-from 1))))
  (function create at (visibility public) (effect pure)
    (type-parameter create U (constraint owned))
    (parameter create index (type I64))
    (parameter create values (type (owned-sequence U)) (use borrow))
    (returns U (borrow-from values))
    (body (borrow-owned-item (type (owned-sequence U)) (local values)
      (index (local index)) (binding view (type U)) (in (local view)))))
  (owned-implementation create Flat (visibility public)
    (type-parameter create T (constraint owned))
    (type-parameter create Phantom (constraint owned))
    (contract Reader) (self (owned-sequence T)) (types T)
    (method method_b9000000000000000000000000000001 at (types T)))
  (owned-implementation create FlatBuffer (visibility public)
    (contract Reader) (self (owned-sequence ByteBuffer)) (types ByteBuffer)
    (method method_b9000000000000000000000000000001 at (types ByteBuffer)))
  (function create probe (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create index (type I64))
    (parameter create values (type (owned-sequence T)) (use borrow))
    (returns T (borrow-from values))
    (body (borrow-call
      (method-call (implementation Flat (types T T)) Reader
        method_b9000000000000000000000000000001 (local index) (local values))
      (binding view (type T)) (in (local view)))))
  (function create forward (visibility public) (effect pure)
    (type-parameter create W (constraint owned))
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_b9000000000000000000000000000001 reader Reader W (types T))
    (parameter create index (type I64))
    (parameter create values (type W) (use borrow))
    (returns T (borrow-from values))
    (body (borrow-call
      (method-call parameter@forward@implparam_b9000000000000000000000000000001 Reader
        method_b9000000000000000000000000000001 (local index) (local values))
      (binding view (type T)) (in (local view)))))))
declarations.end"#;

fn fixture() -> KernelSnapshot {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
        .expect("schemes, mappings and symbolic borrowed applications author through the product")
}

fn declaration(snapshot: &KernelSnapshot, name: &str) -> DeclarationReference {
    let record = snapshot
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Declaration(d) if d.name.as_str() == name => Some(d),
            _ => None,
        })
        .unwrap();
    let OwnerKey::Declaration(declaration) = record.header.owner else {
        unreachable!()
    };
    DeclarationReference {
        package: snapshot.root.package_id,
        declaration,
    }
}

fn implementation(snapshot: &KernelSnapshot) -> OwnedImplementation {
    let reference = declaration(snapshot, "Flat");
    let OwnerRecord::Declaration(d) =
        &snapshot.owners[&OwnerKey::Declaration(reference.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &d.payload else {
        unreachable!()
    };
    i.clone()
}

fn ty(snapshot: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    snapshot.types.insert(digest, object);
    digest
}

#[test]
fn generic_implementation_admits_symbolic_borrowing_and_materializes_nested_applications() {
    let mut source = fixture();
    let cell = ty(&mut source, TypeForm::OwnedI64Cell);
    let buffer = ty(&mut source, TypeForm::ByteBuffer);
    let nested = ty(
        &mut source,
        TypeForm::OwnedProduct {
            fields: vec![StructuralTypeField {
                name: Name::new("payload").unwrap(),
                ty: cell,
            }],
        },
    );
    let operand = ImplementationOperand::Concrete {
        implementation: declaration(&source, "Flat"),
        type_arguments: vec![nested, buffer],
    };
    let applied = owned_contract::method_signature(
        &source,
        &operand,
        declaration(&source, "Reader"),
        METHOD.parse::<MethodId>().unwrap(),
        None,
    )
    .unwrap();
    assert_eq!(applied.method.result, nested);
    assert_eq!(applied.method.result_borrow, Some(1));
    assert_eq!(applied.method.parameters[1].use_mode, ParameterUse::Borrow);
    let self_type = applied.method.parameters[1].ty;
    assert!(
        !source.types.contains_key(&self_type),
        "new structural applications were absent from source"
    );
    assert_eq!(
        applied.types[&self_type].form,
        TypeForm::OwnedSequence { item: nested }
    );
    let overlay = owned_contract::AppliedTypeRead {
        read: &source,
        types: &applied.types,
    };
    owned_product::validate_in_scope(&overlay, self_type, None).unwrap();
    let monomorphic = ImplementationOperand::Concrete {
        implementation: declaration(&source, "FlatBuffer"),
        type_arguments: vec![],
    };
    let selected = owned_contract::method_signature(
        &source,
        &monomorphic,
        declaration(&source, "Reader"),
        METHOD.parse().unwrap(),
        None,
    )
    .unwrap();
    assert_eq!(selected.method.result, buffer);
    assert_eq!(selected.method.result_borrow, Some(1));
}

#[test]
fn generic_implementation_checks_phantom_constraints_and_exact_ordered_witness_arguments() {
    let mut source = fixture();
    let cell = ty(&mut source, TypeForm::OwnedI64Cell);
    let buffer = ty(&mut source, TypeForm::ByteBuffer);
    let ordinary = ty(&mut source, TypeForm::I64);
    let cells = ty(&mut source, TypeForm::OwnedSequence { item: cell });
    let selected = declaration(&source, "Flat");
    for arguments in [vec![cell], vec![cell, ordinary], vec![cell, buffer, cell]] {
        let operand = ImplementationOperand::Concrete {
            implementation: selected,
            type_arguments: arguments,
        };
        assert!(owned_contract::witness_contract(&source, &operand, None).is_err());
    }
    let correct = ImplementationOperand::Concrete {
        implementation: selected,
        type_arguments: vec![cell, buffer],
    };
    let reversed = ImplementationOperand::Concrete {
        implementation: selected,
        type_arguments: vec![buffer, cell],
    };
    let forward = declaration(&source, "forward");
    owned_contract::validate_application(
        &source,
        forward,
        &[cells, cell],
        std::slice::from_ref(&correct),
        None,
    )
    .unwrap();
    assert!(
        owned_contract::validate_application(&source, forward, &[cells, cell], &[reversed], None)
            .is_err()
    );
    let other_phantom = ImplementationOperand::Concrete {
        implementation: selected,
        type_arguments: vec![cell, cell],
    };
    assert_ne!(
        correct, other_phantom,
        "unused arguments retain application identity"
    );
    owned_contract::validate_application(&source, forward, &[cells, cell], &[other_phantom], None)
        .unwrap();
}

#[test]
fn generic_implementation_rejects_invalid_symbolic_mapping_before_equal_concrete_instantiation() {
    let source = fixture();
    let reference = declaration(&source, "Flat");
    let mut mapped = implementation(&source);
    let phantom = source
        .types
        .iter()
        .find_map(|(digest, object)| match object.form {
            TypeForm::TypeParameter { parameter } if parameter == mapped.type_parameters[1] => {
                Some(*digest)
            }
            _ => None,
        });
    // A phantom parameter need not have a type object in accepted source. Materialize
    // it only as hostile test input; both arguments could later be the same carrier.
    let object = TypeObject::new(TypeForm::TypeParameter {
        parameter: mapped.type_parameters[1],
    })
    .unwrap();
    let digest = phantom.unwrap_or_else(|| encode_type_object(&object).unwrap().0);
    let mut altered = source.clone();
    altered.types.insert(digest, object);
    mapped.methods[0].type_arguments = vec![digest];
    assert!(owned_contract::validate_implementation_at(&altered, reference, &mapped).is_err());
    mapped.methods[0].type_arguments.clear();
    assert!(owned_contract::validate_implementation_at(&altered, reference, &mapped).is_err());
}

#[test]
fn generic_implementation_rejects_foreign_constraint_owners_and_wrong_borrow_source() {
    let original = fixture();
    let reference = declaration(&original, "Flat");
    let i = implementation(&original);
    for constraint in [
        TypeParameterConstraints::None,
        TypeParameterConstraints::OwnedTransferable,
    ] {
        let mut changed = original.clone();
        let OwnerRecord::TypeParameter(p) = changed
            .owners
            .get_mut(&OwnerKey::TypeParameter(i.type_parameters[1]))
            .unwrap()
        else {
            unreachable!()
        };
        p.constraints = constraint;
        assert!(owned_contract::validate_implementation_at(&changed, reference, &i).is_err());
    }
    let mut changed = original.clone();
    let at = declaration(&changed, "at");
    let OwnerRecord::Declaration(d) = changed
        .owners
        .get_mut(&OwnerKey::Declaration(at.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(f) = &mut d.payload else {
        unreachable!()
    };
    f.result_borrow = Some(f.parameters[0]);
    assert!(owned_contract::validate_implementation_at(&changed, reference, &i).is_err());

    let mut changed = original.clone();
    let OwnerRecord::TypeParameter(p) = changed
        .owners
        .get_mut(&OwnerKey::TypeParameter(i.type_parameters[1]))
        .unwrap()
    else {
        unreachable!()
    };
    p.declaration = at.declaration;
    assert!(owned_contract::validate_implementation_at(&changed, reference, &i).is_err());
}

#[test]
fn generic_implementation_application_roots_retain_unused_arguments() {
    let mut source = fixture();
    let cell = ty(&mut source, TypeForm::OwnedI64Cell);
    let buffer = ty(&mut source, TypeForm::ByteBuffer);
    let operand = ImplementationOperand::Concrete {
        implementation: declaration(&source, "Flat"),
        type_arguments: vec![cell, buffer],
    };
    let expression = ExpressionRecord {
        id: crate::platform::semantic_id::ExpressionId::migrate(b"scheme-root-test", 1),
        contract_version: 28,
        operation: ExpressionOperation::MethodCall {
            witness: operand,
            contract: declaration(&source, "Reader"),
            method: METHOD.parse().unwrap(),
            arguments: vec![],
        },
    };
    assert_eq!(expression.type_roots(), vec![cell, buffer]);
    let mut predecessor = expression;
    predecessor.contract_version = 27;
    assert_eq!(
        predecessor.validate_local().unwrap_err().code,
        "kernel_implementation_scheme_generation"
    );
}

#[test]
fn generic_implementation_derived_type_storage_accepts_exact_capacity_and_refuses_one_byte_less() {
    let source = fixture();
    let i = implementation(&source);
    let bindings = BTreeMap::from([(i.type_parameters[0], i.type_arguments[0])]);
    let mut observed = parallel_types::AppliedTypes::new(&source);
    observed.substitute(i.self_type, &bindings, 0).unwrap();
    let required = observed.metadata_bytes_for_test();
    assert!(required > 0);
    let mut exact = parallel_types::AppliedTypes::with_metadata_limit(&source, required);
    exact.substitute(i.self_type, &bindings, 0).unwrap();
    let mut refused = parallel_types::AppliedTypes::with_metadata_limit(&source, required - 1);
    let failure = refused.substitute(i.self_type, &bindings, 0).unwrap_err();
    assert_eq!(
        failure.class,
        crate::platform::diagnostic::DiagnosticClass::Resource
    );
    assert_eq!(failure.code, "kernel_parallel_type_storage");
    // The owning read remains usable after a refused derived allocation.
    owned_contract::validate_implementation_at(&source, declaration(&source, "Flat"), &i).unwrap();
}
