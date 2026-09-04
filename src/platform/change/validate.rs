//! Owner-frontier structural and semantic validation over an isolated candidate overlay.

use super::{
    CanonicalBaseRead, CanonicalDelta, DerivedDelta, HttpRouteValidationEvidence, ImpactPlan,
    KernelOverlay, SummaryDelta, ValidationAdmission, WitnessBaseRead, WitnessReadWork,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DeclarationPayload, DeclarationReference, ExactOwnerKey, ExpressionRead,
    ExpressionValidationExhaustion, ExpressionValidationLimits, FunctionEffect, OwnerKey,
    OwnerRecord, PackageId, PackageInterfaceDeclarationPayload, PackageInterfaceRecord,
    ParameterParent, ParameterRecord, ParameterUse, PortImplementation, RelationEdge,
    RelationEndpoint, RelationKind, TypeObject, TypeObjectDigest, TypeObjectInterner,
    analyze_http_route_set, http_route_languages_overlap, http_route_strictly_more_specific,
    validate_affine_roots_with_limits, validate_expression_roots_with_limits,
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
    pub http_routes:
        BTreeMap<crate::platform::semantic_id::HttpRouteId, HttpRouteValidationEvidence>,
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
    let http_routes = match validate_http_topology_frontier(
        overlay,
        impact,
        canonical,
        base_witness,
        &mut structural.work,
        admission,
    ) {
        Ok(http_routes) => http_routes,
        Err(diagnostic) => {
            push_bounded_diagnostic(&mut diagnostics, diagnostic, admission)?;
            BTreeMap::new()
        }
    };
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
            http_routes,
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
) -> Result<
    BTreeMap<crate::platform::semantic_id::HttpRouteId, HttpRouteValidationEvidence>,
    Diagnostic,
> {
    use crate::platform::kernel::contract::MAXIMUM_HTTP_ROUTES_PER_TARGET;
    use crate::platform::package::RunnerKind;

    let package = overlay.package_id();
    let mut targets = BTreeSet::new();
    let mut evidence = BTreeMap::new();
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
            if edge.kind == RelationKind::HttpRouteTarget
                && edge.target == target_endpoint
                && !edges.remove(edge)
            {
                return Err(validation_error(
                    DiagnosticClass::Corrupt,
                    "change_validate_http_route_remove",
                    "candidate HTTP route delta removes an absent accepted target relation",
                ));
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

        let mut route_bindings = Vec::with_capacity(edges.len());
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
            if route.target != target {
                return Err(validation_error(
                    DiagnosticClass::Corrupt,
                    "change_validate_http_route_binding",
                    "HTTP route owner and target relation disagree",
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
            let function = match &port.implementation {
                PortImplementation::Function(function) => *function,
                PortImplementation::Expression(_) => {
                    return Err(validation_error(
                        DiagnosticClass::Semantic,
                        "kernel_http_route_port_implementation",
                        "HTTP route port must be function-backed",
                    ));
                }
            };
            let expected_function_type = crate::platform::http::semantic_http_route_function_type(
                &mut TypeObjectInterner::default(),
                route.selector.capture_count(),
            )?;
            if port.function_type != expected_function_type {
                return Err(validation_error(
                    DiagnosticClass::Semantic,
                    "kernel_type_http_route_port",
                    "HTTP route requires the exact semantic HTTP function-backed port shape",
                ));
            }
            let parameters = candidate_function_parameters(overlay, function, work, admission)?;
            validate_capture_parameters(&route.selector, &parameters)?;
            route_bindings.push((route, function, parameters));
        }
        let route_records = route_bindings
            .iter()
            .map(|(route, _, _)| route.clone())
            .collect::<Vec<_>>();
        let analysis = analyze_http_route_set(&route_records)?;
        for (route, handler, parameters) in route_bindings {
            let OwnerKey::HttpRoute(route_id) = route.header.owner else {
                return Err(validation_error(
                    DiagnosticClass::Corrupt,
                    "change_validate_http_route_identity",
                    "validated HTTP route has a foreign owner identity",
                ));
            };
            let mut overlaps = 0u64;
            let mut more_specific = 0u64;
            let mut less_specific = 0u64;
            let mut parameter_ids = Vec::with_capacity(parameters.len());
            for parameter in parameters {
                let OwnerKey::Parameter(parameter) = parameter.header.owner else {
                    return Err(validation_error(
                        DiagnosticClass::Corrupt,
                        "change_validate_http_route_parameter_identity",
                        "validated HTTP handler parameter has a foreign owner identity",
                    ));
                };
                parameter_ids.push(parameter);
            }
            for other in &route_records {
                if route.header.owner == other.header.owner
                    || !http_route_languages_overlap(&route, other)
                {
                    continue;
                }
                overlaps = overlaps.saturating_add(1);
                if http_route_strictly_more_specific(&route, other) {
                    more_specific = more_specific.saturating_add(1);
                } else if http_route_strictly_more_specific(other, &route) {
                    less_specific = less_specific.saturating_add(1);
                }
            }
            if evidence
                .insert(
                    route_id,
                    HttpRouteValidationEvidence {
                        record: route,
                        handler,
                        parameters: parameter_ids,
                        exact_routes: u64::try_from(analysis.exact_routes).unwrap_or(u64::MAX),
                        pattern_routes: u64::try_from(analysis.pattern_routes).unwrap_or(u64::MAX),
                        pattern_segments: u64::try_from(analysis.pattern_segments)
                            .unwrap_or(u64::MAX),
                        maximum_specificity_chain: u64::try_from(
                            analysis.maximum_specificity_chain,
                        )
                        .unwrap_or(u64::MAX),
                        overlaps,
                        more_specific,
                        less_specific,
                    },
                )
                .is_some()
            {
                return Err(validation_error(
                    DiagnosticClass::Corrupt,
                    "change_validate_http_route_identity",
                    "validated HTTP topology repeats one route identity",
                ));
            }
        }
    }
    Ok(evidence)
}

fn candidate_function_parameters<B: CanonicalBaseRead + ?Sized>(
    overlay: &KernelOverlay<'_, B>,
    function: DeclarationReference,
    work: &mut IncrementalValidationWork,
    admission: ValidationAdmission,
) -> Result<Vec<ParameterRecord>, Diagnostic> {
    charge_http_owner(work, admission, "HTTP route backing function")?;
    let parameter_ids = if function.package == overlay.package_id() {
        match overlay.owner(OwnerKey::Declaration(function.declaration))? {
            Some(OwnerRecord::Declaration(record)) => match record.payload {
                DeclarationPayload::Function(function) => function.parameters,
                _ => {
                    return Err(validation_error(
                        DiagnosticClass::Semantic,
                        "kernel_type_http_route_function",
                        "HTTP route port must resolve to a function declaration",
                    ));
                }
            },
            _ => {
                return Err(validation_error(
                    DiagnosticClass::Semantic,
                    "kernel_type_http_route_function",
                    "HTTP route backing function is missing",
                ));
            }
        }
    } else {
        match overlay.package_interface_owner(
            function.package,
            OwnerKey::Declaration(function.declaration),
        )? {
            Some(PackageInterfaceRecord::Declaration(record)) => match record.payload {
                PackageInterfaceDeclarationPayload::Function(function) => function.parameters,
                _ => {
                    return Err(validation_error(
                        DiagnosticClass::Semantic,
                        "kernel_type_http_route_function",
                        "HTTP route port must resolve to a dependency function declaration",
                    ));
                }
            },
            _ => {
                return Err(validation_error(
                    DiagnosticClass::Semantic,
                    "kernel_type_http_route_function",
                    "HTTP route backing dependency function is missing",
                ));
            }
        }
    };
    parameter_ids
        .into_iter()
        .map(|parameter| {
            charge_http_owner(work, admission, "HTTP route function parameter")?;
            let record = if function.package == overlay.package_id() {
                match overlay.owner(OwnerKey::Parameter(parameter))? {
                    Some(OwnerRecord::Parameter(record)) => record,
                    _ => {
                        return Err(validation_error(
                            DiagnosticClass::Semantic,
                            "kernel_type_http_route_parameter",
                            "HTTP route backing function parameter is missing",
                        ));
                    }
                }
            } else {
                match overlay
                    .package_interface_owner(function.package, OwnerKey::Parameter(parameter))?
                {
                    Some(PackageInterfaceRecord::Parameter(record)) => record,
                    _ => {
                        return Err(validation_error(
                            DiagnosticClass::Semantic,
                            "kernel_type_http_route_parameter",
                            "HTTP route dependency parameter is missing",
                        ));
                    }
                }
            };
            if record.parent != ParameterParent::Function(function.declaration) {
                return Err(validation_error(
                    DiagnosticClass::Semantic,
                    "kernel_type_http_route_parameter_parent",
                    "HTTP route parameter belongs to another function",
                ));
            }
            Ok(record)
        })
        .collect()
}

fn validate_capture_parameters(
    selector: &crate::platform::kernel::HttpRouteSelector,
    parameters: &[ParameterRecord],
) -> Result<(), Diagnostic> {
    let captures = selector.capture_names();
    if parameters.len() != captures.len().saturating_add(1) {
        return Err(validation_error(
            DiagnosticClass::Semantic,
            "kernel_type_http_route_parameters",
            "HTTP route backing function parameter count disagrees with its selector",
        ));
    }
    let text_type =
        crate::platform::http::semantic_http_types(&mut TypeObjectInterner::default())?.text_type;
    for (parameter, capture) in parameters.iter().skip(1).zip(captures) {
        if parameter.name.as_str() != capture.as_str()
            || parameter.ty != text_type
            || parameter.use_mode != ParameterUse::Unrestricted
            || parameter.resource_requirement.is_some()
        {
            return Err(validation_error(
                DiagnosticClass::Semantic,
                "kernel_type_http_route_capture_parameter",
                "HTTP route capture must index one same-named unrestricted Text parameter without a resource binding",
            ));
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
