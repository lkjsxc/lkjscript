//! Independent admission of ordinary and owned categories in complete witnesses.
use super::*;
use crate::platform::semantic_id::{DeclarationId, RevisionId, TypeParameterId};

const SOURCE: &str = r#"declarations.begin
(units (module create mixed
  (record create Marker (visibility public)
    (type-parameter create Tag) (field create number (type I64)))
  (owned-contract create DataPipe (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Item (constraint transferable))
    (type-parameter create Phantom (constraint capture-safe transferable))
    (method method_c3000000000000000000000000000001 pair
      (parameters (Item unrestricted) (Self consume))
      (returns (owned-product (field rest Self) (field value Item))))
    (method method_c3000000000000000000000000000002 echo
      (parameters (Item unrestricted) (Self borrow)) (returns Item)))
  (function create pair (visibility public) (effect pure)
    (type-parameter create O (constraint owned))
    (type-parameter create T (constraint transferable))
    (type-parameter create P (constraint capture-safe transferable))
    (parameter create value (type T))
    (parameter create owner (type O) (use consume))
    (returns (owned-product (field rest O) (field value T)))
    (body (pack-owned (type (owned-product (field rest O) (field value T)))
      (field rest (local owner)) (field value (local value)))))
  (function create echo (visibility public) (effect pure)
    (type-parameter create O (constraint owned))
    (type-parameter create T (constraint transferable))
    (type-parameter create P (constraint capture-safe transferable))
    (parameter create value (type T))
    (parameter create owner (type O) (use borrow))
    (returns T) (body (local value)))
  (owned-implementation create Pair (visibility public)
    (type-parameter create O (constraint owned))
    (type-parameter create T (constraint transferable))
    (type-parameter create P (constraint capture-safe transferable))
    (contract DataPipe) (self O) (types T P)
    (method method_c3000000000000000000000000000001 pair (types O T P))
    (method method_c3000000000000000000000000000002 echo (types O T P)))
  (owned-implementation create Concrete (visibility public)
    (contract DataPipe) (self OwnedI64Cell) (types I64 (Marker Bool))
    (method method_c3000000000000000000000000000001 pair
      (types OwnedI64Cell I64 (Marker Bool)))
    (method method_c3000000000000000000000000000002 echo
      (types OwnedI64Cell I64 (Marker Bool))))
  (function create forward (visibility public) (effect pure)
    (type-parameter create O (constraint owned))
    (type-parameter create T (constraint transferable))
    (type-parameter create P (constraint capture-safe transferable))
    (implementation-parameter implparam_c3000000000000000000000000000001 ops DataPipe O (types T P))
    (parameter create value (type T))
    (parameter create owner (type O) (use consume))
    (returns (owned-product (field rest O) (field value T)))
    (body (method-call parameter@forward@implparam_c3000000000000000000000000000001
      DataPipe method_c3000000000000000000000000000001 (local value) (local owner))))
  (owned-implementation create Delegating (visibility public)
    (type-parameter create O (constraint owned))
    (type-parameter create T (constraint transferable))
    (type-parameter create P (constraint capture-safe transferable))
    (implementation-parameter implparam_c3000000000000000000000000000002 base DataPipe O (types T P))
    (contract DataPipe) (self O) (types T P)
    (method method_c3000000000000000000000000000001 forward (types O T P)
      (implementations parameter@Delegating@implparam_c3000000000000000000000000000002))
    (method method_c3000000000000000000000000000002 echo (types O T P)))
  (function create apply (visibility public) (effect pure)
    (parameter create value (type I64))
    (parameter create owner (type OwnedI64Cell) (use consume))
    (returns (owned-product (field rest OwnedI64Cell) (field value I64)))
    (body (implementation-call forward (types OwnedI64Cell I64 Bool)
      (implementations (implementation Delegating (types OwnedI64Cell I64 Bool)
        (implementations (implementation Pair (types OwnedI64Cell I64 Bool)))))
      (local value) (local owner))))))
declarations.end
"#;

fn source() -> KernelSnapshot {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE).unwrap()
}

fn declaration(snapshot: &KernelSnapshot, name: &str) -> DeclarationId {
    snapshot
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(d)) if d.name.as_str() == name => {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}

fn payload<'a>(snapshot: &'a mut KernelSnapshot, name: &str) -> &'a mut DeclarationPayload {
    let id = declaration(snapshot, name);
    let OwnerRecord::Declaration(d) = snapshot.owners.get_mut(&OwnerKey::Declaration(id)).unwrap()
    else {
        unreachable!()
    };
    &mut d.payload
}

fn formal(snapshot: &KernelSnapshot, name: &str, parameter_name: &str) -> TypeParameterId {
    let d = declaration(snapshot, name);
    snapshot
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::TypeParameter(id), OwnerRecord::TypeParameter(p))
                if p.declaration == d && p.name.as_str() == parameter_name =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}

fn parameter(snapshot: &mut KernelSnapshot, id: TypeParameterId) -> &mut TypeParameterRecord {
    let OwnerRecord::TypeParameter(p) = snapshot
        .owners
        .get_mut(&OwnerKey::TypeParameter(id))
        .unwrap()
    else {
        unreachable!()
    };
    p
}

fn intern(snapshot: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    snapshot.types.insert(ty, object);
    ty
}

fn imported(snapshot: &KernelSnapshot) -> KernelSnapshot {
    let mut imported = snapshot.clone();
    let package = imported.root.package_id;
    imported.root.package_id = PackageId::migrate(b"mixed-oracle-import", 0);
    let revision = PackageRevisionDigest::from_bytes([195; 32]);
    let owners = std::mem::take(&mut imported.owners)
        .into_iter()
        .filter_map(|(key, owner)| {
            PackageInterfaceRecord::project_public(&owner)
                .unwrap()
                .map(|record| (key, record))
        })
        .collect::<BTreeMap<_, _>>();
    imported.dependency_types = std::mem::take(&mut imported.types);
    imported
        .dependency_interfaces
        .insert(revision, owners.into());
    imported.dependencies.insert(
        package,
        DependencyRecord {
            graph_contract_version: 33,
            package,
            semantic_revision: RevisionId::from_digest([196; 32]),
            package_revision: revision,
        },
    );
    imported
}

fn rejected(snapshot: &KernelSnapshot, reason: &str) {
    assert!(!accepts(snapshot), "{reason}");
    assert!(
        !accepts(&imported(snapshot)),
        "unused imported inventory: {reason}"
    );
}

#[test]
fn mixed_contract_oracle_admits_generic_data_results_and_forwarded_phantom_witnesses() {
    let snapshot = source();
    assert!(accepts(&snapshot));
    assert!(accepts(&imported(&snapshot)));
    for item in ["Bool", "(list (option I64))", "(Marker Bool)"] {
        let authored = SOURCE
            .replace(
                "(types I64 (Marker Bool))",
                &format!("(types {item} (Marker Bool))"),
            )
            .replace(
                "(types OwnedI64Cell I64 (Marker Bool))",
                &format!("(types OwnedI64Cell {item} (Marker Bool))"),
            );
        let snapshot =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
                &authored,
            )
            .unwrap();
        assert!(accepts(&snapshot), "ordinary implementation actual {item}");
        assert!(accepts(&imported(&snapshot)));
    }
}

#[test]
fn mixed_contract_oracle_rejects_category_confusion_and_complete_nominal_phantom_attacks() {
    let baseline = source();
    for position in [0, 1] {
        let mut changed = baseline.clone();
        let owner = intern(&mut changed, TypeForm::OwnedI64Cell);
        let DeclarationPayload::OwnedImplementation(i) = payload(&mut changed, "Concrete") else {
            unreachable!()
        };
        i.type_arguments[position] = owner;
        rejected(
            &changed,
            "a transferable owner cannot satisfy an ordinary formal",
        );
    }
    let mut changed = baseline.clone();
    let owner = intern(&mut changed, TypeForm::OwnedI64Cell);
    let marker = DeclarationReference {
        package: changed.root.package_id,
        declaration: declaration(&changed, "Marker"),
    };
    let bad = intern(
        &mut changed,
        TypeForm::Applied {
            declaration: marker,
            arguments: vec![owner],
        },
    );
    let DeclarationPayload::OwnedImplementation(i) = payload(&mut changed, "Concrete") else {
        unreachable!()
    };
    i.type_arguments[1] = bad;
    rejected(
        &changed,
        "ordinary nominal admission includes unused actuals",
    );
    let mut changed = baseline.clone();
    let data = intern(&mut changed, TypeForm::I64);
    let DeclarationPayload::OwnedImplementation(i) = payload(&mut changed, "Concrete") else {
        unreachable!()
    };
    i.self_type = data;
    rejected(&changed, "Self retains the exact owned category");
}

#[test]
fn mixed_contract_oracle_rejects_mapping_and_prerequisite_argument_substitutions() {
    let baseline = source();
    for target in ["Pair", "Delegating"] {
        let mut changed = baseline.clone();
        let owner = intern(&mut changed, TypeForm::OwnedI64Cell);
        let DeclarationPayload::OwnedImplementation(i) = payload(&mut changed, target) else {
            unreachable!()
        };
        i.methods[0].type_arguments[2] = owner;
        rejected(
            &changed,
            "unused mapped ordinary formal requires complete admission",
        );
    }
    let mut changed = baseline.clone();
    let owner = intern(&mut changed, TypeForm::OwnedI64Cell);
    let DeclarationPayload::OwnedImplementation(i) = payload(&mut changed, "Delegating") else {
        unreachable!()
    };
    i.implementation_parameters[0].type_arguments[1] = owner;
    rejected(
        &changed,
        "forwarded prerequisite ordinary arguments retain their formal category",
    );
    let mut changed = baseline.clone();
    let wrong_scope = declaration(&changed, "pair");
    let id = formal(&changed, "Pair", "P");
    parameter(&mut changed, id).declaration = wrong_scope;
    rejected(
        &changed,
        "phantom scheme formal retains exact declaring scope",
    );
    for (name, formal_name, bounds) in [
        (
            "DataPipe",
            "Self",
            TypeParameterConstraints::OwnedTransferable,
        ),
        (
            "DataPipe",
            "Phantom",
            TypeParameterConstraints::OwnedTransferable,
        ),
        ("Pair", "P", TypeParameterConstraints::CaptureSafe),
        ("pair", "P", TypeParameterConstraints::CaptureSafe),
    ] {
        let mut changed = baseline.clone();
        let id = formal(&changed, name, formal_name);
        parameter(&mut changed, id).constraints = bounds;
        rejected(
            &changed,
            "witness formal bounds must match the admitted closed set",
        );
    }
}

#[test]
fn mixed_contract_oracle_rejects_ordinary_modes_and_forged_borrowed_data_results() {
    let baseline = source();
    for attack in 0..4 {
        let mut changed = baseline.clone();
        let data = intern(&mut changed, TypeForm::I64);
        let DeclarationPayload::OwnedContract(c) = payload(&mut changed, "DataPipe") else {
            unreachable!()
        };
        match attack {
            0 => c.methods[0].parameters[0].use_mode = ParameterUse::Consume,
            1 => c.methods[0].parameters.swap(0, 1),
            2 => {
                c.methods[0].result = data;
                c.methods[0].result_borrow = Some(1);
            }
            _ => c.methods[1].result_borrow = Some(1),
        }
        rejected(
            &changed,
            "ordinary data is unrestricted and cannot claim borrowed owner provenance",
        );
    }
}

#[test]
fn mixed_contract_oracle_rejects_predecessor_semantic_generations_at_every_declaring_boundary() {
    let baseline = source();
    let mut changed = baseline.clone();
    changed.root.graph_contract_version = 32;
    rejected(&changed, "Graph32 cannot admit mixed storage contracts");
    for name in ["DataPipe", "Pair", "Concrete", "Delegating", "forward"] {
        let mut changed = baseline.clone();
        let id = declaration(&changed, name);
        let OwnerRecord::Declaration(d) =
            changed.owners.get_mut(&OwnerKey::Declaration(id)).unwrap()
        else {
            unreachable!()
        };
        d.header.contract_version = 32;
        rejected(
            &changed,
            "mixed application semantics require Graph33 declaring scope",
        );
    }
    for name in ["DataPipe", "Pair", "Delegating"] {
        let mut changed = baseline.clone();
        let id = formal(
            &changed,
            name,
            if name == "DataPipe" { "Phantom" } else { "P" },
        );
        parameter(&mut changed, id).header.contract_version = 32;
        rejected(
            &changed,
            "ordinary witness formals require their own Graph33 generation",
        );
    }
    for name in ["forward", "apply"] {
        let mut changed = baseline.clone();
        let DeclarationPayload::Function(f) = payload(&mut changed, name) else {
            unreachable!()
        };
        let body = f.body;
        let OwnerRecord::Expression(expression) =
            changed.owners.get_mut(&OwnerKey::Expression(body)).unwrap()
        else {
            unreachable!()
        };
        expression.contract_version = 32;
        assert!(
            !accepts(&changed),
            "mixed invocation expression requires its own Graph33 generation"
        );
    }
}
