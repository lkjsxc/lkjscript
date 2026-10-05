//! Exact source identities and lexical custody checked without production
//! ownership classifications. Canonical re-encoding deliberately admits the
//! locally valid hostile records before this complete-source oracle rejects.
use super::*;
use crate::platform::persistent_map::{MapContentDigest, MapRoot, PageDigest};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::{DeclarationId, ParameterId, RepositoryId};

const SOURCE: &str = r#"declarations.begin
(units (module create results
  (external create make (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create read (visibility public) (implementation core.cell.read)
    (parameter create owner (type OwnedI64Cell) (use borrow)) (returns I64))
  (function create discard (visibility private) (effect pure)
    (parameter create owner (type OwnedI64Cell) (use consume))
    (returns I64) (body (i64 0)))
  (function create direct (visibility public) (effect pure)
    (parameter create owner (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from owner)) (body (local owner)))
  (function create first (visibility public) (effect pure)
    (parameter create a (type OwnedI64Cell) (use borrow))
    (parameter create b (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from a)) (body (local a)))
  (function create branch-root (visibility public) (effect pure)
    (parameter create flag (type Bool))
    (parameter create a (type OwnedI64Cell) (use borrow))
    (parameter create b (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from a))
    (body (if (local flag) (local a) (local a))))
  (function create field-left (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell))) (use borrow))
    (returns OwnedI64Cell (borrow-from packet))
    (body (borrow-owned-field (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
      (local packet) (field left (binding view (type OwnedI64Cell))) (in (local view)))))
  (function create branch-fields (visibility public) (effect pure)
    (parameter create flag (type Bool))
    (parameter create packet (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell))) (use borrow))
    (returns OwnedI64Cell (borrow-from packet))
    (body (if (local flag)
      (borrow-owned-field (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
        (local packet) (field left (binding left-view (type OwnedI64Cell))) (in (local left-view)))
      (borrow-owned-field (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
        (local packet) (field right (binding right-view (type OwnedI64Cell))) (in (local right-view))))))
  (function create relay (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell))) (use borrow))
    (returns OwnedI64Cell (borrow-from packet))
    (body (borrow-call (call field-left (local packet)) (binding view (type OwnedI64Cell)) (in (local view)))))
  (function create aliased (visibility public) (effect pure)
    (parameter create owner (type OwnedI64Cell) (use consume))
    (returns OwnedI64Cell)
    (body (sequence
      (borrow-call (call first (local owner) (local owner)) (binding view (type OwnedI64Cell))
        (in (call read (local view))))
      (local owner))))
  (function create nested (visibility public) (effect pure)
    (parameter create packet (type (owned-product
      (field inner (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
      (field sibling OwnedI64Cell))) (use borrow))
    (returns OwnedI64Cell (borrow-from packet))
    (body (borrow-owned-field (type (owned-product
      (field inner (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
      (field sibling OwnedI64Cell))) (local packet)
      (field inner (binding inner-view (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))))
      (in (borrow-call (call relay (local inner-view)) (binding view (type OwnedI64Cell)) (in (local view)))))))
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Item (constraint owned))
    (method method_a7000000000000000000000000000001 at
      (parameters (I64 unrestricted) (Self borrow))
      (returns Item (borrow-from 1))))
  (function create at-left (visibility public) (effect pure)
    (parameter create index (type I64))
    (parameter create packet (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell))) (use borrow))
    (returns OwnedI64Cell (borrow-from packet))
    (body (borrow-call (call field-left (local packet)) (binding view (type OwnedI64Cell)) (in (local view)))))
  (owned-implementation create PairReader (visibility public)
    (contract Reader) (self (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell))) (types OwnedI64Cell)
    (method method_a7000000000000000000000000000001 at-left))
  (function create generic-at (visibility public) (effect pure)
    (type-parameter create Storage (constraint owned))
    (type-parameter create Item (constraint owned))
    (implementation-parameter implparam_a7000000000000000000000000000001 reader Reader Storage (types Item))
    (parameter create index (type I64))
    (parameter create storage (type Storage) (use borrow))
    (returns Item (borrow-from storage))
    (body (borrow-call
      (method-call parameter@generic-at@implparam_a7000000000000000000000000000001 Reader
        method_a7000000000000000000000000000001 (local index) (local storage))
      (binding view (type Item)) (in (local view)))))
  (function create generic-use (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell))) (use consume))
    (returns (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
    (body (sequence
      (borrow-call
        (implementation-call generic-at
          (types (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)) OwnedI64Cell)
          (implementations concrete@PairReader) (i64 0) (local packet))
        (binding view (type OwnedI64Cell)) (in (call read (local view))))
      (local packet))))))
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

fn function(snapshot: &KernelSnapshot, name: &str) -> FunctionDeclaration {
    let OwnerRecord::Declaration(d) =
        &snapshot.owners[&OwnerKey::Declaration(declaration(snapshot, name))]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(f) = &d.payload else {
        unreachable!()
    };
    f.clone()
}

fn operation(snapshot: &mut KernelSnapshot, id: ExpressionId) -> &mut ExpressionOperation {
    let OwnerRecord::Expression(e) = snapshot.owners.get_mut(&OwnerKey::Expression(id)).unwrap()
    else {
        unreachable!()
    };
    &mut e.operation
}

fn payload(snapshot: &mut KernelSnapshot, id: DeclarationId) -> &mut DeclarationPayload {
    let OwnerRecord::Declaration(d) = snapshot.owners.get_mut(&OwnerKey::Declaration(id)).unwrap()
    else {
        unreachable!()
    };
    &mut d.payload
}

fn new_expression(
    snapshot: &mut KernelSnapshot,
    seed: &[u8],
    op: ExpressionOperation,
) -> ExpressionId {
    let id = ExpressionId::migrate(seed, 0);
    snapshot.owners.insert(
        OwnerKey::Expression(id),
        OwnerRecord::Expression(ExpressionRecord::new(id, op).unwrap()),
    );
    id
}

fn canonical_round_trip(snapshot: &mut KernelSnapshot) {
    for (key, owner) in &mut snapshot.owners {
        let kind = owner.header().kind;
        let (digest, bytes) = encode_owner(owner).unwrap();
        *owner = decode_owner(&bytes, *key, kind, digest).unwrap();
    }
}

#[test]
fn borrowed_result_oracle_accepts_exact_roots_descendants_forwarding_aliases_and_witnesses() {
    let snapshot = source();
    assert!(accepts(&snapshot));
    validate_full(&snapshot).unwrap();
}

#[test]
fn borrowed_result_oracle_rejects_rehashed_wrong_parameter_even_when_call_arguments_alias() {
    let mut snapshot = source();
    let first = function(&snapshot, "first");
    *operation(&mut snapshot, first.body) = ExpressionOperation::Local {
        value: LocalValueReference::FunctionParameter(first.parameters[1]),
    };
    canonical_round_trip(&mut snapshot);
    assert!(
        !accepts(&snapshot),
        "the same caller local cannot erase distinct callee parameter identities"
    );
}

#[test]
fn borrowed_result_oracle_checks_untaken_return_paths_and_rejects_owning_results() {
    let baseline = source();
    let branch = function(&baseline, "branch-root");
    let ExpressionOperation::If { when_false, .. } =
        *Oracle(&baseline, None).expression(branch.body).unwrap()
    else {
        unreachable!()
    };
    let mut snapshot = baseline.clone();
    *operation(&mut snapshot, when_false) = ExpressionOperation::Local {
        value: LocalValueReference::FunctionParameter(branch.parameters[2]),
    };
    canonical_round_trip(&mut snapshot);
    assert!(!accepts(&snapshot));

    let mut snapshot = baseline.clone();
    let direct = function(&snapshot, "direct");
    let n = new_expression(
        &mut snapshot,
        b"borrow-result-owned-argument",
        ExpressionOperation::I64 { value: 7 },
    );
    let make = DeclarationReference {
        package: snapshot.root.package_id,
        declaration: declaration(&snapshot, "make"),
    };
    *operation(&mut snapshot, direct.body) = ExpressionOperation::Call {
        function: make,
        arguments: vec![n],
        type_arguments: vec![],
        requirement_arguments: vec![],
        effect_arguments: vec![],
    };
    canonical_round_trip(&mut snapshot);
    assert!(
        !accepts(&snapshot),
        "fresh owners cannot satisfy source-tied returns"
    );
}

#[test]
fn borrowed_result_oracle_rejects_source_consumption_and_forged_owning_bindings() {
    let baseline = source();
    let aliased = function(&baseline, "aliased");
    let oracle = Oracle(&baseline, None);
    let ExpressionOperation::Sequence { items } = oracle.expression(aliased.body).unwrap() else {
        unreachable!()
    };
    let scope = items[0];
    let ExpressionOperation::BorrowCall {
        call,
        binding,
        body,
    } = *oracle.expression(scope).unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::Call { arguments, .. } = oracle.expression(call).unwrap() else {
        unreachable!()
    };
    let owner = arguments[0];
    let mut snapshot = baseline.clone();
    let discard = DeclarationReference {
        package: snapshot.root.package_id,
        declaration: declaration(&snapshot, "discard"),
    };
    let consume = new_expression(
        &mut snapshot,
        b"borrow-result-source-consumption",
        ExpressionOperation::Call {
            function: discard,
            arguments: vec![owner],
            type_arguments: vec![],
            requirement_arguments: vec![],
            effect_arguments: vec![],
        },
    );
    let inner = new_expression(
        &mut snapshot,
        b"borrow-result-source-consumption-body",
        ExpressionOperation::Sequence {
            items: vec![consume, body],
        },
    );
    *operation(&mut snapshot, scope) = ExpressionOperation::BorrowCall {
        call,
        binding,
        body: inner,
    };
    canonical_round_trip(&mut snapshot);
    assert!(
        !accepts(&snapshot),
        "the selected source stays protected for the body"
    );

    let mut snapshot = baseline;
    let OwnerRecord::Binding(record) = snapshot
        .owners
        .get_mut(&OwnerKey::Binding(binding))
        .unwrap()
    else {
        unreachable!()
    };
    record.kind = BindingKind::Let;
    record.value = Some(call);
    *operation(&mut snapshot, scope) = ExpressionOperation::Let {
        bindings: vec![binding],
        body,
    };
    canonical_round_trip(&mut snapshot);
    assert!(
        !accepts(&snapshot),
        "a forged owner cannot receive a borrowed-result ordinary call"
    );
}

#[test]
fn borrowed_result_oracle_checks_unused_method_relations_and_function_metadata() {
    let baseline = source();
    for fault in [
        "method-position",
        "method-owning",
        "function-owning",
        "task",
        "unknown-source",
        "consume-source",
    ] {
        let mut snapshot = baseline.clone();
        let reader = declaration(&snapshot, "Reader");
        let direct_id = declaration(&snapshot, "direct");
        let direct = function(&snapshot, "direct");
        match fault {
            "method-position" | "method-owning" => {
                let DeclarationPayload::OwnedContract(c) = payload(&mut snapshot, reader) else {
                    unreachable!()
                };
                c.methods[0].result_borrow = if fault == "method-position" {
                    Some(0)
                } else {
                    None
                };
            }
            "consume-source" => {
                let OwnerRecord::Parameter(p) = snapshot
                    .owners
                    .get_mut(&OwnerKey::Parameter(direct.parameters[0]))
                    .unwrap()
                else {
                    unreachable!()
                };
                p.use_mode = ParameterUse::Consume;
            }
            _ => {
                let DeclarationPayload::Function(f) = payload(&mut snapshot, direct_id) else {
                    unreachable!()
                };
                match fault {
                    "function-owning" => f.result_borrow = None,
                    "task" => {
                        f.effect = FunctionEffect::Task {
                            requirements: vec![],
                            effect_parameters: vec![],
                        }
                    }
                    "unknown-source" => {
                        f.result_borrow =
                            Some(ParameterId::migrate(b"unknown-borrow-result-source", 0))
                    }
                    _ => unreachable!(),
                }
            }
        }
        assert!(!accepts(&snapshot), "complete canonical relation: {fault}");
    }
}

#[test]
fn borrowed_result_oracle_rejects_predecessor_meaning_even_in_unused_records() {
    let baseline = source();
    for target in ["root", "function", "contract", "scope"] {
        let mut snapshot = baseline.clone();
        match target {
            "root" => snapshot.root.graph_contract_version = 26,
            "function" | "contract" => {
                let id = declaration(
                    &snapshot,
                    if target == "function" {
                        "direct"
                    } else {
                        "Reader"
                    },
                );
                let OwnerRecord::Declaration(d) =
                    snapshot.owners.get_mut(&OwnerKey::Declaration(id)).unwrap()
                else {
                    unreachable!()
                };
                d.header.contract_version = 26;
            }
            "scope" => {
                let body = function(&snapshot, "relay").body;
                let OwnerRecord::Expression(e) = snapshot
                    .owners
                    .get_mut(&OwnerKey::Expression(body))
                    .unwrap()
                else {
                    unreachable!()
                };
                e.contract_version = 26;
            }
            _ => unreachable!(),
        }
        assert!(
            !accepts(&snapshot),
            "predecessor cannot carry result meaning: {target}"
        );
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
            package_name: Name::new("result_oracle").unwrap(),
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
        crate::platform::control::decode_compact_change("borrowed-result-oracle", input.as_bytes())
            .unwrap();
    let prepared = repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    repository.publish(&prepared.publication).unwrap();
}

fn imported() -> KernelSnapshot {
    let temporary = tempfile::tempdir().unwrap();
    let producer = GraphRepository::create(
        &temporary.path().join("producer"),
        &empty(b"result-oracle-producer"),
        None,
    )
    .unwrap()
    .repository;
    publish(&producer, SOURCE);
    let export = producer.export_package_transport().unwrap();
    let consumer = GraphRepository::create(
        &temporary.path().join("consumer"),
        &empty(b"result-oracle-consumer"),
        None,
    )
    .unwrap()
    .repository;
    consumer
        .stage_package_transport(export.transport_digest, &export.container)
        .unwrap();
    publish(
        &consumer,
        &format!(
            r#"add.dependency package={} semantic-revision={} package-revision={}
declarations.begin
(units (use results {} {}) (module create consumer
  (function create inspect (visibility public) (effect pure)
    (parameter create owner (type OwnedI64Cell) (use consume)) (returns OwnedI64Cell)
    (body (sequence
      (borrow-call (call results::direct (local owner)) (binding view (type OwnedI64Cell)) (in (call results::read (local view))))
      (local owner))))))
declarations.end
"#,
            export.revision.package,
            export.revision.revision.revision_id().unwrap(),
            export.revision_digest,
            export.revision.package,
            export.revision_digest
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
fn borrowed_result_oracle_validates_imported_unused_signature_source_and_generation() {
    let baseline = imported();
    assert!(accepts(&baseline));
    for fault in ["missing-formal", "consume", "generation"] {
        let mut snapshot = baseline.clone();
        for interface in snapshot.dependency_interfaces.values_mut() {
            let id = interface
                .iter()
                .find_map(|(key, owner)| match (key, owner) {
                    (OwnerKey::Declaration(id), PackageInterfaceRecord::Declaration(d))
                        if d.name.as_str() == "direct" =>
                    {
                        Some(*id)
                    }
                    _ => None,
                })
                .unwrap();
            let PackageInterfaceRecord::Declaration(d) =
                interface.get_mut(&OwnerKey::Declaration(id)).unwrap()
            else {
                unreachable!()
            };
            let PackageInterfaceDeclarationPayload::Function(f) = &mut d.payload else {
                unreachable!()
            };
            let source = f.result_borrow.unwrap();
            match fault {
                "missing-formal" => f.parameters.clear(),
                "generation" => d.header.contract_version = 26,
                "consume" => {
                    let PackageInterfaceRecord::Parameter(p) =
                        interface.get_mut(&OwnerKey::Parameter(source)).unwrap()
                    else {
                        unreachable!()
                    };
                    p.use_mode = ParameterUse::Consume;
                }
                _ => unreachable!(),
            }
        }
        assert!(
            !accepts(&snapshot),
            "imported canonical result source: {fault}"
        );
    }
}
