//! Owner-local summary dimensions and declaration-sized descendant aggregation.

use super::contract::{
    CAPABILITY_DIGEST_DOMAIN, EFFECT_DIGEST_DOMAIN, IMPLEMENTATION_DIGEST_DOMAIN,
    INTERFACE_DIGEST_DOMAIN, PRESENTATION_DIGEST_DOMAIN, RELATION_DIGEST_DOMAIN,
    TEST_DIGEST_DOMAIN, TYPE_DIGEST_DOMAIN, VALIDATION_DEPENDENCY_DIGEST_DOMAIN,
};
use super::entry::{
    BindingContainerRole, ExpressionRootRole, OwnershipEntry, OwnershipParent, OwnershipRole,
    encode_endpoint,
};
use super::summary::{OwnerSummary, current_summary_contract};
use super::{SemanticDigest, witness_error};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    AnnotationClass, DeclarationPayload, DocumentationClass, EncodedOwnerKey, ExactOwnerKey,
    ExpressionOperation, FunctionEffect, KernelSnapshot, OwnerKey, OwnerKind, OwnerRecord,
    PackageId, PortImplementation, PropagationClass, RelationEdge, RelationEndpoint,
};
use bincode::Encode;
use std::collections::{BTreeMap, BTreeSet};

/// Exact point-read surface needed to rebuild a bounded set of owner summaries. Implementations
/// may read accepted authority directly or through an isolated candidate overlay.
pub(crate) trait SummaryRead {
    fn package_id(&self) -> PackageId;

    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic>;

    fn dependency(
        &self,
        package: PackageId,
    ) -> Result<Option<crate::platform::kernel::DependencyRecord>, Diagnostic>;

    fn ownership(&self, owner: OwnerKey) -> Result<Option<OwnershipEntry>, Diagnostic>;

    fn outgoing_relations(&self, owner: OwnerKey) -> Result<Vec<RelationEdge>, Diagnostic>;

    fn base_summary(&self, owner: OwnerKey) -> Result<Option<OwnerSummary>, Diagnostic>;
}

#[derive(Clone)]
struct WorkingSummary {
    owner: OwnerKey,
    kind: OwnerKind,
    record: crate::platform::kernel::OwnerObjectDigest,
    semantic_interface: SemanticDigest,
    implementation: SemanticDigest,
    type_digest: SemanticDigest,
    effect: SemanticDigest,
    capability: SemanticDigest,
    relations: SemanticDigest,
    presentation: SemanticDigest,
    validation_dependencies: SemanticDigest,
    test_seed: Option<SemanticDigest>,
}

#[derive(Clone, Copy)]
struct SummaryDimensions {
    semantic_interface: SemanticDigest,
    implementation: SemanticDigest,
    type_digest: SemanticDigest,
    effect: SemanticDigest,
    capability: SemanticDigest,
    relations: SemanticDigest,
    validation_dependencies: SemanticDigest,
}

impl From<&WorkingSummary> for SummaryDimensions {
    fn from(summary: &WorkingSummary) -> Self {
        Self {
            semantic_interface: summary.semantic_interface,
            implementation: summary.implementation,
            type_digest: summary.type_digest,
            effect: summary.effect,
            capability: summary.capability,
            relations: summary.relations,
            validation_dependencies: summary.validation_dependencies,
        }
    }
}

impl From<&OwnerSummary> for SummaryDimensions {
    fn from(summary: &OwnerSummary) -> Self {
        Self {
            semantic_interface: summary.semantic_interface,
            implementation: summary.implementation,
            type_digest: summary.type_digest,
            effect: summary.effect,
            capability: summary.capability,
            relations: summary.relations,
            validation_dependencies: summary.validation_dependencies,
        }
    }
}

impl WorkingSummary {
    fn finish(self) -> OwnerSummary {
        let test = self.test_seed.map(|seed| {
            combine_digests(
                TEST_DIGEST_DOMAIN,
                seed,
                [
                    self.implementation,
                    self.type_digest,
                    self.effect,
                    self.capability,
                    self.relations,
                    self.validation_dependencies,
                ],
            )
        });
        OwnerSummary {
            contract_version: current_summary_contract(),
            owner: self.owner,
            kind: self.kind,
            record: self.record,
            semantic_interface: self.semantic_interface,
            implementation: self.implementation,
            type_digest: self.type_digest,
            effect: self.effect,
            capability: self.capability,
            relations: self.relations,
            presentation: self.presentation,
            test,
            validation_dependencies: self.validation_dependencies,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AggregationMode {
    None,
    Interface,
    Implementation,
}

pub(super) fn build_owner_summaries(
    snapshot: &KernelSnapshot,
    ownership: &BTreeMap<OwnerKey, OwnershipEntry>,
    relations: &[RelationEdge],
) -> Result<BTreeMap<OwnerKey, OwnerSummary>, Diagnostic> {
    let depths = ownership_depths(ownership)?;
    let children = direct_children(ownership);
    let outgoing = outgoing_relations(snapshot.root.package_id, relations);
    let mut summaries = BTreeMap::new();
    for (owner, record) in &snapshot.owners {
        let summary = local_summary(*owner, record, outgoing.get(owner).map(Vec::as_slice))?;
        summaries.insert(*owner, summary);
    }
    aggregate_semantic_children(&mut summaries, &children, &depths)?;
    derive_validation_dependencies(snapshot, &outgoing, &mut summaries)?;
    aggregate_validation_children(&mut summaries, &children, &depths)?;
    Ok(summaries
        .into_iter()
        .map(|(owner, summary)| (owner, summary.finish()))
        .collect())
}

/// Rebuilds only selected live owner summaries. Unselected child and relation-target summaries are
/// reused from the exact base witness. The selected set must include every aggregating ancestor of
/// a changed owner and every owner whose validation-dependency digest must be refreshed.
pub(crate) fn rebuild_selected_owner_summaries<R: SummaryRead>(
    view: &R,
    selected: &BTreeSet<OwnerKey>,
) -> Result<BTreeMap<OwnerKey, OwnerSummary>, Diagnostic> {
    let mut live = BTreeSet::new();
    for owner in selected {
        if view.owner(*owner)?.is_some() {
            live.insert(*owner);
        }
    }
    let depths = selected_ownership_depths(view, &live)?;
    let mut working = BTreeMap::new();
    for owner in &live {
        let record = view.owner(*owner)?.ok_or_else(|| {
            witness_error(
                DiagnosticClass::Corrupt,
                "witness_selected_owner_missing",
                "selected live owner disappeared during summary construction",
            )
        })?;
        let outgoing = view.outgoing_relations(*owner)?;
        working.insert(*owner, local_summary(*owner, &record, Some(&outgoing))?);
    }

    let mut deepest_first = depths
        .iter()
        .map(|(owner, depth)| (*depth, *owner))
        .collect::<Vec<_>>();
    deepest_first.sort_by(|left, right| right.cmp(left));
    for (_, owner) in &deepest_first {
        let record = view.owner(*owner)?.ok_or_else(|| {
            witness_error(
                DiagnosticClass::Corrupt,
                "witness_selected_owner_missing",
                "selected owner has no candidate record",
            )
        })?;
        let children = selected_aggregation_children(view, *owner, &record)?;
        let child_summaries = children
            .into_iter()
            .filter(|(role, _)| aggregation_mode(*role) != AggregationMode::None)
            .map(|(role, child)| {
                let dimensions = if let Some(summary) = working.get(&child) {
                    Some(SummaryDimensions::from(summary))
                } else if !selected.contains(&child) {
                    view.base_summary(child)?
                        .as_ref()
                        .map(SummaryDimensions::from)
                } else {
                    None
                }
                .ok_or_else(|| {
                    witness_error(
                        DiagnosticClass::Corrupt,
                        "witness_selected_child_summary",
                        format!("aggregating child {child:?} has no candidate summary"),
                    )
                })?;
                Ok((role, child, aggregation_mode(role), dimensions))
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        let parent = working.get_mut(owner).ok_or_else(|| {
            witness_error(
                DiagnosticClass::Corrupt,
                "witness_selected_parent_summary",
                "selected parent has no local working summary",
            )
        })?;
        if !child_summaries.is_empty() {
            aggregate_semantic_parent(parent, &child_summaries)?;
        }
    }

    let semantic = working.clone();
    for (owner, summary) in &mut working {
        let outgoing = view.outgoing_relations(*owner)?;
        summary.validation_dependencies = validation_dependency_digest(
            view.package_id(),
            summary.kind,
            &outgoing,
            |target| {
                if let Some(summary) = semantic.get(&target) {
                    Ok(Some(SummaryDimensions::from(summary)))
                } else {
                    Ok(view
                        .base_summary(target)?
                        .as_ref()
                        .map(SummaryDimensions::from))
                }
            },
            |package| {
                view.dependency(package)?
                    .map(|dependency| crate::platform::kernel::encode_dependency(&dependency))
                    .transpose()
                    .map(|encoded| encoded.map(|(digest, _)| digest))
            },
        )?;
    }

    for (_, owner) in deepest_first {
        let record = view.owner(owner)?.ok_or_else(|| {
            witness_error(
                DiagnosticClass::Corrupt,
                "witness_selected_owner_missing",
                "selected owner has no candidate record",
            )
        })?;
        let children = selected_aggregation_children(view, owner, &record)?;
        let child_validations = children
            .into_iter()
            .filter(|(role, _)| aggregation_mode(*role) != AggregationMode::None)
            .map(|(role, child)| {
                let digest = if let Some(summary) = working.get(&child) {
                    Some(summary.validation_dependencies)
                } else if !selected.contains(&child) {
                    view.base_summary(child)?
                        .map(|summary| summary.validation_dependencies)
                } else {
                    None
                }
                .ok_or_else(|| {
                    witness_error(
                        DiagnosticClass::Corrupt,
                        "witness_selected_child_validation",
                        format!("aggregating child {child:?} has no validation summary"),
                    )
                })?;
                Ok((role, child, digest))
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        let parent = working.get_mut(&owner).ok_or_else(|| {
            witness_error(
                DiagnosticClass::Corrupt,
                "witness_selected_parent_summary",
                "selected parent has no working validation summary",
            )
        })?;
        if !child_validations.is_empty() {
            aggregate_validation_parent(parent, child_validations)?;
        }
    }

    Ok(working
        .into_iter()
        .map(|(owner, summary)| (owner, summary.finish()))
        .collect())
}

fn selected_aggregation_children<R: SummaryRead>(
    view: &R,
    owner: OwnerKey,
    record: &OwnerRecord,
) -> Result<Vec<(OwnershipRole, OwnerKey)>, Diagnostic> {
    let mut children = aggregation_children(record)?;
    let OwnerRecord::Declaration(declaration) = record else {
        return Ok(children);
    };
    let DeclarationPayload::Function(function) = &declaration.payload else {
        return Ok(children);
    };
    let FunctionEffect::Task { requirements } = &function.effect else {
        return Ok(children);
    };
    for requirement in requirements {
        if requirement.package != view.package_id() {
            continue;
        }
        let child = OwnerKey::Requirement(requirement.requirement);
        let expected = OwnershipEntry::new(
            OwnershipParent::Owner(owner),
            OwnershipRole::DeclarationRequirement,
        );
        if view.ownership(child)? == Some(expected) {
            children.push((OwnershipRole::DeclarationRequirement, child));
        }
    }
    children.sort_unstable();
    children.dedup();
    Ok(children)
}

fn selected_ownership_depths<R: SummaryRead>(
    view: &R,
    selected: &BTreeSet<OwnerKey>,
) -> Result<BTreeMap<OwnerKey, usize>, Diagnostic> {
    let mut depths = BTreeMap::new();
    for owner in selected {
        let mut current = *owner;
        let mut observed = BTreeSet::new();
        let mut depth = 0_usize;
        loop {
            if !observed.insert(current) {
                return Err(witness_error(
                    DiagnosticClass::Corrupt,
                    "witness_selected_ownership_cycle",
                    "selected summary ownership path is cyclic",
                ));
            }
            let entry = view.ownership(current)?.ok_or_else(|| {
                witness_error(
                    DiagnosticClass::Corrupt,
                    "witness_selected_ownership_missing",
                    format!("selected owner {current:?} has no candidate ownership entry"),
                )
            })?;
            depth = depth.checked_add(1).ok_or_else(|| {
                witness_error(
                    DiagnosticClass::Resource,
                    "witness_selected_ownership_depth",
                    "selected ownership depth overflowed",
                )
            })?;
            match entry.parent {
                OwnershipParent::Package => break,
                OwnershipParent::Owner(parent) => current = parent,
            }
        }
        depths.insert(*owner, depth);
    }
    Ok(depths)
}

fn ownership_depths(
    ownership: &BTreeMap<OwnerKey, OwnershipEntry>,
) -> Result<BTreeMap<OwnerKey, usize>, Diagnostic> {
    let mut depths = BTreeMap::<OwnerKey, usize>::new();
    for owner in ownership.keys() {
        let mut path = Vec::new();
        let mut current = *owner;
        loop {
            if let Some(depth) = depths.get(&current).copied() {
                let mut next = depth;
                while let Some(item) = path.pop() {
                    next = next.checked_add(1).ok_or_else(|| {
                        witness_error(
                            DiagnosticClass::Resource,
                            "witness_ownership_depth",
                            "ownership depth overflowed",
                        )
                    })?;
                    depths.insert(item, next);
                }
                break;
            }
            if path.contains(&current) || path.len() > ownership.len() {
                return Err(witness_error(
                    DiagnosticClass::Corrupt,
                    "witness_ownership_cycle",
                    format!("ownership path from {owner:?} is cyclic"),
                ));
            }
            path.push(current);
            let entry = ownership.get(&current).ok_or_else(|| {
                witness_error(
                    DiagnosticClass::Corrupt,
                    "witness_ownership_missing",
                    format!("ownership path references missing owner {current:?}"),
                )
            })?;
            match entry.parent {
                OwnershipParent::Package => {
                    let mut next = 0;
                    while let Some(item) = path.pop() {
                        next += 1;
                        depths.insert(item, next);
                    }
                    break;
                }
                OwnershipParent::Owner(parent) => current = parent,
            }
        }
    }
    Ok(depths)
}

fn direct_children(
    ownership: &BTreeMap<OwnerKey, OwnershipEntry>,
) -> BTreeMap<OwnerKey, Vec<(OwnershipRole, OwnerKey)>> {
    let mut children = BTreeMap::<OwnerKey, Vec<(OwnershipRole, OwnerKey)>>::new();
    for (owner, entry) in ownership {
        if let OwnershipParent::Owner(parent) = entry.parent {
            children
                .entry(parent)
                .or_default()
                .push((entry.role, *owner));
        }
    }
    for entries in children.values_mut() {
        entries.sort_unstable();
    }
    children
}

fn aggregation_mode(role: OwnershipRole) -> AggregationMode {
    if !role.aggregates_into_parent() {
        return AggregationMode::None;
    }
    match role {
        OwnershipRole::DeclarationTypeParameter
        | OwnershipRole::DeclarationField
        | OwnershipRole::DeclarationCase
        | OwnershipRole::DeclarationOperation
        | OwnershipRole::DeclarationParameter
        | OwnershipRole::OperationParameter
        | OwnershipRole::DeclarationRequirement
        | OwnershipRole::DeclarationPort => AggregationMode::Interface,
        OwnershipRole::ExpressionRoot(_)
        | OwnershipRole::ExpressionChild { .. }
        | OwnershipRole::ExpressionBinding { .. } => AggregationMode::Implementation,
        OwnershipRole::PackageModule
        | OwnershipRole::PackageTarget
        | OwnershipRole::ModuleDeclaration
        | OwnershipRole::Documentation
        | OwnershipRole::Annotation => AggregationMode::None,
    }
}

fn aggregate_semantic_children(
    summaries: &mut BTreeMap<OwnerKey, WorkingSummary>,
    children: &BTreeMap<OwnerKey, Vec<(OwnershipRole, OwnerKey)>>,
    depths: &BTreeMap<OwnerKey, usize>,
) -> Result<(), Diagnostic> {
    let mut owners = depths
        .iter()
        .map(|(owner, depth)| (*depth, *owner))
        .collect::<Vec<_>>();
    owners.sort_by(|left, right| right.cmp(left));
    for (_, owner) in owners {
        let Some(child_entries) = children.get(&owner) else {
            continue;
        };
        let child_summaries = child_entries
            .iter()
            .filter_map(|(role, child)| {
                let mode = aggregation_mode(*role);
                (mode != AggregationMode::None)
                    .then(|| {
                        summaries
                            .get(child)
                            .map(SummaryDimensions::from)
                            .map(|value| (*role, *child, mode, value))
                    })
                    .flatten()
            })
            .collect::<Vec<_>>();
        if child_summaries.is_empty() {
            continue;
        }
        let parent = summaries.get_mut(&owner).ok_or_else(|| {
            witness_error(
                DiagnosticClass::Corrupt,
                "witness_summary_parent",
                "ownership parent has no owner summary",
            )
        })?;
        aggregate_semantic_parent(parent, &child_summaries)?;
    }
    Ok(())
}

fn aggregate_semantic_parent(
    parent: &mut WorkingSummary,
    child_summaries: &[(OwnershipRole, OwnerKey, AggregationMode, SummaryDimensions)],
) -> Result<(), Diagnostic> {
    parent.semantic_interface = aggregate_dimension(
        INTERFACE_DIGEST_DOMAIN,
        parent.semantic_interface,
        child_summaries
            .iter()
            .filter(|(_, _, mode, _)| *mode == AggregationMode::Interface)
            .map(|(role, child, _, summary)| {
                (
                    *role,
                    *child,
                    vec![
                        summary.semantic_interface,
                        summary.type_digest,
                        summary.effect,
                        summary.capability,
                    ],
                )
            }),
    )?;
    parent.implementation = aggregate_dimension(
        IMPLEMENTATION_DIGEST_DOMAIN,
        parent.implementation,
        child_summaries.iter().map(|(role, child, mode, summary)| {
            let mut dimensions = vec![summary.implementation];
            if *mode == AggregationMode::Implementation {
                dimensions.extend([
                    summary.semantic_interface,
                    summary.type_digest,
                    summary.effect,
                    summary.capability,
                    summary.relations,
                ]);
            }
            (*role, *child, dimensions)
        }),
    )?;
    parent.type_digest = aggregate_dimension(
        TYPE_DIGEST_DOMAIN,
        parent.type_digest,
        child_summaries
            .iter()
            .map(|(role, child, _, summary)| (*role, *child, vec![summary.type_digest])),
    )?;
    parent.effect = aggregate_dimension(
        EFFECT_DIGEST_DOMAIN,
        parent.effect,
        child_summaries
            .iter()
            .map(|(role, child, _, summary)| (*role, *child, vec![summary.effect])),
    )?;
    parent.capability = aggregate_dimension(
        CAPABILITY_DIGEST_DOMAIN,
        parent.capability,
        child_summaries
            .iter()
            .map(|(role, child, _, summary)| (*role, *child, vec![summary.capability])),
    )?;
    parent.relations = aggregate_dimension(
        RELATION_DIGEST_DOMAIN,
        parent.relations,
        child_summaries
            .iter()
            .map(|(role, child, _, summary)| (*role, *child, vec![summary.relations])),
    )?;
    Ok(())
}

fn derive_validation_dependencies(
    snapshot: &KernelSnapshot,
    outgoing: &BTreeMap<OwnerKey, Vec<RelationEdge>>,
    summaries: &mut BTreeMap<OwnerKey, WorkingSummary>,
) -> Result<(), Diagnostic> {
    let package = snapshot.root.package_id;
    let frozen = summaries.clone();
    for (owner, summary) in summaries.iter_mut() {
        summary.validation_dependencies = validation_dependency_digest(
            package,
            summary.kind,
            outgoing.get(owner).map_or(&[], Vec::as_slice),
            |target| Ok(frozen.get(&target).map(SummaryDimensions::from)),
            |target_package| {
                snapshot
                    .dependencies
                    .get(&target_package)
                    .map(crate::platform::kernel::encode_dependency)
                    .transpose()
                    .map(|encoded| encoded.map(|(digest, _)| digest))
            },
        )?;
    }
    Ok(())
}

fn validation_dependency_digest<T, D>(
    package: PackageId,
    kind: OwnerKind,
    outgoing: &[RelationEdge],
    mut target_summary: T,
    mut dependency_digest: D,
) -> Result<SemanticDigest, Diagnostic>
where
    T: FnMut(OwnerKey) -> Result<Option<SummaryDimensions>, Diagnostic>,
    D: FnMut(
        PackageId,
    ) -> Result<Option<crate::platform::kernel::DependencyObjectDigest>, Diagnostic>,
{
    let mut material = Material::new(kind);
    for edge in outgoing {
        let propagation = edge.kind.propagation();
        if matches!(
            propagation,
            PropagationClass::Ownership | PropagationClass::Presentation
        ) {
            continue;
        }
        let mut edge_bytes = Vec::new();
        edge_bytes.push(edge.kind.tag());
        encode_endpoint(&mut edge_bytes, edge.target);
        material.raw_piece(1, &edge_bytes);
        match edge.target {
            RelationEndpoint::Owner(ExactOwnerKey {
                package: target_package,
                owner: target_owner,
            }) if target_package == package => {
                let target = target_summary(target_owner)?.ok_or_else(|| {
                    witness_error(
                        DiagnosticClass::Corrupt,
                        "witness_validation_target",
                        format!("local relation target {target_owner:?} has no summary"),
                    )
                })?;
                append_dependency_dimensions(&mut material, propagation, target);
            }
            RelationEndpoint::Owner(ExactOwnerKey {
                package: target_package,
                ..
            })
            | RelationEndpoint::Package(target_package) => {
                if target_package != package {
                    let digest = dependency_digest(target_package)?.ok_or_else(|| {
                        witness_error(
                            DiagnosticClass::Corrupt,
                            "witness_dependency_target",
                            "foreign relation target has no exact dependency binding",
                        )
                    })?;
                    material.piece(2, &digest)?;
                }
            }
        }
    }
    Ok(material.finish(VALIDATION_DEPENDENCY_DIGEST_DOMAIN))
}

fn append_dependency_dimensions(
    material: &mut Material,
    propagation: PropagationClass,
    target: SummaryDimensions,
) {
    match propagation {
        PropagationClass::Type | PropagationClass::Value => {
            material.digest_piece(3, target.semantic_interface);
            material.digest_piece(4, target.type_digest);
        }
        PropagationClass::Behavior | PropagationClass::Capability => {
            material.digest_piece(3, target.semantic_interface);
            material.digest_piece(5, target.effect);
            material.digest_piece(6, target.capability);
        }
        PropagationClass::Target => {
            material.digest_piece(3, target.semantic_interface);
            material.digest_piece(7, target.implementation);
        }
        PropagationClass::Test => {
            material.digest_piece(7, target.implementation);
        }
        PropagationClass::Package => {
            material.digest_piece(3, target.semantic_interface);
            material.digest_piece(7, target.implementation);
        }
        PropagationClass::Ownership | PropagationClass::Presentation => {}
    }
}

fn aggregate_validation_children(
    summaries: &mut BTreeMap<OwnerKey, WorkingSummary>,
    children: &BTreeMap<OwnerKey, Vec<(OwnershipRole, OwnerKey)>>,
    depths: &BTreeMap<OwnerKey, usize>,
) -> Result<(), Diagnostic> {
    let mut owners = depths
        .iter()
        .map(|(owner, depth)| (*depth, *owner))
        .collect::<Vec<_>>();
    owners.sort_by(|left, right| right.cmp(left));
    for (_, owner) in owners {
        let child_validations = children
            .get(&owner)
            .into_iter()
            .flatten()
            .filter(|(role, _)| aggregation_mode(*role) != AggregationMode::None)
            .filter_map(|(role, child)| {
                summaries
                    .get(child)
                    .map(|summary| (*role, *child, summary.validation_dependencies))
            })
            .collect::<Vec<_>>();
        if child_validations.is_empty() {
            continue;
        }
        let parent = summaries.get_mut(&owner).ok_or_else(|| {
            witness_error(
                DiagnosticClass::Corrupt,
                "witness_validation_parent",
                "validation aggregation parent has no summary",
            )
        })?;
        aggregate_validation_parent(parent, child_validations)?;
    }
    Ok(())
}

fn aggregate_validation_parent(
    parent: &mut WorkingSummary,
    children: impl IntoIterator<Item = (OwnershipRole, OwnerKey, SemanticDigest)>,
) -> Result<(), Diagnostic> {
    parent.validation_dependencies = aggregate_dimension(
        VALIDATION_DEPENDENCY_DIGEST_DOMAIN,
        parent.validation_dependencies,
        children
            .into_iter()
            .map(|(role, child, digest)| (role, child, vec![digest])),
    )?;
    Ok(())
}

pub(crate) fn aggregation_children(
    record: &OwnerRecord,
) -> Result<Vec<(OwnershipRole, OwnerKey)>, Diagnostic> {
    let mut children = Vec::new();
    match record {
        OwnerRecord::Declaration(record) => match &record.payload {
            DeclarationPayload::Record { fields } => children.extend(
                fields
                    .iter()
                    .map(|field| (OwnershipRole::DeclarationField, OwnerKey::Field(*field))),
            ),
            DeclarationPayload::Variant { cases } => children.extend(
                cases
                    .iter()
                    .map(|case| (OwnershipRole::DeclarationCase, OwnerKey::Case(*case))),
            ),
            DeclarationPayload::Interface { operations } => {
                children.extend(operations.iter().map(|operation| {
                    (
                        OwnershipRole::DeclarationOperation,
                        OwnerKey::Operation(*operation),
                    )
                }));
            }
            DeclarationPayload::External(function) => {
                children.extend(function.type_parameters.iter().map(|parameter| {
                    (
                        OwnershipRole::DeclarationTypeParameter,
                        OwnerKey::TypeParameter(*parameter),
                    )
                }));
                children.extend(function.parameters.iter().map(|parameter| {
                    (
                        OwnershipRole::DeclarationParameter,
                        OwnerKey::Parameter(*parameter),
                    )
                }));
            }
            DeclarationPayload::Function(function) => {
                children.extend(function.type_parameters.iter().map(|parameter| {
                    (
                        OwnershipRole::DeclarationTypeParameter,
                        OwnerKey::TypeParameter(*parameter),
                    )
                }));
                children.extend(function.parameters.iter().map(|parameter| {
                    (
                        OwnershipRole::DeclarationParameter,
                        OwnerKey::Parameter(*parameter),
                    )
                }));
                children.push((
                    OwnershipRole::ExpressionRoot(ExpressionRootRole::FunctionBody),
                    OwnerKey::Expression(function.body),
                ));
            }
            DeclarationPayload::Constant { value, .. } => children.push((
                OwnershipRole::ExpressionRoot(ExpressionRootRole::ConstantValue),
                OwnerKey::Expression(*value),
            )),
            DeclarationPayload::Component {
                requirements,
                ports,
            } => {
                children.extend(requirements.iter().map(|requirement| {
                    (
                        OwnershipRole::DeclarationRequirement,
                        OwnerKey::Requirement(*requirement),
                    )
                }));
                children.extend(
                    ports
                        .iter()
                        .map(|port| (OwnershipRole::DeclarationPort, OwnerKey::Port(*port))),
                );
            }
            DeclarationPayload::Test {
                actual, expected, ..
            } => {
                children.push((
                    OwnershipRole::ExpressionRoot(ExpressionRootRole::TestActual),
                    OwnerKey::Expression(*actual),
                ));
                children.push((
                    OwnershipRole::ExpressionRoot(ExpressionRootRole::TestExpected),
                    OwnerKey::Expression(*expected),
                ));
            }
        },
        OwnerRecord::Operation(record) => {
            children.extend(record.parameters.iter().map(|parameter| {
                (
                    OwnershipRole::OperationParameter,
                    OwnerKey::Parameter(*parameter),
                )
            }));
        }
        OwnerRecord::Binding(record) => {
            if let Some(value) = record.value {
                children.push((
                    OwnershipRole::ExpressionRoot(ExpressionRootRole::BindingValue),
                    OwnerKey::Expression(value),
                ));
            }
        }
        OwnerRecord::Port(record) => {
            if let PortImplementation::Expression(expression) = record.implementation {
                children.push((
                    OwnershipRole::ExpressionRoot(ExpressionRootRole::PortImplementation),
                    OwnerKey::Expression(expression),
                ));
            }
        }
        OwnerRecord::Expression(record) => {
            children.extend(record.children().into_iter().map(|child| {
                (
                    OwnershipRole::ExpressionChild {
                        role: child.role,
                        ordinal: child.ordinal,
                    },
                    OwnerKey::Expression(child.expression),
                )
            }));
            match &record.operation {
                ExpressionOperation::Let { bindings, .. } => {
                    for (ordinal, binding) in bindings.iter().enumerate() {
                        children.push((
                            OwnershipRole::ExpressionBinding {
                                role: BindingContainerRole::Let,
                                ordinal: summary_ordinal(ordinal)?,
                            },
                            OwnerKey::Binding(*binding),
                        ));
                    }
                }
                ExpressionOperation::Match { arms, .. } => {
                    for (ordinal, arm) in arms.iter().enumerate() {
                        if let Some(binding) = arm.payload_binding {
                            children.push((
                                OwnershipRole::ExpressionBinding {
                                    role: BindingContainerRole::MatchPayload,
                                    ordinal: summary_ordinal(ordinal)?,
                                },
                                OwnerKey::Binding(binding),
                            ));
                        }
                    }
                }
                ExpressionOperation::Transaction { binding, .. } => children.push((
                    OwnershipRole::ExpressionBinding {
                        role: BindingContainerRole::Transaction,
                        ordinal: 0,
                    },
                    OwnerKey::Binding(*binding),
                )),
                _ => {}
            }
        }
        OwnerRecord::Module(_)
        | OwnerRecord::TypeParameter(_)
        | OwnerRecord::Field(_)
        | OwnerRecord::Case(_)
        | OwnerRecord::Parameter(_)
        | OwnerRecord::Requirement(_)
        | OwnerRecord::Target(_)
        | OwnerRecord::Documentation(_)
        | OwnerRecord::Annotation(_) => {}
    }
    children.sort_unstable();
    Ok(children)
}

fn summary_ordinal(ordinal: usize) -> Result<u32, Diagnostic> {
    u32::try_from(ordinal).map_err(|_| {
        witness_error(
            DiagnosticClass::Resource,
            "witness_summary_ordinal",
            "summary child ordinal cannot be represented",
        )
    })
}

fn outgoing_relations(
    package: crate::platform::kernel::PackageId,
    relations: &[RelationEdge],
) -> BTreeMap<OwnerKey, Vec<RelationEdge>> {
    let mut outgoing = BTreeMap::new();
    for edge in relations {
        if let RelationEndpoint::Owner(ExactOwnerKey {
            package: source_package,
            owner,
        }) = edge.source
            && source_package == package
        {
            outgoing.entry(owner).or_insert_with(Vec::new).push(*edge);
        }
    }
    outgoing
}

fn local_summary(
    owner: OwnerKey,
    record: &OwnerRecord,
    outgoing: Option<&[RelationEdge]>,
) -> Result<WorkingSummary, Diagnostic> {
    let (record_digest, _) = crate::platform::kernel::encode_owner(record)?;
    let kind = record.kind();
    let mut interface = Material::new(kind);
    let mut implementation = Material::new(kind);
    let mut types = Material::new(kind);
    let mut effect = Material::new(kind);
    let mut capability = Material::new(kind);
    let mut relation = Material::new(kind);
    let mut presentation = Material::new(kind);
    let mut test = None;

    let type_roots = record.type_roots();
    types.piece(1, &type_roots)?;

    match record {
        OwnerRecord::Module(record) => presentation.piece(1, &record.name)?,
        OwnerRecord::Declaration(record) => {
            presentation.piece(1, &record.name)?;
            interface.piece(1, &record.visibility)?;
            match &record.payload {
                DeclarationPayload::Record { fields } => {
                    interface.raw_piece(2, &[1]);
                    interface.piece(3, fields)?;
                }
                DeclarationPayload::Variant { cases } => {
                    interface.raw_piece(2, &[2]);
                    interface.piece(3, cases)?;
                }
                DeclarationPayload::Interface { operations } => {
                    interface.raw_piece(2, &[3]);
                    interface.piece(3, operations)?;
                }
                DeclarationPayload::External(function) => {
                    interface.raw_piece(2, &[4]);
                    interface.piece(4, &function.type_parameters)?;
                    interface.piece(5, &function.parameters)?;
                    interface.piece(6, &function.result)?;
                    implementation.piece(1, &function.implementation)?;
                }
                DeclarationPayload::Function(function) => {
                    interface.raw_piece(2, &[5]);
                    interface.piece(4, &function.type_parameters)?;
                    interface.piece(5, &function.parameters)?;
                    interface.piece(6, &function.result)?;
                    implementation.piece(2, &function.body)?;
                    effect.piece(1, &function.effect)?;
                    if let FunctionEffect::Task { requirements } = &function.effect {
                        capability.piece(1, requirements)?;
                    }
                }
                DeclarationPayload::Constant { ty, value } => {
                    interface.raw_piece(2, &[6]);
                    interface.piece(6, ty)?;
                    implementation.piece(2, value)?;
                }
                DeclarationPayload::Component {
                    requirements,
                    ports,
                } => {
                    interface.raw_piece(2, &[7]);
                    interface.piece(7, requirements)?;
                    interface.piece(8, ports)?;
                    capability.piece(1, requirements)?;
                }
                DeclarationPayload::Test {
                    actual,
                    expected,
                    comparison,
                } => {
                    interface.raw_piece(2, &[8]);
                    implementation.piece(3, actual)?;
                    implementation.piece(4, expected)?;
                    let mut material = Material::new(kind);
                    material.piece(1, comparison)?;
                    material.piece(2, actual)?;
                    material.piece(3, expected)?;
                    test = Some(material.finish(TEST_DIGEST_DOMAIN));
                }
            }
        }
        OwnerRecord::TypeParameter(record) => presentation.piece(1, &record.name)?,
        OwnerRecord::Field(record) => {
            presentation.piece(1, &record.name)?;
            interface.piece(1, &record.ty)?;
        }
        OwnerRecord::Case(record) => {
            presentation.piece(1, &record.name)?;
            interface.piece(1, &record.payload)?;
        }
        OwnerRecord::Operation(record) => {
            presentation.piece(1, &record.name)?;
            interface.piece(1, &record.parameters)?;
            interface.piece(2, &record.result)?;
            interface.piece(3, &record.idempotency)?;
            interface.piece(4, &record.external_visibility)?;
            effect.piece(1, &record.idempotency)?;
            effect.piece(2, &record.external_visibility)?;
        }
        OwnerRecord::Parameter(record) => {
            presentation.piece(1, &record.name)?;
            interface.piece(1, &record.ty)?;
            interface.piece(2, &record.use_mode)?;
            interface.piece(3, &record.resource_requirement)?;
            capability.piece(1, &record.resource_requirement)?;
        }
        OwnerRecord::Binding(record) => {
            presentation.piece(1, &record.name)?;
            implementation.piece(1, &record.kind)?;
            implementation.piece(2, &record.value)?;
            implementation.piece(3, &record.declared_type)?;
        }
        OwnerRecord::Expression(record) => {
            implementation.piece(1, &record.id)?;
            implementation.piece(2, &record.operation)?;
        }
        OwnerRecord::Requirement(record) => {
            presentation.piece(1, &record.name)?;
            interface.piece(1, &record.interface)?;
            interface.piece(2, &record.operations)?;
            interface.piece(3, &record.limits)?;
            capability.piece(1, &record.interface)?;
            capability.piece(2, &record.operations)?;
            capability.piece(3, &record.limits)?;
        }
        OwnerRecord::Port(record) => {
            presentation.piece(1, &record.name)?;
            interface.piece(1, &record.function_type)?;
            implementation.piece(1, &record.implementation)?;
        }
        OwnerRecord::Target(record) => {
            presentation.piece(1, &record.name)?;
            implementation.piece(1, &record.component)?;
            implementation.piece(2, &record.port)?;
            implementation.piece(3, &record.runner)?;
        }
        OwnerRecord::Documentation(record) => {
            presentation.piece(1, &record.class)?;
            presentation.piece(2, &record.content)?;
            if record.class == DocumentationClass::Semantic {
                implementation.piece(1, &record.class)?;
                implementation.piece(2, &record.content)?;
            }
        }
        OwnerRecord::Annotation(record) => {
            presentation.piece(1, &record.class)?;
            presentation.piece(2, &record.key)?;
            presentation.piece(3, &record.value)?;
            if record.class == AnnotationClass::Semantic {
                implementation.piece(1, &record.class)?;
                implementation.piece(2, &record.key)?;
                implementation.piece(3, &record.value)?;
            }
        }
    }

    for edge in outgoing.into_iter().flatten() {
        if matches!(
            edge.kind.propagation(),
            PropagationClass::Ownership | PropagationClass::Presentation
        ) {
            continue;
        }
        let mut bytes = Vec::new();
        bytes.push(edge.kind.tag());
        encode_endpoint(&mut bytes, edge.target);
        relation.raw_piece(1, &bytes);
    }

    Ok(WorkingSummary {
        owner,
        kind,
        record: record_digest,
        semantic_interface: interface.finish(INTERFACE_DIGEST_DOMAIN),
        implementation: implementation.finish(IMPLEMENTATION_DIGEST_DOMAIN),
        type_digest: types.finish(TYPE_DIGEST_DOMAIN),
        effect: effect.finish(EFFECT_DIGEST_DOMAIN),
        capability: capability.finish(CAPABILITY_DIGEST_DOMAIN),
        relations: relation.finish(RELATION_DIGEST_DOMAIN),
        presentation: presentation.finish(PRESENTATION_DIGEST_DOMAIN),
        validation_dependencies: Material::new(kind).finish(VALIDATION_DEPENDENCY_DIGEST_DOMAIN),
        test_seed: test,
    })
}

fn aggregate_dimension<I>(
    domain: &str,
    local: SemanticDigest,
    children: I,
) -> Result<SemanticDigest, Diagnostic>
where
    I: IntoIterator<Item = (OwnershipRole, OwnerKey, Vec<SemanticDigest>)>,
{
    let mut material = Material::from_digest(local);
    for (role, child, digests) in children {
        material.piece(1, &role)?;
        material.raw_piece(2, &EncodedOwnerKey::new(child).bytes());
        for digest in digests {
            material.digest_piece(3, digest);
        }
    }
    Ok(material.finish(domain))
}

fn combine_digests<const N: usize>(
    domain: &str,
    seed: SemanticDigest,
    digests: [SemanticDigest; N],
) -> SemanticDigest {
    let mut material = Material::from_digest(seed);
    for digest in digests {
        material.digest_piece(1, digest);
    }
    material.finish(domain)
}

struct Material {
    bytes: Vec<u8>,
}

impl Material {
    fn new(kind: OwnerKind) -> Self {
        Self {
            bytes: vec![kind.tag()],
        }
    }

    fn from_digest(digest: SemanticDigest) -> Self {
        let mut material = Self { bytes: Vec::new() };
        material.digest_piece(0, digest);
        material
    }

    fn piece<T: Encode + ?Sized>(&mut self, tag: u8, value: &T) -> Result<(), Diagnostic> {
        let configuration = bincode::config::standard()
            .with_little_endian()
            .with_variable_int_encoding();
        let bytes = bincode::encode_to_vec(value, configuration).map_err(|error| {
            witness_error(
                DiagnosticClass::Infrastructure,
                "witness_summary_encode",
                format!("summary material encoding failed: {error}"),
            )
        })?;
        self.raw_piece(tag, &bytes);
        Ok(())
    }

    fn digest_piece(&mut self, tag: u8, digest: SemanticDigest) {
        self.raw_piece(tag, &digest.bytes());
    }

    fn raw_piece(&mut self, tag: u8, value: &[u8]) {
        self.bytes.push(tag);
        self.bytes
            .extend_from_slice(&(value.len() as u64).to_le_bytes());
        self.bytes.extend_from_slice(value);
    }

    fn finish(self, domain: &str) -> SemanticDigest {
        SemanticDigest::of(domain, &self.bytes)
    }
}
