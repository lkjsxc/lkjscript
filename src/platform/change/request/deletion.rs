//! Explicit semantic-owner deletion and internal expression-retirement lowering.

use super::{AuthoredDeletePolicy, AuthoredLowerer, OwnerSelector, request_error};
use crate::platform::change::{CanonicalBaseRead, WitnessBaseRead};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
use crate::platform::kernel::{
    DeclarationPayload, ExactOwnerKey, ExpressionOperation, OwnerKey, OwnerRecord, ParameterParent,
    RelationEdge, RelationEndpoint, RelationKind, RetirementRecord, encode_owner,
    extract_owner_relations,
};
use crate::platform::witness::{
    MAXIMUM_RELATION_PREFIX_ITEMS, OwnershipParent, aggregation_children, ownership_contributions,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) fn lower_deletions<
    'request,
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    deletions: impl IntoIterator<Item = (&'request OwnerSelector, &'request AuthoredDeletePolicy)>,
) -> Result<(), Diagnostic> {
    let mut roots = BTreeMap::new();
    for (selector, policy) in deletions {
        let root = lowerer.resolve_owner(selector)?;
        if matches!(root, OwnerKey::Binding(_) | OwnerKey::Expression(_)) {
            return Err(delete_error(
                "change_delete_expression_parent",
                "bindings and expressions must be removed through an exact parent expression or root edit",
            ));
        }
        require_accepted_owner(lowerer, root)?;
        if roots.insert(root, *policy).is_some() {
            return Err(delete_error(
                "change_delete_duplicate",
                format!("owner {root:?} is selected for deletion more than once"),
            ));
        }
    }
    if roots.is_empty() {
        return Ok(());
    }

    let candidate_external_children = index_candidate_external_children(lowerer);
    let mut closure = BTreeSet::new();
    let mut closure_roots = Vec::new();
    for (root, policy) in &roots {
        match policy {
            AuthoredDeletePolicy::Reject => {
                let owned = current_owned_children(lowerer, *root, &candidate_external_children)?;
                if !owned.is_empty() {
                    return Err(delete_error(
                        "change_delete_owned_children",
                        format!(
                            "owner {root:?} owns {} identities; reject policy requires a leaf owner",
                            owned.len()
                        ),
                    ));
                }
                lowerer.admit_owner_edit(*root)?;
                lowerer.admit_retirement_edit(*root)?;
                closure.insert(*root);
                lowerer.check_budget("authored leaf deletion")?;
            }
            AuthoredDeletePolicy::OwnedClosure => closure_roots.push(*root),
        }
    }
    select_owned_closure(
        lowerer,
        closure_roots,
        &candidate_external_children,
        &mut closure,
        "authored deletion ownership closure",
    )?;

    for root in roots.keys() {
        detach_root_from_live_parent(lowerer, *root, &closure)?;
    }
    reject_untouched_incoming_references(lowerer, &closure)?;
    retain_retirements_and_mark_deleted(lowerer, &closure)?;
    Ok(())
}

/// Retires the accepted expression and binding closure detached by a parent edit. The caller
/// replaces the parent's root first, allowing incoming-reference validation to distinguish that
/// deliberate detachment from an untouched live reference.
pub(super) fn retire_replaced_expression_tree<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    root: crate::platform::semantic_id::ExpressionId,
) -> Result<(), Diagnostic> {
    let root = OwnerKey::Expression(root);
    require_accepted_owner(lowerer, root)?;
    if !matches!(lowerer.owners[&root].record, OwnerRecord::Expression(_)) {
        return Err(delete_corrupt(
            "change_replace_body_root_kind",
            "accepted function body identity is not bound to an expression owner",
        ));
    }

    let mut closure = BTreeSet::new();
    let candidate_external_children = index_candidate_external_children(lowerer);
    select_owned_closure(
        lowerer,
        [root],
        &candidate_external_children,
        &mut closure,
        "function-body replacement ownership closure",
    )?;

    reject_untouched_incoming_references(lowerer, &closure)?;
    retain_retirements_and_mark_deleted(lowerer, &closure)
}

fn select_owned_closure<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    roots: impl IntoIterator<Item = OwnerKey>,
    candidate_external_children: &BTreeMap<OwnerKey, BTreeSet<OwnerKey>>,
    closure: &mut BTreeSet<OwnerKey>,
    phase: &str,
) -> Result<(), Diagnostic> {
    let mut frontier = roots.into_iter().collect::<VecDeque<_>>();
    while let Some(owner) = frontier.pop_front() {
        if closure.contains(&owner) {
            continue;
        }
        lowerer.admit_owner_edit(owner)?;
        lowerer.admit_retirement_edit(owner)?;
        require_accepted_owner(lowerer, owner)?;
        closure.insert(owner);
        for child in current_owned_children(lowerer, owner, &candidate_external_children)? {
            if !closure.contains(&child) {
                frontier.push_back(child);
            }
        }
        lowerer.check_budget(phase)?;
    }
    Ok(())
}

fn require_accepted_owner<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
) -> Result<(), Diagnostic> {
    lowerer.require_owner(owner)?;
    let working = lowerer.owners.get(&owner).ok_or_else(|| {
        delete_corrupt(
            "change_delete_owner_cache",
            "selected owner is absent from the authored candidate overlay",
        )
    })?;
    if working.original.is_none() {
        return Err(delete_error(
            "change_delete_created_owner",
            format!(
                "request-local owner {owner:?} cannot be created and deleted in one accepted change"
            ),
        ));
    }
    Ok(())
}

fn current_owned_children<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
    candidate_external_children: &BTreeMap<OwnerKey, BTreeSet<OwnerKey>>,
) -> Result<Vec<OwnerKey>, Diagnostic> {
    require_accepted_owner(lowerer, owner)?;
    let record = lowerer
        .owners
        .get(&owner)
        .map(|working| working.record.clone())
        .ok_or_else(|| {
            delete_corrupt(
                "change_delete_owner_cache",
                "owned-child traversal lost its candidate owner",
            )
        })?;
    let mut children = aggregation_children(&record)?
        .into_iter()
        .map(|(_, child)| child)
        .collect::<BTreeSet<_>>();

    for edge in incoming_relations(lowerer, owner)? {
        if !matches!(
            edge.kind,
            RelationKind::DeclarationModule
                | RelationKind::DocumentationOwnership
                | RelationKind::AnnotationOwnership
        ) {
            continue;
        }
        let source = local_relation_owner(lowerer, edge.source)?;
        let original = accepted_record(lowerer, source)?;
        let working = &lowerer.owners[&source];
        if !record_is_external_child_of(&original, owner) {
            return Err(delete_corrupt(
                "change_delete_ownership_disagreement",
                format!(
                    "ownership witness locates {source:?} under {owner:?}, but accepted canonical meaning does not reproduce that parent"
                ),
            ));
        }
        let candidate = &working.record;
        if !working.deleted && record_is_external_child_of(candidate, owner) {
            children.insert(source);
        }
    }

    if let Some(candidate_children) = candidate_external_children.get(&owner) {
        children.extend(candidate_children);
    }
    children.remove(&owner);
    lowerer.work.ownership_steps = lowerer
        .work
        .ownership_steps
        .saturating_add(u64::try_from(children.len()).unwrap_or(u64::MAX));
    lowerer.check_budget("authored ownership child traversal")?;
    Ok(children.into_iter().collect())
}

fn index_candidate_external_children<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &AuthoredLowerer<'_, B, W>,
) -> BTreeMap<OwnerKey, BTreeSet<OwnerKey>> {
    let mut children = BTreeMap::<OwnerKey, BTreeSet<OwnerKey>>::new();
    for (owner, working) in &lowerer.owners {
        if !working.deleted
            && let Some(parent) = external_parent(&working.record)
        {
            children.entry(parent).or_default().insert(*owner);
        }
    }
    children
}

fn external_parent(record: &OwnerRecord) -> Option<OwnerKey> {
    match record {
        OwnerRecord::Declaration(record) => Some(OwnerKey::Module(record.module)),
        OwnerRecord::Documentation(record) => Some(record.owner),
        OwnerRecord::Annotation(record) => Some(record.owner),
        OwnerRecord::Module(_)
        | OwnerRecord::TypeParameter(_)
        | OwnerRecord::Field(_)
        | OwnerRecord::Case(_)
        | OwnerRecord::Operation(_)
        | OwnerRecord::Parameter(_)
        | OwnerRecord::Binding(_)
        | OwnerRecord::Expression(_)
        | OwnerRecord::Requirement(_)
        | OwnerRecord::Port(_)
        | OwnerRecord::Target(_) => None,
    }
}

fn record_is_external_child_of(record: &OwnerRecord, parent: OwnerKey) -> bool {
    external_parent(record) == Some(parent)
}

fn incoming_relations<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
) -> Result<Vec<RelationEdge>, Diagnostic> {
    if let Some(edge_count) = lowerer.incoming_relations.get(&owner).map(Vec::len) {
        charge_relation_traversal(
            lowerer,
            u64::try_from(edge_count).unwrap_or(u64::MAX),
            "authored deletion cached relation traversal",
        )?;
        return Ok(lowerer.incoming_relations[&owner].clone());
    }
    let remaining = lowerer.budget.remaining_relation_edges(
        lowerer.work.relation_edges_read,
        "authored deletion relation read",
    )?;
    let maximum_items = usize::try_from(remaining)
        .unwrap_or(usize::MAX)
        .min(usize::try_from(lowerer.budget.impact.maximum_relation_fanout).unwrap_or(usize::MAX))
        .min(MAXIMUM_RELATION_PREFIX_ITEMS);
    if maximum_items == 0 {
        let (code, message) = if lowerer.budget.impact.maximum_relation_fanout == 0 {
            (
                "change_budget_relation_fanout",
                "authored deletion has a zero per-owner relation fanout budget",
            )
        } else {
            (
                "change_budget_relation_edges",
                "authored deletion has no remaining relation-edge budget",
            )
        };
        return Err(delete_resource(code, message));
    }
    let read = lowerer
        .witness
        .read_incoming_relations(owner, maximum_items)?;
    lowerer.work.witness.add(read.work);
    charge_relation_traversal(
        lowerer,
        u64::try_from(read.value.edges.len()).unwrap_or(u64::MAX),
        "authored deletion accepted relation traversal",
    )?;
    if read.value.truncated {
        let code = if lowerer.budget.impact.maximum_relation_fanout <= remaining {
            "change_budget_relation_fanout"
        } else {
            "change_budget_relation_edges"
        };
        return Err(delete_resource(
            code,
            format!(
                "owner {owner:?} has more than the admitted {maximum_items}-edge relation prefix"
            ),
        ));
    }
    lowerer
        .incoming_relations
        .insert(owner, read.value.edges.clone());
    lowerer.check_budget("authored deletion relation read")?;
    Ok(read.value.edges)
}

fn charge_relation_traversal<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    edges: u64,
    phase: &str,
) -> Result<(), Diagnostic> {
    let remaining = lowerer
        .budget
        .remaining_relation_edges(lowerer.work.relation_edges_read, phase)?;
    if edges > remaining {
        return Err(delete_resource(
            "change_budget_relation_edges",
            format!("{phase} requires {edges} edge inspections with only {remaining} remaining"),
        ));
    }
    lowerer.work.relation_edges_read = lowerer.work.relation_edges_read.saturating_add(edges);
    Ok(())
}

fn local_relation_owner<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &AuthoredLowerer<'_, B, W>,
    endpoint: RelationEndpoint,
) -> Result<OwnerKey, Diagnostic> {
    let RelationEndpoint::Owner(ExactOwnerKey { package, owner }) = endpoint else {
        return Err(delete_corrupt(
            "change_delete_ownership_endpoint",
            "an owner-containment relation has a package endpoint",
        ));
    };
    if package != lowerer.base.package_id() {
        return Err(delete_corrupt(
            "change_delete_ownership_package",
            "an owner-containment relation has a foreign-package source",
        ));
    }
    Ok(owner)
}

fn detach_root_from_live_parent<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    root: OwnerKey,
    closure: &BTreeSet<OwnerKey>,
) -> Result<(), Diagnostic> {
    let record = lowerer.owners[&root].record.clone();
    let parent = match &record {
        OwnerRecord::TypeParameter(record) => OwnerKey::Declaration(record.declaration),
        OwnerRecord::Field(record) => OwnerKey::Declaration(record.declaration),
        OwnerRecord::Case(record) => OwnerKey::Declaration(record.declaration),
        OwnerRecord::Operation(record) => OwnerKey::Declaration(record.declaration),
        OwnerRecord::Parameter(record) => match record.parent {
            ParameterParent::Function(declaration) => OwnerKey::Declaration(declaration),
            ParameterParent::Operation(operation) => OwnerKey::Operation(operation),
        },
        OwnerRecord::Requirement(record) => OwnerKey::Declaration(record.declaration),
        OwnerRecord::Port(record) => OwnerKey::Declaration(record.declaration),
        OwnerRecord::Module(_)
        | OwnerRecord::Declaration(_)
        | OwnerRecord::Target(_)
        | OwnerRecord::Documentation(_)
        | OwnerRecord::Annotation(_) => return Ok(()),
        OwnerRecord::Binding(_) | OwnerRecord::Expression(_) => {
            return Err(delete_error(
                "change_delete_expression_parent",
                "bindings and expressions require an exact parent edit",
            ));
        }
    };
    if closure.contains(&parent) {
        return Ok(());
    }

    let parent_record = lowerer.candidate_mut(parent)?;
    match (record, parent_record) {
        (OwnerRecord::TypeParameter(_), OwnerRecord::Declaration(parent)) => {
            let OwnerKey::TypeParameter(child) = root else {
                return Err(parent_kind_error(root, parent.header.owner));
            };
            match &mut parent.payload {
                DeclarationPayload::External(function) => {
                    remove_exact(&mut function.type_parameters, child, "type parameter")
                }
                DeclarationPayload::Function(function) => {
                    remove_exact(&mut function.type_parameters, child, "type parameter")
                }
                _ => Err(parent_kind_error(root, parent.header.owner)),
            }
        }
        (OwnerRecord::Field(_), OwnerRecord::Declaration(parent)) => {
            let OwnerKey::Field(child) = root else {
                return Err(parent_kind_error(root, parent.header.owner));
            };
            match &mut parent.payload {
                DeclarationPayload::Record { fields } => remove_exact(fields, child, "field"),
                _ => Err(parent_kind_error(root, parent.header.owner)),
            }
        }
        (OwnerRecord::Case(_), OwnerRecord::Declaration(parent)) => {
            let OwnerKey::Case(child) = root else {
                return Err(parent_kind_error(root, parent.header.owner));
            };
            match &mut parent.payload {
                DeclarationPayload::Variant { cases } => remove_exact(cases, child, "variant case"),
                _ => Err(parent_kind_error(root, parent.header.owner)),
            }
        }
        (OwnerRecord::Operation(child), OwnerRecord::Declaration(parent)) => {
            let OwnerKey::Operation(child) = child.header.owner else {
                return Err(parent_kind_error(root, parent.header.owner));
            };
            match &mut parent.payload {
                DeclarationPayload::Interface { operations } => {
                    remove_exact(operations, child, "interface operation")
                }
                _ => Err(parent_kind_error(root, parent.header.owner)),
            }
        }
        (OwnerRecord::Parameter(child), OwnerRecord::Declaration(parent)) => {
            let OwnerKey::Parameter(child) = child.header.owner else {
                return Err(parent_kind_error(root, parent.header.owner));
            };
            match &mut parent.payload {
                DeclarationPayload::External(function) => {
                    remove_exact(&mut function.parameters, child, "parameter")
                }
                DeclarationPayload::Function(function) => {
                    remove_exact(&mut function.parameters, child, "parameter")
                }
                _ => Err(parent_kind_error(root, parent.header.owner)),
            }
        }
        (OwnerRecord::Parameter(child), OwnerRecord::Operation(parent)) => {
            let OwnerKey::Parameter(child) = child.header.owner else {
                return Err(parent_kind_error(root, parent.header.owner));
            };
            remove_exact(&mut parent.parameters, child, "parameter")
        }
        (OwnerRecord::Requirement(child), OwnerRecord::Declaration(parent)) => {
            let OwnerKey::Requirement(child) = child.header.owner else {
                return Err(parent_kind_error(root, parent.header.owner));
            };
            match &mut parent.payload {
                DeclarationPayload::Component { requirements, .. } => {
                    remove_exact(requirements, child, "component requirement")
                }
                _ => Err(parent_kind_error(root, parent.header.owner)),
            }
        }
        (OwnerRecord::Port(child), OwnerRecord::Declaration(parent)) => match &mut parent.payload {
            DeclarationPayload::Component { ports, .. } => match child.header.owner {
                OwnerKey::Port(child) => remove_exact(ports, child, "component port"),
                _ => Err(parent_kind_error(root, parent.header.owner)),
            },
            _ => Err(parent_kind_error(root, parent.header.owner)),
        },
        (_, parent) => Err(parent_kind_error(root, parent.owner())),
    }
}

fn remove_exact<T: Copy + Eq>(
    values: &mut Vec<T>,
    selected: T,
    kind: &str,
) -> Result<(), Diagnostic> {
    if values.iter().filter(|value| **value == selected).count() != 1 {
        return Err(delete_error(
            "change_delete_parent_membership",
            format!("selected {kind} is not present exactly once in its canonical parent"),
        ));
    }
    values.retain(|value| *value != selected);
    Ok(())
}

fn parent_kind_error(child: OwnerKey, parent: OwnerKey) -> Diagnostic {
    delete_error(
        "change_delete_parent_kind",
        format!("owner {child:?} is incompatible with its canonical parent {parent:?}"),
    )
}

fn reject_untouched_incoming_references<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    closure: &BTreeSet<OwnerKey>,
) -> Result<(), Diagnostic> {
    let mut candidate_relations = BTreeMap::<OwnerKey, Vec<RelationEdge>>::new();
    let mut accepted_relations = BTreeMap::<OwnerKey, Vec<RelationEdge>>::new();
    let deleted_targets = closure
        .iter()
        .map(|owner| {
            RelationEndpoint::Owner(ExactOwnerKey {
                package: lowerer.base.package_id(),
                owner: *owner,
            })
        })
        .collect::<BTreeSet<_>>();
    for target in closure {
        for edge in incoming_relations(lowerer, *target)? {
            let source = match edge.source {
                RelationEndpoint::Owner(ExactOwnerKey { package, owner })
                    if package == lowerer.base.package_id() =>
                {
                    owner
                }
                _ => {
                    return Err(delete_error(
                        "change_delete_live_reference",
                        format!(
                            "owner {target:?} retains incoming {:?} from {:?}",
                            edge.kind, edge.source
                        ),
                    ));
                }
            };
            let source_relations =
                accepted_relations_for(lowerer, &mut accepted_relations, source)?;
            let target_relations =
                accepted_relations_for(lowerer, &mut accepted_relations, *target)?;
            if !source_relations.contains(&edge) && !target_relations.contains(&edge) {
                return Err(delete_corrupt(
                    "change_delete_relation_disagreement",
                    format!(
                        "incoming relation witness edge {edge:?} is not reproduced by accepted canonical owner {source:?}"
                    ),
                ));
            }
            if closure.contains(&source) {
                continue;
            }
            let relations = candidate_relations_for(lowerer, &mut candidate_relations, source)?;
            if let Some(candidate) = relations
                .iter()
                .find(|candidate| deleted_targets.contains(&candidate.target))
            {
                return Err(delete_error(
                    "change_delete_live_reference",
                    format!(
                        "deletion closure retains candidate incoming {:?} from owner {source:?}",
                        candidate.kind
                    ),
                ));
            }
        }
    }

    let mut changed_sources = Vec::new();
    for (owner, working) in &lowerer.owners {
        if !closure.contains(owner) && working_owner_changed(working)? {
            changed_sources.push(*owner);
        }
    }
    for source in changed_sources {
        let relations = candidate_relations_for(lowerer, &mut candidate_relations, source)?;
        if let Some(candidate) = relations
            .iter()
            .find(|candidate| deleted_targets.contains(&candidate.target))
        {
            return Err(delete_error(
                "change_delete_live_reference",
                format!(
                    "changed owner {source:?} introduces or retains candidate {:?} into the deletion closure",
                    candidate.kind
                ),
            ));
        }
    }
    Ok(())
}

fn accepted_relations_for<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    cache: &mut BTreeMap<OwnerKey, Vec<RelationEdge>>,
    owner: OwnerKey,
) -> Result<Vec<RelationEdge>, Diagnostic> {
    if let Some(relations) = cache.get(&owner) {
        return Ok(relations.clone());
    }
    let record = accepted_record(lowerer, owner)?;
    let relations = extract_relations_for_record(lowerer, owner, record, true)?;
    cache.insert(owner, relations.clone());
    Ok(relations)
}

fn candidate_relations_for<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    cache: &mut BTreeMap<OwnerKey, Vec<RelationEdge>>,
    owner: OwnerKey,
) -> Result<Vec<RelationEdge>, Diagnostic> {
    if let Some(relations) = cache.get(&owner) {
        return Ok(relations.clone());
    }
    let relations = extract_candidate_relations(lowerer, owner)?;
    cache.insert(owner, relations.clone());
    Ok(relations)
}

fn working_owner_changed(working: &super::WorkingOwner) -> Result<bool, Diagnostic> {
    let Some(before) = working.before else {
        return Ok(true);
    };
    Ok(working.deleted || encode_owner(&working.record)?.0 != before)
}

fn extract_candidate_relations<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
) -> Result<Vec<RelationEdge>, Diagnostic> {
    let Some(working) = lowerer.owners.get(&owner) else {
        return Err(delete_error(
            "change_delete_live_reference",
            format!("untouched owner {owner:?} retains a live incoming reference"),
        ));
    };
    if working.deleted {
        return Ok(Vec::new());
    }
    let record = working.record.clone();
    extract_relations_for_record(lowerer, owner, record, false)
}

fn extract_relations_for_record<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
    record: OwnerRecord,
    accepted: bool,
) -> Result<Vec<RelationEdge>, Diagnostic> {
    let package = lowerer.base.package_id();
    let mut case_parents = BTreeMap::new();
    if let OwnerRecord::Expression(expression) = &record
        && let ExpressionOperation::Match { arms, .. } = &expression.operation
    {
        for arm in arms {
            if arm.case.package != package || case_parents.contains_key(&arm.case.case) {
                continue;
            }
            let case_owner = OwnerKey::Case(arm.case.case);
            let case_record = if accepted {
                accepted_record(lowerer, case_owner)?
            } else {
                lowerer.require_owner(case_owner)?;
                lowerer.owners[&case_owner].record.clone()
            };
            let parent = match case_record {
                OwnerRecord::Case(record) => Some(record.declaration),
                _ => None,
            };
            case_parents.insert(arm.case.case, parent);
        }
    }
    let relations = extract_owner_relations(
        package,
        owner,
        &record,
        |digest| lowerer.candidate_type_object(digest),
        |target_package, case| {
            if target_package != package {
                return Ok(None);
            }
            Ok(case_parents.get(&case).copied().flatten())
        },
    )?;
    let relation_count = u64::try_from(relations.len()).unwrap_or(u64::MAX);
    if relation_count > lowerer.budget.impact.maximum_relation_fanout {
        return Err(delete_resource(
            "change_budget_relation_fanout",
            format!(
                "candidate owner {owner:?} has {relation_count} outgoing relations, exceeding the declared {}-edge fanout budget",
                lowerer.budget.impact.maximum_relation_fanout
            ),
        ));
    }
    charge_relation_traversal(
        lowerer,
        relation_count,
        "authored deletion candidate relation extraction",
    )?;
    lowerer.check_budget("authored deletion final relation extraction")?;
    Ok(relations)
}

fn accepted_record<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
) -> Result<OwnerRecord, Diagnostic> {
    if !lowerer.owners.contains_key(&owner) {
        require_accepted_owner(lowerer, owner)?;
    }
    lowerer.owners[&owner].original.clone().ok_or_else(|| {
        delete_corrupt(
            "change_delete_owner_cache",
            "accepted relation verification lost its canonical owner record",
        )
    })
}

fn retain_retirements_and_mark_deleted<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    closure: &BTreeSet<OwnerKey>,
) -> Result<(), Diagnostic> {
    let revision = lowerer.base.exact_revision().ok_or_else(|| {
        delete_corrupt(
            "change_delete_revision",
            "authored deletion requires one exact accepted base revision",
        )
    })?;
    for owner in closure {
        let original = lowerer.owners[owner].original.clone().ok_or_else(|| {
            delete_error(
                "change_delete_created_owner",
                "request-local creations cannot be retired",
            )
        })?;
        let last_parent = base_parent(lowerer, *owner)?;
        let record = RetirementRecord {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            owner: *owner,
            last_kind: original.kind(),
            last_name: original.name().cloned(),
            last_parent,
            last_live_revision: revision,
            deletion_change: lowerer.deletion_change,
        };
        if lowerer.retirements.insert(*owner, record).is_some() {
            return Err(delete_error(
                "change_delete_duplicate",
                format!("owner {owner:?} is selected for deletion more than once"),
            ));
        }
        lowerer
            .owners
            .get_mut(owner)
            .ok_or_else(|| {
                delete_corrupt(
                    "change_delete_owner_cache",
                    "retirement lowering lost its selected owner",
                )
            })?
            .deleted = true;
    }
    Ok(())
}

fn base_parent<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
) -> Result<Option<OwnerKey>, Diagnostic> {
    if !lowerer.ownership.contains_key(&owner) {
        let read = lowerer.witness.read_ownership(owner)?;
        lowerer.work.witness.add(read.work);
        lowerer.ownership.insert(owner, read.value);
    }
    let entry = lowerer.ownership[&owner].ok_or_else(|| {
        delete_corrupt(
            "change_delete_ownership_missing",
            format!("accepted owner {owner:?} has no exact base ownership witness"),
        )
    })?;
    let original = lowerer.owners[&owner].original.as_ref().ok_or_else(|| {
        delete_corrupt(
            "change_delete_owner_cache",
            "retirement lowering lost the accepted canonical owner record",
        )
    })?;
    let mut canonical = ownership_contributions(original)?.get(&owner).copied();
    if canonical.is_none()
        && let OwnershipParent::Owner(parent) = entry.parent
    {
        if !lowerer.owners.contains_key(&parent) {
            require_accepted_owner(lowerer, parent)?;
        }
        let parent = lowerer.owners[&parent].original.as_ref().ok_or_else(|| {
            delete_corrupt(
                "change_delete_owner_cache",
                "retirement lowering lost the accepted canonical parent record",
            )
        })?;
        canonical = ownership_contributions(parent)?.get(&owner).copied();
    }
    if canonical != Some(entry) {
        return Err(delete_corrupt(
            "change_delete_ownership_disagreement",
            format!(
                "accepted ownership witness for {owner:?} is not reproduced by canonical meaning"
            ),
        ));
    }
    Ok(match entry.parent {
        OwnershipParent::Package => None,
        OwnershipParent::Owner(parent) => Some(parent),
    })
}

fn delete_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    request_error(DiagnosticClass::Semantic, code, message)
}

fn delete_resource(code: &'static str, message: impl Into<String>) -> Diagnostic {
    request_error(DiagnosticClass::Resource, code, message)
}

fn delete_corrupt(code: &'static str, message: impl Into<String>) -> Diagnostic {
    request_error(DiagnosticClass::Corrupt, code, message)
}
