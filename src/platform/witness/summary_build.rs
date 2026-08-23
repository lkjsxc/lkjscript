//! Owner-local summary dimensions and declaration-sized descendant aggregation.

use super::contract::{
    CAPABILITY_DIGEST_DOMAIN, EFFECT_DIGEST_DOMAIN, IMPLEMENTATION_DIGEST_DOMAIN,
    INTERFACE_DIGEST_DOMAIN, PRESENTATION_DIGEST_DOMAIN, RELATION_DIGEST_DOMAIN,
    TEST_DIGEST_DOMAIN, TYPE_DIGEST_DOMAIN, VALIDATION_DEPENDENCY_DIGEST_DOMAIN,
};
use super::entry::{OwnershipEntry, OwnershipParent, OwnershipRole, encode_endpoint};
use super::summary::{OwnerSummary, current_summary_contract};
use super::{SemanticDigest, witness_error};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    AnnotationClass, DeclarationPayload, DocumentationClass, EncodedOwnerKey, ExactOwnerKey,
    FunctionEffect, KernelSnapshot, OwnerKey, OwnerKind, OwnerRecord, PropagationClass,
    RelationEdge, RelationEndpoint,
};
use bincode::Encode;
use std::collections::BTreeMap;

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
                            .cloned()
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
    }
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
        let mut material = Material::new(summary.kind);
        for edge in outgoing.get(owner).into_iter().flatten() {
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
                    let target = frozen.get(&target_owner).ok_or_else(|| {
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
                        let dependency =
                            snapshot.dependencies.get(&target_package).ok_or_else(|| {
                                witness_error(
                                    DiagnosticClass::Corrupt,
                                    "witness_dependency_target",
                                    "foreign relation target has no exact dependency binding",
                                )
                            })?;
                        let (digest, _) = crate::platform::kernel::encode_dependency(dependency)?;
                        material.piece(2, &digest)?;
                    }
                }
            }
        }
        summary.validation_dependencies = material.finish(VALIDATION_DEPENDENCY_DIGEST_DOMAIN);
    }
    Ok(())
}

fn append_dependency_dimensions(
    material: &mut Material,
    propagation: PropagationClass,
    target: &WorkingSummary,
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
        let selected = children
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
        if selected.is_empty() {
            continue;
        }
        let parent = summaries.get_mut(&owner).ok_or_else(|| {
            witness_error(
                DiagnosticClass::Corrupt,
                "witness_validation_parent",
                "validation aggregation parent has no summary",
            )
        })?;
        parent.validation_dependencies = aggregate_dimension(
            VALIDATION_DEPENDENCY_DIGEST_DOMAIN,
            parent.validation_dependencies,
            selected
                .into_iter()
                .map(|(role, child, digest)| (role, child, vec![digest])),
        )?;
    }
    Ok(())
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
