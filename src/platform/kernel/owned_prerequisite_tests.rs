//! Semantic tests use independently authored meaning and fixed malformed obligations.
use super::*;

const SOURCE: &str = r#"declarations.begin
(units (module create prerequisites
  (external create scalar-read (visibility private) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_b9000000000000000000000000000001 read
      (parameters (Self borrow)) (returns I64)))
  (function create cell-read (visibility public) (effect pure)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64)
    (body (call scalar-read (local value))))
  (function create forward (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_b9000000000000000000000000000001 reader Reader T)
    (parameter create value (type T) (use borrow)) (returns I64)
    (body (method-call parameter@forward@implparam_b9000000000000000000000000000001
      Reader method_b9000000000000000000000000000001 (local value))))
  (owned-implementation create Leaf (visibility public) (contract Reader) (self OwnedI64Cell)
    (method method_b9000000000000000000000000000001 cell-read))
  (owned-implementation create Delegating (visibility public)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_b9000000000000000000000000000002 reader Reader T)
    (contract Reader) (self T)
    (method method_b9000000000000000000000000000001 forward (types T)
      (implementations parameter@Delegating@implparam_b9000000000000000000000000000002)))))
declarations.end
"#;

fn author(source: &str) -> KernelSnapshot {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source).unwrap()
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

fn implementation(
    snapshot: &KernelSnapshot,
    reference: DeclarationReference,
) -> OwnedImplementation {
    let Some(OwnerRecord::Declaration(owner)) = snapshot
        .owners
        .get(&OwnerKey::Declaration(reference.declaration))
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(implementation) = &owner.payload else {
        unreachable!()
    };
    implementation.clone()
}

fn set_implementation(
    snapshot: &mut KernelSnapshot,
    reference: DeclarationReference,
    implementation: OwnedImplementation,
) {
    let Some(OwnerRecord::Declaration(owner)) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(reference.declaration))
    else {
        unreachable!()
    };
    owner.payload = DeclarationPayload::OwnedImplementation(implementation);
}

fn cell(snapshot: &KernelSnapshot) -> TypeObjectDigest {
    snapshot
        .types
        .iter()
        .find_map(|(digest, object)| {
            matches!(object.form, TypeForm::OwnedI64Cell).then_some(*digest)
        })
        .unwrap()
}

fn applied(snapshot: &KernelSnapshot, depth: usize) -> ImplementationOperand {
    let mut operand = ImplementationOperand::Concrete {
        implementation: named(snapshot, "Leaf"),
        type_arguments: Vec::new(),
        implementations: Vec::new(),
    };
    for _ in 0..depth {
        operand = ImplementationOperand::Concrete {
            implementation: named(snapshot, "Delegating"),
            type_arguments: vec![cell(snapshot)],
            implementations: vec![operand],
        };
    }
    operand
}

#[test]
fn owned_prerequisite_semantics_admit_symbolic_maps_and_nested_applications() {
    let snapshot = author(SOURCE);
    let reference = named(&snapshot, "Delegating");
    validate_implementation_at(&snapshot, reference, &implementation(&snapshot, reference))
        .unwrap();
    let result = witness_contract(&snapshot, &applied(&snapshot, 3), None).unwrap();
    assert_eq!(result.contract, named(&snapshot, "Reader"));
    assert_eq!(result.self_type, cell(&snapshot));
    assert!(result.type_arguments.is_empty());
}

#[test]
fn owned_prerequisite_semantics_check_unused_parameter_self_and_map_scope() {
    let baseline = author(SOURCE);
    let reference = named(&baseline, "Delegating");

    let mut changed = baseline.clone();
    let mut scheme = implementation(&changed, reference);
    let mut unused = scheme.implementation_parameters[0].clone();
    unused.id = ImplementationParameterId::migrate(b"unused-prerequisite", 0);
    unused.name = Name::new("unused").unwrap();
    unused.self_type = cell(&changed);
    scheme.implementation_parameters.push(unused);
    set_implementation(&mut changed, reference, scheme.clone());
    assert!(validate_implementation_at(&changed, reference, &scheme).is_err());

    let mut changed = baseline.clone();
    let mut scheme = implementation(&changed, reference);
    let ImplementationOperand::Parameter { scope, .. } = &mut scheme.methods[0].implementations[0]
    else {
        unreachable!()
    };
    *scope = named(&baseline, "forward");
    set_implementation(&mut changed, reference, scheme.clone());
    assert!(validate_implementation_at(&changed, reference, &scheme).is_err());

    let mut changed = baseline.clone();
    let mut scheme = implementation(&changed, reference);
    scheme.methods[0].implementations.clear();
    set_implementation(&mut changed, reference, scheme.clone());
    assert!(validate_implementation_at(&changed, reference, &scheme).is_err());
}

#[test]
fn owned_prerequisite_semantics_reject_lexical_construction_in_maps_and_missing_nested_actuals() {
    let baseline = author(SOURCE);
    let reference = named(&baseline, "Delegating");
    let mut changed = baseline.clone();
    let mut scheme = implementation(&changed, reference);
    let forwarded = scheme.methods[0].implementations[0].clone();
    scheme.methods[0].implementations[0] = ImplementationOperand::Concrete {
        implementation: reference,
        type_arguments: vec![scheme.self_type],
        implementations: vec![forwarded],
    };
    set_implementation(&mut changed, reference, scheme.clone());
    assert!(validate_implementation_at(&changed, reference, &scheme).is_err());

    let mut operand = applied(&baseline, 2);
    let ImplementationOperand::Concrete {
        implementations, ..
    } = &mut operand
    else {
        unreachable!()
    };
    let ImplementationOperand::Concrete {
        implementations, ..
    } = &mut implementations[0]
    else {
        unreachable!()
    };
    implementations.clear();
    assert!(witness_contract(&baseline, &operand, None).is_err());
}

#[test]
fn owned_prerequisite_semantics_admit_finite_closed_map_cycles_and_check_unused_maps() {
    let source = SOURCE.replace(
        "  (owned-implementation create Delegating",
        r#"
  (owned-implementation create A (visibility public) (contract Reader) (self OwnedI64Cell)
    (method method_b9000000000000000000000000000001 forward (types OwnedI64Cell)
      (implementations (implementation B))))
  (owned-implementation create B (visibility public) (contract Reader) (self OwnedI64Cell)
    (method method_b9000000000000000000000000000001 forward (types OwnedI64Cell)
      (implementations (implementation A))))
  (owned-implementation create Delegating"#,
    );
    let baseline = author(&source);
    let reference = named(&baseline, "A");
    validate_implementation_at(&baseline, reference, &implementation(&baseline, reference))
        .unwrap();

    let mut changed = baseline.clone();
    let target = named(&baseline, "B");
    let mut scheme = implementation(&changed, target);
    scheme.methods[0].implementations.clear();
    set_implementation(&mut changed, target, scheme);
    assert!(
        validate_implementation_at(&changed, reference, &implementation(&changed, reference))
            .is_err()
    );
}
