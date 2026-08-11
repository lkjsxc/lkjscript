#![allow(clippy::expect_used, clippy::panic)]

use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_vm::{run_chunk, ExecutionInputs};

use super::*;

fn run_i64(snapshot: &WorkspaceSnapshot) -> i64 {
    let executable = crate::compile_snapshot(snapshot).expect("compile complete snapshot");
    match run_chunk(
        executable.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    ) {
        ExecutionOutcome::Returned(value) => value.as_i64().expect("returned i64"),
        outcome => panic!("unexpected execution outcome: {outcome:?}"),
    }
}

fn push_node(nodes: &mut Vec<DraftNode>, node: DraftNode) -> DraftNodeId {
    let id = DraftNodeId::new(u64::try_from(nodes.len()).expect("draft node identity"));
    nodes.push(node);
    id
}

fn counter_program_draft() -> ExpressionDraft {
    let counter = DraftBindingId::new(0);
    let mut nodes = Vec::new();
    let initial = push_node(&mut nodes, DraftNode::I64(0));
    let one = push_node(&mut nodes, DraftNode::I64(1));
    let set_one = push_node(
        &mut nodes,
        DraftNode::SetLocal {
            target: DraftBindingRef::Local(counter),
            value: one,
        },
    );
    let two = push_node(&mut nodes, DraftNode::I64(2));
    let set_two = push_node(
        &mut nodes,
        DraftNode::SetLocal {
            target: DraftBindingRef::Local(counter),
            value: two,
        },
    );
    let load = push_node(&mut nodes, DraftNode::Load(DraftBindingRef::Local(counter)));
    let sequence = push_node(
        &mut nodes,
        DraftNode::Sequence(vec![set_one, set_two, load]),
    );
    let root = push_node(
        &mut nodes,
        DraftNode::MutableLocal {
            binding: counter,
            name: "counter".to_owned(),
            ty: SemanticType::I64,
            initial,
            body: sequence,
        },
    );
    ExpressionDraft::new(nodes, root)
}

fn reordered_sequence_draft(counter: EntityId) -> ExpressionDraft {
    let mut nodes = Vec::new();
    let two = push_node(&mut nodes, DraftNode::I64(2));
    let set_two = push_node(
        &mut nodes,
        DraftNode::SetLocal {
            target: DraftBindingRef::Entity(counter),
            value: two,
        },
    );
    let one = push_node(&mut nodes, DraftNode::I64(1));
    let set_one = push_node(
        &mut nodes,
        DraftNode::SetLocal {
            target: DraftBindingRef::Entity(counter),
            value: one,
        },
    );
    let load = push_node(
        &mut nodes,
        DraftNode::Load(DraftBindingRef::Entity(counter)),
    );
    let sequence = push_node(
        &mut nodes,
        DraftNode::Sequence(vec![set_two, set_one, load]),
    );
    ExpressionDraft::new(nodes, sequence)
}

fn complete_counter_workspace(seed: u64) -> (Workspace, std::sync::Arc<WorkspaceSnapshot>) {
    let mut workspace = Workspace::empty_deterministic(seed).expect("counter workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create counter main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: counter_program_draft(),
            }],
        })
        .expect("fill counter main");
    (workspace, completed.snapshot)
}

fn entity_named(snapshot: &WorkspaceSnapshot, kind: EntityKind, name: &str) -> EntityId {
    snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == kind && entity.name.as_ref() == name)
        .expect("named entity")
        .id
}

fn unique_node(snapshot: &WorkspaceSnapshot, kind: NodeKind) -> NodeId {
    let mut matches = snapshot.nodes().iter().filter(|node| node.kind == kind);
    let id = matches.next().expect("node kind").id;
    assert!(matches.next().is_none(), "node kind must be unique");
    id
}

fn subtree_nodes(snapshot: &WorkspaceSnapshot, root: NodeId) -> Vec<NodeId> {
    let mut pending = vec![root];
    let mut nodes = Vec::new();
    while let Some(node) = pending.pop() {
        nodes.push(node);
        if let Some(children) = snapshot.indexes.node_children.get(&node) {
            pending.extend(children.iter().rev().copied());
        }
    }
    nodes
}

fn direct_children(snapshot: &WorkspaceSnapshot, parent: NodeId) -> Vec<NodeId> {
    snapshot
        .containment()
        .iter()
        .filter_map(|edge| match (edge.owner, edge.child) {
            (SemanticOwner::Node(owner), SemanticChild::Node(child)) if owner == parent => {
                Some(child)
            }
            _ => None,
        })
        .collect()
}

fn complete_i64_sequence(
    seed: u64,
    kinds: Vec<DraftNode>,
    children: Vec<DraftNodeId>,
) -> (Workspace, std::sync::Arc<WorkspaceSnapshot>, NodeId) {
    let mut workspace = Workspace::empty_deterministic(seed).expect("sequence workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create sequence main");
    let hole = created.snapshot.holes().next().expect("sequence hole").id;
    let mut nodes = kinds;
    let root = push_node(&mut nodes, DraftNode::Sequence(children));
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(nodes, root),
            }],
        })
        .expect("fill sequence main");
    let sequence = unique_node(&completed.snapshot, NodeKind::Sequence);
    (workspace, completed.snapshot, sequence)
}

fn assert_atomic_error(
    workspace: &mut Workspace,
    published: &std::sync::Arc<WorkspaceSnapshot>,
    edits: Vec<Edit>,
) -> WorkspaceError {
    let error = workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits,
        })
        .expect_err("transaction must fail");
    assert!(std::sync::Arc::ptr_eq(published, &workspace.current()));
    assert_eq!(workspace.current().revision(), published.revision());
    error
}

fn owner_sequence_draft() -> ExpressionDraft {
    let owner = DraftBindingId::new(0);
    ExpressionDraft::new(
        vec![
            DraftNode::I64(1),
            DraftNode::Operation {
                operation: crate::Operation::ByteVectorNew,
                arguments: vec![DraftNodeId::new(0)],
            },
            DraftNode::Move(DraftBindingRef::Local(owner)),
            DraftNode::I64(2),
            DraftNode::Operation {
                operation: crate::Operation::ByteVectorNew,
                arguments: vec![DraftNodeId::new(3)],
            },
            DraftNode::SetLocal {
                target: DraftBindingRef::Local(owner),
                value: DraftNodeId::new(4),
            },
            DraftNode::Move(DraftBindingRef::Local(owner)),
            DraftNode::I64(7),
            DraftNode::Sequence(vec![
                DraftNodeId::new(2),
                DraftNodeId::new(5),
                DraftNodeId::new(6),
                DraftNodeId::new(7),
            ]),
            DraftNode::MutableLocal {
                binding: owner,
                name: "owner".to_owned(),
                ty: SemanticType::ByteVector,
                initial: DraftNodeId::new(1),
                body: DraftNodeId::new(8),
            },
        ],
        DraftNodeId::new(9),
    )
}

fn complete_draft(
    seed: u64,
    return_type: SemanticType,
    draft: ExpressionDraft,
) -> (Workspace, std::sync::Arc<WorkspaceSnapshot>) {
    let mut workspace = Workspace::empty_deterministic(seed).expect("draft workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain { return_type }],
        })
        .expect("create draft main");
    let hole = created.snapshot.holes().next().expect("draft hole").id;
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole { hole, draft }],
        })
        .expect("fill draft main");
    (workspace, completed.snapshot)
}

#[test]
fn replacement_reorder_rebuilds_sequence_descendants_and_cannot_preserve_their_identities() {
    let (mut workspace, before) = complete_counter_workspace(300);
    assert_eq!(run_i64(&before), 2);
    let sequence = unique_node(&before, NodeKind::Sequence);
    let counter = entity_named(&before, EntityKind::MutableLocal, "counter");
    let old_descendants: Vec<_> = subtree_nodes(&before, sequence)
        .into_iter()
        .filter(|node| *node != sequence)
        .collect();

    let replaced = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: sequence,
                draft: reordered_sequence_draft(counter),
            }],
        })
        .expect("replace sequence to reorder children");

    assert_eq!(run_i64(&replaced.snapshot), 1);
    assert_eq!(
        replaced.snapshot.node(sequence).expect("sequence root").id,
        sequence
    );
    assert!(old_descendants
        .iter()
        .all(|node| replaced.snapshot.node(*node).is_err()));
    assert_eq!(
        replaced.snapshot.entity(counter).expect("counter").id,
        counter
    );
}

#[test]
fn same_sequence_move_changes_order_and_runtime_without_identity_churn() {
    let (mut workspace, before) = complete_counter_workspace(301);
    let sequence = unique_node(&before, NodeKind::Sequence);
    let old_children = before
        .indexes
        .node_children
        .get(&sequence)
        .expect("sequence children")
        .clone();
    assert_eq!(old_children.len(), 3);
    let old_nodes: std::collections::BTreeSet<_> =
        before.nodes().iter().map(|node| node.id).collect();
    let old_entities: std::collections::BTreeSet<_> =
        before.entities().iter().map(|entity| entity.id).collect();

    let moved = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::MoveSequenceChild {
                sequence,
                child: old_children[1],
                before: Some(old_children[0]),
            }],
        })
        .expect("move second assignment before first");

    assert_eq!(run_i64(&before), 2);
    assert_eq!(run_i64(&moved.snapshot), 1);
    moved
        .snapshot
        .check_consistency()
        .expect("moved snapshot consistency");
    let old_effects = before
        .node_semantics(before.revision(), sequence)
        .expect("old sequence effects")
        .effects;
    let new_effects = moved
        .snapshot
        .node_semantics(moved.snapshot.revision(), sequence)
        .expect("new sequence effects")
        .effects;
    assert_eq!(old_effects, new_effects);
    assert!(new_effects.contains(EffectSummary::MUTATES_LOCAL));
    for shifted in [old_children[0], old_children[1]] {
        let old_index = before.indexes.node_lookup[&shifted];
        let new_index = moved.snapshot.indexes.node_lookup[&shifted];
        assert_ne!(
            before.indexes.node_addresses[old_index],
            moved.snapshot.indexes.node_addresses[new_index]
        );
    }
    assert_eq!(
        moved
            .snapshot
            .indexes
            .node_children
            .get(&sequence)
            .expect("moved sequence children"),
        &[old_children[1], old_children[0], old_children[2]]
    );
    assert_eq!(
        moved
            .snapshot
            .nodes()
            .iter()
            .map(|node| node.id)
            .collect::<std::collections::BTreeSet<_>>(),
        old_nodes
    );
    assert_eq!(
        moved
            .snapshot
            .entities()
            .iter()
            .map(|entity| entity.id)
            .collect::<std::collections::BTreeSet<_>>(),
        old_entities
    );
    let main = entity_named(&before, EntityKind::Main, "main");
    assert_ne!(
        before
            .project(&[ProjectionSlice::Body(main)])
            .expect("old projection"),
        moved
            .snapshot
            .project(&[ProjectionSlice::Body(main)])
            .expect("moved projection")
    );
    assert_eq!(
        moved.diff.entries,
        vec![SemanticDiffEntry::SequenceChildMoved {
            sequence,
            child: old_children[1],
            old_ordinal: 1,
            new_ordinal: 0,
        }]
    );
}

fn scalar_sequence(
    seed: u64,
    values: &[i64],
) -> (Workspace, std::sync::Arc<WorkspaceSnapshot>, NodeId) {
    let kinds = values.iter().copied().map(DraftNode::I64).collect();
    let children = (0..values.len())
        .map(|index| DraftNodeId::new(u64::try_from(index).expect("scalar child")))
        .collect();
    complete_i64_sequence(seed, kinds, children)
}

fn assert_scalar_move(
    seed: u64,
    child_index: usize,
    anchor_index: Option<usize>,
    expected_indices: &[usize],
    expected_result: i64,
) {
    let (mut workspace, before, sequence) = scalar_sequence(seed, &[1, 2, 3, 4]);
    let children = direct_children(&before, sequence);
    let moved = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::MoveSequenceChild {
                sequence,
                child: children[child_index],
                before: anchor_index.map(|index| children[index]),
            }],
        })
        .expect("move scalar sequence child");
    assert_eq!(
        direct_children(&moved.snapshot, sequence),
        expected_indices
            .iter()
            .map(|index| children[*index])
            .collect::<Vec<_>>()
    );
    assert_eq!(run_i64(&moved.snapshot), expected_result);
    assert_eq!(
        moved.diff.entries,
        vec![SemanticDiffEntry::SequenceChildMoved {
            sequence,
            child: children[child_index],
            old_ordinal: u64::try_from(child_index).expect("old ordinal"),
            new_ordinal: u64::try_from(
                expected_indices
                    .iter()
                    .position(|index| *index == child_index)
                    .expect("new ordinal"),
            )
            .expect("new ordinal u64"),
        }]
    );
}

#[test]
fn sequence_anchor_semantics_cover_both_directions_append_and_middle_steps() {
    assert_scalar_move(302, 0, Some(3), &[1, 2, 0, 3], 4);
    assert_scalar_move(303, 2, Some(0), &[2, 0, 1, 3], 4);
    assert_scalar_move(304, 0, None, &[1, 2, 3, 0], 1);
    assert_scalar_move(305, 3, Some(0), &[3, 0, 1, 2], 3);
    assert_scalar_move(306, 2, Some(1), &[0, 2, 1, 3], 4);
    assert_scalar_move(307, 1, Some(3), &[0, 2, 1, 3], 4);
}

#[test]
fn moved_subtree_preserves_contained_lexical_entities_and_descendants() {
    let local = DraftBindingId::new(0);
    let draft = ExpressionDraft::new(
        vec![
            DraftNode::I64(1),
            DraftNode::Load(DraftBindingRef::Local(local)),
            DraftNode::Let {
                bindings: vec![LocalDraft {
                    binding: local,
                    name: "nested".to_owned(),
                    value: DraftNodeId::new(0),
                }],
                body: DraftNodeId::new(1),
            },
            DraftNode::I64(2),
            DraftNode::I64(3),
            DraftNode::Sequence(vec![
                DraftNodeId::new(2),
                DraftNodeId::new(3),
                DraftNodeId::new(4),
            ]),
        ],
        DraftNodeId::new(5),
    );
    let (mut workspace, before) = complete_draft(331, SemanticType::I64, draft);
    let sequence = unique_node(&before, NodeKind::Sequence);
    let children = direct_children(&before, sequence);
    let local = entity_named(&before, EntityKind::ImmutableLocal, "nested");
    let subtree = subtree_nodes(&before, children[0]);
    let moved = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::MoveSequenceChild {
                sequence,
                child: children[0],
                before: Some(children[2]),
            }],
        })
        .expect("move local-owning subtree");
    assert_eq!(run_i64(&moved.snapshot), 3);
    assert_eq!(
        moved.snapshot.entity(local).expect("nested local").id,
        local
    );
    assert!(subtree
        .iter()
        .all(|node| moved.snapshot.node(*node).is_ok()));
    assert_eq!(
        direct_children(&moved.snapshot, sequence),
        vec![children[1], children[0], children[2]]
    );
}

#[test]
fn invalid_identity_parent_anchor_and_no_op_requests_are_atomic() {
    let (mut workspace, published, sequence) = scalar_sequence(308, &[1, 2, 3]);
    let children = direct_children(&published, sequence);
    let wrong_kind = children[0];

    assert!(matches!(
        assert_atomic_error(
            &mut workspace,
            &published,
            vec![Edit::MoveSequenceChild {
                sequence: wrong_kind,
                child: children[1],
                before: None,
            }],
        ),
        WorkspaceError::WrongEntityKind { .. }
    ));
    for edit in [
        Edit::MoveSequenceChild {
            sequence,
            child: children[0],
            before: Some(children[0]),
        },
        Edit::MoveSequenceChild {
            sequence,
            child: children[2],
            before: None,
        },
        Edit::MoveSequenceChild {
            sequence,
            child: children[0],
            before: Some(children[1]),
        },
    ] {
        assert!(matches!(
            assert_atomic_error(&mut workspace, &published, vec![edit]),
            WorkspaceError::InvalidTransaction(_)
        ));
    }

    let (foreign_workspace, foreign, foreign_sequence) = scalar_sequence(309, &[4, 5]);
    drop(foreign_workspace);
    let foreign_children = direct_children(&foreign, foreign_sequence);
    for edit in [
        Edit::MoveSequenceChild {
            sequence: foreign_sequence,
            child: children[0],
            before: None,
        },
        Edit::MoveSequenceChild {
            sequence,
            child: foreign_children[0],
            before: None,
        },
        Edit::MoveSequenceChild {
            sequence,
            child: children[0],
            before: Some(foreign_children[0]),
        },
    ] {
        assert!(matches!(
            assert_atomic_error(&mut workspace, &published, vec![edit]),
            WorkspaceError::ForeignNamespace(_)
        ));
    }
}

#[test]
fn nonchild_cross_sequence_duplicate_and_structural_overlap_requests_are_atomic() {
    let local = DraftBindingId::new(0);
    let draft = ExpressionDraft::new(
        vec![
            DraftNode::I64(0),
            DraftNode::I64(1),
            DraftNode::I64(2),
            DraftNode::Sequence(vec![DraftNodeId::new(1), DraftNodeId::new(2)]),
            DraftNode::I64(3),
            DraftNode::I64(4),
            DraftNode::Sequence(vec![DraftNodeId::new(4), DraftNodeId::new(5)]),
            DraftNode::Sequence(vec![DraftNodeId::new(3), DraftNodeId::new(6)]),
            DraftNode::MutableLocal {
                binding: local,
                name: "unused".to_owned(),
                ty: SemanticType::I64,
                initial: DraftNodeId::new(0),
                body: DraftNodeId::new(7),
            },
        ],
        DraftNodeId::new(8),
    );
    let (mut workspace, published) = complete_draft(310, SemanticType::I64, draft);
    let sequences: Vec<_> = published
        .nodes()
        .iter()
        .filter(|node| node.kind == NodeKind::Sequence)
        .map(|node| node.id)
        .collect();
    assert_eq!(sequences.len(), 3);
    let outer = sequences
        .iter()
        .copied()
        .find(|node| {
            direct_children(&published, *node).len() == 2
                && direct_children(&published, *node).iter().all(|child| {
                    published
                        .node(*child)
                        .is_ok_and(|header| header.kind == NodeKind::Sequence)
                })
        })
        .expect("outer sequence");
    let inner: Vec<_> = direct_children(&published, outer);
    let first_children = direct_children(&published, inner[0]);
    let second_children = direct_children(&published, inner[1]);
    let descendant = first_children[0];

    for edits in [
        vec![Edit::MoveSequenceChild {
            sequence: outer,
            child: descendant,
            before: Some(inner[1]),
        }],
        vec![Edit::MoveSequenceChild {
            sequence: inner[0],
            child: second_children[0],
            before: None,
        }],
        vec![Edit::MoveSequenceChild {
            sequence: inner[0],
            child: first_children[0],
            before: Some(second_children[0]),
        }],
        vec![
            Edit::MoveSequenceChild {
                sequence: inner[0],
                child: first_children[1],
                before: Some(first_children[0]),
            },
            Edit::MoveSequenceChild {
                sequence: inner[1],
                child: second_children[1],
                before: Some(second_children[0]),
            },
        ],
        vec![
            Edit::MoveSequenceChild {
                sequence: inner[0],
                child: first_children[1],
                before: Some(first_children[0]),
            },
            Edit::ReplaceExpression {
                target: first_children[0],
                draft: ExpressionDraft::scalar_i64(9),
            },
        ],
        vec![
            Edit::MoveSequenceChild {
                sequence: inner[0],
                child: first_children[1],
                before: Some(first_children[0]),
            },
            Edit::IntroduceHole {
                target: first_children[0],
                goal: "conflicting hole".to_owned(),
            },
        ],
        vec![
            Edit::MoveSequenceChild {
                sequence: inner[0],
                child: first_children[1],
                before: Some(first_children[0]),
            },
            Edit::IntroduceUnresolvedValueReference {
                target: first_children[0],
                requested_name: "conflicting".to_owned(),
            },
        ],
    ] {
        assert!(matches!(
            assert_atomic_error(&mut workspace, &published, edits),
            WorkspaceError::InvalidTransaction(_)
        ));
    }
}

#[test]
fn final_child_type_changes_when_the_containing_context_accepts_it() {
    let draft = ExpressionDraft::new(
        vec![
            DraftNode::Unit,
            DraftNode::I64(7),
            DraftNode::Sequence(vec![DraftNodeId::new(0), DraftNodeId::new(1)]),
            DraftNode::I64(9),
            DraftNode::Sequence(vec![DraftNodeId::new(2), DraftNodeId::new(3)]),
        ],
        DraftNodeId::new(4),
    );
    let (mut workspace, before) = complete_draft(330, SemanticType::I64, draft);
    let inner = before
        .nodes()
        .iter()
        .find(|node| {
            node.kind == NodeKind::Sequence
                && direct_children(&before, node.id).iter().all(|child| {
                    before
                        .node(*child)
                        .is_ok_and(|item| item.kind == NodeKind::Literal)
                })
        })
        .expect("inner sequence")
        .id;
    let outer = before
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Sequence && node.id != inner)
        .expect("outer sequence")
        .id;
    let children = direct_children(&before, inner);
    let moved = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::MoveSequenceChild {
                sequence: inner,
                child: children[1],
                before: Some(children[0]),
            }],
        })
        .expect("move scalar before unit in nonfinal sequence");
    assert_eq!(run_i64(&moved.snapshot), 9);
    assert_eq!(
        moved
            .snapshot
            .node_semantics(moved.snapshot.revision(), inner)
            .expect("inner type")
            .actual,
        SemanticType::Unit
    );
    assert_eq!(
        moved
            .snapshot
            .node_semantics(moved.snapshot.revision(), outer)
            .expect("outer type")
            .actual,
        SemanticType::I64
    );
}

#[test]
fn type_change_propagates_through_conditional_branch_before_fixed_context_validation() {
    let draft = ExpressionDraft::new(
        vec![
            DraftNode::Unit,
            DraftNode::I64(1),
            DraftNode::Sequence(vec![DraftNodeId::new(0), DraftNodeId::new(1)]),
            DraftNode::I64(9),
            DraftNode::Return {
                value: DraftNodeId::new(3),
            },
            DraftNode::Bool(true),
            DraftNode::If {
                condition: DraftNodeId::new(5),
                then_branch: DraftNodeId::new(2),
                else_branch: DraftNodeId::new(4),
            },
            DraftNode::I64(7),
            DraftNode::Sequence(vec![DraftNodeId::new(6), DraftNodeId::new(7)]),
        ],
        DraftNodeId::new(8),
    );
    let (mut workspace, before) = complete_draft(332, SemanticType::I64, draft);
    let inner = before
        .nodes()
        .iter()
        .find(|node| {
            node.kind == NodeKind::Sequence
                && direct_children(&before, node.id).iter().all(|child| {
                    before
                        .node(*child)
                        .is_ok_and(|item| item.kind == NodeKind::Literal)
                })
        })
        .expect("conditional branch sequence")
        .id;
    let conditional = unique_node(&before, NodeKind::Conditional);
    let children = direct_children(&before, inner);
    let moved = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::MoveSequenceChild {
                sequence: inner,
                child: children[1],
                before: Some(children[0]),
            }],
        })
        .expect("propagate movement type through conditional");
    assert_eq!(run_i64(&moved.snapshot), 7);
    assert_eq!(
        moved
            .snapshot
            .node_semantics(moved.snapshot.revision(), inner)
            .expect("inner sequence type")
            .actual,
        SemanticType::Unit
    );
    assert_eq!(
        moved
            .snapshot
            .node_semantics(moved.snapshot.revision(), conditional)
            .expect("conditional type")
            .actual,
        SemanticType::Unit
    );
}

#[test]
fn final_child_type_and_divergence_are_recomputed_before_publication() {
    let (mut type_workspace, typed, sequence) = complete_i64_sequence(
        311,
        vec![DraftNode::Unit, DraftNode::I64(7)],
        vec![DraftNodeId::new(0), DraftNodeId::new(1)],
    );
    let typed_children = direct_children(&typed, sequence);
    assert!(matches!(
        assert_atomic_error(
            &mut type_workspace,
            &typed,
            vec![Edit::MoveSequenceChild {
                sequence,
                child: typed_children[1],
                before: Some(typed_children[0]),
            }],
        ),
        WorkspaceError::TypeMismatch { .. }
    ));

    let condition_draft = ExpressionDraft::new(
        vec![
            DraftNode::I64(1),
            DraftNode::Bool(true),
            DraftNode::Sequence(vec![DraftNodeId::new(0), DraftNodeId::new(1)]),
            DraftNode::I64(3),
            DraftNode::I64(4),
            DraftNode::If {
                condition: DraftNodeId::new(2),
                then_branch: DraftNodeId::new(3),
                else_branch: DraftNodeId::new(4),
            },
        ],
        DraftNodeId::new(5),
    );
    let (mut condition_workspace, condition) =
        complete_draft(333, SemanticType::I64, condition_draft);
    let sequence = unique_node(&condition, NodeKind::Sequence);
    let condition_children = direct_children(&condition, sequence);
    assert!(matches!(
        assert_atomic_error(
            &mut condition_workspace,
            &condition,
            vec![Edit::MoveSequenceChild {
                sequence,
                child: condition_children[1],
                before: Some(condition_children[0]),
            }],
        ),
        WorkspaceError::TypeMismatch { .. }
    ));

    let divergent_draft = ExpressionDraft::new(
        vec![
            DraftNode::Unit,
            DraftNode::I64(7),
            DraftNode::Return {
                value: DraftNodeId::new(1),
            },
            DraftNode::Sequence(vec![DraftNodeId::new(0), DraftNodeId::new(2)]),
        ],
        DraftNodeId::new(3),
    );
    let (mut control_workspace, control) = complete_draft(312, SemanticType::I64, divergent_draft);
    assert_eq!(run_i64(&control), 7);
    let sequence = unique_node(&control, NodeKind::Sequence);
    let control_children = direct_children(&control, sequence);
    assert!(matches!(
        assert_atomic_error(
            &mut control_workspace,
            &control,
            vec![Edit::MoveSequenceChild {
                sequence,
                child: control_children[1],
                before: Some(control_children[0]),
            }],
        ),
        WorkspaceError::Validation(message)
            if message.as_ref()
                == "sequence movement leaves an expression after a divergent expression"
    ));
}

#[test]
fn canonical_ownership_rejects_live_overwrite_caused_by_movement_and_preserves_cleanup_state() {
    let (mut workspace, published) = complete_draft(313, SemanticType::I64, owner_sequence_draft());
    assert_eq!(run_i64(&published), 7);
    let sequence = unique_node(&published, NodeKind::Sequence);
    let children = direct_children(&published, sequence);
    assert_eq!(
        published.node(children[0]).expect("move").kind,
        NodeKind::Move
    );
    assert_eq!(
        published.node(children[1]).expect("set").kind,
        NodeKind::SetLocal
    );

    assert!(matches!(
        assert_atomic_error(
            &mut workspace,
            &published,
            vec![Edit::MoveSequenceChild {
                sequence,
                child: children[0],
                before: Some(children[2]),
            }],
        ),
        WorkspaceError::Validation(_)
    ));
    let executable =
        crate::compile_snapshot(&published).expect("compile preserved ownership state");
    let outcome = run_chunk(
        executable.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert!(outcome.cleanup_failures().is_none());
    assert!(matches!(
        outcome,
        ExecutionOutcome::Returned(value) if value.as_i64() == Some(7)
    ));
}

#[test]
fn pure_move_consumes_no_node_or_entity_identity_slots() {
    let (mut moved_workspace, before) = complete_counter_workspace(314);
    let mut control_workspace =
        Workspace::new(before.as_ref().clone()).expect("reopen control snapshot");
    let sequence = unique_node(&before, NodeKind::Sequence);
    let children = direct_children(&before, sequence);
    let moved = moved_workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::MoveSequenceChild {
                sequence,
                child: children[1],
                before: Some(children[0]),
            }],
        })
        .expect("move before later creation");

    let create = |workspace: &mut Workspace| {
        workspace
            .apply(Transaction {
                base_revision: workspace.current().revision(),
                edits: vec![Edit::CreateFunction {
                    name: "later".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: DeclarationType::I64,
                }],
            })
            .expect("create later function")
            .snapshot
    };
    let moved_created = create(&mut moved_workspace);
    let control_created = create(&mut control_workspace);
    let moved_function = entity_named(&moved_created, EntityKind::Function, "later");
    let control_function = entity_named(&control_created, EntityKind::Function, "later");
    assert_eq!(moved_function, control_function);
    let moved_hole = moved_created
        .holes()
        .find(|hole| hole.owner == moved_function)
        .expect("moved branch hole")
        .id;
    let control_hole = control_created
        .holes()
        .find(|hole| hole.owner == control_function)
        .expect("control branch hole")
        .id;
    assert_eq!(moved_hole, control_hole);
    assert_eq!(run_i64(&moved.snapshot), 1);
}

#[test]
fn mixed_rename_and_movement_diff_is_stably_sorted() {
    let (mut workspace, before) = complete_counter_workspace(315);
    let sequence = unique_node(&before, NodeKind::Sequence);
    let children = direct_children(&before, sequence);
    let counter = entity_named(&before, EntityKind::MutableLocal, "counter");
    let outcome = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![
                Edit::MoveSequenceChild {
                    sequence,
                    child: children[1],
                    before: Some(children[0]),
                },
                Edit::RenameEntity {
                    entity: counter,
                    new_name: "renamed-counter".to_owned(),
                },
            ],
        })
        .expect("rename and move atomically");
    assert_eq!(run_i64(&outcome.snapshot), 1);
    assert!(matches!(
        outcome.diff.entries.as_slice(),
        [
            SemanticDiffEntry::EntityRenamed { entity, .. },
            SemanticDiffEntry::SequenceChildMoved {
                sequence: diff_sequence,
                child,
                old_ordinal: 1,
                new_ordinal: 0,
            },
        ] if *entity == counter && *diff_sequence == sequence && *child == children[1]
    ));
}

#[test]
fn movement_composes_with_unrelated_callable_deletion_and_private_root_relocation() {
    let mut workspace = Workspace::empty_deterministic(329).expect("relocation workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "remove".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: DeclarationType::I64,
                },
                Edit::CreateFunction {
                    name: "keep".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: DeclarationType::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create relocation declarations");
    let removed = entity_named(&created.snapshot, EntityKind::Function, "remove");
    let kept = entity_named(&created.snapshot, EntityKind::Function, "keep");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let hole_for = |owner| {
        created
            .snapshot
            .holes()
            .find(|hole| hole.owner == owner)
            .expect("owned hole")
            .id
    };
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![
                Edit::FillHole {
                    hole: hole_for(removed),
                    draft: ExpressionDraft::scalar_i64(9),
                },
                Edit::FillHole {
                    hole: hole_for(kept),
                    draft: ExpressionDraft::new(
                        vec![
                            DraftNode::I64(1),
                            DraftNode::I64(2),
                            DraftNode::Sequence(vec![DraftNodeId::new(0), DraftNodeId::new(1)]),
                        ],
                        DraftNodeId::new(2),
                    ),
                },
                Edit::FillHole {
                    hole: hole_for(main),
                    draft: ExpressionDraft::new(
                        vec![DraftNode::Call {
                            callee: kept,
                            type_arguments: Vec::new(),
                            arguments: Vec::new(),
                        }],
                        DraftNodeId::new(0),
                    ),
                },
            ],
        })
        .expect("complete relocation program");
    assert_eq!(run_i64(&completed.snapshot), 2);
    let sequence = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| {
            node.kind == NodeKind::Sequence
                && completed.snapshot.indexes.node_enclosing_entities
                    [completed.snapshot.indexes.node_lookup[&node.id]]
                    == kept
        })
        .expect("kept function sequence")
        .id;
    let children = direct_children(&completed.snapshot, sequence);
    let kept_nodes: std::collections::BTreeSet<_> = subtree_nodes(&completed.snapshot, sequence)
        .into_iter()
        .collect();
    let outcome = workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![
                Edit::DeleteEntity { entity: removed },
                Edit::MoveSequenceChild {
                    sequence,
                    child: children[0],
                    before: None,
                },
            ],
        })
        .expect("delete earlier callable and move retained sequence");
    assert_eq!(run_i64(&outcome.snapshot), 1);
    assert_eq!(
        outcome.snapshot.entity(kept).expect("kept function").id,
        kept
    );
    assert!(outcome.snapshot.entity(removed).is_err());
    assert_eq!(
        subtree_nodes(&outcome.snapshot, sequence)
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        kept_nodes
    );
    assert!(outcome.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::SequenceChildMoved { sequence: moved, .. } if *moved == sequence
    )));
}

#[test]
fn moved_hole_and_unresolved_reference_keep_identity_and_block_execution() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let (mut hole_workspace, complete) = complete_counter_workspace(316);
    let sequence = unique_node(&complete, NodeKind::Sequence);
    let complete_children = direct_children(&complete, sequence);
    let introduced = hole_workspace
        .apply(Transaction {
            base_revision: complete.revision(),
            edits: vec![Edit::IntroduceHole {
                target: complete_children[0],
                goal: "retain one assignment action".to_owned(),
            }],
        })
        .expect("introduce moved hole");
    let hole_before = introduced
        .snapshot
        .holes()
        .next()
        .expect("typed hole")
        .clone();
    let moved = hole_workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::MoveSequenceChild {
                sequence,
                child: hole_before.id.node(),
                before: Some(complete_children[2]),
            }],
        })
        .expect("move typed hole");
    let hole_after = moved.snapshot.holes().next().expect("moved typed hole");
    assert_eq!(hole_after.id, hole_before.id);
    assert_eq!(hole_after.expected_type, hole_before.expected_type);
    assert_eq!(hole_after.visible_entities, hole_before.visible_entities);
    assert_eq!(
        direct_children(&moved.snapshot, sequence),
        vec![
            complete_children[1],
            complete_children[0],
            complete_children[2]
        ]
    );
    assert!(matches!(
        crate::compile_snapshot(&moved.snapshot),
        Err(crate::CompileSnapshotError::Incomplete(_))
    ));
    assert_eq!(
        moved.diff.entries,
        vec![SemanticDiffEntry::SequenceChildMoved {
            sequence,
            child: complete_children[0],
            old_ordinal: 0,
            new_ordinal: 1,
        }]
    );

    let (mut unresolved_workspace, complete) = complete_counter_workspace(317);
    let sequence = unique_node(&complete, NodeKind::Sequence);
    let complete_children = direct_children(&complete, sequence);
    let introduced = unresolved_workspace
        .apply(Transaction {
            base_revision: complete.revision(),
            edits: vec![Edit::IntroduceUnresolvedValueReference {
                target: complete_children[0],
                requested_name: "future-action".to_owned(),
            }],
        })
        .expect("introduce moved unresolved reference");
    let unresolved_before = introduced
        .snapshot
        .unresolved_value_references()
        .next()
        .expect("unresolved reference")
        .clone();
    let moved = unresolved_workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::MoveSequenceChild {
                sequence,
                child: unresolved_before.id.node(),
                before: Some(complete_children[2]),
            }],
        })
        .expect("move unresolved reference");
    let unresolved_after = moved
        .snapshot
        .unresolved_value_references()
        .next()
        .expect("moved unresolved reference");
    assert_eq!(unresolved_after.id, unresolved_before.id);
    assert_eq!(
        unresolved_after.requested_name,
        unresolved_before.requested_name
    );
    assert_eq!(unresolved_after.intent, unresolved_before.intent);
    assert_eq!(
        unresolved_after.expected_type,
        unresolved_before.expected_type
    );
    assert_eq!(
        unresolved_after.visible_entities,
        unresolved_before.visible_entities
    );
    assert!(matches!(
        crate::compile_snapshot(&moved.snapshot),
        Err(crate::CompileSnapshotError::Incomplete(_))
    ));
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

const IMPORTED_COUNTER: &str = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nvar/\nname/\ncounter\n/name\ntype/\ni64\n/type\n0\ndo/\nset/\ncounter\n1\n/set\nset/\ncounter\n2\n/set\ncounter\n/do\n/var\n/main\n";
const IMPORTED_COUNTER_MOVED: &str = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nvar/\nname/\ncounter\n/name\ntype/\ni64\n/type\n0\ndo/\nset/\ncounter\n2\n/set\nset/\ncounter\n1\n/set\ncounter\n/do\n/var\n/main\n";

fn public_node_shape(
    snapshot: &WorkspaceSnapshot,
) -> Vec<(NodeKind, SemanticType, Option<SemanticType>, EffectSummary)> {
    snapshot
        .nodes()
        .iter()
        .map(|node| {
            let facts = snapshot
                .node_semantics(snapshot.revision(), node.id)
                .expect("node semantics");
            (node.kind, facts.actual, facts.expected, facts.effects)
        })
        .collect()
}

#[test]
fn incomplete_expected_context_refreshes_when_moved_to_final_position() {
    for (seed, unresolved) in [(326, false), (327, true)] {
        let (mut workspace, complete, sequence) = scalar_sequence(seed, &[1, 2, 3]);
        let children = direct_children(&complete, sequence);
        let introduced = workspace
            .apply(Transaction {
                base_revision: complete.revision(),
                edits: vec![if unresolved {
                    Edit::IntroduceUnresolvedValueReference {
                        target: children[0],
                        requested_name: "future-value".to_owned(),
                    }
                } else {
                    Edit::IntroduceHole {
                        target: children[0],
                        goal: "supply a scalar value".to_owned(),
                    }
                }],
            })
            .expect("introduce incomplete scalar");
        assert_eq!(
            introduced
                .snapshot
                .node_semantics(introduced.snapshot.revision(), children[0])
                .expect("nonfinal incomplete semantics")
                .expected,
            None
        );
        let moved = workspace
            .apply(Transaction {
                base_revision: introduced.snapshot.revision(),
                edits: vec![Edit::MoveSequenceChild {
                    sequence,
                    child: children[0],
                    before: None,
                }],
            })
            .expect("move incomplete scalar to final position");
        let facts = moved
            .snapshot
            .node_semantics(moved.snapshot.revision(), children[0])
            .expect("final incomplete semantics");
        assert_eq!(facts.actual, SemanticType::I64);
        assert_eq!(facts.expected, Some(SemanticType::I64));
        assert!(matches!(
            crate::compile_snapshot(&moved.snapshot),
            Err(crate::CompileSnapshotError::Incomplete(_))
        ));
    }
}

#[test]
fn one_child_sequence_reorder_is_a_rejected_no_op() {
    let (mut workspace, published, sequence) = scalar_sequence(328, &[1]);
    let child = direct_children(&published, sequence)[0];
    assert!(matches!(
        assert_atomic_error(
            &mut workspace,
            &published,
            vec![Edit::MoveSequenceChild {
                sequence,
                child,
                before: None,
            }],
        ),
        WorkspaceError::InvalidTransaction(message)
            if message.as_ref() == "sequence movement would not change semantic order"
    ));
}

#[test]
fn imported_movement_does_not_reparse_and_converges_with_independent_final_order() {
    let imported = importer::import_source_with_namespace(
        IMPORTED_COUNTER,
        "movement-counter.lkjscript",
        WorkspaceNamespace::deterministic(318),
    )
    .expect("import counter program");
    let final_imported = importer::import_source_with_namespace(
        IMPORTED_COUNTER_MOVED,
        "movement-counter-final.lkjscript",
        WorkspaceNamespace::deterministic(319),
    )
    .expect("import independently moved counter program");
    assert!(imported.attachments().is_some());
    assert!(final_imported.attachments().is_some());

    let mut workspace = Workspace::new(imported).expect("open imported workspace");
    let before = workspace.current();
    let sequence = unique_node(&before, NodeKind::Sequence);
    let children = direct_children(&before, sequence);
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let moved = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::MoveSequenceChild {
                sequence,
                child: children[1],
                before: Some(children[0]),
            }],
        })
        .expect("move imported sequence child");
    let main = entity_named(&moved.snapshot, EntityKind::Main, "main");
    moved
        .snapshot
        .project(&[ProjectionSlice::Body(main)])
        .expect("project imported movement");
    let moved_executable = crate::compile_snapshot(&moved.snapshot).expect("compile moved import");
    let final_executable = crate::compile_snapshot(&final_imported).expect("compile final import");
    let (mut source_free_workspace, source_free_before) = complete_counter_workspace(322);
    let source_free_sequence = unique_node(&source_free_before, NodeKind::Sequence);
    let source_free_children = direct_children(&source_free_before, source_free_sequence);
    let source_free_moved = source_free_workspace
        .apply(Transaction {
            base_revision: source_free_before.revision(),
            edits: vec![Edit::MoveSequenceChild {
                sequence: source_free_sequence,
                child: source_free_children[1],
                before: Some(source_free_children[0]),
            }],
        })
        .expect("move source-free sequence child");
    let source_free_executable =
        crate::compile_snapshot(&source_free_moved.snapshot).expect("compile source-free movement");
    assert_eq!(run_i64(&before), 2);
    assert_eq!(run_i64(&moved.snapshot), 1);
    assert_eq!(run_i64(&final_imported), 1);
    assert_eq!(run_i64(&source_free_moved.snapshot), 1);
    assert!(moved.snapshot.attachments().is_none());
    assert!(source_free_moved.snapshot.attachments().is_none());
    assert_eq!(
        public_node_shape(&moved.snapshot),
        public_node_shape(&final_imported)
    );
    assert_eq!(
        public_node_shape(&source_free_moved.snapshot),
        public_node_shape(&final_imported)
    );
    assert_eq!(
        moved_executable.bytecode().main().code,
        final_executable.bytecode().main().code
    );
    assert_eq!(
        source_free_executable.bytecode().main().code,
        final_executable.bytecode().main().code
    );
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn projection_diff_containment_and_continuations_are_deterministic_and_revision_bound() {
    let run = |seed| {
        let (mut workspace, before) = complete_counter_workspace(seed);
        let main = entity_named(&before, EntityKind::Main, "main");
        let sequence = unique_node(&before, NodeKind::Sequence);
        let children = direct_children(&before, sequence);
        let request = PageRequest::new(1).expect("page request");
        let first_page = before
            .entity_page(before.revision(), request, None)
            .expect("base entity page");
        let continuation = first_page.continuation.expect("base continuation");
        let moved = workspace
            .apply(Transaction {
                base_revision: before.revision(),
                edits: vec![Edit::MoveSequenceChild {
                    sequence,
                    child: children[1],
                    before: Some(children[0]),
                }],
            })
            .expect("deterministic movement");
        assert!(moved
            .snapshot
            .entity_page(moved.snapshot.revision(), request, Some(&continuation))
            .is_err());
        before
            .entity_page(before.revision(), request, Some(&continuation))
            .expect("old continuation remains valid for old snapshot");
        (
            moved.diff,
            moved
                .snapshot
                .project(&[ProjectionSlice::Body(main)])
                .expect("moved projection"),
            direct_children(&moved.snapshot, sequence),
        )
    };
    assert_eq!(run(320), run(320));
}

#[test]
fn stale_nodes_deleted_owner_and_stale_revision_reject_without_publication() {
    let (mut workspace, before) = complete_counter_workspace(321);
    let main = entity_named(&before, EntityKind::Main, "main");
    let sequence = unique_node(&before, NodeKind::Sequence);
    let children = direct_children(&before, sequence);
    assert!(matches!(
        assert_atomic_error(
            &mut workspace,
            &before,
            vec![
                Edit::DeleteEntity { entity: main },
                Edit::MoveSequenceChild {
                    sequence,
                    child: children[0],
                    before: Some(children[1]),
                },
            ],
        ),
        WorkspaceError::InvalidTransaction(_)
    ));

    let mut deleted_workspace =
        Workspace::new(before.as_ref().clone()).expect("reopen deletion branch");
    let deleted = deleted_workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::DeleteEntity { entity: main }],
        })
        .expect("delete movement owner");
    assert!(matches!(
        assert_atomic_error(
            &mut deleted_workspace,
            &deleted.snapshot,
            vec![Edit::MoveSequenceChild {
                sequence,
                child: children[0],
                before: None,
            }],
        ),
        WorkspaceError::StaleIdentity(_)
    ));

    let replaced = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: sequence,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(4),
                        DraftNode::I64(5),
                        DraftNode::Sequence(vec![DraftNodeId::new(0), DraftNodeId::new(1)]),
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect("replace sequence descendants");
    assert!(matches!(
        assert_atomic_error(
            &mut workspace,
            &replaced.snapshot,
            vec![Edit::MoveSequenceChild {
                sequence,
                child: children[0],
                before: None,
            }],
        ),
        WorkspaceError::StaleIdentity(_)
    ));
    let replacement_children = direct_children(&replaced.snapshot, sequence);
    assert!(matches!(
        assert_atomic_error(
            &mut workspace,
            &replaced.snapshot,
            vec![Edit::MoveSequenceChild {
                sequence,
                child: replacement_children[0],
                before: Some(children[1]),
            }],
        ),
        WorkspaceError::StaleIdentity(_)
    ));
    let current = workspace.current();
    assert!(matches!(
        workspace.apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::MoveSequenceChild {
                sequence,
                child: direct_children(&current, sequence)[0],
                before: None,
            }],
        }),
        Err(WorkspaceError::StaleRevision)
    ));
    assert!(std::sync::Arc::ptr_eq(&current, &workspace.current()));
}

fn deep_movable_sequence_draft(depth: usize) -> ExpressionDraft {
    let mut nodes = Vec::new();
    let mut deep = push_node(&mut nodes, DraftNode::I64(1));
    for _ in 0..(depth / 2) {
        deep = push_node(&mut nodes, DraftNode::Sequence(vec![deep]));
    }
    let two = push_node(&mut nodes, DraftNode::I64(2));
    let mut root = push_node(&mut nodes, DraftNode::Sequence(vec![deep, two]));
    for _ in (depth / 2)..depth {
        root = push_node(&mut nodes, DraftNode::Sequence(vec![root]));
    }
    ExpressionDraft::new(nodes, root)
}

fn run_deep_movement(depth: usize, seed: u64) {
    let (mut workspace, before) =
        complete_draft(seed, SemanticType::I64, deep_movable_sequence_draft(depth));
    let sequence = before
        .nodes()
        .iter()
        .find(|node| {
            node.kind == NodeKind::Sequence && direct_children(&before, node.id).len() == 2
        })
        .expect("movable sequence")
        .id;
    let children = direct_children(&before, sequence);
    let old_nodes: std::collections::BTreeSet<_> =
        before.nodes().iter().map(|node| node.id).collect();
    let moved = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::MoveSequenceChild {
                sequence,
                child: children[0],
                before: None,
            }],
        })
        .expect("move deep subtree");
    assert_eq!(run_i64(&moved.snapshot), 1);
    assert_eq!(
        moved
            .snapshot
            .nodes()
            .iter()
            .map(|node| node.id)
            .collect::<std::collections::BTreeSet<_>>(),
        old_nodes
    );
}

#[test]
fn deep_moved_subtree_and_wide_sibling_permutation_are_stack_safe_and_identity_stable() {
    std::thread::Builder::new()
        .name("workspace-sequence-movement-small-stack".to_owned())
        .stack_size(128 * 1024)
        .spawn(|| {
            run_deep_movement(256, 323);
            let width = 2_048_usize;
            let values: Vec<_> = (1..=width)
                .map(|value| i64::try_from(value).expect("wide value"))
                .collect();
            let (mut workspace, before, sequence) = scalar_sequence(324, &values);
            let children = direct_children(&before, sequence);
            let old_nodes: std::collections::BTreeSet<_> =
                before.nodes().iter().map(|node| node.id).collect();
            let moved = workspace
                .apply(Transaction {
                    base_revision: before.revision(),
                    edits: vec![Edit::MoveSequenceChild {
                        sequence,
                        child: children[0],
                        before: None,
                    }],
                })
                .expect("move first wide child to end");
            let measurement = super::transaction::take_transaction_measurement();
            assert_eq!(measurement.program_clones, 1);
            assert_eq!(measurement.compaction_invocations, 1);
            assert_eq!(measurement.effect_inference_invocations, 1);
            assert_eq!(measurement.index_build_invocations, 1);
            assert_eq!(measurement.identity_reconciliation_invocations, 1);
            assert_eq!(measurement.index_nodes_built, moved.snapshot.nodes().len());
            assert_eq!(run_i64(&moved.snapshot), 1);
            assert_eq!(
                moved
                    .snapshot
                    .nodes()
                    .iter()
                    .map(|node| node.id)
                    .collect::<std::collections::BTreeSet<_>>(),
                old_nodes
            );
        })
        .expect("spawn movement small-stack worker")
        .join()
        .expect("movement small-stack worker completes");
}

#[test]
#[ignore = "20k-level locked-release same-sequence movement stress geometry"]
fn twenty_thousand_level_moved_subtree_is_stack_safe() {
    std::thread::Builder::new()
        .name("workspace-deep-sequence-movement".to_owned())
        .stack_size(128 * 1024)
        .spawn(|| run_deep_movement(20_000, 325))
        .expect("spawn deep movement worker")
        .join()
        .expect("deep movement worker completes");
}
