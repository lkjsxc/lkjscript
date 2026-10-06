//! Canonical-inventory attacks checked by the separate owner/loan/provenance model.
//! Authoring supplies a well-formed baseline; mutations never use production
//! ownership classifications as the expected answer.
use super::*;
use crate::platform::persistent_map::{MapContentDigest, MapRoot, PageDigest};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::{BindingId, DeclarationId, RepositoryId};

const ABSTRACT: &str = include_str!("../../../tests/fixtures/owned-witness-library.lkjc");
const CELL: &str = include_str!("../../../tests/fixtures/owned-witness-cell.lkjc");

const GENERIC: &str = r#"declarations.begin
(units (module create read-loans
  (function create generic-read (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_74000000000000000000000000000001 ops abstraction::Storage T)
    (parameter create packet (type (owned-product (field payload T) (field tag I64))) (use borrow))
    (returns I64)
    (body (borrow-owned-field (type (owned-product (field payload T) (field tag I64)))
      (local packet) (field payload (binding view (type T)))
      (in (implementation-call abstraction::read-helper (types T)
        (implementations parameter@generic-read@implparam_74000000000000000000000000000001)
        (local view))))))
  (function create generic-method (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_74000000000000000000000000000002 ops abstraction::Storage T)
    (parameter create packet (type (owned-product (field payload T) (field tag I64))) (use consume))
    (returns I64)
    (body (borrow-owned-field (type (owned-product (field payload T) (field tag I64)))
      (local packet) (field payload (binding view (type T)))
      (in (method-call parameter@generic-method@implparam_74000000000000000000000000000002
        abstraction::Storage method_10000000000000000000000000000003 (local view))))))))
declarations.end
"#;

const CONCRETE: &str = r#"declarations.begin
(units (module create concrete-loans
  (function create dispose (visibility private) (effect pure)
    (parameter create packet (type (owned-product (field payload OwnedI64Cell) (field tag I64))) (use consume))
    (returns I64) (body (i64 1)))
  (function create dispose-choice (visibility private) (effect pure)
    (parameter create outcome (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell))) (use consume))
    (returns I64) (body (i64 1)))
  (function create dispose-nested (visibility private) (effect pure)
    (parameter create packet (type (owned-product
      (field inner (owned-product (field payload OwnedI64Cell) (field tag I64)))
      (field sibling OwnedI64Cell) (field tag I64))) (use consume))
    (returns I64) (body (i64 1)))
  (function create worker (visibility private) (effect (task))
    (parameter create cell (type OwnedI64Cell) (use consume)) (returns I64) (body (i64 1)))
  (function create mark (visibility private) (effect (task)) (returns I64) (body (i64 1)))
  (function create product-read (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field payload OwnedI64Cell) (field tag I64))) (use consume))
    (returns I64)
    (body (borrow-owned-field (type (owned-product (field payload OwnedI64Cell) (field tag I64)))
      (local packet) (field payload (binding view (type OwnedI64Cell)))
      (in (call cell::read (local view))))))
  (function create product-relay (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field payload OwnedI64Cell) (field tag I64))) (use consume))
    (returns (owned-product (field payload OwnedI64Cell) (field tag I64)))
    (body (sequence
      (borrow-owned-field (type (owned-product (field payload OwnedI64Cell) (field tag I64)))
        (local packet) (field payload (binding view (type OwnedI64Cell)))
        (in (sequence (call cell::read (local view)) (field (local packet) (name tag)))))
      (local packet))))
  (function create borrowed-source (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field payload OwnedI64Cell) (field tag I64))) (use borrow))
    (returns I64)
    (body (borrow-owned-field (type (owned-product (field payload OwnedI64Cell) (field tag I64)))
      (local packet) (field payload (binding view (type OwnedI64Cell)))
      (in (call cell::read (local view))))))
  (function create nested-read (visibility public) (effect pure)
    (parameter create packet (type (owned-product
      (field inner (owned-product (field payload OwnedI64Cell) (field tag I64)))
      (field sibling OwnedI64Cell) (field tag I64))) (use consume))
    (returns I64)
    (body (borrow-owned-field (type (owned-product
      (field inner (owned-product (field payload OwnedI64Cell) (field tag I64)))
      (field sibling OwnedI64Cell) (field tag I64)))
      (local packet)
      (field inner (binding inner-view (type (owned-product (field payload OwnedI64Cell) (field tag I64)))))
      (in (borrow-owned-field (type (owned-product (field payload OwnedI64Cell) (field tag I64)))
        (local inner-view) (field payload (binding view (type OwnedI64Cell)))
        (in (sequence (field (local packet) (name tag)) (call cell::read (local view)))))))))
  (function create sibling-read (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell))) (use consume))
    (returns I64)
    (body (borrow-owned-field (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
      (local packet) (field left (binding left-view (type OwnedI64Cell)))
      (in (borrow-owned-field (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
        (local packet) (field right (binding right-view (type OwnedI64Cell)))
        (in (sequence (call cell::read (local left-view)) (call cell::read (local right-view)))))))))
  (function create fresh-result (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field payload OwnedI64Cell) (field tag I64))) (use consume))
    (returns OwnedI64Cell)
    (body (borrow-owned-field (type (owned-product (field payload OwnedI64Cell) (field tag I64)))
      (local packet) (field payload (binding view (type OwnedI64Cell)))
      (in (sequence (call cell::read (local view)) (call cell::create (i64 8)))))))
  (function create effect-body (visibility public) (effect (task))
    (parameter create packet (type (owned-product (field payload OwnedI64Cell) (field tag I64))) (use consume))
    (returns I64)
    (body (borrow-owned-field (type (owned-product (field payload OwnedI64Cell) (field tag I64)))
      (local packet) (field payload (binding view (type OwnedI64Cell)))
      (in (sequence (call mark) (call cell::read (local view)))))))
  (function create choice-read (visibility public) (effect pure)
    (parameter create outcome (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell))) (use consume))
    (returns I64)
    (body (match-borrowed-owned (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell)))
      (local outcome)
      (case accepted (binding ordinary (type I64)) (in (local ordinary)))
      (case rejected (binding view (type OwnedI64Cell)) (in (call cell::read (local view)))))))
  (function create choice-relay (visibility public) (effect pure)
    (parameter create outcome (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell))) (use consume))
    (returns (owned-choice (case accepted I64) (case rejected OwnedI64Cell)))
    (body (sequence
      (match-borrowed-owned (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell)))
        (local outcome)
        (case accepted (binding ordinary (type I64)) (in (local ordinary)))
        (case rejected (binding view (type OwnedI64Cell)) (in (call cell::read (local view)))))
      (local outcome))))
  (function create choice-fresh-result (visibility public) (effect pure)
    (parameter create outcome (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell))) (use consume))
    (returns OwnedI64Cell)
    (body (match-borrowed-owned (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell)))
      (local outcome)
      (case accepted (binding ordinary (type I64)) (in (call cell::create (local ordinary))))
      (case rejected (binding view (type OwnedI64Cell))
        (in (sequence (call cell::read (local view)) (call cell::create (i64 8))))))))))
declarations.end
"#;

fn author(source: &str) -> KernelSnapshot {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source).unwrap()
}

fn source() -> KernelSnapshot {
    author(&format!("{ABSTRACT}{GENERIC}{CELL}{CONCRETE}"))
}

fn declaration(source: &KernelSnapshot, name: &str) -> DeclarationId {
    source
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

fn function(source: &KernelSnapshot, name: &str) -> FunctionDeclaration {
    let OwnerRecord::Declaration(d) =
        &source.owners[&OwnerKey::Declaration(declaration(source, name))]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(f) = &d.payload else {
        unreachable!()
    };
    f.clone()
}

fn operation(source: &mut KernelSnapshot, id: ExpressionId) -> &mut ExpressionOperation {
    let OwnerRecord::Expression(e) = source.owners.get_mut(&OwnerKey::Expression(id)).unwrap()
    else {
        unreachable!()
    };
    &mut e.operation
}

fn binding(source: &mut KernelSnapshot, id: BindingId) -> &mut BindingRecord {
    let OwnerRecord::Binding(b) = source.owners.get_mut(&OwnerKey::Binding(id)).unwrap() else {
        unreachable!()
    };
    b
}

fn projection(source: &KernelSnapshot, name: &str) -> (ExpressionId, BindingId, ExpressionId) {
    let f = function(source, name);
    let OwnerRecord::Expression(e) = &source.owners[&OwnerKey::Expression(f.body)] else {
        unreachable!()
    };
    let ExpressionOperation::BorrowOwnedField { binding, body, .. } = e.operation else {
        unreachable!()
    };
    (f.body, binding, body)
}

fn reference(source: &KernelSnapshot, name: &str) -> DeclarationReference {
    DeclarationReference {
        package: source.root.package_id,
        declaration: declaration(source, name),
    }
}

fn call(function: DeclarationReference, argument: ExpressionId) -> ExpressionOperation {
    ExpressionOperation::Call {
        function,
        arguments: vec![argument],
        type_arguments: Vec::new(),
        requirement_arguments: Vec::new(),
        effect_arguments: Vec::new(),
    }
}

#[test]
fn borrowed_memory_oracle_accepts_retained_sources_nested_siblings_results_and_effects() {
    // The fixed literal exercises borrowed and owning sources, ancestor reads,
    // nested custody, independently owned results, ordinary choice payloads,
    // exact generic witnesses, and a task body with no transferred view.
    let source = source();
    assert!(accepts(&source));
    validate_full(&source).unwrap();
}

#[test]
fn borrowed_memory_oracle_rejects_counterfeit_roles_annotations_and_generation() {
    let source = source();
    let (projection, view, _) = projection(&source, "product-read");
    for role in [
        BindingKind::Let,
        BindingKind::OwnedUnpack,
        BindingKind::OwnedChoicePayload,
    ] {
        let mut changed = source.clone();
        binding(&mut changed, view).kind = role;
        assert!(!accepts(&changed), "forged read-view role: {role:?}");
    }
    let mut changed = source.clone();
    let wrong = function(&source, "product-read").result;
    binding(&mut changed, view).declared_type = Some(wrong);
    assert!(
        !accepts(&changed),
        "view type must equal the selected child"
    );
    for selected in [OwnerKey::Expression(projection), OwnerKey::Binding(view)] {
        let mut changed = source.clone();
        match changed.owners.get_mut(&selected).unwrap() {
            OwnerRecord::Expression(e) => e.contract_version = 23,
            OwnerRecord::Binding(b) => b.header.contract_version = 23,
            _ => unreachable!(),
        }
        assert!(
            !accepts(&changed),
            "read loans have no predecessor encoding"
        );
    }
    let mut changed = source.clone();
    changed.root.graph_contract_version = 23;
    assert!(!accepts(&changed));

    let mut changed = source.clone();
    *operation(&mut changed, projection) = ExpressionOperation::I64 { value: 0 };
    assert!(
        !accepts(&changed),
        "removing a read scope cannot leave its view inventory orphaned"
    );
}

#[test]
fn borrowed_memory_oracle_rejects_root_consumption_view_escape_and_task_transfer() {
    let source = source();
    let (id, _, body) = projection(&source, "product-read");
    let OwnerRecord::Expression(e) = &source.owners[&OwnerKey::Expression(id)] else {
        unreachable!()
    };
    let ExpressionOperation::BorrowOwnedField { source: root, .. } = e.operation else {
        unreachable!()
    };
    let mut changed = source.clone();
    *operation(&mut changed, body) = call(reference(&source, "dispose"), root);
    assert!(
        !accepts(&changed),
        "source is frozen throughout the read scope"
    );

    let (_, view, body) = projection(&source, "fresh-result");
    let mut changed = source.clone();
    *operation(&mut changed, body) = ExpressionOperation::Local {
        value: LocalValueReference::LexicalBinding(view),
    };
    assert!(
        !accepts(&changed),
        "a view cannot escape as an owning result"
    );

    let (_, _, body) = projection(&source, "effect-body");
    let OwnerRecord::Expression(e) = &source.owners[&OwnerKey::Expression(body)] else {
        unreachable!()
    };
    let ExpressionOperation::Sequence { items } = &e.operation else {
        unreachable!()
    };
    let OwnerRecord::Expression(read) = &source.owners[&OwnerKey::Expression(items[1])] else {
        unreachable!()
    };
    let ExpressionOperation::Call { arguments, .. } = &read.operation else {
        unreachable!()
    };
    let mut changed = source.clone();
    *operation(&mut changed, items[1]) = call(reference(&source, "worker"), arguments[0]);
    assert!(
        !accepts(&changed),
        "a consuming task parameter requires owning authority"
    );
}

#[test]
fn borrowed_memory_oracle_rejects_consumed_sources_and_nested_ancestor_moves() {
    let source = source();
    let (outer, _, inner) = projection(&source, "nested-read");
    let OwnerRecord::Expression(e) = &source.owners[&OwnerKey::Expression(outer)] else {
        unreachable!()
    };
    let ExpressionOperation::BorrowOwnedField { source: root, .. } = e.operation else {
        unreachable!()
    };
    let OwnerRecord::Expression(e) = &source.owners[&OwnerKey::Expression(inner)] else {
        unreachable!()
    };
    let ExpressionOperation::BorrowOwnedField { body, .. } = e.operation else {
        unreachable!()
    };
    let mut changed = source.clone();
    *operation(&mut changed, body) = call(reference(&source, "dispose-nested"), root);
    assert!(
        !accepts(&changed),
        "nested loans freeze the original ancestor"
    );

    let f = function(&source, "product-relay");
    let mut changed = source.clone();
    let ExpressionOperation::Sequence { items } = operation(&mut changed, f.body) else {
        unreachable!()
    };
    items.swap(0, 1);
    assert_eq!(items.len(), 2);
    assert!(!accepts(&changed), "a consumed source has no read rights");

    let OwnerRecord::Expression(sequence) = &source.owners[&OwnerKey::Expression(f.body)] else {
        unreachable!()
    };
    let ExpressionOperation::Sequence { items } = &sequence.operation else {
        unreachable!()
    };
    let OwnerRecord::Expression(scope) = &source.owners[&OwnerKey::Expression(items[0])] else {
        unreachable!()
    };
    let ExpressionOperation::BorrowOwnedField { binding: view, .. } = scope.operation else {
        unreachable!()
    };
    let mut changed = source.clone();
    *operation(&mut changed, items[1]) = ExpressionOperation::Local {
        value: LocalValueReference::LexicalBinding(view),
    };
    assert!(!accepts(&changed), "read rights expire at the lexical exit");
}

#[test]
fn borrowed_memory_oracle_checks_every_choice_arm_including_ordinary_payloads() {
    let source = source();
    let f = function(&source, "choice-read");
    let OwnerRecord::Expression(e) = &source.owners[&OwnerKey::Expression(f.body)] else {
        unreachable!()
    };
    let ExpressionOperation::MatchBorrowedOwned {
        source: root, arms, ..
    } = &e.operation
    else {
        unreachable!()
    };
    for arm in arms {
        let mut changed = source.clone();
        *operation(&mut changed, arm.body) = call(reference(&source, "dispose-choice"), *root);
        assert!(!accepts(&changed), "source is frozen in {} arm", arm.name);
    }
    for fault in [
        "missing",
        "duplicate-name",
        "duplicate-binding",
        "wrong-kind",
    ] {
        let mut changed = source.clone();
        if fault == "wrong-kind" {
            binding(&mut changed, arms[0].binding).kind = BindingKind::OwnedChoicePayload;
        } else {
            let ExpressionOperation::MatchBorrowedOwned { arms, .. } =
                operation(&mut changed, f.body)
            else {
                unreachable!()
            };
            match fault {
                "missing" => {
                    arms.pop();
                }
                "duplicate-name" => arms[1].name = arms[0].name.clone(),
                "duplicate-binding" => arms[1].binding = arms[0].binding,
                _ => unreachable!(),
            }
        }
        assert!(!accepts(&changed), "all arm contracts matter: {fault}");
    }
}

fn empty(seed: &[u8]) -> KernelSnapshot {
    let map = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(seed, 0),
            package_id: PackageId::migrate(seed, 0),
            package_name: Name::new("borrow_oracle").unwrap(),
            owners: map,
            dependencies: map,
            retirements: map,
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    }
}

fn publish(repository: &GraphRepository, source: &str) {
    let input = format!(
        "request base={}\n{source}",
        repository.view_current().unwrap().revision()
    );
    let request =
        crate::platform::control::decode_compact_change("borrow-oracle", input.as_bytes()).unwrap();
    let prepared = repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    repository.publish(&prepared.publication).unwrap();
}

fn imported() -> KernelSnapshot {
    let temporary = tempfile::tempdir().unwrap();
    let producer = GraphRepository::create(
        &temporary.path().join("producer"),
        &empty(b"borrow-reader"),
        None,
    )
    .unwrap()
    .repository;
    // The reader is checked and exported before any concrete storage witness exists.
    publish(&producer, &format!("{ABSTRACT}{GENERIC}"));
    let exported = producer.export_package_transport().unwrap();
    let consumer = GraphRepository::create(
        &temporary.path().join("consumer"),
        &empty(b"borrow-consumer"),
        None,
    )
    .unwrap()
    .repository;
    consumer
        .stage_package_transport(exported.transport_digest, &exported.container)
        .unwrap();
    let uses = ["abstraction", "read-loans"]
        .map(|name| {
            format!(
                "(use {name} {} {})",
                exported.revision.package, exported.revision_digest
            )
        })
        .join(" ");
    publish(
        &consumer,
        &format!(
            r#"add.dependency package={} semantic-revision={} package-revision={}
declarations.begin
(units {uses})
declarations.end
{CELL}
declarations.begin
(units (module create imported-reader
  (function create inspect (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field payload OwnedI64Cell) (field tag I64))) (use consume))
    (returns I64)
    (body (implementation-call read-loans::generic-read (types OwnedI64Cell)
      (implementations concrete@cell::Scalar) (local packet))))
  (function create alternate (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field payload OwnedI64Cell) (field tag I64))) (use consume))
    (returns I64)
    (body (implementation-call read-loans::generic-read (types OwnedI64Cell)
      (implementations concrete@cell::Alternate) (local packet))))))
declarations.end
"#,
            exported.revision.package,
            exported.revision.revision.revision_id().unwrap(),
            exported.revision_digest
        ),
    );
    consumer
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value
}

#[test]
fn borrowed_memory_oracle_accepts_exact_imported_generic_readers_and_witnesses() {
    let source = imported();
    assert!(accepts(&source));
    validate_full(&source).unwrap();
}

#[test]
fn borrowed_memory_oracle_rejects_weakened_imported_bounds_and_foreign_witnesses() {
    let source = imported();
    for fault in ["ordinary", "foreign-declaration", "missing-formal"] {
        let mut changed = source.clone();
        for interface in changed.dependency_interfaces.values_mut() {
            let interface = std::sync::Arc::make_mut(interface);
            let (id, f) = interface
                .iter()
                .find_map(|(key, owner)| match (key, owner) {
                    (OwnerKey::Declaration(id), PackageInterfaceRecord::Declaration(d))
                        if d.name.as_str() == "generic-read" =>
                    {
                        let PackageInterfaceDeclarationPayload::Function(f) = &d.payload else {
                            unreachable!()
                        };
                        Some((*id, f.clone()))
                    }
                    _ => None,
                })
                .unwrap();
            if fault == "missing-formal" {
                let PackageInterfaceRecord::Declaration(d) =
                    interface.get_mut(&OwnerKey::Declaration(id)).unwrap()
                else {
                    unreachable!()
                };
                let PackageInterfaceDeclarationPayload::Function(f) = &mut d.payload else {
                    unreachable!()
                };
                f.type_parameters.clear();
            } else {
                let PackageInterfaceRecord::TypeParameter(p) = interface
                    .get_mut(&OwnerKey::TypeParameter(f.type_parameters[0]))
                    .unwrap()
                else {
                    unreachable!()
                };
                if fault == "ordinary" {
                    p.constraints = TypeParameterConstraints::None;
                } else {
                    p.declaration = DeclarationId::migrate(b"foreign-borrow-formal", 0);
                }
            }
        }
        assert!(!accepts(&changed), "exact imported Owned bound: {fault}");
    }
    let local = self::source();
    let (_, _, body) = projection(&local, "generic-read");
    let mut changed = local.clone();
    let ExpressionOperation::ImplementationCall {
        implementations, ..
    } = operation(&mut changed, body)
    else {
        unreachable!()
    };
    let ImplementationOperand::Parameter { scope, .. } = &mut implementations[0] else {
        unreachable!()
    };
    scope.declaration = declaration(&local, "generic-method");
    assert!(!accepts(&changed), "a loan grants no foreign witness scope");
}
