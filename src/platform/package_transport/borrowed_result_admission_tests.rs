// Included by the source-loader test owner. All attacks use the neutral rehash
// helper and retain the original public interface; private meaning is changed.

const BORROWED_RESULT_TRANSPORT_SOURCE: &str = r#"declarations.begin
(units (module create returned-views
  (function create first (visibility private) (effect pure)
    (parameter create a (type OwnedI64Cell) (use borrow))
    (parameter create b (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from a)) (body (local a)))
  (function create public-view (visibility public) (effect pure)
    (parameter create a (type OwnedI64Cell) (use borrow))
    (parameter create b (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from a))
    (body (borrow-call (call first (local a) (local b))
      (binding selected (type OwnedI64Cell)) (in (local selected)))))
  (function create untaken (visibility private) (effect pure)
    (parameter create a (type OwnedI64Cell) (use borrow))
    (parameter create b (type OwnedI64Cell) (use borrow))
    (returns OwnedI64Cell (borrow-from a))
    (body (if (bool false) (local a) (local a))))
  (function create field-left (visibility private) (effect pure)
    (parameter create packet (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell))) (use borrow))
    (returns OwnedI64Cell (borrow-from packet))
    (body (borrow-owned-field (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
      (local packet) (field left (binding child (type OwnedI64Cell))) (in (local child)))))
  (function create public-child (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell))) (use borrow))
    (returns OwnedI64Cell (borrow-from packet))
    (body (borrow-call (call field-left (local packet))
      (binding selected (type OwnedI64Cell)) (in (local selected)))))
  (owned-contract create UnusedReader (visibility private)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_a7010000000000000000000000000001 at
      (parameters (Self borrow) (Self borrow))
      (returns Self (borrow-from 0))))
  (owned-implementation create UnusedCellReader (visibility private)
    (contract UnusedReader) (self OwnedI64Cell)
    (method method_a7010000000000000000000000000001 first))))
declarations.end"#;

fn borrowed_result_transport() -> AdmittedClosure {
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        BORROWED_RESULT_TRANSPORT_SOURCE,
    ).unwrap();
    let directory = tempfile::tempdir().unwrap();
    GraphRepository::create(&directory.path().join("source"), &snapshot, None)
        .unwrap().repository.export_package_container().unwrap()
}

fn borrowed_transport_declaration(snapshot: &KernelSnapshot, name: &str) -> OwnerRecord {
    snapshot.owners.values().find(|owner| matches!(owner,
        OwnerRecord::Declaration(record) if record.name.as_str() == name
    )).unwrap().clone()
}

fn borrowed_transport_function(snapshot: &KernelSnapshot, name: &str) -> FunctionDeclaration {
    let OwnerRecord::Declaration(record) = borrowed_transport_declaration(snapshot, name) else { unreachable!() };
    let DeclarationPayload::Function(function) = record.payload else { unreachable!() };
    function
}

#[test]
fn borrowed_results_transport_rejects_consistently_rehashed_sources_modes_guards_and_unused_paths() {
    let original = borrowed_result_transport();
    let package = &original.packages[&original.container.root.package_revision];
    let snapshot = &package.snapshot;
    assert_eq!(snapshot.root.graph_contract_version, 27);
    assert!(crate::platform::kernel::memory_reference::accepts(snapshot));

    let first = borrowed_transport_function(snapshot, "first");
    let neutral = rehash_owner(&original, borrowed_transport_declaration(snapshot, "first"));
    let neutral_bytes = neutral.encode().unwrap();
    let neutral_decoded = PackageContainer::decode(&neutral_bytes, neutral.root.transport).unwrap();
    neutral_decoded.admit().unwrap();
    let independent = crate::platform::package_transport::oracle::reconstruct(&neutral_decoded).unwrap();
    assert_eq!(independent.snapshots[&snapshot.root.package_id].owners, snapshot.owners);
    assert_eq!(neutral.root, original.container.root);

    let mut attacks = Vec::new();
    let mut changed_source = borrowed_transport_declaration(snapshot, "first");
    let OwnerRecord::Declaration(d) = &mut changed_source else { unreachable!() };
    let DeclarationPayload::Function(f) = &mut d.payload else { unreachable!() };
    f.result_borrow = Some(first.parameters[1]);
    attacks.push(("exact-source", changed_source, "kernel_buffer_ownership"));

    let mut changed_mode = borrowed_transport_declaration(snapshot, "first");
    let OwnerRecord::Declaration(d) = &mut changed_mode else { unreachable!() };
    let DeclarationPayload::Function(f) = &mut d.payload else { unreachable!() };
    f.result_borrow = None;
    attacks.push(("counterfeit-owning-result", changed_mode, "kernel_buffer_ownership"));

    let untaken = borrowed_transport_function(snapshot, "untaken");
    let OwnerRecord::Expression(branch) = &snapshot.owners[&OwnerKey::Expression(untaken.body)] else { unreachable!() };
    let ExpressionOperation::If { condition, when_true, .. } = branch.operation else { unreachable!() };
    assert!(matches!(snapshot.owners[&OwnerKey::Expression(condition)],
        OwnerRecord::Expression(ExpressionRecord { operation: ExpressionOperation::Bool { value: false }, .. })));
    let mut wrong_untaken_root = snapshot.owners[&OwnerKey::Expression(when_true)].clone();
    let OwnerRecord::Expression(expression) = &mut wrong_untaken_root else { unreachable!() };
    expression.operation = ExpressionOperation::Local {
        value: LocalValueReference::FunctionParameter(untaken.parameters[1]),
    };
    attacks.push(("untaken-wrong-root", wrong_untaken_root, "kernel_buffer_ownership"));

    // The binding remains locally well-formed and exactly typed, but its read
    // guard has been replaced with owning rights. No absent-object attack is used.
    let projected = borrowed_transport_function(snapshot, "field-left");
    let OwnerRecord::Expression(scope) = &snapshot.owners[&OwnerKey::Expression(projected.body)] else { unreachable!() };
    let ExpressionOperation::BorrowOwnedField { binding, .. } = scope.operation else { unreachable!() };
    let mut erased_guard = snapshot.owners[&OwnerKey::Binding(binding)].clone();
    let OwnerRecord::Binding(record) = &mut erased_guard else { unreachable!() };
    record.kind = BindingKind::OwnedUnpack;
    attacks.push(("erased-ancestor-guard", erased_guard, "kernel_full_binding_kind"));

    for (fault, mode) in [("unused-method-position", Some(1)), ("unused-method-owning", None)] {
        let mut changed_method = borrowed_transport_declaration(snapshot, "UnusedReader");
        let OwnerRecord::Declaration(d) = &mut changed_method else { unreachable!() };
        let DeclarationPayload::OwnedContract(contract) = &mut d.payload else { unreachable!() };
        contract.methods[0].result_borrow = mode;
        attacks.push((fault, changed_method, "kernel_owned_contract"));
    }

    for (fault, replacement, expected) in attacks {
        // Canonical owner admission succeeds. Every enclosing source/map,
        // revision, transport and witness binding is recomputed by the helper.
        let (digest, bytes) = encode_owner(&replacement).unwrap();
        assert_eq!(decode_owner(&bytes, replacement.owner(), replacement.kind(), digest).unwrap(), replacement);
        let mut hostile_inventory = snapshot.clone();
        hostile_inventory.owners.insert(replacement.owner(), replacement.clone());
        assert!(!crate::platform::kernel::memory_reference::accepts(&hostile_inventory), "independent memory accepted {fault}");
        let hostile = rehash_owner(&original, replacement);
        let bytes = hostile.encode().unwrap();
        let decoded = PackageContainer::decode(&bytes, hostile.root.transport).unwrap();
        let failure = decoded.admit().unwrap_err();
        assert_eq!(failure.code, expected, "{fault}: {failure:?}");
        let independent = crate::platform::package_transport::oracle::reconstruct(&decoded).unwrap_err();
        assert!(independent.code == expected || independent.message.contains(expected), "{fault}: {independent:?}");
        assert_not_ready(&bytes, hostile.root.transport);
    }
}
