//! Sequence attacks through the independent canonical owner/loan inventory model.
use super::*;
use crate::platform::semantic_id::RevisionId;
use crate::platform::semantic_id::{BindingId, DeclarationId};

const SOURCE: &str = include_str!("sequence_memory_test_source.lkjc");

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

fn binding(snapshot: &mut KernelSnapshot, id: BindingId) -> &mut BindingRecord {
    let OwnerRecord::Binding(binding) = snapshot.owners.get_mut(&OwnerKey::Binding(id)).unwrap()
    else {
        unreachable!()
    };
    binding
}

fn read(
    snapshot: &KernelSnapshot,
    name: &str,
) -> (
    ExpressionId,
    ExpressionId,
    ExpressionId,
    BindingId,
    ExpressionId,
) {
    let f = function(snapshot, name);
    let OwnerRecord::Expression(e) = &snapshot.owners[&OwnerKey::Expression(f.body)] else {
        unreachable!()
    };
    let ExpressionOperation::BorrowOwnedItem {
        source,
        index,
        binding,
        body,
        ..
    } = e.operation
    else {
        unreachable!()
    };
    (f.body, source, index, binding, body)
}

fn dispose(snapshot: &KernelSnapshot, argument: ExpressionId) -> ExpressionOperation {
    ExpressionOperation::Call {
        function: DeclarationReference {
            package: snapshot.root.package_id,
            declaration: declaration(snapshot, "dispose-sequence"),
        },
        type_arguments: Vec::new(),
        requirement_arguments: Vec::new(),
        effect_arguments: Vec::new(),
        arguments: vec![argument],
    }
}

#[test]
fn sequence_oracle_accepts_generic_and_nested_native_owner_flows() {
    let snapshot = source();
    assert!(accepts(&snapshot));
    validate_full(&snapshot).unwrap();
}

#[test]
fn sequence_oracle_rejects_counterfeit_views_orphans_and_moved_index_sources() {
    let snapshot = source();
    let (id, input, index, view, body) = read(&snapshot, "read");
    for role in [
        BindingKind::Let,
        BindingKind::OwnedUnpack,
        BindingKind::OwnedChoicePayload,
    ] {
        let mut changed = snapshot.clone();
        binding(&mut changed, view).kind = role;
        assert!(!accepts(&changed), "forged sequence read view {role:?}");
    }
    let mut changed = snapshot.clone();
    binding(&mut changed, view).declared_type = Some(function(&snapshot, "read").result);
    assert!(
        !accepts(&changed),
        "a sequence view has its exact element type"
    );
    let mut changed = snapshot.clone();
    *operation(&mut changed, id) = ExpressionOperation::I64 { value: 0 };
    assert!(
        !accepts(&changed),
        "read bindings cannot survive without their defining scope"
    );
    let mut changed = snapshot.clone();
    *operation(&mut changed, index) = dispose(&snapshot, input);
    assert!(
        !accepts(&changed),
        "source liveness is checked after evaluating the index"
    );
    let mut changed = snapshot.clone();
    *operation(&mut changed, body) = dispose(&snapshot, input);
    assert!(
        !accepts(&changed),
        "active item reads freeze source custody"
    );
}

#[test]
fn sequence_oracle_rejects_item_escape_and_counterfeit_source_identity() {
    let snapshot = source();
    let (_, _, _, view, body) = read(&snapshot, "independent-result");
    let mut changed = snapshot.clone();
    *operation(&mut changed, body) = ExpressionOperation::Local {
        value: LocalValueReference::LexicalBinding(view),
    };
    assert!(
        !accepts(&changed),
        "an item view cannot become a returned owner"
    );
    let (_, input, _, _, _) = read(&snapshot, "read");
    let mut changed = snapshot.clone();
    *operation(&mut changed, input) = ExpressionOperation::Local {
        value: LocalValueReference::LexicalBinding(BindingId::migrate(
            b"counterfeit-sequence-source",
            0,
        )),
    };
    assert!(
        !accepts(&changed),
        "the annotation grants no owner or read right"
    );
}

#[test]
fn sequence_oracle_rejects_wrong_pop_envelopes_and_push_element_types() {
    let snapshot = source();
    let pop = function(&snapshot, "pop").body;
    let mut changed = snapshot.clone();
    let ExpressionOperation::SequencePop {
        sequence_type,
        result_type,
        ..
    } = operation(&mut changed, pop)
    else {
        unreachable!()
    };
    *result_type = *sequence_type;
    assert!(
        !accepts(&changed),
        "pop returns its exact empty or item envelope"
    );
    let push = function(&snapshot, "push").body;
    let mut changed = snapshot.clone();
    let ExpressionOperation::SequencePush { value, source, .. } = operation(&mut changed, push)
    else {
        unreachable!()
    };
    *value = *source;
    assert!(
        !accepts(&changed),
        "the push item must have the exact declared element type"
    );
    let mut changed = snapshot.clone();
    let (_, input, _, _, _) = read(&snapshot, "read");
    let ExpressionOperation::Local { value } = operation(&mut changed, input) else {
        unreachable!()
    };
    let ty = Oracle(&snapshot, None).local_type(*value).unwrap();
    let i64 = function(&snapshot, "read").result;
    changed.types.get_mut(&ty).unwrap().form = TypeForm::OwnedSequence { item: i64 };
    assert!(
        !accepts(&changed),
        "ordinary data cannot be a sequence element"
    );
}

#[test]
fn sequence_oracle_rejects_predecessor_authority_even_for_local_only_signatures() {
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create sequence-generation
  (function create relay (visibility public) (effect pure)
    (parameter create seq (type (owned-sequence ByteBuffer)) (use consume))
    (returns (owned-sequence ByteBuffer)) (body (local seq)))
  (function create discard (visibility public) (effect pure)
    (parameter create seq (type (owned-sequence ByteBuffer)) (use consume))
    (returns I64) (body (i64 0)))))
declarations.end"#,
    )
    .unwrap();
    assert!(accepts(&snapshot));
    let mut changed = snapshot.clone();
    changed.root.graph_contract_version = 24;
    assert!(!accepts(&changed), "a sequence type closure needs Graph 25");
    for name in ["relay", "discard"] {
        let mut changed = snapshot.clone();
        let id = declaration(&snapshot, name);
        let OwnerRecord::Declaration(record) =
            changed.owners.get_mut(&OwnerKey::Declaration(id)).unwrap()
        else {
            unreachable!()
        };
        record.header.contract_version = 24;
        assert!(
            !accepts(&changed),
            "owner generations cover complete sequence signatures: {name}"
        );
        assert!(
            super::super::memory::validate_owner(
                &changed,
                OwnerKey::Declaration(id),
                &changed.owners[&OwnerKey::Declaration(id)]
            )
            .is_err()
        );
    }
}

#[test]
fn sequence_oracle_compares_generic_item_types_with_exact_argument_locals() {
    let snapshot = source();
    let buffer_call = function(&snapshot, "buffer-count").body;
    let cell_call = function(&snapshot, "cell-count").body;
    let OwnerRecord::Expression(cell) = &snapshot.owners[&OwnerKey::Expression(cell_call)] else {
        unreachable!()
    };
    let ExpressionOperation::Call {
        type_arguments: cell_types,
        ..
    } = &cell.operation
    else {
        unreachable!()
    };
    let mut changed = snapshot.clone();
    let ExpressionOperation::Call { type_arguments, .. } = operation(&mut changed, buffer_call)
    else {
        unreachable!()
    };
    *type_arguments = cell_types.clone();
    assert!(
        !accepts(&changed),
        "Owned arguments retain exact instantiated sequence element types"
    );
}

#[test]
fn sequence_oracle_admits_generations_of_complete_unused_imported_signatures() {
    let mut snapshot = source();
    let id = declaration(&snapshot, "dispose-sequence");
    let parameter = function(&snapshot, "dispose-sequence").parameters[0];
    let mut declaration = snapshot.owners[&OwnerKey::Declaration(id)].clone();
    let OwnerRecord::Declaration(record) = &mut declaration else {
        unreachable!()
    };
    record.visibility = DeclarationVisibility::Public;
    let owners = BTreeMap::from([
        (
            OwnerKey::Declaration(id),
            PackageInterfaceRecord::project_public(&declaration)
                .unwrap()
                .unwrap(),
        ),
        (
            OwnerKey::Parameter(parameter),
            PackageInterfaceRecord::project_public(
                &snapshot.owners[&OwnerKey::Parameter(parameter)],
            )
            .unwrap()
            .unwrap(),
        ),
    ]);
    let package = PackageId::migrate(b"unused-sequence-import-generation", 0);
    let revision = PackageRevisionDigest::from_bytes([81; 32]);
    snapshot.dependencies.insert(
        package,
        DependencyRecord {
            graph_contract_version: 25,
            package,
            semantic_revision: RevisionId::from_digest([82; 32]),
            package_revision: revision,
        },
    );
    snapshot
        .dependency_interfaces
        .insert(revision, owners.into());
    assert!(accepts(&snapshot));
    let mut changed = snapshot.clone();
    let PackageInterfaceRecord::Declaration(declaration) =
        std::sync::Arc::make_mut(changed.dependency_interfaces.get_mut(&revision).unwrap())
            .get_mut(&OwnerKey::Declaration(id))
            .unwrap()
    else {
        unreachable!()
    };
    declaration.header.contract_version = 24;
    assert!(
        !accepts(&changed),
        "new parameter owners cannot upgrade an older imported function"
    );
    let mut changed = snapshot.clone();
    let PackageInterfaceRecord::Parameter(parameter) =
        std::sync::Arc::make_mut(changed.dependency_interfaces.get_mut(&revision).unwrap())
            .get_mut(&OwnerKey::Parameter(parameter))
            .unwrap()
    else {
        unreachable!()
    };
    parameter.header.contract_version = 24;
    assert!(
        !accepts(&changed),
        "every unused imported parameter keeps its type-generation boundary"
    );
}
