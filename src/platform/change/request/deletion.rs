//! Explicit Graph 5 owner deletion, ownership closure, and retirement lowering.

use super::{AuthoredLowerer, OwnerSelector, request_error};
use crate::platform::change::{CanonicalBaseRead, WitnessBaseRead};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
use crate::platform::kernel::{
    DeclarationPayload, ExactOwnerKey, ExpressionOperation, OwnerKey, OwnerRecord, ParameterParent,
    RelationEdge, RelationEndpoint, RelationKind, RetirementRecord, encode_owner,
    extract_owner_relations,
};
use crate::platform::witness::{
    MAXIMUM_RELATION_PREFIX_ITEMS, OwnershipParent, aggregation_children,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub(super) fn lower_deletions<
    'request,
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    deletions: impl IntoIterator<Item = (&'request OwnerSelector, bool)>,
) -> Result<(), Diagnostic> {
    let mut roots = BTreeMap::new();
    for (selector, cascade) in deletions {
        let root = lowerer.resolve_owner(selector)?;
        if matches!(root, OwnerKey::Binding(_) | OwnerKey::Expression(_)) {
            return Err(delete_error(
                "change_delete_expression_parent",
                "bindings and expressions must be removed through an exact parent expression or root edit",
            ));
        }
        require_accepted_owner(lowerer, root)?;
        if roots.insert(root, cascade).is_some() {
            return Err(delete_error(
                "change_delete_duplicate",
                format!("owner {root:?} is selected for deletion more than once"),
            ));
        }
    }
    if roots.is_empty() {
        return Ok(());
    }

    let mut closure = BTreeSet::new();
    for (root, cascade) in &roots {
        let direct_children = current_owned_children(lowerer, *root)?;
        let undeclared_children = direct_children
            .iter()
            .filter(|child| !roots.contains_key(child))
            .count();
        if !cascade && undeclared_children != 0 {
            return Err(delete_error(
                "change_delete_requires_cascade",
                format!(
                    "owner {root:?} has {} owned children; deletion requires explicit cascade",
                    undeclared_children
                ),
            ));
        }
        let mut frontier = VecDeque::from([*root]);
        while let Some(owner) = frontier.pop_front() {
            if !closure.insert(owner) {
                continue;
            }
            require_accepted_owner(lowerer, owner)?;
            if *cascade {
                for child in current_owned_children(lowerer, owner)? {
                    if !closure.contains(&child) {
                        frontier.push_back(child);
                    }
                }
            }
            lowerer.check_budget("authored deletion ownership closure")?;
        }
    }

    for root in roots.keys() {
        detach_root_from_live_parent(lowerer, *root, &closure)?;
    }
    reject_untouched_incoming_references(lowerer, &closure)?;
    retain_retirements_and_mark_deleted(lowerer, &closure)?;
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
        lowerer.require_owner(source)?;
        let candidate = &lowerer.owners[&source].record;
        if record_is_external_child_of(candidate, owner) {
            children.insert(source);
        }
    }

    for (candidate_owner, working) in &lowerer.owners {
        if !working.deleted && record_is_external_child_of(&working.record, owner) {
            children.insert(*candidate_owner);
        }
    }
    children.remove(&owner);
    Ok(children.into_iter().collect())
}

fn record_is_external_child_of(record: &OwnerRecord, parent: OwnerKey) -> bool {
    match (record, parent) {
        (OwnerRecord::Declaration(record), OwnerKey::Module(module)) => record.module == module,
        (OwnerRecord::Documentation(record), owner) => record.owner == owner,
        (OwnerRecord::Annotation(record), owner) => record.owner == owner,
        _ => false,
    }
}

fn incoming_relations<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
) -> Result<Vec<RelationEdge>, Diagnostic> {
    if let Some(edges) = lowerer.incoming_relations.get(&owner) {
        return Ok(edges.clone());
    }
    let read = lowerer
        .witness
        .read_incoming_relations(owner, MAXIMUM_RELATION_PREFIX_ITEMS)?;
    lowerer.work.witness.add(read.work);
    lowerer.work.relation_edges_read = lowerer
        .work
        .relation_edges_read
        .saturating_add(u64::try_from(read.value.edges.len()).unwrap_or(u64::MAX));
    if read.value.truncated {
        return Err(delete_resource(
            "change_delete_relation_budget",
            format!(
                "owner {owner:?} has more than {MAXIMUM_RELATION_PREFIX_ITEMS} incoming relations; use a narrower dependency-closed change"
            ),
        ));
    }
    lowerer
        .incoming_relations
        .insert(owner, read.value.edges.clone());
    lowerer.check_budget("authored deletion relation read")?;
    Ok(read.value.edges)
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
    let record = working.record.clone();
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
            lowerer.require_owner(case_owner)?;
            let parent = match &lowerer.owners[&case_owner].record {
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
    lowerer.check_budget("authored deletion final relation extraction")?;
    Ok(relations)
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
