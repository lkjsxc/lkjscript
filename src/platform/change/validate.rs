//! Owner-frontier structural and semantic validation over an isolated candidate overlay.

use super::{
    CanonicalBaseRead, CanonicalDelta, DerivedDelta, ImpactPlan, KernelOverlay, SummaryDelta,
    ValidationAdmission, WitnessBaseRead, WitnessReadWork,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    ExactOwnerKey, ExpressionRead, ExpressionValidationExhaustion, ExpressionValidationLimits,
    OwnerKey, OwnerRecord, PackageId, RelationEdge, RelationEndpoint, RelationKind, TypeObject,
    TypeObjectDigest, validate_affine_roots_with_limits, validate_expression_roots_with_limits,
    validate_http_route_key,
};
use crate::platform::witness::{OwnershipEntry, OwnershipParent, aggregation_children};
use std::collections::{BTreeMap, BTreeSet};

pub const INCREMENTAL_VALIDATION_PROFILE: &str = "incremental_owner_frontier";

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct IncrementalValidationWork {
    pub owner_records_checked: u64,
    pub ownership_entries_checked: u64,
    pub type_objects_checked: u64,
    pub expression_work: u64,
    pub witness_reads: WitnessReadWork,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IncrementalValidationReport {
    pub profile: &'static str,
    pub canonical_owners_changed: u64,
    pub structurally_checked: BTreeSet<OwnerKey>,
    pub semantically_checked: BTreeSet<OwnerKey>,
    pub summaries_reused: u64,
    pub tests_selected: u64,
    pub work: IncrementalValidationWork,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralValidationReport {
    pub structurally_checked: BTreeSet<OwnerKey>,
    pub work: IncrementalValidationWork,
}

pub fn validate_structural_frontier<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    overlay: &KernelOverlay<'_, B>,
    canonical: &CanonicalDelta,
    derived: &DerivedDelta,
    base_witness: &W,
) -> Result<StructuralValidationReport, Vec<Diagnostic>> {
    validate_structural_frontier_with_admission(
        overlay,
        canonical,
        derived,
        base_witness,
        ValidationAdmission::default(),
    )
}

pub(crate) fn validate_structural_frontier_with_admission<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    overlay: &KernelOverlay<'_, B>,
    canonical: &CanonicalDelta,
    derived: &DerivedDelta,
    base_witness: &W,
    admission: ValidationAdmission,
) -> Result<StructuralValidationReport, Vec<Diagnostic>> {
    let mut validator = IncrementalValidator {
        overlay,
        canonical,
        derived,
        base_witness,
        diagnostics: Vec::new(),
        work: IncrementalValidationWork::default(),
        ownership_edits: derived
            .ownership
            .iter()
            .map(|edit| (edit.key, edit.after))
            .collect(),
        ownership_cache: BTreeMap::new(),
        admission,
        budget_error: None,
    };
    validator.validate_structural();
    if let Some(error) = validator.budget_error {
        return Err(vec![error]);
    }
    if validator.diagnostics.is_empty() {
        Ok(StructuralValidationReport {
            structurally_checked: canonical.owners.keys().copied().collect(),
            work: validator.work,
        })
    } else {
        Err(validator.diagnostics)
    }
}

pub fn validate_incremental_frontier<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    overlay: &KernelOverlay<'_, B>,
    canonical: &CanonicalDelta,
    impact: &ImpactPlan,
    summaries: &SummaryDelta,
    base_witness: &W,
    structural: StructuralValidationReport,
) -> Result<IncrementalValidationReport, Vec<Diagnostic>> {
    validate_incremental_frontier_with_admission(
        overlay,
        canonical,
        impact,
        summaries,
        base_witness,
        structural,
        ValidationAdmission::default(),
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "incremental validation binds one exact candidate and its independent admissions"
)]
pub(crate) fn validate_incremental_frontier_with_admission<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
>(
    overlay: &KernelOverlay<'_, B>,
    canonical: &CanonicalDelta,
    impact: &ImpactPlan,
    summaries: &SummaryDelta,
    base_witness: &W,
    mut structural: StructuralValidationReport,
    admission: ValidationAdmission,
) -> Result<IncrementalValidationReport, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut work = 0_usize;
    if let Err(diagnostic) = validate_http_topology_frontier(
        overlay,
        impact,
        canonical,
        base_witness,
        &mut structural.work,
        admission,
    ) {
        push_bounded_diagnostic(&mut diagnostics, diagnostic, admission)?;
    }
    let mut live_semantic_roots = Vec::new();
    for owner in &impact.semantically_checked {
        match overlay.owner(*owner) {
            Ok(Some(_)) => live_semantic_roots.push(*owner),
            Ok(None) => {}
            Err(diagnostic) => push_bounded_diagnostic(&mut diagnostics, diagnostic, admission)?,
        }
    }
    let exhaustion = validate_expression_roots_with_limits(
        overlay,
        live_semantic_roots.iter().copied(),
        &mut diagnostics,
        &mut work,
        ExpressionValidationLimits {
            maximum_steps: usize::try_from(admission.maximum_expression_steps)
                .unwrap_or(usize::MAX),
            maximum_diagnostics: usize::try_from(admission.maximum_diagnostics)
                .unwrap_or(usize::MAX),
        },
    );
    structural.work.expression_work = u64::try_from(work).unwrap_or(u64::MAX);
    if let Err(exhaustion) = exhaustion {
        let (code, message) = match exhaustion {
            ExpressionValidationExhaustion::Steps => (
                "change_budget_validation_expression_steps",
                format!(
                    "semantic validation exceeds the declared {}-expression-step budget",
                    admission.maximum_expression_steps
                ),
            ),
            ExpressionValidationExhaustion::Diagnostics => (
                "change_budget_validation_diagnostics",
                format!(
                    "semantic validation exceeds the declared {}-diagnostic budget",
                    admission.maximum_diagnostics
                ),
            ),
        };
        return Err(vec![validation_budget_error(code, message)]);
    }
    let exhaustion = validate_affine_roots_with_limits(
        overlay,
        live_semantic_roots,
        &mut diagnostics,
        &mut work,
        ExpressionValidationLimits {
            maximum_steps: usize::try_from(admission.maximum_expression_steps)
                .unwrap_or(usize::MAX),
            maximum_diagnostics: usize::try_from(admission.maximum_diagnostics)
                .unwrap_or(usize::MAX),
        },
    );
    structural.work.expression_work = u64::try_from(work).unwrap_or(u64::MAX);
    if let Err(exhaustion) = exhaustion {
        let (code, message) = match exhaustion {
            ExpressionValidationExhaustion::Steps => (
                "change_budget_validation_affine_steps",
                format!(
                    "affine validation exceeds the declared {}-expression-step budget",
                    admission.maximum_expression_steps
                ),
            ),
            ExpressionValidationExhaustion::Diagnostics => (
                "change_budget_validation_diagnostics",
                format!(
                    "semantic validation exceeds the declared {}-diagnostic budget",
                    admission.maximum_diagnostics
                ),
            ),
        };
        return Err(vec![validation_budget_error(code, message)]);
    }
    if summaries.selected != impact.summary_owners {
        push_bounded_diagnostic(
            &mut diagnostics,
            validation_error(
                DiagnosticClass::Corrupt,
                "change_validate_summary_selection",
                "validation summary selection disagrees with the exact impact plan",
            ),
            admission,
        )?;
    }
    let summaries_reused = base_witness
        .owner_summary_count()
        .checked_sub(summaries.base_summaries_selected)
        .ok_or_else(|| {
            vec![validation_error(
                DiagnosticClass::Corrupt,
                "change_validate_summary_count",
                "selected base summaries exceed the committed witness summary count",
            )]
        })?;
    if diagnostics.is_empty() {
        Ok(IncrementalValidationReport {
            profile: INCREMENTAL_VALIDATION_PROFILE,
            canonical_owners_changed: canonical.owners.len() as u64,
            structurally_checked: structural.structurally_checked,
            semantically_checked: impact.semantically_checked.clone(),
            summaries_reused,
            tests_selected: impact.tests.len() as u64,
            work: structural.work,
        })
    } else {
        Err(diagnostics)
    }
}

fn validate_http_topology_frontier<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    overlay: &KernelOverlay<'_, B>,
    impact: &ImpactPlan,
    canonical: &CanonicalDelta,
    base_witness: &W,
    work: &mut IncrementalValidationWork,
    admission: ValidationAdmission,
) -> Result<(), Diagnostic> {
    use crate::platform::kernel::PortImplementation;
    use crate::platform::kernel::contract::{
        MAXIMUM_HTTP_ROUTE_KEY_BYTES_PER_TARGET, MAXIMUM_HTTP_ROUTES_PER_TARGET,
    };
    use crate::platform::package::RunnerKind;

    let package = overlay.package_id();
    let mut targets = BTreeSet::new();
    let removed_route_edges = candidate_http_route_relation_removals(canonical, overlay)?;
    let added_route_edges = canonical_http_route_relation_additions(canonical, overlay)?;
    for owner in &impact.semantically_checked {
        match owner {
            OwnerKey::Target(target) => {
                targets.insert(*target);
            }
            OwnerKey::HttpRoute(_) => {
                if let Some(OwnerRecord::HttpRoute(route)) = overlay.owner(*owner)? {
                    targets.insert(route.target);
                }
            }
            _ => {}
        }
    }
    for edge in removed_route_edges.iter().chain(&added_route_edges) {
        let RelationEndpoint::Owner(ExactOwnerKey {
            package: target_package,
            owner: OwnerKey::Target(target),
        }) = edge.target
        else {
            return Err(validation_error(
                DiagnosticClass::Corrupt,
                "change_validate_http_route_target_relation",
                "HTTP route target relation has a foreign target endpoint",
            ));
        };
        if target_package != package {
            return Err(validation_error(
                DiagnosticClass::Corrupt,
                "change_validate_http_route_target_package",
                "HTTP route target relation escaped the root package",
            ));
        }
        targets.insert(target);
    }

    let http_function_type = crate::platform::http::semantic_http_types(
        &mut crate::platform::kernel::TypeObjectInterner::default(),
    )?
    .function_type;
    for target in targets {
        charge_http_owner(work, admission, "HTTP target topology")?;
        let target_owner = OwnerKey::Target(target);
        let candidate_target = overlay.owner(target_owner)?;
        let base_read = base_witness.read_incoming_relations_of_kind(
            target_owner,
            RelationKind::HttpRouteTarget,
            MAXIMUM_HTTP_ROUTES_PER_TARGET.saturating_add(1),
        )?;
        work.witness_reads.add(base_read.work);
        if base_read.value.truncated {
            return Err(validation_error(
                DiagnosticClass::Corrupt,
                "change_validate_http_route_base_count",
                "accepted HTTP route relations exceed the graph contract bound",
            ));
        }
        let target_endpoint = RelationEndpoint::Owner(ExactOwnerKey {
            package,
            owner: target_owner,
        });
        let mut edges = base_read.value.edges.into_iter().collect::<BTreeSet<_>>();
        for edge in &removed_route_edges {
            if edge.kind == RelationKind::HttpRouteTarget && edge.target == target_endpoint {
                if !edges.remove(edge) {
                    return Err(validation_error(
                        DiagnosticClass::Corrupt,
                        "change_validate_http_route_remove",
                        "candidate HTTP route delta removes an absent accepted target relation",
                    ));
                }
            }
        }
        for edge in &added_route_edges {
            if edge.kind == RelationKind::HttpRouteTarget && edge.target == target_endpoint {
                edges.insert(*edge);
            }
        }

        let Some(OwnerRecord::Target(target_record)) = candidate_target else {
            if !edges.is_empty() {
                return Err(validation_error(
                    DiagnosticClass::Semantic,
                    "kernel_http_route_target_missing",
                    "a live HTTP route relation names a deleted or foreign target",
                ));
            }
            continue;
        };
        match target_record.runner {
            RunnerKind::Http => {
                if target_record.port.is_some() {
                    return Err(validation_error(
                        DiagnosticClass::Semantic,
                        "kernel_http_target_universal_port",
                        "HTTP target must not retain a universal port",
                    ));
                }
                if edges.is_empty() || edges.len() > MAXIMUM_HTTP_ROUTES_PER_TARGET {
                    return Err(validation_error(
                        DiagnosticClass::Semantic,
                        "kernel_http_target_route_count",
                        format!(
                            "HTTP target must own 1 through {MAXIMUM_HTTP_ROUTES_PER_TARGET} routes"
                        ),
                    ));
                }
            }
            _ => {
                if target_record.port.is_none() {
                    return Err(validation_error(
                        DiagnosticClass::Semantic,
                        "kernel_target_port_missing",
                        "non-HTTP target must select one exact port",
                    ));
                }
                if !edges.is_empty() {
                    return Err(validation_error(
                        DiagnosticClass::Semantic,
                        "kernel_http_route_non_http_target",
                        "HTTP routes may belong only to an HTTP target",
                    ));
                }
                continue;
            }
        }

        let mut keys = BTreeSet::new();
        let mut aggregate = 0usize;
        for edge in edges {
            if edge.kind != RelationKind::HttpRouteTarget || edge.target != target_endpoint {
                return Err(validation_error(
                    DiagnosticClass::Corrupt,
                    "change_validate_http_route_relation",
                    "candidate HTTP route read returned an unrelated relation",
                ));
            }
            let RelationEndpoint::Owner(ExactOwnerKey {
                package: route_package,
                owner: route_owner @ OwnerKey::HttpRoute(_),
            }) = edge.source
            else {
                return Err(validation_error(
                    DiagnosticClass::Corrupt,
                    "change_validate_http_route_source",
                    "HTTP route target relation has a foreign source endpoint",
                ));
            };
            if route_package != package {
                return Err(validation_error(
                    DiagnosticClass::Corrupt,
                    "change_validate_http_route_source_package",
                    "HTTP route target relation has a foreign source package",
                ));
            }
            charge_http_owner(work, admission, "HTTP route topology")?;
            let Some(OwnerRecord::HttpRoute(route)) = overlay.owner(route_owner)? else {
                return Err(validation_error(
                    DiagnosticClass::Corrupt,
                    "change_validate_http_route_owner",
                    "HTTP route target relation names a missing or foreign owner",
                ));
            };
            validate_http_route_key(&route.method, &route.path)?;
            if route.target != target {
                return Err(validation_error(
                    DiagnosticClass::Corrupt,
                    "change_validate_http_route_binding",
                    "HTTP route owner and target relation disagree",
                ));
            }
            if !keys.insert((route.method.clone(), route.path.clone())) {
                return Err(validation_error(
                    DiagnosticClass::Semantic,
                    "kernel_http_route_duplicate",
                    "HTTP target contains a duplicate exact method/path pair",
                ));
            }
            aggregate = aggregate
                .checked_add(route.method.len())
                .and_then(|value| value.checked_add(route.path.len()))
                .ok_or_else(|| {
                    validation_error(
                        DiagnosticClass::Semantic,
                        "kernel_http_target_route_bytes",
                        "HTTP target route-key byte count overflowed",
                    )
                })?;
            if aggregate > MAXIMUM_HTTP_ROUTE_KEY_BYTES_PER_TARGET {
                return Err(validation_error(
                    DiagnosticClass::Semantic,
                    "kernel_http_target_route_bytes",
                    format!(
                        "HTTP target route keys exceed {MAXIMUM_HTTP_ROUTE_KEY_BYTES_PER_TARGET} bytes"
                    ),
                ));
            }
            if route.port.package != package {
                return Err(validation_error(
                    DiagnosticClass::Semantic,
                    "kernel_http_route_port_package",
                    "HTTP route port must belong to the root package",
                ));
            }
            charge_http_owner(work, admission, "HTTP route port topology")?;
            let Some(OwnerRecord::Port(port)) = overlay.owner(OwnerKey::Port(route.port.port))?
            else {
                return Err(validation_error(
                    DiagnosticClass::Semantic,
                    "kernel_http_route_port_missing",
                    "HTTP route references a missing or foreign port",
                ));
            };
            if port.declaration != target_record.component.declaration {
                return Err(validation_error(
                    DiagnosticClass::Semantic,
                    "kernel_http_route_port_owner",
                    "HTTP route port does not belong to its target component",
                ));
            }
            if !matches!(port.implementation, PortImplementation::Function(_)) {
                return Err(validation_error(
                    DiagnosticClass::Semantic,
                    "kernel_http_route_port_implementation",
                    "HTTP route port must be function-backed",
                ));
            }
            if port.function_type != http_function_type {
                return Err(validation_error(
                    DiagnosticClass::Semantic,
                    "kernel_type_http_route_port",
                    "HTTP route requires the exact semantic HTTP function-backed port shape",
                ));
            }
        }
    }
    Ok(())
}

fn charge_http_owner(
    work: &mut IncrementalValidationWork,
    admission: ValidationAdmission,
    label: &str,
) -> Result<(), Diagnostic> {
    if work.owner_records_checked >= admission.maximum_owner_records {
        return Err(validation_budget_error(
            "change_budget_validation_owner_records",
            format!(
                "{label} exceeds the declared {}-owner-record validation budget",
                admission.maximum_owner_records
            ),
        ));
    }
    work.owner_records_checked = work.owner_records_checked.saturating_add(1);
    Ok(())
}

fn candidate_http_route_relation_removals<B: CanonicalBaseRead + ?Sized>(
    canonical: &CanonicalDelta,
    overlay: &KernelOverlay<'_, B>,
) -> Result<Vec<RelationEdge>, Diagnostic> {
    let mut edges = Vec::new();
    for (owner, edit) in &canonical.owners {
        if !matches!(owner, OwnerKey::HttpRoute(_)) || edit.before.is_none() {
            continue;
        }
        if let Some(OwnerRecord::HttpRoute(route)) = overlay.base_owner(*owner)? {
            edges.push(http_route_target_edge(
                overlay.package_id(),
                *owner,
                route.target,
            ));
        }
    }
    Ok(edges)
}

fn canonical_http_route_relation_additions<B: CanonicalBaseRead + ?Sized>(
    canonical: &CanonicalDelta,
    overlay: &KernelOverlay<'_, B>,
) -> Result<Vec<RelationEdge>, Diagnostic> {
    let mut edges = Vec::new();
    for (owner, edit) in &canonical.owners {
        let Some((_, OwnerRecord::HttpRoute(route))) = &edit.after else {
            continue;
        };
        edges.push(http_route_target_edge(
            overlay.package_id(),
            *owner,
            route.target,
        ));
    }
    Ok(edges)
}

fn http_route_target_edge(
    package: PackageId,
    route: OwnerKey,
    target: crate::platform::semantic_id::TargetId,
) -> RelationEdge {
    RelationEdge {
        source: RelationEndpoint::Owner(ExactOwnerKey {
            package,
            owner: route,
        }),
        kind: RelationKind::HttpRouteTarget,
        target: RelationEndpoint::Owner(ExactOwnerKey {
            package,
            owner: OwnerKey::Target(target),
        }),
    }
}

struct IncrementalValidator<'a, B: ?Sized, W: ?Sized> {
    overlay: &'a KernelOverlay<'a, B>,
    canonical: &'a CanonicalDelta,
    derived: &'a DerivedDelta,
    base_witness: &'a W,
    diagnostics: Vec<Diagnostic>,
    work: IncrementalValidationWork,
    ownership_edits: BTreeMap<OwnerKey, Option<OwnershipEntry>>,
    ownership_cache: BTreeMap<OwnerKey, Option<OwnershipEntry>>,
    admission: ValidationAdmission,
    budget_error: Option<Diagnostic>,
}

impl<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized> IncrementalValidator<'_, B, W> {
    fn validate_structural(&mut self) {
        self.validate_changed_records();
        if self.budget_error.is_some() {
            return;
        }
        self.validate_ownership_frontier();
        if self.budget_error.is_some() {
            return;
        }
        self.validate_type_frontier();
    }

    fn validate_changed_records(&mut self) {
        for (owner, edit) in &self.canonical.owners {
            if self.budget_error.is_some() {
                return;
            }
            if !self.charge_owner_record() {
                return;
            }
            if let Some((_, record)) = &edit.after {
                self.capture(record.validate_local());
                if self.budget_error.is_some() {
                    return;
                }
                if record.owner() != *owner {
                    self.error(
                        "change_validate_owner_key",
                        format!("candidate owner key {owner:?} disagrees with its record"),
                    );
                    if self.budget_error.is_some() {
                        return;
                    }
                }
                self.capture(crate::platform::kernel::encode_owner(record).map(|_| ()));
            }
        }
        for (package, edit) in &self.canonical.dependencies {
            if self.budget_error.is_some() {
                return;
            }
            if let Some((_, dependency)) = &edit.after {
                self.capture(dependency.validate_local());
                if self.budget_error.is_some() {
                    return;
                }
                if dependency.package != *package || dependency.package == self.overlay.package_id()
                {
                    self.error(
                        "change_validate_dependency_key",
                        "candidate dependency key is foreign or self-referential",
                    );
                }
            }
        }
        for (owner, edit) in &self.canonical.retirements {
            if self.budget_error.is_some() {
                return;
            }
            if let Some((_, retirement)) = &edit.after {
                self.capture(retirement.validate_local());
                if self.budget_error.is_some() {
                    return;
                }
                let remains_live = match self.overlay.owner(*owner) {
                    Ok(record) => record.is_some(),
                    Err(diagnostic) => {
                        self.push_diagnostic(diagnostic);
                        continue;
                    }
                };
                if retirement.owner != *owner || remains_live {
                    self.error(
                        "change_validate_retirement_key",
                        "candidate retirement key is foreign or remains live",
                    );
                }
            }
        }
    }

    fn validate_ownership_frontier(&mut self) {
        let mut parents = BTreeSet::new();
        for edit in &self.derived.ownership {
            for entry in [edit.before, edit.after].into_iter().flatten() {
                if let OwnershipParent::Owner(parent) = entry.parent {
                    parents.insert(parent);
                }
            }
        }
        let changed_owners = self.canonical.owners.keys().copied().collect::<Vec<_>>();
        for owner in &changed_owners {
            if self.budget_error.is_some() {
                return;
            }
            if !self.charge_ownership_entry() {
                return;
            }
            let candidate_ownership = match self.ownership(*owner) {
                Ok(entry) => entry,
                Err(diagnostic) => {
                    self.push_diagnostic(diagnostic);
                    continue;
                }
            };
            let candidate_record = match self.overlay.owner(*owner) {
                Ok(record) => record,
                Err(diagnostic) => {
                    self.push_diagnostic(diagnostic);
                    continue;
                }
            };
            match candidate_record {
                Some(_) if candidate_ownership.is_none() => self.error(
                    "change_validate_ownership_missing",
                    format!("live candidate owner {owner:?} has no ownership witness"),
                ),
                None if candidate_ownership.is_some() => self.error(
                    "change_validate_ownership_stale",
                    format!("deleted candidate owner {owner:?} retains ownership witness"),
                ),
                Some(_) | None => {}
            }
        }
        for parent in parents {
            if self.budget_error.is_some() {
                return;
            }
            let record = match self.overlay.owner(parent) {
                Ok(Some(record)) => record,
                Ok(None) => continue,
                Err(diagnostic) => {
                    self.push_diagnostic(diagnostic);
                    continue;
                }
            };
            let children = match aggregation_children(&record) {
                Ok(children) => children,
                Err(diagnostic) => {
                    self.push_diagnostic(diagnostic);
                    continue;
                }
            };
            for (role, child) in children {
                if self.budget_error.is_some() {
                    return;
                }
                if !role.aggregates_into_parent() {
                    continue;
                }
                if !self.charge_ownership_entry() {
                    return;
                }
                let expected = OwnershipEntry::new(OwnershipParent::Owner(parent), role);
                match self.ownership(child) {
                    Ok(Some(actual)) if actual == expected => {}
                    Ok(_) => self.error(
                        "change_validate_ownership_child",
                        format!("candidate parent {parent:?} and child {child:?} disagree"),
                    ),
                    Err(diagnostic) => self.push_diagnostic(diagnostic),
                }
            }
        }
        let mut parent_entries = Vec::new();
        for owner in changed_owners {
            if self.budget_error.is_some() {
                return;
            }
            match self.ownership(owner) {
                Ok(Some(entry)) => parent_entries.push((owner, entry)),
                Ok(None) => {}
                Err(diagnostic) => self.push_diagnostic(diagnostic),
            }
        }
        for (owner, entry) in parent_entries {
            if self.budget_error.is_some() {
                return;
            }
            if let OwnershipParent::Owner(parent) = entry.parent {
                match self.overlay.owner(parent) {
                    Ok(Some(_)) => {}
                    Ok(None) => self.error(
                        "change_validate_parent_missing",
                        format!("candidate owner {owner:?} has missing parent {parent:?}"),
                    ),
                    Err(diagnostic) => self.push_diagnostic(diagnostic),
                }
            }
        }
    }

    fn validate_type_frontier(&mut self) {
        let mut pending = self
            .canonical
            .owners
            .values()
            .filter_map(|edit| edit.after.as_ref())
            .flat_map(|(_, record)| record.type_roots())
            .map(|digest| (digest, 0_usize))
            .collect::<Vec<_>>();
        let mut visited = BTreeSet::new();
        while let Some((digest, depth)) = pending.pop() {
            if self.budget_error.is_some() {
                return;
            }
            if !visited.insert(digest) {
                continue;
            }
            if !self.charge_type_object() {
                return;
            }
            if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
                self.error(
                    "change_validate_type_depth",
                    "candidate type closure exceeds the hostile-input depth bound",
                );
                continue;
            }
            let object = match self.overlay.type_object(digest) {
                Ok(Some(object)) => object,
                Ok(None) => {
                    self.error(
                        "change_validate_type_missing",
                        format!("candidate type object {digest} is missing"),
                    );
                    continue;
                }
                Err(diagnostic) => {
                    self.push_diagnostic(diagnostic);
                    continue;
                }
            };
            self.capture(object.validate_local());
            if self.budget_error.is_some() {
                return;
            }
            match crate::platform::kernel::encode_type_object(&object) {
                Ok((actual, _)) if actual == digest => {}
                Ok(_) => self.error(
                    "change_validate_type_digest",
                    "candidate type object is bound under a foreign digest",
                ),
                Err(diagnostic) => self.push_diagnostic(diagnostic),
            }
            pending.extend(
                object
                    .child_types()
                    .into_iter()
                    .map(|child| (child, depth.saturating_add(1))),
            );
        }
        for digest in self.canonical.type_additions.keys() {
            if self.budget_error.is_some() {
                return;
            }
            if !visited.contains(digest) {
                self.error(
                    "change_validate_type_unreachable",
                    format!("new type object {digest} is unreachable from changed meaning"),
                );
            }
        }
    }

    fn ownership(&mut self, owner: OwnerKey) -> Result<Option<OwnershipEntry>, Diagnostic> {
        match self.ownership_edits.get(&owner) {
            Some(entry) => Ok(*entry),
            None => {
                if let Some(cached) = self.ownership_cache.get(&owner) {
                    return Ok(*cached);
                }
                let read = self.base_witness.read_ownership(owner)?;
                self.work.witness_reads.add(read.work);
                self.ownership_cache.insert(owner, read.value);
                Ok(read.value)
            }
        }
    }

    fn capture(&mut self, result: Result<(), Diagnostic>) {
        if let Err(diagnostic) = result {
            self.push_diagnostic(diagnostic);
        }
    }

    fn error(&mut self, code: &'static str, message: impl Into<String>) {
        self.push_diagnostic(Diagnostic::new(DiagnosticClass::Semantic, code, message));
    }

    fn charge_owner_record(&mut self) -> bool {
        if self.work.owner_records_checked >= self.admission.maximum_owner_records {
            self.budget_error = Some(validation_budget_error(
                "change_budget_validation_owner_records",
                format!(
                    "structural validation exceeds the declared {}-owner-record budget",
                    self.admission.maximum_owner_records
                ),
            ));
            return false;
        }
        self.work.owner_records_checked = self.work.owner_records_checked.saturating_add(1);
        true
    }

    fn charge_ownership_entry(&mut self) -> bool {
        if self.work.ownership_entries_checked >= self.admission.maximum_ownership_entries {
            self.budget_error = Some(validation_budget_error(
                "change_budget_validation_ownership_entries",
                format!(
                    "structural validation exceeds the declared {}-ownership-entry budget",
                    self.admission.maximum_ownership_entries
                ),
            ));
            return false;
        }
        self.work.ownership_entries_checked = self.work.ownership_entries_checked.saturating_add(1);
        true
    }

    fn charge_type_object(&mut self) -> bool {
        if self.work.type_objects_checked >= self.admission.maximum_type_objects {
            self.budget_error = Some(validation_budget_error(
                "change_budget_validation_type_objects",
                format!(
                    "structural validation exceeds the declared {}-type-object budget",
                    self.admission.maximum_type_objects
                ),
            ));
            return false;
        }
        self.work.type_objects_checked = self.work.type_objects_checked.saturating_add(1);
        true
    }

    fn push_diagnostic(&mut self, diagnostic: Diagnostic) {
        if u64::try_from(self.diagnostics.len()).unwrap_or(u64::MAX)
            >= self.admission.maximum_diagnostics
        {
            self.budget_error = Some(validation_budget_error(
                "change_budget_validation_diagnostics",
                format!(
                    "structural validation exceeds the declared {}-diagnostic budget",
                    self.admission.maximum_diagnostics
                ),
            ));
            return;
        }
        self.diagnostics.push(diagnostic);
    }
}

fn validation_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

fn push_bounded_diagnostic(
    diagnostics: &mut Vec<Diagnostic>,
    diagnostic: Diagnostic,
    admission: ValidationAdmission,
) -> Result<(), Vec<Diagnostic>> {
    if u64::try_from(diagnostics.len()).unwrap_or(u64::MAX) >= admission.maximum_diagnostics {
        return Err(vec![validation_budget_error(
            "change_budget_validation_diagnostics",
            format!(
                "semantic validation exceeds the declared {}-diagnostic budget",
                admission.maximum_diagnostics
            ),
        )]);
    }
    diagnostics.push(diagnostic);
    Ok(())
}

fn validation_budget_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Resource, code, message)
}

impl<B: CanonicalBaseRead + ?Sized> ExpressionRead for KernelOverlay<'_, B> {
    fn package_id(&self) -> PackageId {
        KernelOverlay::package_id(self)
    }

    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        KernelOverlay::owner(self, owner)
    }

    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        KernelOverlay::type_object(self, digest)
    }

    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<crate::platform::kernel::PackageInterfaceRecord>, Diagnostic> {
        KernelOverlay::package_interface_owner(self, package, owner)
    }

    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        Ok(self.dependency(package)?.is_some())
    }
}
