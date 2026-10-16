//! Canonical owner mutations with independently specified nominal/loan outcomes.
use super::*;
use crate::platform::semantic_id::{DeclarationId, ExpressionId};

const SOURCE: &str = r#"declarations.begin
(units (module create nominal-reference
  (owned-record create Packet (visibility private)
    (field create payload (type ByteBuffer))
    (field create sibling (type ByteBuffer)) (field create tag (type I64)))
  (owned-record create Other (visibility private)
    (field create payload (type ByteBuffer))
    (field create sibling (type ByteBuffer)) (field create tag (type I64)))
  (owned-record create Empty (visibility private))
  (owned-record create Tagged (visibility private) (field create tag (type I64)))
  (owned-record create Box (visibility private)
    (type-parameter create T (constraint owned transferable shareable))
    (type-parameter create Label (constraint transferable))
    (field create value (type T)) (field create label (type Label)))
  (owned-record create Phantom (visibility private)
    (type-parameter create T (constraint owned)))
  (external create length (visibility private) (implementation core.buffer.length)
    (parameter create value (type ByteBuffer) (use borrow)) (returns I64))
  (function create pack (visibility private) (effect pure)
    (parameter create a (type ByteBuffer) (use consume))
    (parameter create b (type ByteBuffer) (use consume)) (returns Packet)
    (body (pack-owned (type Packet)
      (field tag (i64 7)) (field sibling (local b)) (field payload (local a)))))
  (function create unpack (visibility private) (effect pure)
    (parameter create packet (type Packet) (use consume)) (returns Unit)
    (body (unpack-owned (type Packet) (local packet)
      (field tag (binding tag (type I64)))
      (field sibling (binding b (type ByteBuffer)))
      (field payload (binding a (type ByteBuffer))) (in (unit)))))
  (function create consume (visibility private) (effect pure)
    (parameter create packet (type Packet) (use consume)) (returns I64) (body (i64 0)))
  (function create read (visibility private) (effect pure)
    (parameter create packet (type Packet) (use consume)) (returns I64)
    (body (if (bool true) (i64 7)
      (borrow-owned-field (type Packet) (local packet)
        (field payload (binding view (type ByteBuffer)))
        (in (call length (local view)))))))
  (function create read-tag (visibility private) (effect pure)
    (parameter create packet (type Packet) (use borrow)) (returns I64)
    (body (field (local packet) Packet::tag)))
  (function create nested-read (visibility private) (effect pure)
    (parameter create packet (type (Box Packet I64)) (use consume)) (returns I64)
    (body (borrow-owned-field (type (Box Packet I64)) (local packet)
      (field value (binding parent (type Packet)))
      (in (borrow-owned-field (type Packet) (local parent)
        (field payload (binding view (type ByteBuffer)))
        (in (call length (local view))))))))
  (function create consume-box (visibility private) (effect pure)
    (parameter create packet (type (Box Packet I64)) (use consume)) (returns I64) (body (i64 0)))
  (function create empty (visibility private) (effect pure) (returns Unit)
    (body (let (binding value (type Empty) (pack-owned (type Empty)))
      (in (unpack-owned (type Empty) (local value) (in (unit)))))))
  (function create tagged (visibility private) (effect pure) (returns Tagged)
    (body (pack-owned (type Tagged) (field tag (i64 3)))))
  (function create nested (visibility private) (effect pure)
    (parameter create value (type (Box (Box ByteBuffer I64) I64)) (use consume))
    (returns (Box (Box ByteBuffer I64) I64)) (body (local value)))
  (function create phantom (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create value (type (Phantom T)) (use consume))
    (returns (Phantom T)) (body (local value)))
  (constant create transient (visibility private) (type I64)
    (value (let (binding value (type Empty) (pack-owned (type Empty)))
      (in (unpack-owned (type Empty) (local value) (in (i64 0)))))))
  (test create transient-test (visibility private)
    (actual (let (binding value (type Empty) (pack-owned (type Empty)))
      (in (unpack-owned (type Empty) (local value) (in (i64 0))))))
    (expected (i64 0)))
  (component create transient-port (visibility private)
    (port create read (type (function () Unit))
      (value (let (binding value (type Empty) (pack-owned (type Empty)))
        (in (unpack-owned (type Empty) (local value) (in (function-value empty)))))))))
  (module create foreign
    (function create marker (visibility private) (effect pure) (returns Unit) (body (unit)))))
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
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.name.as_str() == name =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}

fn reference(snapshot: &KernelSnapshot, name: &str) -> DeclarationReference {
    DeclarationReference {
        package: snapshot.root.package_id,
        declaration: declaration(snapshot, name),
    }
}

fn function(snapshot: &KernelSnapshot, name: &str) -> FunctionDeclaration {
    let OwnerRecord::Declaration(record) =
        &snapshot.owners[&OwnerKey::Declaration(declaration(snapshot, name))]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &record.payload else {
        unreachable!()
    };
    function.clone()
}

fn operation(snapshot: &mut KernelSnapshot, id: ExpressionId) -> &mut ExpressionOperation {
    let OwnerRecord::Expression(record) =
        snapshot.owners.get_mut(&OwnerKey::Expression(id)).unwrap()
    else {
        unreachable!()
    };
    &mut record.operation
}

fn borrowed(snapshot: &KernelSnapshot) -> ExpressionId {
    let function = function(snapshot, "read");
    let OwnerRecord::Expression(record) = &snapshot.owners[&OwnerKey::Expression(function.body)]
    else {
        unreachable!()
    };
    let ExpressionOperation::If { when_false, .. } = record.operation else {
        unreachable!()
    };
    when_false
}

#[test]
fn nominal_reference_accepts_intrinsic_empty_data_only_and_nested_generic_custody() {
    let snapshot = source();
    assert!(accepts(&snapshot));
    validate_full(&snapshot).unwrap();
    let phantom = reference(&snapshot, "phantom");
    let oracle = Oracle(&snapshot, Some(phantom));
    let ty = function(&snapshot, "phantom").result.ty;
    assert!(oracle.buffer(ty));
    assert!(oracle.product_shape(ty));
    let mut remaining = contract::MAXIMUM_VALIDATION_WORK;
    assert_eq!(
        oracle.parallel_shape(ty, &BTreeMap::new(), 0, &mut remaining),
        None,
        "phantom Owned formal does not grant transferable proof"
    );
    assert_eq!(
        oracle.read_shape(ty, &BTreeMap::new(), 0, &mut remaining),
        None,
        "phantom Owned formal does not grant shared-read proof"
    );
}

#[test]
fn nominal_reference_rejects_untaken_foreign_module_representation() {
    let baseline = source();
    let OwnerRecord::Declaration(marker) =
        &baseline.owners[&OwnerKey::Declaration(declaration(&baseline, "marker"))]
    else {
        unreachable!()
    };
    for name in [
        "pack",
        "unpack",
        "read",
        "read-tag",
        "tagged",
        "empty",
        "transient",
        "transient-test",
        "transient-port",
    ] {
        let mut changed = baseline.clone();
        let OwnerRecord::Declaration(record) = changed
            .owners
            .get_mut(&OwnerKey::Declaration(declaration(&baseline, name)))
            .unwrap()
        else {
            unreachable!()
        };
        record.module = marker.module;
        assert!(
            !accepts(&changed),
            "{name}: exact module authority covers untaken syntax"
        );
    }
}

#[test]
fn nominal_reference_rejects_guessed_field_ids_unordered_unpacks_and_parent_consumption() {
    let baseline = source();
    let pack = function(&baseline, "pack").body;
    let mut changed = baseline.clone();
    let foreign_field = baseline
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Field(field)
                if field.declaration == declaration(&baseline, "Other")
                    && field.name.as_str() == "tag" =>
            {
                let OwnerKey::Field(id) = field.header.owner else {
                    unreachable!()
                };
                Some(FieldReference {
                    package: baseline.root.package_id,
                    field: id,
                })
            }
            _ => None,
        })
        .unwrap();
    let ExpressionOperation::PackOwnedRecord { fields, .. } = operation(&mut changed, pack) else {
        unreachable!()
    };
    fields[0].field = foreign_field;
    assert!(
        !accepts(&changed),
        "an identical field shape grants no field identity"
    );

    let mut changed = baseline.clone();
    let unpack = function(&baseline, "unpack").body;
    let ExpressionOperation::UnpackOwnedRecord { fields, .. } = operation(&mut changed, unpack)
    else {
        unreachable!()
    };
    fields.swap(0, 1);
    assert!(
        !accepts(&changed),
        "unpack order belongs to the canonical exact layout"
    );

    let mut changed = baseline.clone();
    let read = borrowed(&baseline);
    let ExpressionOperation::BorrowOwnedRecordField { source, body, .. } =
        operation(&mut changed, read)
    else {
        unreachable!()
    };
    let (source, body) = (*source, *body);
    *operation(&mut changed, body) = ExpressionOperation::Call {
        function: reference(&baseline, "consume"),
        type_arguments: vec![],
        requirement_arguments: vec![],
        effect_arguments: vec![],
        arguments: vec![source],

        read_scope_arguments: Vec::new(),
    };
    assert!(
        !accepts(&changed),
        "a field loan protects parent custody even in an untaken branch"
    );
}

#[test]
fn nominal_reference_rejects_recursive_sequence_layouts_and_old_generation() {
    let baseline = source();
    let mut changed = baseline.clone();
    let packet = function(&baseline, "pack").result.ty;
    let object = TypeObject::new(TypeForm::OwnedSequence { item: packet }).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    changed.types.insert(ty, object);
    let record = declaration(&baseline, "Packet");
    let field = changed
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Field(field) if field.declaration == record => Some(field),
            _ => None,
        })
        .unwrap();
    field.ty = ty;
    assert!(
        !accepts(&changed),
        "recursive representation through a sequence is rejected before expansion"
    );

    let mut changed = baseline;
    changed.root.graph_contract_version = 31;
    assert!(
        !accepts(&changed),
        "nominal custody requires the current schema epoch"
    );
}

#[test]
fn nominal_reference_requires_current_bound_owners_and_rejects_unused_phantom_actuals() {
    let baseline = source();
    assert!(accepts(&baseline));
    let box_id = declaration(&baseline, "Box");
    let bounded_parameters = baseline
        .owners
        .iter()
        .filter_map(|(key, owner)| {
            matches!(owner, OwnerRecord::TypeParameter(p) if p.declaration == box_id)
                .then_some(*key)
        })
        .collect::<Vec<_>>();
    assert_eq!(bounded_parameters.len(), 2);
    for key in bounded_parameters {
        for generation in [
            0,
            contract::GRAPH_CONTRACT_VERSION - 1,
            contract::GRAPH_CONTRACT_VERSION + 1,
        ] {
            let mut changed = baseline.clone();
            let owner = changed.owners.get_mut(&key).unwrap();
            owner.set_encoding_for_edit(generation);
            assert_eq!(encode_owner(owner).unwrap_err().code, "kernel_owner_header");
            assert!(
                !accepts(&changed),
                "a bound parameter needs its own current owner contract"
            );
        }
    }

    for (form, admitted) in [(TypeForm::ByteBuffer, true), (TypeForm::I64, false)] {
        let mut changed = baseline.clone();
        let actual = baseline
            .types
            .iter()
            .find_map(|(digest, object)| (object.form == form).then_some(*digest))
            .unwrap();
        let object = TypeObject::new(TypeForm::Applied {
            declaration: reference(&baseline, "Phantom"),
            arguments: vec![actual],
        })
        .unwrap();
        changed
            .types
            .insert(encode_type_object(&object).unwrap().0, object);
        assert_eq!(
            accepts(&changed),
            admitted,
            "an unused phantom actual still has to prove its Owned bound: {form:?}"
        );
    }

    let mut changed = baseline;
    let phantom = declaration(&changed, "Phantom");
    let OwnerRecord::Declaration(record) = changed
        .owners
        .get_mut(&OwnerKey::Declaration(phantom))
        .unwrap()
    else {
        unreachable!()
    };
    record.header.kind = OwnerKind::Record;
    assert!(
        !accepts(&changed),
        "a reclassified nominal cannot erase intrinsic custody"
    );
}

#[test]
fn nominal_reference_freezes_every_ancestor_and_rejects_structural_field_aliases() {
    let baseline = source();
    let outer = function(&baseline, "nested-read").body;
    let OwnerRecord::Expression(record) = &baseline.owners[&OwnerKey::Expression(outer)] else {
        unreachable!()
    };
    let ExpressionOperation::BorrowOwnedRecordField {
        source,
        body: inner,
        ..
    } = record.operation
    else {
        unreachable!()
    };
    let OwnerRecord::Expression(record) = &baseline.owners[&OwnerKey::Expression(inner)] else {
        unreachable!()
    };
    let ExpressionOperation::BorrowOwnedRecordField { body, .. } = record.operation else {
        unreachable!()
    };
    let mut changed = baseline.clone();
    *operation(&mut changed, body) = ExpressionOperation::Call {
        function: reference(&baseline, "consume-box"),
        type_arguments: vec![],
        requirement_arguments: vec![],
        effect_arguments: vec![],
        arguments: vec![source],

        read_scope_arguments: Vec::new(),
    };
    assert!(
        !accepts(&changed),
        "nested projected loans retain the complete enclosing owner ancestry"
    );

    let mut changed = baseline.clone();
    let read = function(&baseline, "read-tag").body;
    let ExpressionOperation::Field { selector, .. } = operation(&mut changed, read) else {
        unreachable!()
    };
    *selector = FieldSelector::Structural(Name::new("tag").unwrap());
    assert!(
        !accepts(&changed),
        "a structural name cannot select a nominal owned field"
    );
}

#[test]
fn nominal_reference_counts_one_combined_layout_depth_through_record_and_sequence_edges() {
    let baseline = source();
    for extra in [0, 1] {
        let mut changed = baseline.clone();
        let object = TypeObject::new(TypeForm::Named {
            declaration: reference(&baseline, "Empty"),
        })
        .unwrap();
        let mut child = encode_type_object(&object).unwrap().0;
        changed.types.insert(child, object);
        for _ in 0..contract::MAXIMUM_TYPE_DEPTH - 1 + extra {
            let object = TypeObject::new(TypeForm::OwnedSequence { item: child }).unwrap();
            child = encode_type_object(&object).unwrap().0;
            changed.types.insert(child, object);
        }
        let tagged = declaration(&baseline, "Tagged");
        let field = changed
            .owners
            .values_mut()
            .find_map(|owner| match owner {
                OwnerRecord::Field(field) if field.declaration == tagged => Some(field),
                _ => None,
            })
            .unwrap();
        field.ty = child;
        let oracle = Oracle(&changed, None);
        assert_eq!(
            oracle.product_shape(function(&baseline, "tagged").result.ty),
            extra == 0,
            "the declaration field edge and the structural sequence depth share one bound"
        );
    }
}

#[test]
fn nominal_reference_reads_trusted_transitive_layout_without_granting_representation_access() {
    let mut snapshot = source();
    let package_b = PackageId::migrate(b"transitive-nominal-oracle", 1);
    let package_c = PackageId::migrate(b"transitive-nominal-oracle", 2);
    let revision_b = PackageRevisionDigest::from_bytes([41; 32]);
    let revision_c = PackageRevisionDigest::from_bytes([42; 32]);
    let tagged = declaration(&snapshot, "Tagged");
    let OwnerRecord::Declaration(record) = &snapshot.owners[&OwnerKey::Declaration(tagged)] else {
        unreachable!()
    };
    let mut public = PackageInterfaceDeclaration {
        header: record.header,
        name: Name::new("Wrapper").unwrap(),
        payload: match &record.payload {
            DeclarationPayload::OwnedRecord {
                type_parameters,
                fields,
            } => PackageInterfaceDeclarationPayload::OwnedRecord {
                type_parameters: type_parameters.clone(),
                fields: fields.clone(),
            },
            _ => unreachable!(),
        },
    };
    let mut hidden = public.clone();
    hidden.name = Name::new("Hidden").unwrap();
    let OwnerRecord::Field(field) = snapshot
        .owners
        .values()
        .find(|owner| matches!(owner, OwnerRecord::Field(field) if field.declaration == tagged))
        .unwrap()
    else {
        unreachable!()
    };
    let mut public_field = field.clone();
    let hidden_field = field.clone();
    let object = TypeObject::new(TypeForm::Named {
        declaration: DeclarationReference {
            package: package_c,
            declaration: tagged,
        },
    })
    .unwrap();
    let hidden_type = encode_type_object(&object).unwrap().0;
    snapshot.dependency_types.insert(hidden_type, object);
    public_field.ty = hidden_type;
    public.name = Name::new("Wrapper").unwrap();
    snapshot.dependency_interfaces.insert(
        revision_b,
        std::sync::Arc::new(BTreeMap::from([
            (
                OwnerKey::Declaration(tagged),
                PackageInterfaceRecord::Declaration(public),
            ),
            (
                public_field.header.owner,
                PackageInterfaceRecord::Field(public_field),
            ),
        ])),
    );
    snapshot.dependency_interfaces.insert(
        revision_c,
        std::sync::Arc::new(BTreeMap::from([
            (
                OwnerKey::Declaration(tagged),
                PackageInterfaceRecord::RepresentationDeclaration(hidden),
            ),
            (
                hidden_field.header.owner,
                PackageInterfaceRecord::Field(hidden_field),
            ),
        ])),
    );
    snapshot.dependencies.insert(
        package_b,
        DependencyRecord {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            package: package_b,
            semantic_revision: crate::platform::semantic_id::RevisionId::from_digest([43; 32]),
            package_revision: revision_b,
        },
    );
    snapshot.representation_dependencies.insert(
        package_c,
        DependencyRecord {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            package: package_c,
            semantic_revision: crate::platform::semantic_id::RevisionId::from_digest([44; 32]),
            package_revision: revision_c,
        },
    );
    let object = TypeObject::new(TypeForm::Named {
        declaration: DeclarationReference {
            package: package_b,
            declaration: tagged,
        },
    })
    .unwrap();
    let wrapper_type = encode_type_object(&object).unwrap().0;
    snapshot.types.insert(wrapper_type, object);
    assert!(
        accepts(&snapshot),
        "the exact B-to-C metadata proves B's private layout"
    );
    let oracle = Oracle(&snapshot, Some(reference(&snapshot, "pack")));
    assert!(oracle.owned_type_in_scope(wrapper_type));
    assert!(
        oracle
            .foreign(package_c, OwnerKey::Declaration(tagged))
            .is_none()
    );
    assert!(!oracle.record_access(hidden_type));

    let mut broken = snapshot.clone();
    broken.representation_dependencies.clear();
    assert!(
        !accepts(&broken),
        "a dangling private transitive nominal does not grant an opaque owner"
    );

    let mut broken = snapshot.clone();
    let body = function(&snapshot, "pack").body;
    *operation(&mut broken, body) = ExpressionOperation::PackOwnedRecord {
        record_type: hidden_type,
        fields: vec![],
    };
    assert!(
        !accepts(&broken),
        "trusted metadata never authorizes consumer construction"
    );

    let mut broken = snapshot.clone();
    let mut conflicting = broken.dependencies[&package_b].clone();
    conflicting.package_revision = revision_c;
    broken
        .representation_dependencies
        .insert(package_b, conflicting);
    assert!(
        !accepts(&broken),
        "the two inventories cannot select different suppliers for one package"
    );
}
