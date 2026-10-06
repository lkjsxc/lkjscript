//! Independent admission of complete structured contracts and phantom witness arguments.
use super::*;
use crate::platform::semantic_id::{DeclarationId, RevisionId};

const SOURCE: &str = r#"declarations.begin
(units (module create structured
  (owned-contract create Pipe (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Item (constraint owned))
    (type-parameter create Phantom (constraint owned))
    (method method_a6000000000000000000000000000001 relay
      (parameters ((owned-choice
        (case empty Self) (case item (owned-product (field rest Self) (field value (owned-sequence Item))))) consume))
      (returns (owned-choice
        (case empty Self) (case item (owned-product (field rest Self) (field value (owned-sequence Item))))))))
  (function create concrete-relay (visibility public)
    (parameter create owner (type (owned-choice
      (case empty OwnedI64Cell) (case item (owned-product (field rest OwnedI64Cell) (field value (owned-sequence ByteBuffer))))))
      (use consume))
    (returns (owned-choice
      (case empty OwnedI64Cell) (case item (owned-product (field rest OwnedI64Cell) (field value (owned-sequence ByteBuffer))))))
    (effect pure) (body (local owner)))
  (owned-implementation create Concrete (visibility public)
    (contract Pipe) (self OwnedI64Cell) (types ByteBuffer (owned-sequence ByteBuffer))
    (method method_a6000000000000000000000000000001 concrete-relay))
  (function create relay (visibility public)
    (type-parameter create Storage (constraint owned))
    (type-parameter create Item (constraint owned))
    (type-parameter create Phantom (constraint owned))
    (implementation-parameter implparam_a6000000000000000000000000000001 ops Pipe Storage
      (types Item (owned-sequence Phantom)))
    (parameter create owner (type (owned-choice
      (case empty Storage) (case item (owned-product (field rest Storage) (field value (owned-sequence Item))))))
      (use consume))
    (returns (owned-choice
      (case empty Storage) (case item (owned-product (field rest Storage) (field value (owned-sequence Item))))))
    (effect pure)
    (body (method-call parameter@relay@implparam_a6000000000000000000000000000001
      Pipe method_a6000000000000000000000000000001 (local owner))))
  (function create apply (visibility public)
    (parameter create owner (type (owned-choice
      (case empty OwnedI64Cell) (case item (owned-product (field rest OwnedI64Cell) (field value (owned-sequence ByteBuffer))))))
      (use consume))
    (returns (owned-choice
      (case empty OwnedI64Cell) (case item (owned-product (field rest OwnedI64Cell) (field value (owned-sequence ByteBuffer))))))
    (effect pure)
    (body (implementation-call relay (types OwnedI64Cell ByteBuffer ByteBuffer)
      (implementations concrete@Concrete) (local owner))))))
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

fn payload(snapshot: &mut KernelSnapshot, id: DeclarationId) -> &mut DeclarationPayload {
    let OwnerRecord::Declaration(d) = snapshot.owners.get_mut(&OwnerKey::Declaration(id)).unwrap()
    else {
        unreachable!()
    };
    &mut d.payload
}

fn intern(snapshot: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    snapshot.types.insert(digest, object);
    digest
}

#[test]
fn parameterized_contract_oracle_admits_structured_methods_and_composite_phantom_arguments() {
    assert!(accepts(&source()));
}

#[test]
fn parameterized_contract_source_rejects_invalid_unused_arguments_scope_and_task_mapping_mismatch()
{
    source();
    let task_borrow = SOURCE
        .replacen("))) consume))", "))) borrow))", 1)
        .replacen(
            "(owned-sequence Item))))))))",
            "(owned-sequence Item)))))) (effect (task))))",
            1,
        );
    for (name, expected, authored) in [
        (
            "ordinary phantom argument",
            "kernel_owned_contract",
            SOURCE.replacen(
                "(types ByteBuffer (owned-sequence ByteBuffer))",
                "(types ByteBuffer I64)",
                1,
            ),
        ),
        (
            "missing phantom argument",
            "kernel_owned_contract",
            SOURCE.replacen(
                "(types ByteBuffer (owned-sequence ByteBuffer))",
                "(types ByteBuffer)",
                1,
            ),
        ),
        (
            "wrong exact phantom witness",
            "kernel_owned_contract",
            SOURCE.replacen(
                "(types OwnedI64Cell ByteBuffer ByteBuffer)",
                "(types OwnedI64Cell ByteBuffer OwnedI64Cell)",
                1,
            ),
        ),
        (
            "unconstrained contract phantom",
            "kernel_owned_contract",
            SOURCE.replacen(
                "(type-parameter create Phantom (constraint owned))",
                "(type-parameter create Phantom)",
                1,
            ),
        ),
        (
            "foreign contract formal in witness argument",
            "change_unit_unresolved",
            SOURCE.replacen(
                "(types Item (owned-sequence Phantom))",
                "(types Item (owned-sequence Self))",
                1,
            ),
        ),
        (
            "task method has a pure consuming map",
            "kernel_owned_contract",
            task_borrow,
        ),
    ] {
        assert_ne!(
            authored, SOURCE,
            "{name} mutation must affect its native input"
        );
        let failure =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
                &authored,
            )
            .unwrap_err();
        assert!(
            !failure.contains("change_unit_form"),
            "{name} must reach semantic admission: {failure}"
        );
        assert!(
            failure.contains(expected),
            "{name} requires {expected}: {failure}"
        );
    }
}

#[test]
fn parameterized_contract_oracle_rejects_phantom_argument_arity_scope_and_exact_binding_failures() {
    let snapshot = source();
    assert!(accepts(&snapshot));
    let concrete = declaration(&snapshot, "Concrete");
    let pipe = declaration(&snapshot, "Pipe");
    let relay = declaration(&snapshot, "relay");
    let mut changed = snapshot.clone();
    let ordinary = intern(&mut changed, TypeForm::I64);
    let DeclarationPayload::OwnedImplementation(i) = payload(&mut changed, concrete) else {
        unreachable!()
    };
    i.type_arguments[1] = ordinary;
    assert!(!accepts(&changed), "unused actuals still require ownership");

    let mut changed = snapshot.clone();
    let DeclarationPayload::OwnedImplementation(i) = payload(&mut changed, concrete) else {
        unreachable!()
    };
    i.type_arguments.pop();
    assert!(
        !accepts(&changed),
        "contract argument arity includes phantoms"
    );

    let mut changed = snapshot.clone();
    let cell = intern(&mut changed, TypeForm::OwnedI64Cell);
    let different = intern(&mut changed, TypeForm::OwnedSequence { item: cell });
    let DeclarationPayload::OwnedImplementation(i) = payload(&mut changed, concrete) else {
        unreachable!()
    };
    i.type_arguments[1] = different;
    assert!(
        Oracle(&changed, None).valid_implementation_at(DeclarationReference {
            package: changed.root.package_id,
            declaration: concrete,
        })
    );
    assert!(
        !accepts(&changed),
        "phantom witnesses retain exact structured bindings"
    );

    let mut changed = snapshot.clone();
    let DeclarationPayload::OwnedContract(c) = payload(&mut changed, pipe) else {
        unreachable!()
    };
    let phantom = c.type_parameters[1];
    let OwnerRecord::TypeParameter(p) = changed
        .owners
        .get_mut(&OwnerKey::TypeParameter(phantom))
        .unwrap()
    else {
        unreachable!()
    };
    p.declaration = relay;
    assert!(
        !accepts(&changed),
        "unused formals retain exact owner scope"
    );

    let mut changed = snapshot.clone();
    let DeclarationPayload::OwnedContract(c) = payload(&mut changed, pipe) else {
        unreachable!()
    };
    c.type_parameters.swap(0, 1);
    assert!(
        !accepts(&changed),
        "contract arguments follow authored formal order"
    );
}

#[test]
fn parameterized_contract_oracle_rejects_duplicate_self_and_extra_parameter_names() {
    let snapshot = source();
    let pipe = declaration(&snapshot, "Pipe");
    let OwnerRecord::Declaration(d) = &snapshot.owners[&OwnerKey::Declaration(pipe)] else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(c) = &d.payload else {
        unreachable!()
    };
    for duplicate in [c.self_parameter, c.type_parameters[0]] {
        let OwnerRecord::TypeParameter(prototype) =
            &snapshot.owners[&OwnerKey::TypeParameter(duplicate)]
        else {
            unreachable!()
        };
        let mut changed = snapshot.clone();
        let OwnerRecord::TypeParameter(phantom) = changed
            .owners
            .get_mut(&OwnerKey::TypeParameter(c.type_parameters[1]))
            .unwrap()
        else {
            unreachable!()
        };
        phantom.name = prototype.name.clone();
        assert!(
            !accepts(&changed),
            "contract formal names must remain distinct from Self and extras"
        );
        assert!(
            !accepts(&import_only(&changed).0),
            "unused imported formals retain name distinctness"
        );
    }
}

#[test]
fn parameterized_contract_oracle_matches_complete_method_structure_and_owned_modes() {
    let snapshot = source();
    let target = declaration(&snapshot, "concrete-relay");
    let mut changed = snapshot.clone();
    let cell = intern(&mut changed, TypeForm::OwnedI64Cell);
    let DeclarationPayload::Function(f) = payload(&mut changed, target) else {
        unreachable!()
    };
    f.result = cell;
    assert!(
        !accepts(&changed),
        "an owned result must match the full substituted template"
    );

    let pipe = declaration(&snapshot, "Pipe");
    let mut changed = snapshot.clone();
    let DeclarationPayload::OwnedContract(c) = payload(&mut changed, pipe) else {
        unreachable!()
    };
    c.methods[0].parameters[0].use_mode = ParameterUse::Borrow;
    c.methods[0].effect = FunctionEffect::Task {
        requirements: vec![],
        effect_parameters: vec![],
    };
    assert!(
        Oracle(&changed, None).valid_contract(DeclarationReference {
            package: changed.root.package_id,
            declaration: pipe,
        }),
        "task methods can synchronously borrow structured owners"
    );
    assert!(
        !accepts(&changed),
        "the mapped function still has its original pure consuming signature"
    );
}

fn import_only(snapshot: &KernelSnapshot) -> (KernelSnapshot, PackageRevisionDigest) {
    let mut imported = snapshot.clone();
    let package = imported.root.package_id;
    imported.root.package_id = PackageId::migrate(b"parameterized-oracle-import", 0);
    let revision = PackageRevisionDigest::from_bytes([166; 32]);
    let mut owners = BTreeMap::new();
    for (key, owner) in std::mem::take(&mut imported.owners) {
        if let Some(projected) = PackageInterfaceRecord::project_public(&owner).unwrap() {
            owners.insert(key, projected);
        }
    }
    imported.dependency_types = std::mem::take(&mut imported.types);
    imported.dependency_interfaces.insert(revision, owners);
    imported.dependencies.insert(
        package,
        DependencyRecord {
            graph_contract_version: 26,
            package,
            semantic_revision: RevisionId::from_digest([167; 32]),
            package_revision: revision,
        },
    );
    (imported, revision)
}

#[test]
fn parameterized_contract_oracle_admits_unused_imports_and_rejects_downgraded_record_generations() {
    let snapshot = source();
    let (imported, revision) = import_only(&snapshot);
    assert!(accepts(&imported));
    let pipe = declaration(&snapshot, "Pipe");
    let OwnerRecord::Declaration(d) = &snapshot.owners[&OwnerKey::Declaration(pipe)] else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(c) = &d.payload else {
        unreachable!()
    };
    let phantom = c.type_parameters[1];
    let mut changed = snapshot.clone();
    let OwnerRecord::TypeParameter(p) = changed
        .owners
        .get_mut(&OwnerKey::TypeParameter(phantom))
        .unwrap()
    else {
        unreachable!()
    };
    p.header.contract_version = 25;
    assert!(
        !accepts(&changed),
        "old extra formal cannot introduce current contract arguments"
    );
    let mut changed = imported.clone();
    let PackageInterfaceRecord::TypeParameter(p) = changed
        .dependency_interfaces
        .get_mut(&revision)
        .unwrap()
        .get_mut(&OwnerKey::TypeParameter(phantom))
        .unwrap()
    else {
        unreachable!()
    };
    p.header.contract_version = 25;
    assert!(
        !accepts(&changed),
        "unused imported extra formals retain their generation"
    );
    for name in ["Pipe", "Concrete", "relay"] {
        let id = declaration(&snapshot, name);
        let mut changed = snapshot.clone();
        let OwnerRecord::Declaration(d) =
            changed.owners.get_mut(&OwnerKey::Declaration(id)).unwrap()
        else {
            unreachable!()
        };
        d.header.contract_version = 25;
        assert!(
            !accepts(&changed),
            "old local {name} cannot contain generation 26 meaning"
        );

        let mut changed = imported.clone();
        let PackageInterfaceRecord::Declaration(d) = changed
            .dependency_interfaces
            .get_mut(&revision)
            .unwrap()
            .get_mut(&OwnerKey::Declaration(id))
            .unwrap()
        else {
            unreachable!()
        };
        d.header.contract_version = 25;
        assert!(
            !accepts(&changed),
            "old unused imported {name} cannot contain generation 26 meaning"
        );
    }
    let mut changed = snapshot;
    let ordinary = intern(&mut changed, TypeForm::I64);
    let id = declaration(&changed, "Concrete");
    let DeclarationPayload::OwnedImplementation(i) = payload(&mut changed, id) else {
        unreachable!()
    };
    i.type_arguments[1] = ordinary;
    assert!(
        !accepts(&import_only(&changed).0),
        "unused imported actuals require independent admission"
    );
}

#[test]
fn structured_self_methods_require_current_generation_even_without_extra_parameters() {
    let source = r#"declarations.begin
(units (module create structured-self
  (owned-contract create Identity (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_a6000000000000000000000000000002 relay
      (parameters ((owned-sequence Self) consume)) (returns (owned-sequence Self))))))
declarations.end
"#;
    let mut snapshot =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source)
            .unwrap();
    assert!(accepts(&snapshot));
    let id = declaration(&snapshot, "Identity");
    let OwnerRecord::Declaration(d) = snapshot.owners.get_mut(&OwnerKey::Declaration(id)).unwrap()
    else {
        unreachable!()
    };
    d.header.contract_version = 25;
    assert!(
        !accepts(&snapshot),
        "structured Self is new meaning even with no argument inventory"
    );
}
