//! Implementation-disjoint full validator for normalized Graph 10 authority.

use super::affine::validate_affine_meaning;
use super::contract::{
    MAXIMUM_EXPRESSION_DEPTH, MAXIMUM_HTTP_ROUTES_PER_TARGET, MAXIMUM_TYPE_DEPTH,
    MAXIMUM_VALIDATION_WORK,
};
use super::expression::{ExpressionOperation, FieldSelector, LocalValueReference, TextValue};
use super::id::{OwnerKey, OwnerKind, PackageId};
use super::infer::validate_expression_meaning;
use super::owner::{
    BindingKind, DeclarationPayload, FunctionEffect, OwnerRecord, ParameterParent, ParameterRecord,
    ParameterUse, PortImplementation, PortRecord,
};
use super::owner_namespace;
use super::relation::{RelationEdge, extract_relations};
use super::root::{DependencyRecord, RetirementRecord, SemanticRoot};
use super::type_object::{TypeForm, TypeObject};
use super::{
    BlobObjectDigest, DeclarationReference, HttpRouteRecord, PackageInterfaceDeclarationPayload,
    PackageInterfaceRecord, PackageRevisionDigest, ReadScopeArgument, ReadScopeBinder,
    ReadViewFieldKind, RequirementRecord, RequirementReference, ResultRelation, TypeObjectDigest,
    TypeObjectInterner, encode_type_object, requirement_is_covered_by,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{BindingId, ExpressionId, TargetId};
use std::collections::{BTreeMap, BTreeSet};

/// Exact logical authority supplied to the full oracle. Object-store and persistent-map integrity
/// are checked by the repository layer; this view contains only the records reachable from the
/// candidate semantic root.
#[derive(Clone, Debug)]
pub struct KernelSnapshot {
    pub root: SemanticRoot,
    pub owners: BTreeMap<OwnerKey, OwnerRecord>,
    pub types: BTreeMap<TypeObjectDigest, TypeObject>,
    /// Derived exact interfaces for bound dependency package revisions. These are oracle inputs,
    /// not fields of the local semantic root.
    pub dependency_interfaces: BTreeMap<PackageRevisionDigest, DependencyInterface>,
    /// Structural types reachable from the bound dependency interfaces.
    pub dependency_types: BTreeMap<TypeObjectDigest, TypeObject>,
    /// Exact selections reachable through admitted imports, used only for private
    /// representation proof. They do not grant authored-reference authority.
    pub representation_dependencies: BTreeMap<PackageId, DependencyRecord>,
    pub blobs: BTreeMap<BlobObjectDigest, u64>,
    pub dependencies: BTreeMap<PackageId, DependencyRecord>,
    pub retirements: BTreeMap<OwnerKey, RetirementRecord>,
}

/// Immutable derived records for one exact dependency revision. Not an admission certificate.
pub type DependencyInterface = std::sync::Arc<BTreeMap<OwnerKey, PackageInterfaceRecord>>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FullValidationReport {
    pub owners_checked: u64,
    pub type_objects_checked: u64,
    pub expression_records_checked: u64,
    pub relation_edges: u64,
    pub dependencies_checked: u64,
    pub retirements_checked: u64,
    pub work_consumed: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BindingContainerKind {
    Let,
    MatchPayload,
    Transaction,
    OwnedUnpack,
    OwnedChoicePayload,
    OwnedBorrow,
    ReadView,
    ReadViewLet,
    ReadViewBorrow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct BindingContainer {
    expression: ExpressionId,
    scope_roots: Vec<ExpressionId>,
    kind: BindingContainerKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ExpressionParent {
    Root(OwnerKey),
    Expression(ExpressionId),
}

pub fn validate_full(snapshot: &KernelSnapshot) -> Result<FullValidationReport, Vec<Diagnostic>> {
    validate_full_with_limit(snapshot, MAXIMUM_VALIDATION_WORK)
}

pub(crate) fn validate_full_with_limit(
    snapshot: &KernelSnapshot,
    maximum_work: usize,
) -> Result<FullValidationReport, Vec<Diagnostic>> {
    validate_full_observed(snapshot, maximum_work, &mut 0)
}

pub(crate) fn validate_full_observed(
    snapshot: &KernelSnapshot,
    maximum_work: usize,
    consumed: &mut u64,
) -> Result<FullValidationReport, Vec<Diagnostic>> {
    validate_full_checked(snapshot, maximum_work, consumed, &|| Ok(()))
}

pub(crate) fn validate_full_checked(
    snapshot: &KernelSnapshot,
    maximum_work: usize,
    consumed: &mut u64,
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
) -> Result<FullValidationReport, Vec<Diagnostic>> {
    checkpoint().map_err(|error| vec![error])?;
    let mut validator = FullValidator {
        snapshot,
        read: super::infer::CheckedExpressionRead {
            read: snapshot,
            checkpoint,
        },
        interrupted: false,
        diagnostics: Vec::new(),
        work: 0,
        maximum_work: maximum_work.min(MAXIMUM_VALIDATION_WORK),
        associated_admission: super::owned_associated::OwnedAssociatedAdmission::new(),
        relations: Vec::new(),
        expression_parents: BTreeMap::new(),
        expression_root_owners: BTreeMap::new(),
        binding_containers: BTreeMap::new(),
    };
    validator.validate();
    *consumed = validator.work as u64;
    validator
        .diagnostics
        .sort_by(|left, right| (&left.code, &left.message).cmp(&(&right.code, &right.message)));
    validator.diagnostics.dedup();
    if validator.diagnostics.is_empty() {
        Ok(FullValidationReport {
            owners_checked: snapshot.owners.len() as u64,
            type_objects_checked: snapshot.types.len() as u64,
            expression_records_checked: snapshot
                .owners
                .values()
                .filter(|record| matches!(record, OwnerRecord::Expression(_)))
                .count() as u64,
            relation_edges: validator.relations.len() as u64,
            dependencies_checked: snapshot.dependencies.len() as u64,
            retirements_checked: snapshot.retirements.len() as u64,
            work_consumed: validator.work as u64,
        })
    } else {
        Err(validator.diagnostics)
    }
}

struct FullValidator<'a> {
    snapshot: &'a KernelSnapshot,
    read: super::infer::CheckedExpressionRead<'a, KernelSnapshot>,
    interrupted: bool,
    diagnostics: Vec<Diagnostic>,
    work: usize,
    maximum_work: usize,
    associated_admission: super::owned_associated::OwnedAssociatedAdmission,
    relations: Vec<RelationEdge>,
    expression_parents: BTreeMap<ExpressionId, Vec<ExpressionParent>>,
    expression_root_owners: BTreeMap<ExpressionId, OwnerKey>,
    binding_containers: BTreeMap<BindingId, Vec<BindingContainer>>,
}

fn snapshot_type_is_direct_resource(snapshot: &KernelSnapshot, digest: TypeObjectDigest) -> bool {
    snapshot
        .types
        .get(&digest)
        .or_else(|| snapshot.dependency_types.get(&digest))
        .is_some_and(|object| matches!(object.form, TypeForm::CapabilityResource { .. }))
}

fn snapshot_resource_interface(
    snapshot: &KernelSnapshot,
    digest: TypeObjectDigest,
) -> Option<super::DeclarationReference> {
    fn visit(
        snapshot: &KernelSnapshot,
        digest: TypeObjectDigest,
        observed: &mut BTreeSet<TypeObjectDigest>,
        interfaces: &mut BTreeSet<super::DeclarationReference>,
    ) {
        if !observed.insert(digest) {
            return;
        }
        let Some(form) = snapshot
            .types
            .get(&digest)
            .or_else(|| snapshot.dependency_types.get(&digest))
            .map(|object| &object.form)
        else {
            return;
        };
        match form {
            TypeForm::CapabilityResource { interface } => {
                interfaces.insert(*interface);
            }
            TypeForm::Named { declaration } => {
                for child in snapshot_named_member_types(snapshot, *declaration) {
                    visit(snapshot, child, observed, interfaces);
                }
            }
            _ => {
                for child in object_child_digests(form) {
                    visit(snapshot, child, observed, interfaces);
                }
            }
        }
    }

    let mut interfaces = BTreeSet::new();
    visit(snapshot, digest, &mut BTreeSet::new(), &mut interfaces);
    let mut values = interfaces.into_iter();
    let first = values.next()?;
    values.next().is_none().then_some(first)
}

fn object_child_digests(form: &TypeForm) -> Vec<TypeObjectDigest> {
    match form {
        TypeForm::Applied { arguments, .. } | TypeForm::ReadViewApplied { arguments, .. } => {
            arguments.clone()
        }
        TypeForm::StructuralRecord { fields }
        | TypeForm::OwnedProduct { fields }
        | TypeForm::OwnedChoice { cases: fields } => fields.iter().map(|field| field.ty).collect(),
        TypeForm::List { item }
        | TypeForm::Option { item }
        | TypeForm::Stream { item }
        | TypeForm::OwnedSequence { item } => {
            vec![*item]
        }
        TypeForm::Map { key, value }
        | TypeForm::Result {
            ok: key,
            error: value,
        } => vec![*key, *value],
        TypeForm::Function { parameters, result }
        | TypeForm::TaskFunction {
            parameters, result, ..
        } => {
            let mut values = parameters.clone();
            values.push(*result);
            values
        }
        _ => Vec::new(),
    }
}

fn snapshot_named_member_types(
    snapshot: &KernelSnapshot,
    declaration: super::DeclarationReference,
) -> Vec<TypeObjectDigest> {
    if declaration.package == snapshot.root.package_id {
        let Some(OwnerRecord::Declaration(record)) = snapshot
            .owners
            .get(&OwnerKey::Declaration(declaration.declaration))
        else {
            return Vec::new();
        };
        return match &record.payload {
            DeclarationPayload::Record { fields, .. }
            | DeclarationPayload::OwnedRecord { fields, .. } => fields
                .iter()
                .filter_map(
                    |field| match snapshot.owners.get(&OwnerKey::Field(*field)) {
                        Some(OwnerRecord::Field(record)) => Some(record.ty),
                        _ => None,
                    },
                )
                .collect(),
            DeclarationPayload::Variant { cases, .. } => cases
                .iter()
                .filter_map(|case| match snapshot.owners.get(&OwnerKey::Case(*case)) {
                    Some(OwnerRecord::Case(record)) => record.payload,
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
    }
    let Some(owners) = snapshot
        .dependencies
        .get(&declaration.package)
        .and_then(|dependency| {
            snapshot
                .dependency_interfaces
                .get(&dependency.package_revision)
        })
    else {
        return Vec::new();
    };
    let Some(
        PackageInterfaceRecord::Declaration(record)
        | PackageInterfaceRecord::RepresentationDeclaration(record),
    ) = owners.get(&OwnerKey::Declaration(declaration.declaration))
    else {
        return Vec::new();
    };
    match &record.payload {
        super::PackageInterfaceDeclarationPayload::Record { fields, .. }
        | super::PackageInterfaceDeclarationPayload::OwnedRecord { fields, .. } => fields
            .iter()
            .filter_map(|field| match owners.get(&OwnerKey::Field(*field)) {
                Some(PackageInterfaceRecord::Field(record)) => Some(record.ty),
                _ => None,
            })
            .collect(),
        super::PackageInterfaceDeclarationPayload::Variant { cases, .. } => cases
            .iter()
            .filter_map(|case| match owners.get(&OwnerKey::Case(*case)) {
                Some(PackageInterfaceRecord::Case(record)) => record.payload,
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

impl FullValidator<'_> {
    fn type_contains_resource(&mut self, digest: TypeObjectDigest) -> bool {
        // Reachability is independent of the path. Retain completed nodes so a compact
        // shared type DAG cannot expand into an exponential tree of validation work.
        if !self.consume_work() {
            return false;
        }
        let mut pending = vec![digest];
        let mut seen_types = BTreeSet::new();
        let mut seen_declarations = BTreeSet::new();
        while let Some(digest) = pending.pop() {
            if !self.consume_work() {
                return false;
            }
            if !seen_types.insert(digest) {
                continue;
            }
            let children = match self
                .snapshot
                .types
                .get(&digest)
                .or_else(|| self.snapshot.dependency_types.get(&digest))
                .map(|object| &object.form)
            {
                Some(TypeForm::CapabilityResource { .. }) => return true,
                Some(TypeForm::Named { declaration }) => {
                    if !seen_declarations.insert(*declaration) {
                        continue;
                    }
                    snapshot_named_member_types(self.snapshot, *declaration)
                }
                Some(form) => object_child_digests(form),
                None => Vec::new(),
            };
            for child in children {
                if !self.consume_work() {
                    return false;
                }
                pending.push(child);
            }
        }
        false
    }

    fn validate(&mut self) {
        self.validate_root_and_records();
        if self.exhausted() {
            return;
        }
        self.validate_namespaces();
        if self.exhausted() {
            return;
        }
        self.validate_owner_structure();
        if self.exhausted() {
            return;
        }
        self.validate_http_topology();
        if self.exhausted() {
            return;
        }
        self.validate_expressions();
        if self.exhausted() {
            return;
        }
        self.validate_types();
        if self.exhausted() {
            return;
        }
        self.validate_resource_shapes();
        if self.exhausted() {
            return;
        }
        self.validate_references();
        if self.exhausted() {
            return;
        }
        if self.diagnostics.is_empty() {
            validate_expression_meaning(
                self.snapshot,
                &self.read,
                &mut self.diagnostics,
                &mut self.work,
                self.maximum_work,
            );
        }
        if self.diagnostics.is_empty() {
            validate_affine_meaning(
                self.snapshot,
                &self.read,
                &mut self.diagnostics,
                &mut self.work,
                self.maximum_work,
            );
        }
        if self.exhausted() {
            return;
        }
        self.validate_relations();
        if self.diagnostics.is_empty()
            && let Err(diagnostic) = super::callable_flow::validate_callable_flow(
                &self.read,
                self.snapshot.owners.keys().copied(),
                &mut self.work,
                self.maximum_work,
            )
        {
            self.diagnostics.push(diagnostic);
        }
    }

    fn validate_root_and_records(&mut self) {
        self.capture(self.snapshot.root.validate_local());
        self.compare_map_count(
            "owner",
            self.snapshot.root.owners.entries(),
            self.snapshot.owners.len(),
        );
        self.compare_map_count(
            "dependency",
            self.snapshot.root.dependencies.entries(),
            self.snapshot.dependencies.len(),
        );
        self.compare_map_count(
            "retirement",
            self.snapshot.root.retirements.entries(),
            self.snapshot.retirements.len(),
        );

        for (key, record) in &self.snapshot.owners {
            if !self.consume_work() {
                return;
            }
            self.capture(record.validate_local());
            if record.header().contract_version != self.snapshot.root.graph_contract_version {
                self.error(
                    "kernel_owner_graph_generation",
                    "owner schema epoch differs from its containing graph",
                );
            }
            if *key != record.owner() {
                self.error(
                    "kernel_full_owner_key",
                    format!(
                        "owner map key {key:?} does not match record {:?}",
                        record.owner()
                    ),
                );
            }
        }
        for (digest, object) in &self.snapshot.types {
            if !self.consume_work() {
                return;
            }
            match encode_type_object(object) {
                Ok((encoded_digest, _)) if encoded_digest == *digest => {}
                Ok((encoded_digest, _)) => self.error(
                    "kernel_full_type_digest",
                    format!("type key {digest} does not match canonical digest {encoded_digest}"),
                ),
                Err(diagnostic) => self.diagnostics.push(diagnostic),
            }
        }
        for (package, dependency) in &self.snapshot.dependencies {
            if !self.consume_work() {
                return;
            }
            self.capture(dependency.validate_local());
            if dependency.graph_contract_version != self.snapshot.root.graph_contract_version {
                self.error(
                    "kernel_dependency_graph_generation",
                    "dependency encoding is newer than its containing graph",
                );
            }
            if package != &dependency.package || package == &self.snapshot.root.package_id {
                self.error(
                    "kernel_full_dependency_key",
                    "dependency key is foreign to its record or names the current package",
                );
            }
        }
        for (owner, retirement) in &self.snapshot.retirements {
            if !self.consume_work() {
                return;
            }
            self.capture(retirement.validate_local());
            if retirement.graph_contract_version != self.snapshot.root.graph_contract_version {
                self.error(
                    "kernel_retirement_graph_generation",
                    "retirement encoding is newer than its containing graph",
                );
            }
            if owner != &retirement.owner {
                self.error(
                    "kernel_full_retirement_key",
                    "retirement key does not match its record",
                );
            }
            if self.snapshot.owners.contains_key(owner) {
                self.error(
                    "kernel_full_live_retired_overlap",
                    format!("owner {owner:?} is both live and retired"),
                );
            }
        }
    }

    fn validate_namespaces(&mut self) {
        let mut names = BTreeMap::<(Option<OwnerKey>, u8, String), OwnerKey>::new();
        for (key, record) in &self.snapshot.owners {
            if !self.consume_work() {
                return;
            }
            let Some(entry) = owner_namespace(record) else {
                continue;
            };
            let namespace = (
                entry.parent,
                entry.class.tag(),
                entry.name.as_str().to_owned(),
            );
            if let Some(previous) = names.insert(namespace, *key) {
                self.error(
                    "kernel_full_namespace_duplicate",
                    format!("owners {previous:?} and {key:?} have the same canonical name"),
                );
            }
        }
    }

    fn validate_owner_structure(&mut self) {
        let records = self.snapshot.owners.iter().collect::<Vec<_>>();
        for (key, record) in records {
            if !self.consume_work() {
                return;
            }
            match record {
                OwnerRecord::Module(_) | OwnerRecord::Expression(_) => {}
                OwnerRecord::Declaration(declaration) => {
                    self.require_local_kind(
                        OwnerKey::Module(declaration.module),
                        &[OwnerKind::Module],
                        "declaration module",
                    );
                    self.validate_declaration_children(*key, &declaration.payload);
                }
                OwnerRecord::TypeParameter(parameter) => {
                    self.require_parent_listed(
                        *key,
                        OwnerKey::Declaration(parameter.declaration),
                        "type parameter",
                    );
                    if parameter.constraints.has_owned()
                        && !matches!(self.snapshot.owners.get(&OwnerKey::Declaration(parameter.declaration)), Some(OwnerRecord::Declaration(declaration)) if matches!(declaration.payload, DeclarationPayload::OwnedContract(_) | DeclarationPayload::OwnedImplementation(_) | DeclarationPayload::Function(_) | DeclarationPayload::OwnedRecord { .. } | DeclarationPayload::ReadView(_)))
                    {
                        self.error(
                            "kernel_owned_parameter_owner",
                            "Owned is supported by graph functions, owned contracts and implementation schemes",
                        );
                    }
                    if parameter.constraints.requires_transfer()
                        && !matches!(self.snapshot.owners.get(&OwnerKey::Declaration(parameter.declaration)), Some(OwnerRecord::Declaration(declaration)) if matches!(declaration.payload, DeclarationPayload::Function(_) | DeclarationPayload::OwnedContract(_) | DeclarationPayload::OwnedImplementation(_) | DeclarationPayload::OwnedRecord { .. } | DeclarationPayload::ReadView(_)))
                    {
                        self.error(
                            "kernel_transfer_parameter_owner",
                            "Transferable is supported by graph-function, owned-contract and implementation-scheme type parameters",
                        );
                    }
                    if parameter.constraints.requires_share()
                        && !matches!(self.snapshot.owners.get(&OwnerKey::Declaration(parameter.declaration)), Some(OwnerRecord::Declaration(declaration)) if matches!(declaration.payload, DeclarationPayload::Function(_) | DeclarationPayload::OwnedImplementation(_) | DeclarationPayload::OwnedRecord { .. } | DeclarationPayload::ReadView(_)))
                    {
                        self.error(
                            "kernel_share_parameter_owner",
                            "Shareable is supported by graph-function and implementation-scheme type parameters",
                        );
                    }
                    if parameter.constraints != super::TypeParameterConstraints::None
                        && matches!(self.snapshot.owners.get(&OwnerKey::Declaration(parameter.declaration)), Some(OwnerRecord::Declaration(declaration)) if matches!(declaration.payload, DeclarationPayload::External(_)))
                    {
                        self.error("intrinsic_signature", "closed external parameters require the intrinsic inventory's empty constraint set");
                    }
                }
                OwnerRecord::EffectParameter(parameter) => self.require_parent_listed(
                    *key,
                    OwnerKey::Declaration(parameter.declaration),
                    "effect parameter",
                ),
                OwnerRecord::RequirementParameter(parameter) => self.require_parent_listed(
                    *key,
                    OwnerKey::Declaration(parameter.declaration),
                    "requirement parameter",
                ),
                OwnerRecord::Field(field) => self.require_parent_listed(
                    *key,
                    OwnerKey::Declaration(field.declaration),
                    "record field",
                ),
                OwnerRecord::Case(case) => self.require_parent_listed(
                    *key,
                    OwnerKey::Declaration(case.declaration),
                    "variant case",
                ),
                OwnerRecord::Operation(operation) => {
                    self.require_parent_listed(
                        *key,
                        OwnerKey::Declaration(operation.declaration),
                        "interface operation",
                    );
                    let OwnerKey::Operation(operation_id) = key else {
                        self.error(
                            "kernel_full_operation_domain",
                            "operation record has a foreign identity domain",
                        );
                        continue;
                    };
                    for parameter in &operation.parameters {
                        self.require_parameter_parent(
                            *parameter,
                            ParameterParent::Operation(*operation_id),
                        );
                    }
                }
                OwnerRecord::Parameter(parameter) => match parameter.parent {
                    ParameterParent::Function(declaration) => {
                        self.require_parent_listed(
                            *key,
                            OwnerKey::Declaration(declaration),
                            "function parameter",
                        );
                        if let Some(requirement) = parameter.resource_requirement {
                            self.require_exact_kind(
                                requirement.package,
                                OwnerKey::Requirement(requirement.requirement),
                                &[OwnerKind::Requirement],
                                "function parameter resource requirement",
                            );
                        }
                    }
                    ParameterParent::Operation(operation) => {
                        self.require_parent_listed(
                            *key,
                            OwnerKey::Operation(operation),
                            "operation parameter",
                        );
                    }
                },
                OwnerRecord::Binding(_) => {}
                OwnerRecord::Requirement(requirement) => self.require_parent_listed(
                    *key,
                    OwnerKey::Declaration(requirement.declaration),
                    "component requirement",
                ),
                OwnerRecord::Port(port) => self.require_parent_listed(
                    *key,
                    OwnerKey::Declaration(port.declaration),
                    "component port",
                ),
                OwnerRecord::Target(_) => {}
                OwnerRecord::HttpRoute(route) => self.require_local_kind(
                    OwnerKey::Target(route.target),
                    &[OwnerKind::Target],
                    "HTTP route target",
                ),
                OwnerRecord::Documentation(documentation) => {
                    self.require_owner(documentation.owner, "documentation owner");
                    if documentation.owner == *key {
                        self.error(
                            "kernel_full_documentation_self",
                            "documentation cannot own itself",
                        );
                    }
                }
                OwnerRecord::Annotation(annotation) => {
                    self.require_owner(annotation.owner, "annotation owner");
                    if annotation.owner == *key {
                        self.error(
                            "kernel_full_annotation_self",
                            "annotation cannot own itself",
                        );
                    }
                }
            }
        }
    }

    fn validate_http_topology(&mut self) {
        let mut routes = BTreeMap::<TargetId, Vec<(OwnerKey, &super::HttpRouteRecord)>>::new();
        for (owner, record) in &self.snapshot.owners {
            let OwnerRecord::HttpRoute(route) = record else {
                continue;
            };
            routes
                .entry(route.target)
                .or_default()
                .push((*owner, route));
        }

        let targets = self
            .snapshot
            .owners
            .iter()
            .filter_map(|(owner, record)| match (owner, record) {
                (OwnerKey::Target(target), OwnerRecord::Target(record)) => Some((*target, record)),
                _ => None,
            })
            .collect::<Vec<_>>();
        for (target_id, target) in targets {
            if !self.consume_work() {
                return;
            }
            let owned = routes.get(&target_id).map(Vec::as_slice).unwrap_or(&[]);
            if target.runner == crate::platform::package::RunnerKind::Http {
                if target.port.is_some() {
                    self.error(
                        "kernel_http_target_universal_port",
                        "HTTP target must not retain a universal port",
                    );
                }
                if owned.is_empty() || owned.len() > MAXIMUM_HTTP_ROUTES_PER_TARGET {
                    self.error(
                        "kernel_http_target_route_count",
                        format!(
                            "HTTP target must own 1 through {MAXIMUM_HTTP_ROUTES_PER_TARGET} routes"
                        ),
                    );
                }
                let route_set = owned
                    .iter()
                    .map(|(_, route)| (*route).clone())
                    .collect::<Vec<_>>();
                if let Err(diagnostic) = super::analyze_http_route_set(&route_set) {
                    self.diagnostics.push(diagnostic);
                }
                for (_route_owner, route) in owned {
                    if !self.consume_work() {
                        return;
                    }
                    if route.port.package != self.snapshot.root.package_id {
                        self.error(
                            "kernel_http_route_port_package",
                            "HTTP route port must belong to the root package",
                        );
                        continue;
                    }
                    match self.snapshot.owners.get(&OwnerKey::Port(route.port.port)) {
                        Some(OwnerRecord::Port(port)) => {
                            if port.declaration != target.component.declaration {
                                self.error(
                                    "kernel_http_route_port_owner",
                                    "HTTP route port does not belong to its target component",
                                );
                            }
                            match port.implementation {
                                PortImplementation::Function(_) => {
                                    self.validate_http_route_signature(route, port);
                                }
                                PortImplementation::Expression(_) => self.error(
                                    "kernel_http_route_port_implementation",
                                    "HTTP route port must be function-backed",
                                ),
                            }
                        }
                        Some(_) => self.error(
                            "kernel_http_route_port_kind",
                            "HTTP route port identity names another owner kind",
                        ),
                        None => self.error(
                            "kernel_http_route_port_missing",
                            "HTTP route references a missing port",
                        ),
                    }
                }
            } else {
                if target.port.is_none() {
                    self.error(
                        "kernel_target_port_missing",
                        "non-HTTP target must select one exact port",
                    );
                }
                if !owned.is_empty() {
                    self.error(
                        "kernel_http_route_non_http_target",
                        "HTTP routes may belong only to an HTTP target",
                    );
                }
            }
        }
    }

    fn validate_http_route_signature(&mut self, route: &HttpRouteRecord, port: &PortRecord) {
        let PortImplementation::Function(function) = port.implementation else {
            return;
        };
        let Some((type_parameters, parameter_ids, result, effect)) =
            self.http_route_function_contract(function)
        else {
            return;
        };
        let mut types = TypeObjectInterner::default();
        let expected_type = match crate::platform::http::semantic_http_route_callable_type(
            &mut types,
            route.selector.capture_count(),
            &effect,
        ) {
            Ok(expected) => expected,
            Err(diagnostic) => {
                self.diagnostics.push(diagnostic);
                return;
            }
        };
        if port.function_type != expected_type {
            self.error(
                "kernel_type_http_route_port",
                "HTTP route port type disagrees with its selector-indexed function contract",
            );
        }
        let mut parameters = Vec::with_capacity(parameter_ids.len());
        for parameter in parameter_ids {
            if !self.consume_work() {
                return;
            }
            if let Some(parameter) = self.http_route_parameter(function, parameter) {
                parameters.push(parameter);
            }
        }
        let http = match crate::platform::http::semantic_http_types(&mut types) {
            Ok(http) => http,
            Err(diagnostic) => {
                self.diagnostics.push(diagnostic);
                return;
            }
        };
        let captures = route.selector.capture_names();
        if type_parameters != 0
            || parameters.len() != captures.len().saturating_add(1)
            || parameters
                .first()
                .is_none_or(|parameter| parameter.ty != http.request_type)
            || result != http.response_type
        {
            self.error(
                "kernel_type_http_route_parameters",
                "HTTP route function must have exactly request then selector-indexed capture parameters and the HTTP response result",
            );
            return;
        }
        for (parameter, capture) in parameters.iter().skip(1).zip(captures) {
            if parameter.name.as_str() != capture.as_str()
                || parameter.ty != http.text_type
                || parameter.use_mode != ParameterUse::Unrestricted
                || parameter.resource_requirement.is_some()
            {
                self.error(
                    "kernel_type_http_route_capture_parameter",
                    "HTTP route capture must index one same-named unrestricted Text parameter without a resource binding",
                );
            }
        }
        let row = effect.row();
        if row.requirements.iter().any(|r| r.concrete().is_none()) {
            self.error(
                "kernel_http_route_requirement_closure",
                "entry requirements must be concrete",
            );
            return;
        }
        self.validate_http_route_requirement_closure(
            route,
            &row.requirements
                .iter()
                .filter_map(|r| r.concrete())
                .collect::<Vec<_>>(),
        );
    }

    fn http_route_function_contract(
        &mut self,
        function: DeclarationReference,
    ) -> Option<(
        usize,
        Vec<crate::platform::semantic_id::ParameterId>,
        TypeObjectDigest,
        FunctionEffect,
    )> {
        if function.package == self.snapshot.root.package_id {
            return match self
                .snapshot
                .owners
                .get(&OwnerKey::Declaration(function.declaration))
            {
                Some(OwnerRecord::Declaration(record)) => match &record.payload {
                    DeclarationPayload::Function(signature) => Some((
                        signature.type_parameters.len()
                            + signature.effect_parameters.len()
                            + signature.requirement_parameters.len()
                            + signature.read_scope_parameters.len(),
                        signature.parameters.clone(),
                        signature.result.ty,
                        signature.effect.clone(),
                    )),
                    _ => {
                        self.error(
                            "kernel_type_http_route_function",
                            "HTTP route port must resolve to a function declaration",
                        );
                        None
                    }
                },
                _ => {
                    self.error(
                        "kernel_type_http_route_function",
                        "HTTP route backing function is missing",
                    );
                    None
                }
            };
        }
        let Some(dependency) = self.snapshot.dependencies.get(&function.package) else {
            self.error(
                "kernel_type_http_route_function",
                "HTTP route backing function package is not an exact dependency",
            );
            return None;
        };
        let Some(owners) = self
            .snapshot
            .dependency_interfaces
            .get(&dependency.package_revision)
        else {
            self.error(
                "kernel_type_http_route_function",
                "HTTP route backing dependency interface is missing",
            );
            return None;
        };
        match owners.get(&OwnerKey::Declaration(function.declaration)) {
            Some(PackageInterfaceRecord::Declaration(record)) => match &record.payload {
                PackageInterfaceDeclarationPayload::Function(signature) => Some((
                    signature.type_parameters.len()
                        + signature.effect_parameters.len()
                        + signature.requirement_parameters.len()
                        + signature.read_scope_parameters.len(),
                    signature.parameters.clone(),
                    signature.result.ty,
                    signature.effect.clone(),
                )),
                _ => {
                    self.error(
                        "kernel_type_http_route_function",
                        "HTTP route dependency owner is not a function declaration",
                    );
                    None
                }
            },
            _ => {
                self.error(
                    "kernel_type_http_route_function",
                    "HTTP route backing dependency function is missing",
                );
                None
            }
        }
    }

    fn validate_http_route_requirement_closure(
        &mut self,
        route: &HttpRouteRecord,
        requirements: &[RequirementReference],
    ) {
        let component = match self.snapshot.owners.get(&OwnerKey::Target(route.target)) {
            Some(OwnerRecord::Target(target)) => target.component,
            _ => return,
        };
        if component.package != self.snapshot.root.package_id {
            self.error(
                "kernel_http_route_requirement_closure",
                "HTTP route target component must belong to the root package",
            );
            return;
        }
        let component_requirements = match self
            .snapshot
            .owners
            .get(&OwnerKey::Declaration(component.declaration))
        {
            Some(OwnerRecord::Declaration(record)) => match &record.payload {
                DeclarationPayload::Component { requirements, .. } => requirements.clone(),
                _ => return,
            },
            _ => return,
        };
        let component_requirements = component_requirements
            .into_iter()
            .filter_map(|requirement| {
                let reference = RequirementReference {
                    package: component.package,
                    requirement,
                };
                self.http_route_requirement(reference)
                    .map(|record| (reference, record))
            })
            .collect::<Vec<_>>();
        // Imported authority is admitted only by the selected port's explicit
        // closed row. It is not an alias of a similarly named local slot.
        let imported = match self.snapshot.owners.get(&OwnerKey::Port(route.port.port)) {
            Some(OwnerRecord::Port(port)) => self
                .snapshot
                .types
                .get(&port.function_type)
                .and_then(|object| match &object.form {
                    TypeForm::TaskFunction { effect, .. } => Some(
                        effect
                            .requirements
                            .iter()
                            .filter_map(|requirement| requirement.concrete())
                            .filter(|requirement| requirement.package != component.package)
                            .collect::<BTreeSet<_>>(),
                    ),
                    _ => None,
                })
                .unwrap_or_default(),
            _ => BTreeSet::new(),
        };
        for requirement in requirements {
            if !self.consume_work() {
                return;
            }
            let Some(candidate) = self.http_route_requirement(*requirement) else {
                continue;
            };
            if imported.contains(requirement) {
                continue;
            }
            let matches = component_requirements
                .iter()
                .filter(|(reference, component)| {
                    requirement_is_covered_by(
                        requirement.package,
                        &candidate,
                        reference.package,
                        component,
                    )
                })
                .count();
            if matches != 1 {
                self.error(
                    "kernel_http_route_requirement_closure",
                    "each HTTP route handler requirement must have one unambiguous name-, interface-, operation-, and limit-compatible component capability slot",
                );
            }
        }
    }

    fn http_route_requirement(
        &mut self,
        requirement: RequirementReference,
    ) -> Option<RequirementRecord> {
        if requirement.package == self.snapshot.root.package_id {
            return match self
                .snapshot
                .owners
                .get(&OwnerKey::Requirement(requirement.requirement))
            {
                Some(OwnerRecord::Requirement(record)) => Some(record.clone()),
                _ => {
                    self.error(
                        "kernel_http_route_requirement_closure",
                        "HTTP route requirement is missing from local authority",
                    );
                    None
                }
            };
        }
        let Some(dependency) = self.snapshot.dependencies.get(&requirement.package) else {
            self.error(
                "kernel_http_route_requirement_closure",
                "HTTP route requirement belongs to an unbound package",
            );
            return None;
        };
        let Some(owners) = self
            .snapshot
            .dependency_interfaces
            .get(&dependency.package_revision)
        else {
            self.error(
                "kernel_http_route_requirement_closure",
                "HTTP route requirement package interface is unavailable",
            );
            return None;
        };
        match owners.get(&OwnerKey::Requirement(requirement.requirement)) {
            Some(PackageInterfaceRecord::Requirement(record)) => Some(record.clone()),
            _ => {
                self.error(
                    "kernel_http_route_requirement_closure",
                    "HTTP route requirement is missing from its exact package interface",
                );
                None
            }
        }
    }

    fn http_route_parameter(
        &mut self,
        function: DeclarationReference,
        parameter: crate::platform::semantic_id::ParameterId,
    ) -> Option<ParameterRecord> {
        let record = if function.package == self.snapshot.root.package_id {
            match self.snapshot.owners.get(&OwnerKey::Parameter(parameter)) {
                Some(OwnerRecord::Parameter(record)) => record.clone(),
                _ => {
                    self.error(
                        "kernel_type_http_route_parameter",
                        "HTTP route backing function parameter is missing",
                    );
                    return None;
                }
            }
        } else {
            let dependency = self.snapshot.dependencies.get(&function.package)?;
            let owners = self
                .snapshot
                .dependency_interfaces
                .get(&dependency.package_revision)?;
            match owners.get(&OwnerKey::Parameter(parameter)) {
                Some(PackageInterfaceRecord::Parameter(record)) => record.clone(),
                _ => {
                    self.error(
                        "kernel_type_http_route_parameter",
                        "HTTP route dependency parameter is missing",
                    );
                    return None;
                }
            }
        };
        if record.parent != ParameterParent::Function(function.declaration) {
            self.error(
                "kernel_type_http_route_parameter_parent",
                "HTTP route parameter belongs to another function",
            );
            return None;
        }
        Some(record)
    }

    fn validate_declaration_children(&mut self, owner: OwnerKey, payload: &DeclarationPayload) {
        let OwnerKey::Declaration(declaration_id) = owner else {
            self.error(
                "kernel_full_declaration_domain",
                "declaration record has a foreign identity domain",
            );
            return;
        };
        match payload {
            DeclarationPayload::ReadView(view) => {
                let declaration = DeclarationReference {
                    package: self.snapshot.root.package_id,
                    declaration: declaration_id,
                };
                self.capture(view.validate_local_at(declaration));
                for parameter in &view.type_parameters {
                    self.require_parent_listed(
                        OwnerKey::TypeParameter(*parameter),
                        owner,
                        "read-view type parameter",
                    );
                }
            }
            DeclarationPayload::OwnedContract(c) => {
                self.require_local_kind(
                    OwnerKey::TypeParameter(c.self_parameter),
                    &[OwnerKind::TypeParameter],
                    "owned Self parameter",
                );
                for parameter in &c.type_parameters {
                    self.require_local_kind(
                        OwnerKey::TypeParameter(*parameter),
                        &[OwnerKind::TypeParameter],
                        "owned contract type parameter",
                    );
                }
                for method in &c.methods {
                    if !self.consume_work() {
                        return;
                    }
                    self.validate_effect_row(owner, &method.effect.row());
                    self.capture(super::validate_read_scope_inventory_at(
                        ReadScopeBinder::ContractMethod {
                            contract: DeclarationReference {
                                package: self.snapshot.root.package_id,
                                declaration: declaration_id,
                            },
                            method: method.id,
                        },
                        &method.read_scope_parameters,
                        &method.read_scope_sources,
                        |source| (source as usize) < method.parameters.len(),
                    ));
                }
            }
            DeclarationPayload::OwnedImplementation(i) => {
                for parameter in &i.type_parameters {
                    self.require_local_kind(
                        OwnerKey::TypeParameter(*parameter),
                        &[OwnerKind::TypeParameter],
                        "owned implementation type parameter",
                    );
                }
                self.require_exact_kind(
                    i.contract.package,
                    OwnerKey::Declaration(i.contract.declaration),
                    &[OwnerKind::OwnedContract],
                    "owned implementation contract",
                );
                for prerequisite in &i.implementation_parameters {
                    self.require_exact_kind(
                        prerequisite.contract.package,
                        OwnerKey::Declaration(prerequisite.contract.declaration),
                        &[OwnerKind::OwnedContract],
                        "owned implementation prerequisite contract",
                    );
                }
                for mapping in &i.methods {
                    for argument in &mapping.read_scope_arguments {
                        self.validate_read_scope_argument(
                            owner,
                            *argument,
                            Some(ReadScopeBinder::ContractMethod {
                                contract: i.contract,
                                method: mapping.method,
                            }),
                        );
                    }
                    self.require_exact_kind(
                        mapping.function.package,
                        OwnerKey::Declaration(mapping.function.declaration),
                        &[OwnerKind::PureFunction, OwnerKind::TaskFunction],
                        "owned implementation mapped target",
                    );
                    for operand in &mapping.implementations {
                        self.validate_implementation_operand(operand);
                    }
                }
            }
            DeclarationPayload::Record {
                fields,
                type_parameters,
            }
            | DeclarationPayload::OwnedRecord {
                fields,
                type_parameters,
            } => {
                for parameter in type_parameters {
                    self.require_local_kind(
                        OwnerKey::TypeParameter(*parameter),
                        &[OwnerKind::TypeParameter],
                        "record type parameter",
                    );
                }
                for field in fields {
                    self.require_local_kind(OwnerKey::Field(*field), &[OwnerKind::Field], "field");
                }
            }
            DeclarationPayload::Variant {
                cases,
                type_parameters,
            } => {
                for parameter in type_parameters {
                    self.require_local_kind(
                        OwnerKey::TypeParameter(*parameter),
                        &[OwnerKind::TypeParameter],
                        "variant type parameter",
                    );
                }
                for case in cases {
                    self.require_local_kind(OwnerKey::Case(*case), &[OwnerKind::Case], "case");
                }
            }
            DeclarationPayload::Interface { operations } => {
                for operation in operations {
                    if !self.consume_work() {
                        return;
                    }
                    self.require_local_kind(
                        OwnerKey::Operation(*operation),
                        &[OwnerKind::Operation],
                        "operation",
                    );
                    if self
                        .operation_parent(*operation)
                        .is_some_and(|parent| parent != declaration_id)
                    {
                        self.error(
                            "kernel_full_parent_mismatch",
                            "interface lists an operation belonging to another declaration",
                        );
                    }
                }
            }
            DeclarationPayload::External(function) => {
                for parameter in &function.parameters {
                    self.require_parameter_parent(
                        *parameter,
                        ParameterParent::Function(declaration_id),
                    );
                }
                for parameter in &function.type_parameters {
                    self.require_local_kind(
                        OwnerKey::TypeParameter(*parameter),
                        &[OwnerKind::TypeParameter],
                        "external type parameter",
                    );
                }
            }
            DeclarationPayload::Function(function) => {
                self.capture(super::validate_read_scope_inventory_at(
                    ReadScopeBinder::Declaration(DeclarationReference {
                        package: self.snapshot.root.package_id,
                        declaration: declaration_id,
                    }),
                    &function.read_scope_parameters,
                    &function.read_scope_sources,
                    |source| function.parameters.contains(&source),
                ));
                for parameter in &function.requirement_parameters {
                    self.require_local_kind(
                        OwnerKey::RequirementParameter(*parameter),
                        &[OwnerKind::RequirementParameter],
                        "function requirement parameter",
                    );
                    if !matches!(self.snapshot.owners.get(&OwnerKey::RequirementParameter(*parameter)),
                        Some(OwnerRecord::RequirementParameter(record)) if record.declaration == declaration_id)
                    {
                        self.error(
                            "kernel_requirement_parameter_owner",
                            "requirement parameter belongs to another function",
                        );
                    }
                }
                for parameter in &function.effect_parameters {
                    self.require_local_kind(
                        OwnerKey::EffectParameter(*parameter),
                        &[OwnerKind::EffectParameter],
                        "function effect parameter",
                    );
                    if !matches!(self.snapshot.owners.get(&OwnerKey::EffectParameter(*parameter)),
                        Some(OwnerRecord::EffectParameter(record)) if record.declaration == declaration_id)
                    {
                        self.error(
                            "kernel_effect_parameter_owner",
                            "effect parameter belongs to another function",
                        );
                    }
                }
                self.validate_effect_row(owner, &function.effect.row());
                for parameter in &function.parameters {
                    self.require_parameter_parent(
                        *parameter,
                        ParameterParent::Function(declaration_id),
                    );
                }
                for parameter in &function.type_parameters {
                    self.require_local_kind(
                        OwnerKey::TypeParameter(*parameter),
                        &[OwnerKind::TypeParameter],
                        "function type parameter",
                    );
                }
                if let FunctionEffect::Task {
                    effect_parameters: _,
                    requirements,
                } = &function.effect
                {
                    for requirement in requirements {
                        self.require_exact_kind(
                            requirement.package(),
                            requirement.owner(),
                            &[OwnerKind::Requirement, OwnerKind::RequirementParameter],
                            "function requirement",
                        );
                    }
                }
            }
            DeclarationPayload::Component {
                requirements,
                ports,
            } => {
                for requirement in requirements {
                    self.require_local_kind(
                        OwnerKey::Requirement(*requirement),
                        &[OwnerKind::Requirement],
                        "component requirement",
                    );
                }
                for port in ports {
                    self.require_local_kind(OwnerKey::Port(*port), &[OwnerKind::Port], "port");
                }
            }
            DeclarationPayload::Constant { .. } | DeclarationPayload::Test { .. } => {}
        }
    }

    fn validate_types(&mut self) {
        let mut reachable = BTreeSet::new();
        let mut checked = BTreeSet::new();
        let mut roots = Vec::new();
        for (owner, record) in &self.snapshot.owners {
            let type_roots = record.type_roots();
            let declaration =
                self.semantic_declaration(*owner)
                    .map(|declaration| DeclarationReference {
                        package: self.snapshot.root.package_id,
                        declaration,
                    });
            match record {
                OwnerRecord::Declaration(record) => match &record.payload {
                    DeclarationPayload::OwnedContract(contract) => {
                        for method in &contract.methods {
                            let binder =
                                declaration.map(|contract| ReadScopeBinder::ContractMethod {
                                    contract,
                                    method: method.id,
                                });
                            roots.extend(
                                method
                                    .parameters
                                    .iter()
                                    .map(|parameter| (*owner, parameter.ty, binder)),
                            );
                            roots.push((*owner, method.result.ty, binder));
                        }
                    }
                    DeclarationPayload::OwnedImplementation(implementation) => {
                        let binder = declaration.map(ReadScopeBinder::Declaration);
                        roots.push((*owner, implementation.self_type, binder));
                        roots.extend(
                            implementation
                                .type_arguments
                                .iter()
                                .map(|ty| (*owner, *ty, binder)),
                        );
                        for parameter in &implementation.implementation_parameters {
                            roots.push((*owner, parameter.self_type, binder));
                            roots.extend(
                                parameter
                                    .type_arguments
                                    .iter()
                                    .map(|ty| (*owner, *ty, binder)),
                            );
                        }
                        for method in &implementation.methods {
                            roots.extend(
                                method.type_arguments.iter().map(|ty| (*owner, *ty, binder)),
                            );
                            for operand in &method.implementations {
                                roots.extend(operand.walk().flat_map(|operand| {
                                    operand
                                        .type_arguments()
                                        .iter()
                                        .map(|ty| (*owner, *ty, binder))
                                }));
                            }
                        }
                        for binding in &implementation.associated_types {
                            let binder = declaration.map(|implementation| {
                                ReadScopeBinder::AssociatedBinding {
                                    implementation,
                                    member: binding.member,
                                }
                            });
                            roots.push((*owner, binding.ty(), binder));
                        }
                    }
                    _ => roots.extend(
                        type_roots
                            .into_iter()
                            .map(|ty| (*owner, ty, declaration.map(ReadScopeBinder::Declaration))),
                    ),
                },
                _ => roots.extend(
                    type_roots
                        .into_iter()
                        .map(|ty| (*owner, ty, declaration.map(ReadScopeBinder::Declaration))),
                ),
            }
        }
        for (source, root, binder) in roots {
            let mut active = BTreeSet::new();
            let mut pending = vec![(root, false, 0_usize)];
            while let Some((digest, exiting, depth)) = pending.pop() {
                if !self.consume_work() {
                    return;
                }
                if exiting {
                    active.remove(&digest);
                    reachable.insert(digest);
                    continue;
                }
                if depth > MAXIMUM_TYPE_DEPTH {
                    self.error(
                        "kernel_full_type_depth",
                        format!("type object {digest} exceeds the maximum structural depth"),
                    );
                    continue;
                }
                if active.contains(&digest) {
                    self.error(
                        "kernel_full_type_cycle",
                        format!("type object {digest} participates in a structural cycle"),
                    );
                    continue;
                }
                if !checked.insert((source, digest, binder)) {
                    continue;
                }
                active.insert(digest);
                let Some(object) = self.snapshot.types.get(&digest) else {
                    self.error(
                        "kernel_full_type_missing",
                        format!("referenced type object {digest} is missing"),
                    );
                    active.remove(&digest);
                    continue;
                };
                self.validate_type_reference(source, &object.form, binder);
                pending.push((digest, true, depth));
                let mut children = object.child_types();
                children.reverse();
                pending.extend(
                    children
                        .into_iter()
                        .map(|child| (child, false, depth.saturating_add(1))),
                );
            }
        }
        for digest in self.snapshot.types.keys() {
            if !reachable.contains(digest) {
                self.error(
                    "kernel_full_type_unreachable",
                    format!("type object {digest} is not reachable from live meaning"),
                );
            }
        }
    }

    fn validate_type_reference(
        &mut self,
        source: OwnerKey,
        form: &TypeForm,
        binder: Option<ReadScopeBinder>,
    ) {
        match form {
            TypeForm::ReadViewApplied {
                declaration,
                read_scope,
                ..
            } => {
                self.require_exact_kind(
                    declaration.package,
                    OwnerKey::Declaration(declaration.declaration),
                    &[OwnerKind::ReadView],
                    "read-view declaration",
                );
                self.validate_read_scope_argument(source, *read_scope, binder);
            }
            TypeForm::AssociatedReadViewMember {
                contract,
                read_scope,
                ..
            }
            | TypeForm::AssociatedReadViewProjection {
                contract,
                read_scope,
                ..
            } => {
                self.require_exact_kind(
                    contract.package,
                    OwnerKey::Declaration(contract.declaration),
                    &[OwnerKind::OwnedContract],
                    "read-view family contract",
                );
                self.validate_read_scope_argument(source, *read_scope, binder);
            }
            TypeForm::TaskFunction { effect, .. } => self.validate_effect_row(source, effect),
            TypeForm::TypeParameter { parameter } => {
                self.require_local_kind(
                    OwnerKey::TypeParameter(*parameter),
                    &[OwnerKind::TypeParameter],
                    "type parameter use",
                );
                if let Some(OwnerRecord::TypeParameter(record)) = self
                    .snapshot
                    .owners
                    .get(&OwnerKey::TypeParameter(*parameter))
                    && self.semantic_declaration(source) != Some(record.declaration)
                {
                    self.error(
                        "kernel_full_type_parameter_scope",
                        "type parameter is used outside its owning declaration",
                    );
                }
            }
            TypeForm::Named { declaration } | TypeForm::Applied { declaration, .. } => self
                .require_exact_kind(
                    declaration.package,
                    OwnerKey::Declaration(declaration.declaration),
                    &[
                        OwnerKind::Record,
                        OwnerKind::Variant,
                        OwnerKind::OwnedRecord,
                    ],
                    "named type",
                ),
            TypeForm::CapabilityResource { interface } => self.require_exact_kind(
                interface.package,
                OwnerKey::Declaration(interface.declaration),
                &[OwnerKind::Interface],
                "capability resource interface",
            ),
            _ => {}
        }
    }

    fn validate_read_scope_argument(
        &mut self,
        source: OwnerKey,
        argument: ReadScopeArgument,
        binder: Option<ReadScopeBinder>,
    ) {
        let expression = match source {
            OwnerKey::Expression(expression) => Some(expression),
            OwnerKey::Binding(binding) => {
                if !self.consume_work() {
                    return;
                }
                self.binding_containers
                    .get(&binding)
                    .and_then(|containers| containers.first())
                    .map(|container| {
                        // A let annotation occurs at its initializer position,
                        // within earlier bindings' exact ordered scope roots.
                        // Body-only loan annotations keep their container site.
                        match self.snapshot.owners.get(&OwnerKey::Binding(binding)) {
                            Some(OwnerRecord::Binding(record))
                                if matches!(
                                    container.kind,
                                    BindingContainerKind::Let | BindingContainerKind::ReadViewLet
                                ) =>
                            {
                                record.value.unwrap_or(container.expression)
                            }
                            _ => container.expression,
                        }
                    })
            }
            _ => None,
        };
        let lexical = expression
            .and_then(|_| self.semantic_declaration(source))
            .map(|declaration| DeclarationReference {
                package: self.snapshot.root.package_id,
                declaration,
            });
        let mut loans = BTreeSet::new();
        if let ReadScopeArgument::Lexical { binding, .. } = argument {
            if !self.consume_work() {
                return;
            }
            if !lexical.is_some_and(|callable| matches!(self.snapshot.owners.get(&OwnerKey::Declaration(callable.declaration)),
                Some(OwnerRecord::Declaration(record)) if record.header.owner == OwnerKey::Declaration(callable.declaration)
                    && matches!(record.header.kind, OwnerKind::PureFunction | OwnerKind::TaskFunction)
                    && matches!(record.payload, DeclarationPayload::Function(_))))
            {
                self.error("kernel_read_view", "lexical read scopes require an exact named Function declaration");
                return;
            }
            if let Some(OwnerRecord::Binding(record)) =
                self.snapshot.owners.get(&OwnerKey::Binding(binding))
            {
                let containers = self
                    .binding_containers
                    .get(&binding)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                let body_scope = expression.is_some_and(|expression| {
                    containers.iter().any(|container| {
                        container
                            .scope_roots
                            .iter()
                            .any(|root| self.is_expression_descendant(expression, *root))
                    })
                });
                let owning_view = record.kind == BindingKind::ReadView
                    && record.value.is_some()
                    && containers.len() == 1
                    && containers[0].kind == BindingContainerKind::ReadViewLet;
                let declared_scope = record
                    .declared_type
                    .and_then(|ty| {
                        self.snapshot
                            .types
                            .get(&ty)
                            .or_else(|| self.snapshot.dependency_types.get(&ty))
                    })
                    .and_then(|object| match object.form {
                        TypeForm::ReadViewApplied { read_scope, .. }
                        | TypeForm::AssociatedReadViewMember { read_scope, .. }
                        | TypeForm::AssociatedReadViewProjection { read_scope, .. } => {
                            Some(read_scope)
                        }
                        _ => None,
                    });
                let initializer_scope = owning_view
                    && declared_scope == Some(argument)
                    && (source == OwnerKey::Binding(binding)
                        || expression.is_some_and(|expression| {
                            record.value.is_some_and(|initializer| {
                                self.is_expression_descendant(expression, initializer)
                            })
                        }));
                let explicit_loan = record.value.is_none()
                    && matches!(
                        record.kind,
                        BindingKind::OwnedBorrow
                            | BindingKind::ReadView
                            | BindingKind::ReadViewBorrow
                    )
                    && (source == OwnerKey::Binding(binding) || body_scope
                        || expression.is_some_and(|expression| containers.len() == 1
                            && matches!(self.snapshot.owners.get(&OwnerKey::Expression(containers[0].expression)), Some(OwnerRecord::Expression(parent))
                                if parent.contract_version == super::contract::GRAPH_CONTRACT_VERSION
                                    && matches!(parent.operation, ExpressionOperation::BorrowCall { call, binding: owner, .. } if call == expression && owner == binding))));
                if explicit_loan
                    || initializer_scope
                    || (owning_view && declared_scope.is_some() && body_scope)
                {
                    loans.insert(binding);
                }
            }
        }
        let generation = self
            .snapshot
            .owners
            .get(&source)
            .map(|record| record.header().contract_version);
        let result = self.with_associated_read(|read, admission| {
            super::owned_associated::read_view_admission::require_generation(
                generation.ok_or_else(|| {
                    super::owned_associated::read_view_admission::reject(
                        "read-scope consuming owner is absent",
                    )
                })?,
            )?;
            super::owned_associated::read_view_admission::ScopeAuthority {
                binder,
                lexical,
                live_loans: &loans,
            }
            .validate(read, argument, admission)
        });
        self.capture(result);
    }

    fn with_associated_read<T>(
        &mut self,
        action: impl FnOnce(
            &dyn super::ExpressionRead,
            &super::owned_associated::OwnedAssociatedAdmission,
        ) -> Result<T, Diagnostic>,
    ) -> Result<T, Diagnostic> {
        let work = std::cell::Cell::new(self.work);
        let interrupted = std::cell::Cell::new(false);
        let result = {
            let checkpoint = || {
                if let Err(error) = (self.read.checkpoint)() {
                    interrupted.set(true);
                    return Err(error);
                }
                let next = work.get().checked_add(1).ok_or_else(full_work_exhausted)?;
                work.set(next);
                if next > self.maximum_work {
                    return Err(full_work_exhausted());
                }
                Ok(())
            };
            let read = super::infer::CheckedExpressionRead {
                read: self.snapshot,
                checkpoint: &checkpoint,
            };
            action(&read, &self.associated_admission)
        };
        self.work = work.get();
        self.interrupted |= interrupted.get();
        result
    }

    fn validate_requirement_operand(
        &mut self,
        source: OwnerKey,
        operand: super::RequirementOperand,
    ) {
        match operand {
            super::RequirementOperand::Concrete(reference) => self.require_exact_kind(
                reference.package,
                OwnerKey::Requirement(reference.requirement),
                &[OwnerKind::Requirement],
                "concrete requirement",
            ),
            super::RequirementOperand::Parameter(reference) => {
                if reference.package != self.snapshot.root.package_id
                    || !matches!(self.snapshot.owners.get(&OwnerKey::RequirementParameter(reference.parameter)), Some(OwnerRecord::RequirementParameter(record)) if Some(record.declaration) == self.semantic_declaration(source))
                {
                    self.error(
                        "kernel_requirement_parameter_scope",
                        "requirement parameter is outside the exact owning function",
                    );
                }
            }
        }
    }

    fn validate_effect_row(&mut self, source: OwnerKey, row: &super::EffectRow) {
        if let Err(error) = row.validate() {
            self.diagnostics.push(error);
            return;
        }
        for requirement in &row.requirements {
            if !self.consume_work() {
                return;
            }
            self.validate_requirement_operand(source, *requirement);
        }
        for parameter in &row.parameters {
            if !self.consume_work() {
                return;
            }
            if parameter.package != self.snapshot.root.package_id
                || !matches!(self.snapshot.owners.get(&OwnerKey::EffectParameter(parameter.parameter)),
                    Some(OwnerRecord::EffectParameter(record)) if Some(record.declaration) == self.semantic_declaration(source))
            {
                self.error(
                    "kernel_effect_parameter_scope",
                    format!(
                        "effect parameter {}/{} is outside the exact owning function",
                        parameter.package, parameter.parameter
                    ),
                );
            }
        }
    }

    fn validate_resource_shapes(&mut self) {
        let types = self
            .snapshot
            .types
            .iter()
            .map(|(digest, object)| (*digest, object.clone()))
            .collect::<Vec<_>>();
        for (digest, object) in types {
            if !self.consume_work() {
                return;
            }
            let view = super::owned_associated::read_view_admission::direct(&object.form);
            if !view {
                match self.with_associated_read(|read, admission| super::owned_associated::read_view_admission::contains(read, digest, admission)) {
                    Ok(true) => self.error("kernel_read_view_container", format!("type object {digest} contains a read view outside an explicit view occurrence")),
                    Ok(false) => {}
                    Err(error) => self.diagnostics.push(error),
                }
                if self.exhausted() {
                    return;
                }
            }
            let nominal_owned = match super::owned_record::is_owned(
                &self.read,
                self.snapshot.root.package_id,
                digest,
            ) {
                Ok(owned) => owned,
                Err(error) => {
                    self.diagnostics.push(error);
                    return;
                }
            };
            if !matches!(
                object.form,
                TypeForm::OwnedProduct { .. }
                    | TypeForm::OwnedChoice { .. }
                    | TypeForm::OwnedSequence { .. }
            ) && !nominal_owned
                && !view
                && object.child_types().into_iter().any(|t| {
                    self.snapshot
                        .types
                        .get(&t)
                        .or_else(|| self.snapshot.dependency_types.get(&t))
                        .is_some_and(|t| {
                            matches!(
                                t.form,
                                TypeForm::ByteBuffer
                                    | TypeForm::OwnedI64Cell
                                    | TypeForm::OwnedProduct { .. }
                                    | TypeForm::OwnedChoice { .. }
                                    | TypeForm::OwnedSequence { .. }
                            )
                        })
                })
            {
                self.error("kernel_buffer_container", format!("type object {digest} contains ByteBuffer in an unsupported container or callable descriptor"));
            }
            let forbidden = match &object.form {
                TypeForm::StructuralRecord { fields } => fields
                    .iter()
                    .any(|field| self.type_contains_resource(field.ty)),
                TypeForm::List { item } | TypeForm::Option { item } | TypeForm::Stream { item } => {
                    self.type_contains_resource(*item)
                }
                TypeForm::Map { key, value }
                | TypeForm::Result {
                    ok: key,
                    error: value,
                } => self.type_contains_resource(*key) || self.type_contains_resource(*value),
                TypeForm::Function { parameters, result }
                | TypeForm::TaskFunction {
                    parameters, result, ..
                } => {
                    parameters
                        .iter()
                        .any(|parameter| self.type_contains_resource(*parameter))
                        || self.type_contains_resource(*result)
                }
                _ => false,
            };
            if self.exhausted() {
                return;
            }
            if forbidden {
                self.error(
                    "kernel_affine_resource_container",
                    format!(
                        "type object {digest} places a capability resource in a forbidden structural, collection, stream, result, option, or function container"
                    ),
                );
            }
        }

        let owners = self
            .snapshot
            .owners
            .iter()
            .map(|(owner, record)| (*owner, record.clone()))
            .collect::<Vec<_>>();
        for (owner, record) in owners {
            if !self.consume_work() {
                return;
            }
            let forbidden_views = match &record {
                OwnerRecord::Field(field) => vec![field.ty],
                OwnerRecord::Case(case) => case.payload.into_iter().collect(),
                OwnerRecord::Operation(operation) => vec![operation.result],
                OwnerRecord::Port(port) => vec![port.function_type],
                OwnerRecord::Parameter(parameter) => {
                    let direct_view = self
                        .snapshot
                        .types
                        .get(&parameter.ty)
                        .or_else(|| self.snapshot.dependency_types.get(&parameter.ty))
                        .is_some_and(|object| {
                            super::owned_associated::read_view_admission::direct(&object.form)
                        });
                    if direct_view {
                        if !matches!(parameter.parent, ParameterParent::Function(_))
                            || parameter.use_mode != ParameterUse::Borrow
                            || parameter.resource_requirement.is_some()
                        {
                            self.error("kernel_read_view_parameter", "view parameters require an exact named function borrow without capability binding");
                        }
                        Vec::new()
                    } else {
                        vec![parameter.ty]
                    }
                }
                OwnerRecord::Declaration(declaration) => match &declaration.payload {
                    DeclarationPayload::External(function) => vec![function.result],
                    DeclarationPayload::Constant { ty, .. } => vec![*ty],
                    DeclarationPayload::Function(function)
                        if matches!(function.result.relation, ResultRelation::Value) =>
                    {
                        vec![function.result.ty]
                    }
                    DeclarationPayload::ReadView(view) => view
                        .fields
                        .iter()
                        .filter(|field| matches!(field.kind, ReadViewFieldKind::Ordinary))
                        .map(|field| field.ty)
                        .collect(),
                    _ => Vec::new(),
                },
                _ => Vec::new(),
            };
            for ty in forbidden_views {
                match self.with_associated_read(|read, admission| super::owned_associated::read_view_admission::contains(read, ty, admission)) {
                    Ok(true) => self.error("kernel_read_view_container", format!("owner {owner:?} stores or returns a read view in a forbidden value position")),
                    Ok(false) => {}
                    Err(error) => self.diagnostics.push(error),
                }
                if self.exhausted() {
                    return;
                }
            }
            match record {
                OwnerRecord::Parameter(ref parameter) => {
                    let owned = match self.parameter_owns_storage(owner, &record) {
                        Ok(owned) => owned,
                        Err(error) => {
                            self.diagnostics.push(error);
                            if self.exhausted() {
                                return;
                            }
                            continue;
                        }
                    };
                    if owned {
                        continue;
                    }
                    let contains = self.type_contains_resource(parameter.ty);
                    if self.exhausted() {
                        return;
                    }
                    let direct = snapshot_type_is_direct_resource(self.snapshot, parameter.ty);
                    match parameter.parent {
                        ParameterParent::Function(_) => {
                            if direct {
                                if parameter.use_mode == ParameterUse::Unrestricted {
                                    self.error(
                                        "kernel_affine_function_resource_use",
                                        format!(
                                            "direct resource function parameter {owner:?} must borrow or consume"
                                        ),
                                    );
                                }
                                if parameter.resource_requirement.is_none() {
                                    self.error(
                                        "kernel_affine_function_resource_requirement",
                                        format!(
                                            "direct resource function parameter {owner:?} requires one exact requirement binding"
                                        ),
                                    );
                                }
                            } else if contains {
                                self.error(
                                    "kernel_affine_function_parameter_container",
                                    format!(
                                        "function parameter {owner:?} may contain a resource only as its direct type"
                                    ),
                                );
                            } else if parameter.use_mode != ParameterUse::Unrestricted {
                                self.error(
                                    "kernel_affine_function_parameter_use",
                                    format!(
                                        "nonresource function parameter {owner:?} must be unrestricted"
                                    ),
                                );
                            }
                            if !direct && parameter.resource_requirement.is_some() {
                                self.error(
                                    "kernel_affine_parameter_requirement_extra",
                                    format!(
                                        "nonresource function parameter {owner:?} cannot bind a resource requirement"
                                    ),
                                );
                            }
                        }
                        ParameterParent::Operation(_) => {
                            if parameter.resource_requirement.is_some() {
                                self.error(
                                    "kernel_affine_parameter_requirement_extra",
                                    format!(
                                        "operation parameter {owner:?} cannot bind a function resource requirement"
                                    ),
                                );
                            }
                            if direct {
                                if parameter.use_mode == ParameterUse::Unrestricted {
                                    self.error(
                                        "kernel_affine_resource_parameter_use",
                                        format!(
                                            "resource operation parameter {owner:?} must explicitly borrow or consume"
                                        ),
                                    );
                                }
                            } else {
                                if contains {
                                    self.error(
                                        "kernel_affine_operation_parameter_container",
                                        format!(
                                            "operation parameter {owner:?} may contain a resource only as the direct parameter type"
                                        ),
                                    );
                                }
                                if parameter.use_mode != ParameterUse::Unrestricted {
                                    self.error(
                                        "kernel_affine_nonresource_parameter_use",
                                        format!(
                                            "nonresource operation parameter {owner:?} must be unrestricted"
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
                OwnerRecord::Field(field) => {
                    if self.type_contains_resource(field.ty) {
                        self.error(
                            "kernel_affine_record_field",
                            format!("record field {owner:?} cannot contain a capability resource"),
                        );
                    }
                }
                OwnerRecord::Case(case) => {
                    if let Some(payload) = case.payload
                        && self.type_contains_resource(payload)
                        && !snapshot_type_is_direct_resource(self.snapshot, payload)
                    {
                        self.error(
                            "kernel_affine_variant_payload",
                            format!(
                                "variant case {owner:?} may carry only one direct capability resource"
                            ),
                        );
                    }
                }
                OwnerRecord::Operation(operation) => {
                    if let Some(interface) =
                        snapshot_resource_interface(self.snapshot, operation.result)
                        && (interface.package != self.snapshot.root.package_id
                            || interface.declaration != operation.declaration)
                    {
                        self.error(
                            "kernel_affine_operation_result_interface",
                            format!(
                                "resource result of operation {owner:?} is not bound to its exact owning interface"
                            ),
                        );
                    }
                }
                OwnerRecord::Declaration(declaration) => match declaration.payload {
                    DeclarationPayload::Variant { cases, .. } => {
                        let resource_cases = cases
                            .iter()
                            .filter(|case| {
                                self.snapshot
                                    .owners
                                    .get(&OwnerKey::Case(**case))
                                    .and_then(|record| match record {
                                        OwnerRecord::Case(record) => record.payload,
                                        _ => None,
                                    })
                                    .is_some_and(|payload| {
                                        snapshot_type_is_direct_resource(self.snapshot, payload)
                                    })
                            })
                            .count();
                        if resource_cases > 1 {
                            self.error(
                                "kernel_affine_variant_resource_count",
                                format!(
                                    "variant declaration {owner:?} contains more than one direct resource payload"
                                ),
                            );
                        }
                    }
                    DeclarationPayload::External(signature) => {
                        if self.type_contains_resource(signature.result) {
                            self.error(
                                "kernel_affine_external_result",
                                format!(
                                    "external declaration {owner:?} cannot return a capability resource"
                                ),
                            );
                        }
                    }
                    DeclarationPayload::Function(function) => {
                        if self.type_contains_resource(function.result.ty) {
                            self.error(
                                "kernel_affine_function_result",
                                format!(
                                    "function declaration {owner:?} cannot return a capability resource"
                                ),
                            );
                        }
                        if self.exhausted() {
                            return;
                        }
                        let resource_parameters = function
                            .parameters
                            .iter()
                            .enumerate()
                            .filter_map(|(index, parameter)| {
                                let Some(OwnerRecord::Parameter(record)) =
                                    self.snapshot.owners.get(&OwnerKey::Parameter(*parameter))
                                else {
                                    return None;
                                };
                                snapshot_type_is_direct_resource(self.snapshot, record.ty)
                                    .then_some((index, *parameter, record))
                            })
                            .collect::<Vec<_>>();
                        if !resource_parameters.is_empty() {
                            let (index, parameter, _) = resource_parameters[0];
                            if index.saturating_add(resource_parameters.len())
                                != function.parameters.len()
                            {
                                self.error(
                                    "kernel_affine_function_resource_order",
                                    format!(
                                        "resource parameter {parameter} must belong to a contiguous suffix of its function signature"
                                    ),
                                );
                            }
                            if !function.requirement_parameters.is_empty() {
                                self.error(
                                    "kernel_affine_function_resource_generic",
                                    format!(
                                        "resource-bearing function declaration {owner:?} cannot have requirement parameters"
                                    ),
                                );
                            }
                            let FunctionEffect::Task {
                                effect_parameters: _,
                                requirements,
                            } = &function.effect
                            else {
                                self.error(
                                    "kernel_affine_function_resource_effect",
                                    format!(
                                        "resource-bearing function declaration {owner:?} must be a task"
                                    ),
                                );
                                continue;
                            };
                            for (_, parameter, record) in resource_parameters {
                                let Some(requirement) = record.resource_requirement else {
                                    continue;
                                };
                                if !requirements.contains(&requirement.into()) {
                                    self.error(
                                    "kernel_affine_function_resource_effect",
                                    format!(
                                        "resource parameter {parameter} binding is absent from its function effect"
                                    ),
                                );
                                }
                                let Some(interface) =
                                    snapshot_resource_interface(self.snapshot, record.ty)
                                else {
                                    continue;
                                };
                                let key = OwnerKey::Requirement(requirement.requirement);
                                let bound_interface =
                                    if requirement.package == self.snapshot.root.package_id {
                                        match self.snapshot.owners.get(&key) {
                                            Some(OwnerRecord::Requirement(bound)) => {
                                                Some(bound.interface)
                                            }
                                            _ => None,
                                        }
                                    } else {
                                        self.snapshot
                                            .dependencies
                                            .get(&requirement.package)
                                            .and_then(|dependency| {
                                                self.snapshot
                                                    .dependency_interfaces
                                                    .get(&dependency.package_revision)
                                            })
                                            .and_then(|owners| owners.get(&key))
                                            .and_then(|record| match record {
                                                PackageInterfaceRecord::Requirement(bound) => {
                                                    Some(bound.interface)
                                                }
                                                _ => None,
                                            })
                                    };
                                match bound_interface {
                                Some(bound) if bound == interface => {}
                                Some(_) => self.error(
                                    "kernel_affine_function_resource_interface",
                                    format!(
                                        "resource parameter {parameter} type disagrees with its exact requirement interface"
                                    ),
                                ),
                                None => self.error(
                                    "kernel_affine_function_resource_requirement",
                                    format!("resource parameter {parameter} binds an unavailable exact requirement"),
                                ),
                            }
                            }
                        }
                    }
                    DeclarationPayload::Constant { ty, .. } if self.type_contains_resource(ty) => {
                        self.error(
                            "kernel_affine_constant",
                            format!(
                                "constant declaration {owner:?} cannot contain a capability resource"
                            ),
                        );
                    }
                    _ => {}
                },
                OwnerRecord::Port(port) if self.type_contains_resource(port.function_type) => {
                    self.error(
                        "kernel_affine_port_signature",
                        format!("port {owner:?} cannot transfer a capability resource"),
                    );
                }
                _ => {}
            }
        }
    }

    fn semantic_declaration(
        &self,
        owner: OwnerKey,
    ) -> Option<crate::platform::semantic_id::DeclarationId> {
        let mut current = owner;
        for _ in 0..=MAXIMUM_EXPRESSION_DEPTH {
            match current {
                OwnerKey::Declaration(declaration) => return Some(declaration),
                OwnerKey::RequirementParameter(parameter) => {
                    return match self
                        .snapshot
                        .owners
                        .get(&OwnerKey::RequirementParameter(parameter))
                    {
                        Some(OwnerRecord::RequirementParameter(record)) => Some(record.declaration),
                        _ => None,
                    };
                }
                OwnerKey::EffectParameter(parameter) => {
                    return match self
                        .snapshot
                        .owners
                        .get(&OwnerKey::EffectParameter(parameter))
                    {
                        Some(OwnerRecord::EffectParameter(record)) => Some(record.declaration),
                        _ => None,
                    };
                }
                OwnerKey::TypeParameter(parameter) => {
                    return match self
                        .snapshot
                        .owners
                        .get(&OwnerKey::TypeParameter(parameter))
                    {
                        Some(OwnerRecord::TypeParameter(record)) => Some(record.declaration),
                        _ => None,
                    };
                }
                OwnerKey::Field(field) => {
                    return match self.snapshot.owners.get(&OwnerKey::Field(field)) {
                        Some(OwnerRecord::Field(record)) => Some(record.declaration),
                        _ => None,
                    };
                }
                OwnerKey::Case(case) => {
                    return match self.snapshot.owners.get(&OwnerKey::Case(case)) {
                        Some(OwnerRecord::Case(record)) => Some(record.declaration),
                        _ => None,
                    };
                }
                OwnerKey::Operation(operation) => {
                    return self.operation_parent(operation);
                }
                OwnerKey::Parameter(parameter) => {
                    return match self.snapshot.owners.get(&OwnerKey::Parameter(parameter)) {
                        Some(OwnerRecord::Parameter(record)) => match record.parent {
                            ParameterParent::Function(declaration) => Some(declaration),
                            ParameterParent::Operation(operation) => {
                                self.operation_parent(operation)
                            }
                        },
                        _ => None,
                    };
                }
                OwnerKey::Binding(binding) => {
                    let container = self.binding_containers.get(&binding)?.first()?;
                    current = OwnerKey::Expression(container.expression);
                }
                OwnerKey::Expression(expression) => {
                    current = *self.expression_root_owners.get(&expression)?;
                }
                OwnerKey::Requirement(requirement) => {
                    return match self
                        .snapshot
                        .owners
                        .get(&OwnerKey::Requirement(requirement))
                    {
                        Some(OwnerRecord::Requirement(record)) => Some(record.declaration),
                        _ => None,
                    };
                }
                OwnerKey::Port(port) => return self.port_parent(port),
                OwnerKey::Documentation(documentation) => {
                    current = match self
                        .snapshot
                        .owners
                        .get(&OwnerKey::Documentation(documentation))
                    {
                        Some(OwnerRecord::Documentation(record)) => record.owner,
                        _ => return None,
                    };
                }
                OwnerKey::Annotation(annotation) => {
                    current = match self.snapshot.owners.get(&OwnerKey::Annotation(annotation)) {
                        Some(OwnerRecord::Annotation(record)) => record.owner,
                        _ => return None,
                    };
                }
                OwnerKey::Module(_) | OwnerKey::Target(_) | OwnerKey::HttpRoute(_) => return None,
            }
        }
        None
    }

    fn validate_expressions(&mut self) {
        self.build_expression_ownership();
        self.validate_expression_parent_counts();
        if self.exhausted() {
            return;
        }
        self.validate_expression_cycles_and_depth();
        if self.exhausted() {
            return;
        }
        self.assign_expression_roots();
        if self.exhausted() {
            return;
        }
        self.validate_binding_ownership();
    }

    fn build_expression_ownership(&mut self) {
        let owners = self.snapshot.owners.iter().collect::<Vec<_>>();
        for (owner, record) in owners {
            for root in record.expression_roots() {
                self.expression_parents
                    .entry(root)
                    .or_default()
                    .push(ExpressionParent::Root(*owner));
            }
            let OwnerRecord::Expression(expression) = record else {
                continue;
            };
            for child in expression.children() {
                self.expression_parents
                    .entry(child.expression)
                    .or_default()
                    .push(ExpressionParent::Expression(expression.id));
            }
            self.collect_binding_containers(expression.id, &expression.operation);
        }
    }

    fn collect_binding_containers(
        &mut self,
        expression: ExpressionId,
        operation: &ExpressionOperation,
    ) {
        match operation {
            ExpressionOperation::BorrowReadView { binding, body, .. } => {
                self.binding_containers
                    .entry(*binding)
                    .or_default()
                    .push(BindingContainer {
                        expression,
                        scope_roots: vec![*body],
                        kind: BindingContainerKind::ReadViewBorrow,
                    });
            }
            ExpressionOperation::BorrowReadViewField {
                field,
                binding,
                body,
                ..
            } => {
                let kind = self.with_associated_read(|read, _| {
                    let Some(OwnerRecord::Declaration(record)) =
                        read.owner(OwnerKey::Declaration(field.declaration.declaration))?
                    else {
                        return Err(super::owned_associated::read_view_admission::reject(
                            "missing exact view reference-field declaration",
                        ));
                    };
                    if field.declaration.package != read.package_id()
                        || record.header.owner
                            != OwnerKey::Declaration(field.declaration.declaration)
                    {
                        return Err(super::owned_associated::read_view_admission::reject(
                            "view reference field has a foreign exact declaration",
                        ));
                    }
                    let DeclarationPayload::ReadView(view) = record.payload else {
                        return Err(super::owned_associated::read_view_admission::reject(
                            "reference field is not a nominal view field",
                        ));
                    };
                    let mut found = view.fields.iter().filter(|entry| entry.id == field.field);
                    let entry = found.next().ok_or_else(|| {
                        super::owned_associated::read_view_admission::reject(
                            "missing exact view reference field",
                        )
                    })?;
                    if found.next().is_some()
                        || !matches!(entry.kind, ReadViewFieldKind::ReadReference { .. })
                    {
                        return Err(super::owned_associated::read_view_admission::reject(
                            "view borrow requires one exact declared read-reference field",
                        ));
                    }
                    let object = read.type_object(entry.ty)?.ok_or_else(|| {
                        super::owned_associated::read_view_admission::reject(
                            "missing exact view referent type",
                        )
                    })?;
                    Ok(
                        if super::owned_associated::read_view_admission::direct(&object.form) {
                            BindingContainerKind::ReadViewBorrow
                        } else {
                            BindingContainerKind::OwnedBorrow
                        },
                    )
                });
                match kind {
                    Ok(kind) => self.binding_containers.entry(*binding).or_default().push(
                        BindingContainer {
                            expression,
                            scope_roots: vec![*body],
                            kind,
                        },
                    ),
                    Err(error) => self.diagnostics.push(error),
                }
            }
            ExpressionOperation::BorrowCall {
                call,
                binding,
                body,
            } => {
                let kind = self.with_associated_read(|read, _| {
                    let Some(OwnerRecord::Expression(call)) =
                        read.owner(OwnerKey::Expression(*call))?
                    else {
                        return Err(super::owned_associated::read_view_admission::reject(
                            "missing exact scoped call",
                        ));
                    };
                    let relation = match call.operation {
                        ExpressionOperation::Call { function, .. }
                        | ExpressionOperation::ImplementationCall { function, .. } => {
                            super::owned_contract::function_contract(read, function)?
                                .result
                                .relation
                        }
                        ExpressionOperation::MethodCall {
                            contract, method, ..
                        } => {
                            let contract = super::owned_contract::contract_record(read, contract)?;
                            let mut methods =
                                contract.methods.iter().filter(|entry| entry.id == method);
                            let entry = methods.next().ok_or_else(|| {
                                super::owned_associated::read_view_admission::reject(
                                    "scoped call names no exact contract method",
                                )
                            })?;
                            if methods.next().is_some() {
                                return Err(super::owned_associated::read_view_admission::reject(
                                    "duplicate exact scoped contract method",
                                ));
                            }
                            // Method source positions and function source IDs have different
                            // coordinates. Only the relation kind is compared here.
                            return match entry.result.relation {
                                ResultRelation::ViewFrom { .. } => {
                                    Ok(BindingContainerKind::ReadView)
                                }
                                ResultRelation::ReadFrom { .. } => {
                                    Ok(BindingContainerKind::OwnedBorrow)
                                }
                                ResultRelation::Value => {
                                    Err(super::owned_associated::read_view_admission::reject(
                                        "scoped call requires a declared source-tied result",
                                    ))
                                }
                            };
                        }
                        _ => {
                            return Err(super::owned_associated::read_view_admission::reject(
                                "scoped call requires a named function or exact method",
                            ));
                        }
                    };
                    match relation {
                        ResultRelation::ViewFrom { .. } => Ok(BindingContainerKind::ReadView),
                        ResultRelation::ReadFrom { .. } => Ok(BindingContainerKind::OwnedBorrow),
                        ResultRelation::Value => {
                            Err(super::owned_associated::read_view_admission::reject(
                                "scoped call requires a declared source-tied result",
                            ))
                        }
                    }
                });
                match kind {
                    Ok(kind) => self.binding_containers.entry(*binding).or_default().push(
                        BindingContainer {
                            expression,
                            scope_roots: vec![*body],
                            kind,
                        },
                    ),
                    Err(error) => self.diagnostics.push(error),
                }
            }
            ExpressionOperation::BorrowOwnedField { binding, body, .. }
            | ExpressionOperation::BorrowOwnedRecordField { binding, body, .. }
            | ExpressionOperation::BorrowOwnedItem { binding, body, .. } => {
                self.binding_containers
                    .entry(*binding)
                    .or_default()
                    .push(BindingContainer {
                        expression,
                        scope_roots: vec![*body],
                        kind: BindingContainerKind::OwnedBorrow,
                    });
            }
            ExpressionOperation::MatchBorrowedOwned { arms, .. } => {
                for arm in arms {
                    self.binding_containers
                        .entry(arm.binding)
                        .or_default()
                        .push(BindingContainer {
                            expression,
                            scope_roots: vec![arm.body],
                            kind: BindingContainerKind::OwnedBorrow,
                        });
                }
            }
            ExpressionOperation::MatchOwned { arms, .. } => {
                for arm in arms {
                    self.binding_containers
                        .entry(arm.binding)
                        .or_default()
                        .push(BindingContainer {
                            expression,
                            scope_roots: vec![arm.body],
                            kind: BindingContainerKind::OwnedChoicePayload,
                        });
                }
            }
            ExpressionOperation::UnpackOwned { fields, body, .. } => {
                for field in fields {
                    self.binding_containers
                        .entry(field.binding)
                        .or_default()
                        .push(BindingContainer {
                            expression,
                            scope_roots: vec![*body],
                            kind: BindingContainerKind::OwnedUnpack,
                        });
                }
            }
            ExpressionOperation::UnpackOwnedRecord { fields, body, .. } => {
                for field in fields {
                    self.binding_containers
                        .entry(field.binding)
                        .or_default()
                        .push(BindingContainer {
                            expression,
                            scope_roots: vec![*body],
                            kind: BindingContainerKind::OwnedUnpack,
                        });
                }
            }
            ExpressionOperation::Let { bindings, body } => {
                for (index, binding) in bindings.iter().enumerate() {
                    let kind = match self.snapshot.owners.get(&OwnerKey::Binding(*binding)) {
                        Some(OwnerRecord::Binding(record))
                            if record.declared_type.is_some_and(|ty| {
                                self.snapshot
                                    .types
                                    .get(&ty)
                                    .or_else(|| self.snapshot.dependency_types.get(&ty))
                                    .is_some_and(|object| {
                                        super::owned_associated::read_view_admission::direct(
                                            &object.form,
                                        )
                                    })
                            }) =>
                        {
                            BindingContainerKind::ReadViewLet
                        }
                        _ => BindingContainerKind::Let,
                    };
                    let mut scope_roots = bindings[index.saturating_add(1)..]
                        .iter()
                        .filter_map(|binding| {
                            match self.snapshot.owners.get(&OwnerKey::Binding(*binding)) {
                                Some(OwnerRecord::Binding(record)) => record.value,
                                _ => None,
                            }
                        })
                        .collect::<Vec<_>>();
                    scope_roots.push(*body);
                    self.binding_containers
                        .entry(*binding)
                        .or_default()
                        .push(BindingContainer {
                            expression,
                            scope_roots,
                            kind,
                        });
                }
            }
            ExpressionOperation::Match { arms, .. } => {
                for arm in arms {
                    if let Some(binding) = arm.payload_binding {
                        self.binding_containers.entry(binding).or_default().push(
                            BindingContainer {
                                expression,
                                scope_roots: vec![arm.body],
                                kind: BindingContainerKind::MatchPayload,
                            },
                        );
                    }
                }
            }
            ExpressionOperation::Transaction { binding, body, .. }
            | ExpressionOperation::TransactionOutcome { binding, body, .. } => {
                self.binding_containers
                    .entry(*binding)
                    .or_default()
                    .push(BindingContainer {
                        expression,
                        scope_roots: vec![*body],
                        kind: BindingContainerKind::Transaction,
                    });
            }
            _ => {}
        }
    }

    fn validate_expression_parent_counts(&mut self) {
        let expressions = self
            .snapshot
            .owners
            .iter()
            .filter_map(|(key, record)| match (key, record) {
                (OwnerKey::Expression(id), OwnerRecord::Expression(_)) => Some(*id),
                _ => None,
            })
            .collect::<Vec<_>>();
        for expression in expressions {
            if !self.consume_work() {
                return;
            }
            match self.expression_parents.get(&expression).map(Vec::len) {
                None | Some(0) => self.error(
                    "kernel_full_expression_unreachable",
                    format!("expression {expression} has no semantic parent"),
                ),
                Some(1) => {}
                Some(count) => self.error(
                    "kernel_full_expression_shared",
                    format!("expression {expression} has {count} semantic parents"),
                ),
            }
        }
        let referenced = self.expression_parents.keys().copied().collect::<Vec<_>>();
        for expression in referenced {
            self.require_local_kind(
                OwnerKey::Expression(expression),
                &[OwnerKind::Expression],
                "expression child or root",
            );
        }
    }

    fn validate_expression_cycles_and_depth(&mut self) {
        let expressions = self
            .snapshot
            .owners
            .values()
            .filter_map(|record| match record {
                OwnerRecord::Expression(expression) => Some(expression.id),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut complete = BTreeSet::new();
        for start in expressions {
            if complete.contains(&start) {
                continue;
            }
            let mut active = BTreeSet::new();
            let mut pending = vec![(start, false, 0_usize)];
            while let Some((expression, exiting, depth)) = pending.pop() {
                if !self.consume_work() {
                    return;
                }
                if exiting {
                    active.remove(&expression);
                    complete.insert(expression);
                    continue;
                }
                if complete.contains(&expression) {
                    continue;
                }
                if depth > MAXIMUM_EXPRESSION_DEPTH {
                    self.error(
                        "kernel_full_expression_depth",
                        format!("expression {expression} exceeds the maximum depth"),
                    );
                    continue;
                }
                if !active.insert(expression) {
                    self.error(
                        "kernel_full_expression_cycle",
                        format!("expression {expression} participates in a cycle"),
                    );
                    continue;
                }
                let Some(OwnerRecord::Expression(record)) =
                    self.snapshot.owners.get(&OwnerKey::Expression(expression))
                else {
                    active.remove(&expression);
                    continue;
                };
                pending.push((expression, true, depth));
                let mut children = record.children();
                children.reverse();
                pending.extend(
                    children
                        .into_iter()
                        .map(|child| (child.expression, false, depth.saturating_add(1))),
                );
            }
        }
    }

    fn assign_expression_roots(&mut self) {
        let roots = self
            .expression_parents
            .iter()
            .filter_map(|(expression, parents)| match parents.as_slice() {
                [ExpressionParent::Root(owner)] => Some((*expression, *owner)),
                _ => None,
            })
            .collect::<Vec<_>>();
        for (root, owner) in roots {
            if !self.consume_work() {
                return;
            }
            let mut pending = vec![root];
            let mut visited = BTreeSet::new();
            while let Some(expression) = pending.pop() {
                if !self.consume_work() {
                    return;
                }
                if !visited.insert(expression) {
                    continue;
                }
                if self
                    .expression_root_owners
                    .insert(expression, owner)
                    .is_some_and(|previous| previous != owner)
                {
                    self.error(
                        "kernel_full_expression_root_conflict",
                        format!("expression {expression} belongs to multiple semantic roots"),
                    );
                }
                let Some(OwnerRecord::Expression(record)) =
                    self.snapshot.owners.get(&OwnerKey::Expression(expression))
                else {
                    continue;
                };
                pending.extend(record.children().into_iter().map(|child| child.expression));
            }
        }
    }

    fn validate_binding_ownership(&mut self) {
        let bindings = self
            .snapshot
            .owners
            .iter()
            .filter_map(|(key, record)| match (key, record) {
                (OwnerKey::Binding(binding), OwnerRecord::Binding(record)) => {
                    Some((*binding, record.kind))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        for (binding, kind) in bindings {
            if !self.consume_work() {
                return;
            }
            let containers = self
                .binding_containers
                .get(&binding)
                .map(Vec::as_slice)
                .unwrap_or_default();
            if containers.len() != 1 {
                self.error(
                    "kernel_full_binding_parent",
                    format!(
                        "binding {binding} has {} semantic containers",
                        containers.len()
                    ),
                );
                continue;
            }
            let expected = match containers[0].kind {
                BindingContainerKind::Let => BindingKind::Let,
                BindingContainerKind::MatchPayload => BindingKind::MatchPayload,
                BindingContainerKind::Transaction => BindingKind::Transaction,
                BindingContainerKind::OwnedUnpack => BindingKind::OwnedUnpack,
                BindingContainerKind::OwnedChoicePayload => BindingKind::OwnedChoicePayload,
                BindingContainerKind::OwnedBorrow => BindingKind::OwnedBorrow,
                BindingContainerKind::ReadView => BindingKind::ReadView,
                BindingContainerKind::ReadViewLet => BindingKind::ReadView,
                BindingContainerKind::ReadViewBorrow => BindingKind::ReadViewBorrow,
            };
            if kind != expected {
                self.error(
                    "kernel_full_binding_kind",
                    format!("binding {binding} kind disagrees with its expression role"),
                );
            }
        }
    }

    fn validate_references(&mut self) {
        let records = self.snapshot.owners.values().collect::<Vec<_>>();
        for record in records {
            if !self.consume_work() {
                return;
            }
            match record {
                OwnerRecord::Expression(expression) => {
                    self.validate_expression_references(expression.id, &expression.operation);
                }
                OwnerRecord::RequirementParameter(parameter) => {
                    self.require_exact_kind(
                        parameter.constraint.interface.package,
                        OwnerKey::Declaration(parameter.constraint.interface.declaration),
                        &[OwnerKind::Interface],
                        "requirement interface",
                    );
                    for operation in &parameter.constraint.operations {
                        self.require_exact_kind(
                            operation.package,
                            OwnerKey::Operation(operation.operation),
                            &[OwnerKind::Operation],
                            "requirement operation",
                        );
                        if operation.package != parameter.constraint.interface.package {
                            self.error(
                                "kernel_full_requirement_package",
                                "requirement interface and operations must belong to one package",
                            );
                        }
                        if self
                            .exact_operation_parent(operation.package, operation.operation)
                            .is_some_and(|parent| {
                                parent != parameter.constraint.interface.declaration
                            })
                        {
                            self.error(
                                "kernel_full_requirement_operation_owner",
                                "requirement operation does not belong to its interface",
                            );
                        }
                    }
                }
                OwnerRecord::Requirement(requirement) => {
                    self.require_exact_kind(
                        requirement.interface.package,
                        OwnerKey::Declaration(requirement.interface.declaration),
                        &[OwnerKind::Interface],
                        "requirement interface",
                    );
                    for operation in &requirement.operations {
                        self.require_exact_kind(
                            operation.package,
                            OwnerKey::Operation(operation.operation),
                            &[OwnerKind::Operation],
                            "requirement operation",
                        );
                        if operation.package != requirement.interface.package {
                            self.error(
                                "kernel_full_requirement_package",
                                "requirement interface and operations must belong to one package",
                            );
                        }
                        if self
                            .exact_operation_parent(operation.package, operation.operation)
                            .is_some_and(|parent| parent != requirement.interface.declaration)
                        {
                            self.error(
                                "kernel_full_requirement_operation_owner",
                                "requirement operation does not belong to its interface",
                            );
                        }
                    }
                }
                OwnerRecord::Port(port) => {
                    if let PortImplementation::Function(function) = &port.implementation {
                        self.require_exact_kind(
                            function.package,
                            OwnerKey::Declaration(function.declaration),
                            &[OwnerKind::PureFunction, OwnerKind::TaskFunction],
                            "port function",
                        );
                    }
                }
                OwnerRecord::Target(target) => {
                    self.require_exact_kind(
                        target.component.package,
                        OwnerKey::Declaration(target.component.declaration),
                        &[OwnerKind::Component],
                        "target component",
                    );
                    if let Some(port) = target.port {
                        self.require_exact_kind(
                            port.package,
                            OwnerKey::Port(port.port),
                            &[OwnerKind::Port],
                            "target port",
                        );
                        if target.component.package != port.package {
                            self.error(
                                "kernel_full_target_package",
                                "target component and port must belong to one package",
                            );
                        }
                        if port.package == self.snapshot.root.package_id
                            && self
                                .port_parent(port.port)
                                .is_some_and(|parent| parent != target.component.declaration)
                        {
                            self.error(
                                "kernel_full_target_port_owner",
                                "target port does not belong to its component",
                            );
                        }
                        if port.package == target.component.package
                            && port.package != self.snapshot.root.package_id
                        {
                            if !self.consume_work() {
                                return;
                            }
                            let owners = self.snapshot.dependencies.get(&port.package).and_then(
                                |dependency| {
                                    self.snapshot
                                        .dependency_interfaces
                                        .get(&dependency.package_revision)
                                },
                            );
                            if let Some(owners) = owners {
                                let parent = match owners.get(&OwnerKey::Port(port.port)) {
                                    Some(PackageInterfaceRecord::Port(record)) => {
                                        Some(record.declaration)
                                    }
                                    _ => None,
                                };
                                let listed = match owners
                                    .get(&OwnerKey::Declaration(target.component.declaration))
                                {
                                    Some(PackageInterfaceRecord::Declaration(record)) => {
                                        match &record.payload {
                                            PackageInterfaceDeclarationPayload::Component {
                                                ports,
                                                ..
                                            } => {
                                                let mut listed = false;
                                                for candidate in ports {
                                                    if !self.consume_work() {
                                                        return;
                                                    }
                                                    listed |= *candidate == port.port;
                                                }
                                                listed
                                            }
                                            _ => false,
                                        }
                                    }
                                    _ => false,
                                };
                                if !listed || parent != Some(target.component.declaration) {
                                    self.error(
                                        "kernel_full_target_port_owner",
                                        "target port does not belong to its component",
                                    );
                                }
                            }
                        }
                    }
                }
                OwnerRecord::HttpRoute(route) => self.require_exact_kind(
                    route.port.package,
                    OwnerKey::Port(route.port.port),
                    &[OwnerKind::Port],
                    "HTTP route port",
                ),
                OwnerRecord::Documentation(documentation) => {
                    self.validate_document_content(&documentation.content)
                }
                _ => {}
            }
        }
    }

    fn validate_implementation_operand(&mut self, operand: &super::ImplementationOperand) {
        if let Err(error) = operand.validate_local() {
            self.error(&error.code, error.message);
            return;
        }
        for operand in operand.walk() {
            if !self.consume_work() {
                return;
            }
            let (reference, kinds): (_, &[OwnerKind]) = match operand {
                super::ImplementationOperand::Concrete { implementation, .. } => {
                    (implementation, &[OwnerKind::OwnedImplementation])
                }
                super::ImplementationOperand::Parameter { scope, .. } => (
                    scope,
                    &[
                        OwnerKind::PureFunction,
                        OwnerKind::TaskFunction,
                        OwnerKind::OwnedImplementation,
                    ],
                ),
            };
            self.require_exact_kind(
                reference.package,
                OwnerKey::Declaration(reference.declaration),
                kinds,
                "static implementation operand",
            );
        }
    }

    fn validate_expression_references(
        &mut self,
        expression: ExpressionId,
        operation: &ExpressionOperation,
    ) {
        let source = OwnerKey::Expression(expression);
        let binder = self.semantic_declaration(source).map(|declaration| {
            ReadScopeBinder::Declaration(DeclarationReference {
                package: self.snapshot.root.package_id,
                declaration,
            })
        });
        match operation {
            ExpressionOperation::Call {
                read_scope_arguments,
                ..
            }
            | ExpressionOperation::ImplementationCall {
                read_scope_arguments,
                ..
            }
            | ExpressionOperation::FunctionValue {
                read_scope_arguments,
                ..
            }
            | ExpressionOperation::MethodCall {
                read_scope_arguments,
                ..
            } => {
                for argument in read_scope_arguments {
                    self.validate_read_scope_argument(source, *argument, binder);
                }
            }
            ExpressionOperation::CreateReadView {
                read_scope,
                source: value,
                ..
            } => {
                self.validate_read_scope_argument(source, *read_scope, binder);
                self.validate_local_reference(expression, *value);
            }
            ExpressionOperation::ReborrowReadView { read_scope, value } => {
                self.validate_read_scope_argument(source, *read_scope, binder);
                self.validate_local_reference(expression, *value);
            }
            ExpressionOperation::BorrowReadView { value, .. }
            | ExpressionOperation::MoveReadView { value }
            | ExpressionOperation::DropReadView { value }
            | ExpressionOperation::ReadViewDataField { value, .. }
            | ExpressionOperation::BorrowReadViewField { value, .. } => {
                self.validate_local_reference(expression, *value)
            }
            _ => {}
        }
        match operation {
            ExpressionOperation::CreateReadView {
                declaration,
                fields,
                ..
            } => {
                self.validate_read_view_access(expression, *declaration, None);
                if let Some(OwnerRecord::Declaration(record)) = self
                    .snapshot
                    .owners
                    .get(&OwnerKey::Declaration(declaration.declaration))
                    && let DeclarationPayload::ReadView(view) = &record.payload
                    && (fields.len() != view.fields.len()
                        || fields
                            .iter()
                            .zip(&view.fields)
                            .any(|(actual, formal)| actual.field != formal.id))
                {
                    self.error("kernel_read_view_field", "view construction requires the complete exact field inventory in authored order");
                }
            }
            ExpressionOperation::ReadViewDataField { field, .. }
            | ExpressionOperation::BorrowReadViewField { field, .. } => {
                self.validate_read_view_access(expression, field.declaration, Some(field.field))
            }
            ExpressionOperation::PackOwnedRecord { fields, .. } => {
                for field in fields {
                    self.require_exact_kind(
                        field.field.package,
                        OwnerKey::Field(field.field.field),
                        &[OwnerKind::Field],
                        "nominal owned construction field",
                    );
                }
            }
            ExpressionOperation::UnpackOwnedRecord { fields, .. } => {
                for field in fields {
                    self.require_exact_kind(
                        field.field.package,
                        OwnerKey::Field(field.field.field),
                        &[OwnerKind::Field],
                        "nominal owned decomposition field",
                    );
                }
            }
            ExpressionOperation::BorrowOwnedRecordField { field, .. } => {
                self.require_exact_kind(
                    field.package,
                    OwnerKey::Field(field.field),
                    &[OwnerKind::Field],
                    "nominal owned child field",
                );
            }
            ExpressionOperation::Text { value } | ExpressionOperation::StaticText { value } => {
                if let TextValue::Blob { digest, bytes } = value {
                    self.require_blob(*digest, *bytes, "text");
                }
            }
            ExpressionOperation::Local { value } => {
                self.validate_local_reference(expression, *value);
            }
            ExpressionOperation::Constant { declaration } => self.require_exact_kind(
                declaration.package,
                OwnerKey::Declaration(declaration.declaration),
                &[OwnerKind::Constant],
                "constant reference",
            ),
            ExpressionOperation::ImplementationCall {
                function,
                requirement_arguments,
                implementations,
                ..
            } => {
                for argument in requirement_arguments {
                    self.validate_requirement_operand(OwnerKey::Expression(expression), *argument);
                }
                self.require_exact_kind(
                    function.package,
                    OwnerKey::Declaration(function.declaration),
                    &[OwnerKind::PureFunction, OwnerKind::TaskFunction],
                    "implementation callee",
                );
                for operand in implementations {
                    self.validate_implementation_operand(operand);
                }
            }
            ExpressionOperation::MethodCall {
                witness, contract, ..
            } => {
                self.validate_implementation_operand(witness);
                self.require_exact_kind(
                    contract.package,
                    OwnerKey::Declaration(contract.declaration),
                    &[OwnerKind::OwnedContract],
                    "owned method contract",
                );
            }
            ExpressionOperation::Call {
                function,
                requirement_arguments,
                ..
            }
            | ExpressionOperation::FunctionValue {
                function,
                requirement_arguments,
                ..
            } => {
                for argument in requirement_arguments {
                    self.validate_requirement_operand(OwnerKey::Expression(expression), *argument);
                }
                self.require_exact_kind(
                    function.package,
                    OwnerKey::Declaration(function.declaration),
                    &[
                        OwnerKind::PureFunction,
                        OwnerKind::TaskFunction,
                        OwnerKind::External,
                    ],
                    "function reference",
                );
            }
            ExpressionOperation::Record {
                nominal_type: Some(declaration),
                fields,
                ..
            } => {
                self.require_exact_kind(
                    declaration.package,
                    OwnerKey::Declaration(declaration.declaration),
                    &[OwnerKind::Record],
                    "record type",
                );
                for field in fields {
                    if let FieldSelector::Nominal(field) = field.selector {
                        self.require_exact_kind(
                            field.package,
                            OwnerKey::Field(field.field),
                            &[OwnerKind::Field],
                            "record field",
                        );
                        if field.package != declaration.package {
                            self.error(
                                "kernel_full_record_field_package",
                                "nominal record and field belong to different packages",
                            );
                        }
                        if field.package == self.snapshot.root.package_id
                            && self
                                .field_parent(field.field)
                                .is_some_and(|parent| parent != declaration.declaration)
                        {
                            self.error(
                                "kernel_full_record_field_owner",
                                "nominal record field belongs to another declaration",
                            );
                        }
                    }
                }
            }
            ExpressionOperation::Variant { case, .. } => self.require_exact_kind(
                case.package,
                OwnerKey::Case(case.case),
                &[OwnerKind::Case],
                "variant case",
            ),
            ExpressionOperation::Field {
                selector: FieldSelector::Nominal(field),
                ..
            } => self.require_exact_kind(
                field.package,
                OwnerKey::Field(field.field),
                &[OwnerKind::Field],
                "field selector",
            ),
            ExpressionOperation::Match { arms, .. } => {
                for arm in arms {
                    self.require_exact_kind(
                        arm.case.package,
                        OwnerKey::Case(arm.case.case),
                        &[OwnerKind::Case],
                        "match case",
                    );
                }
            }
            ExpressionOperation::CapabilityCall {
                requirement,
                operation,
                ..
            } => {
                self.validate_requirement_operand(OwnerKey::Expression(expression), *requirement);
                self.require_exact_kind(
                    requirement.package(),
                    requirement.owner(),
                    &[OwnerKind::Requirement, OwnerKind::RequirementParameter],
                    "capability requirement",
                );
                self.require_exact_kind(
                    operation.package,
                    OwnerKey::Operation(operation.operation),
                    &[OwnerKind::Operation],
                    "capability operation",
                );
                if !self.requirement_allows(*requirement, *operation) {
                    self.error(
                        "kernel_full_capability_operation",
                        format!(
                            "capability operation {operation:?} is outside requirement {requirement:?}'s exact interface or allowed-operation set"
                        ),
                    );
                }
            }
            ExpressionOperation::Transaction { requirement, .. } => self.require_exact_kind(
                requirement.package(),
                requirement.owner(),
                &[OwnerKind::Requirement, OwnerKind::RequirementParameter],
                "transaction requirement",
            ),
            ExpressionOperation::TransactionOutcome {
                requirement,
                outcome,
                ..
            } => {
                self.require_exact_kind(
                    requirement.package(),
                    requirement.owner(),
                    &[OwnerKind::Requirement, OwnerKind::RequirementParameter],
                    "transaction requirement",
                );
                for declaration in [outcome.outcome, outcome.abort_reason] {
                    self.require_exact_kind(
                        declaration.package,
                        OwnerKey::Declaration(declaration.declaration),
                        &[OwnerKind::Variant],
                        "transaction outcome declaration",
                    );
                }
                for case in [
                    outcome.committed,
                    outcome.aborted,
                    outcome.condition_failed,
                    outcome.conflict,
                ] {
                    self.require_exact_kind(
                        case.package,
                        OwnerKey::Case(case.case),
                        &[OwnerKind::Case],
                        "transaction outcome case",
                    );
                }
            }
            _ => {}
        }
    }

    fn validate_read_view_access(
        &mut self,
        expression: ExpressionId,
        declaration: DeclarationReference,
        field: Option<crate::platform::semantic_id::FieldId>,
    ) {
        self.require_exact_kind(
            declaration.package,
            OwnerKey::Declaration(declaration.declaration),
            &[OwnerKind::ReadView],
            "read-view field declaration",
        );
        let caller = self
            .semantic_declaration(OwnerKey::Expression(expression))
            .map(|declaration| DeclarationReference {
                package: self.snapshot.root.package_id,
                declaration,
            });
        let result = self.with_associated_read(|read, admission| {
            let caller = caller.ok_or_else(|| {
                super::owned_associated::read_view_admission::reject(
                    "view operations require an exact enclosing declaration",
                )
            })?;
            super::owned_associated::read_view_admission::require_access(
                read,
                declaration,
                caller,
                admission,
            )?;
            if let Some(field) = field {
                let Some(OwnerRecord::Declaration(record)) =
                    read.owner(OwnerKey::Declaration(declaration.declaration))?
                else {
                    return Err(super::owned_associated::read_view_admission::reject(
                        "missing exact view field owner",
                    ));
                };
                let DeclarationPayload::ReadView(view) = record.payload else {
                    return Err(super::owned_associated::read_view_admission::reject(
                        "field owner is not a read view",
                    ));
                };
                let mut found = false;
                for entry in view.fields {
                    admission.step(read)?;
                    if entry.id == field {
                        if found {
                            return Err(super::owned_associated::read_view_admission::reject(
                                "duplicate exact view field identity",
                            ));
                        }
                        found = true;
                    }
                }
                if !found {
                    return Err(super::owned_associated::read_view_admission::reject(
                        "field is absent from the exact view inventory",
                    ));
                }
            }
            Ok(())
        });
        self.capture(result);
    }

    fn validate_local_reference(
        &mut self,
        expression: ExpressionId,
        reference: LocalValueReference,
    ) {
        match reference {
            LocalValueReference::FunctionParameter(parameter) => {
                self.require_local_kind(
                    OwnerKey::Parameter(parameter),
                    &[OwnerKind::Parameter],
                    "function parameter reference",
                );
                let Some(OwnerRecord::Parameter(parameter_record)) =
                    self.snapshot.owners.get(&OwnerKey::Parameter(parameter))
                else {
                    return;
                };
                if !matches!(parameter_record.parent, ParameterParent::Function(_)) {
                    self.error(
                        "kernel_full_local_parameter_domain",
                        "function-parameter reference names an operation parameter",
                    );
                }
            }
            LocalValueReference::OperationParameter(parameter) => {
                self.require_local_kind(
                    OwnerKey::Parameter(parameter),
                    &[OwnerKind::Parameter],
                    "operation parameter reference",
                );
                let Some(OwnerRecord::Parameter(parameter_record)) =
                    self.snapshot.owners.get(&OwnerKey::Parameter(parameter))
                else {
                    return;
                };
                if !matches!(parameter_record.parent, ParameterParent::Operation(_)) {
                    self.error(
                        "kernel_full_local_parameter_domain",
                        "operation-parameter reference names a function parameter",
                    );
                } else {
                    self.error(
                        "kernel_type_parameter_scope",
                        "operation parameter has no executable operation context",
                    );
                }
            }
            LocalValueReference::LexicalBinding(binding)
            | LocalValueReference::MatchPayload(binding)
            | LocalValueReference::TransactionBinding(binding) => {
                self.require_local_kind(
                    OwnerKey::Binding(binding),
                    &[OwnerKind::Binding],
                    "binding reference",
                );
                let Some(containers) = self.binding_containers.get(&binding) else {
                    return;
                };
                let Some(container) = containers.first().cloned() else {
                    return;
                };
                let expected = match reference {
                    LocalValueReference::LexicalBinding(_)
                        if matches!(
                            container.kind,
                            BindingContainerKind::ReadView
                                | BindingContainerKind::ReadViewLet
                                | BindingContainerKind::ReadViewBorrow
                        ) =>
                    {
                        container.kind
                    }
                    LocalValueReference::LexicalBinding(_)
                        if container.kind == BindingContainerKind::OwnedBorrow =>
                    {
                        BindingContainerKind::OwnedBorrow
                    }
                    LocalValueReference::LexicalBinding(_)
                        if container.kind == BindingContainerKind::OwnedChoicePayload =>
                    {
                        BindingContainerKind::OwnedChoicePayload
                    }
                    LocalValueReference::LexicalBinding(_)
                        if container.kind == BindingContainerKind::OwnedUnpack =>
                    {
                        BindingContainerKind::OwnedUnpack
                    }
                    LocalValueReference::LexicalBinding(_) => BindingContainerKind::Let,
                    LocalValueReference::MatchPayload(_) => BindingContainerKind::MatchPayload,
                    LocalValueReference::TransactionBinding(_) => BindingContainerKind::Transaction,
                    _ => return,
                };
                if container.kind != expected {
                    self.error(
                        "kernel_full_local_binding_domain",
                        "local reference uses the wrong binding domain",
                    );
                }
                if !container
                    .scope_roots
                    .iter()
                    .any(|scope_root| self.is_expression_descendant(expression, *scope_root))
                {
                    self.error(
                        "kernel_full_lexical_scope",
                        format!("expression {expression} uses binding {binding} outside its scope"),
                    );
                }
            }
        }
    }

    fn validate_relations(&mut self) {
        match extract_relations(
            self.snapshot.root.package_id,
            &self.snapshot.owners,
            &self.snapshot.types,
            &self.snapshot.dependencies,
        ) {
            Ok(relations) => {
                match extract_relations(
                    self.snapshot.root.package_id,
                    &self.snapshot.owners,
                    &self.snapshot.types,
                    &self.snapshot.dependencies,
                ) {
                    Ok(second) if second == relations => self.relations = relations,
                    Ok(_) => self.error(
                        "kernel_full_relation_nondeterministic",
                        "relation extraction returned different ordered edges",
                    ),
                    Err(diagnostic) => self.diagnostics.push(diagnostic),
                }
            }
            Err(diagnostic) => self.diagnostics.push(diagnostic),
        }
    }

    fn require_parent_listed(&mut self, child: OwnerKey, parent: OwnerKey, label: &str) {
        let Some(parent_record) = self.snapshot.owners.get(&parent) else {
            self.error(
                "kernel_full_parent_missing",
                format!("{label} parent {parent:?} is missing"),
            );
            return;
        };
        let listed = match (child, parent_record) {
            (OwnerKey::RequirementParameter(id), OwnerRecord::Declaration(declaration)) => {
                matches!(
                    &declaration.payload, DeclarationPayload::Function(function) if function.requirement_parameters.contains(&id)
                )
            }
            (OwnerKey::EffectParameter(id), OwnerRecord::Declaration(declaration)) => matches!(
                &declaration.payload, DeclarationPayload::Function(function) if function.effect_parameters.contains(&id)
            ),
            (OwnerKey::TypeParameter(id), OwnerRecord::Declaration(declaration)) => {
                match &declaration.payload {
                    DeclarationPayload::Record {
                        type_parameters, ..
                    }
                    | DeclarationPayload::OwnedRecord {
                        type_parameters, ..
                    }
                    | DeclarationPayload::Variant {
                        type_parameters, ..
                    } => type_parameters.contains(&id),
                    DeclarationPayload::OwnedContract(c) => {
                        c.self_parameter == id || c.type_parameters.contains(&id)
                    }
                    DeclarationPayload::OwnedImplementation(i) => i.type_parameters.contains(&id),
                    DeclarationPayload::ReadView(view) => view.type_parameters.contains(&id),
                    DeclarationPayload::External(function) => {
                        function.type_parameters.contains(&id)
                    }
                    DeclarationPayload::Function(function) => {
                        function.type_parameters.contains(&id)
                    }
                    _ => false,
                }
            }
            (OwnerKey::Field(id), OwnerRecord::Declaration(declaration)) => matches!(
                &declaration.payload,
                DeclarationPayload::Record { fields, .. } | DeclarationPayload::OwnedRecord { fields, .. } if fields.contains(&id)
            ),
            (OwnerKey::Case(id), OwnerRecord::Declaration(declaration)) => matches!(
                &declaration.payload,
                DeclarationPayload::Variant { cases, .. } if cases.contains(&id)
            ),
            (OwnerKey::Operation(id), OwnerRecord::Declaration(declaration)) => matches!(
                &declaration.payload,
                DeclarationPayload::Interface { operations } if operations.contains(&id)
            ),
            (OwnerKey::Parameter(id), OwnerRecord::Declaration(declaration)) => {
                match &declaration.payload {
                    DeclarationPayload::External(function) => function.parameters.contains(&id),
                    DeclarationPayload::Function(function) => function.parameters.contains(&id),
                    _ => false,
                }
            }
            (OwnerKey::Parameter(id), OwnerRecord::Operation(operation)) => {
                operation.parameters.contains(&id)
            }
            (OwnerKey::Requirement(id), OwnerRecord::Declaration(declaration)) => {
                match &declaration.payload {
                    DeclarationPayload::Component { requirements, .. } => {
                        requirements.contains(&id)
                    }
                    DeclarationPayload::Function(function) => match &function.effect {
                        FunctionEffect::Task {
                            effect_parameters: _,
                            requirements,
                        } => requirements.iter().any(|reference| {
                            reference.package() == self.snapshot.root.package_id
                                && reference.owner() == OwnerKey::Requirement(id)
                        }),
                        FunctionEffect::Pure => false,
                    },
                    _ => false,
                }
            }
            (OwnerKey::Port(id), OwnerRecord::Declaration(declaration)) => matches!(
                &declaration.payload,
                DeclarationPayload::Component { ports, .. } if ports.contains(&id)
            ),
            _ => false,
        };
        if !listed {
            self.error(
                "kernel_full_parent_mismatch",
                format!("{label} {child:?} is not listed by parent {parent:?}"),
            );
        }
    }

    fn require_parameter_parent(
        &mut self,
        parameter: crate::platform::semantic_id::ParameterId,
        expected: ParameterParent,
    ) {
        let Some(OwnerRecord::Parameter(record)) =
            self.snapshot.owners.get(&OwnerKey::Parameter(parameter))
        else {
            self.error(
                "kernel_full_parameter_missing",
                format!("parameter {parameter} is missing"),
            );
            return;
        };
        if record.parent != expected {
            self.error(
                "kernel_full_parameter_parent",
                format!("parameter {parameter} has the wrong semantic parent"),
            );
        }
    }

    fn require_owner(&mut self, owner: OwnerKey, label: &str) {
        if !self.snapshot.owners.contains_key(&owner) {
            self.error(
                "kernel_full_owner_missing",
                format!("{label} {owner:?} is missing"),
            );
        }
    }

    fn require_local_kind(&mut self, owner: OwnerKey, kinds: &[OwnerKind], label: &str) {
        let Some(record) = self.snapshot.owners.get(&owner) else {
            self.error(
                "kernel_full_reference_missing",
                format!("{label} {owner:?} is missing"),
            );
            return;
        };
        if !kinds.contains(&record.kind()) {
            self.error(
                "kernel_full_reference_kind",
                format!("{label} {owner:?} has kind {:?}", record.kind()),
            );
        }
    }

    fn require_exact_kind(
        &mut self,
        package: PackageId,
        owner: OwnerKey,
        kinds: &[OwnerKind],
        label: &str,
    ) {
        if package == self.snapshot.root.package_id {
            self.require_local_kind(owner, kinds, label);
        } else if let Some(dependency) = self.snapshot.dependencies.get(&package) {
            match self
                .snapshot
                .dependency_interfaces
                .get(&dependency.package_revision)
                .and_then(|owners| owners.get(&owner))
            {
                Some(PackageInterfaceRecord::RepresentationDeclaration(_)) => self.error(
                    "kernel_full_foreign_reference_private",
                    format!("{label} {owner:?} names private representation evidence rather than a public dependency declaration"),
                ),
                Some(record) if kinds.contains(&record.header().kind) => {}
                Some(record) => self.error(
                    "kernel_full_foreign_reference_kind",
                    format!(
                        "{label} {owner:?} has dependency-interface kind {:?}",
                        record.header().kind
                    ),
                ),
                None => self.error(
                    "kernel_full_foreign_reference_missing",
                    format!(
                        "{label} {owner:?} is absent from exact dependency interface {package}"
                    ),
                ),
            }
        } else {
            self.error(
                "kernel_full_foreign_package_missing",
                format!("{label} names unbound package {package}"),
            );
        }
    }

    fn operation_parent(
        &self,
        operation: crate::platform::semantic_id::OperationId,
    ) -> Option<crate::platform::semantic_id::DeclarationId> {
        match self.snapshot.owners.get(&OwnerKey::Operation(operation)) {
            Some(OwnerRecord::Operation(record)) => Some(record.declaration),
            _ => None,
        }
    }

    fn exact_operation_parent(
        &self,
        package: PackageId,
        operation: crate::platform::semantic_id::OperationId,
    ) -> Option<crate::platform::semantic_id::DeclarationId> {
        if package == self.snapshot.root.package_id {
            return self.operation_parent(operation);
        }
        let dependency = self.snapshot.dependencies.get(&package)?;
        match self
            .snapshot
            .dependency_interfaces
            .get(&dependency.package_revision)?
            .get(&OwnerKey::Operation(operation))?
        {
            PackageInterfaceRecord::Operation(record) => Some(record.declaration),
            _ => None,
        }
    }

    fn field_parent(
        &self,
        field: crate::platform::semantic_id::FieldId,
    ) -> Option<crate::platform::semantic_id::DeclarationId> {
        match self.snapshot.owners.get(&OwnerKey::Field(field)) {
            Some(OwnerRecord::Field(record)) => Some(record.declaration),
            _ => None,
        }
    }

    fn port_parent(
        &self,
        port: crate::platform::semantic_id::PortId,
    ) -> Option<crate::platform::semantic_id::DeclarationId> {
        match self.snapshot.owners.get(&OwnerKey::Port(port)) {
            Some(OwnerRecord::Port(record)) => Some(record.declaration),
            _ => None,
        }
    }

    fn requirement_allows(
        &self,
        requirement: super::RequirementOperand,
        operation: super::OperationReference,
    ) -> bool {
        let parts = if requirement.package() == self.snapshot.root.package_id {
            match self.snapshot.owners.get(&requirement.owner()) {
                Some(OwnerRecord::Requirement(record)) => {
                    Some((record.interface, &record.operations))
                }
                Some(OwnerRecord::RequirementParameter(record)) => {
                    Some((record.constraint.interface, &record.constraint.operations))
                }
                _ => None,
            }
        } else {
            self.snapshot
                .dependencies
                .get(&requirement.package())
                .and_then(|dependency| {
                    self.snapshot
                        .dependency_interfaces
                        .get(&dependency.package_revision)
                })
                .and_then(|owners| owners.get(&requirement.owner()))
                .and_then(|record| match record {
                    PackageInterfaceRecord::Requirement(record) => {
                        Some((record.interface, &record.operations))
                    }
                    PackageInterfaceRecord::RequirementParameter(record) => {
                        Some((record.constraint.interface, &record.constraint.operations))
                    }
                    _ => None,
                })
        };
        parts.is_some_and(|(interface, operations)| {
            interface.package == operation.package
                && operations.contains(&operation)
                && self.exact_operation_parent(operation.package, operation.operation)
                    == Some(interface.declaration)
        })
    }

    fn validate_document_content(&mut self, content: &super::owner::DocumentContent) {
        if let super::owner::DocumentContent::Blob { digest, bytes } = content {
            self.require_blob(*digest, *bytes, "documentation");
        }
    }

    fn require_blob(&mut self, digest: BlobObjectDigest, bytes: u64, label: &str) {
        match self.snapshot.blobs.get(&digest) {
            Some(actual) if *actual == bytes => {}
            Some(actual) => self.error(
                "kernel_full_blob_length",
                format!("{label} blob {digest} claims {bytes} bytes but contains {actual}"),
            ),
            None => self.error(
                "kernel_full_blob_missing",
                format!("{label} blob {digest} is missing"),
            ),
        }
    }

    fn is_expression_descendant(&self, expression: ExpressionId, scope_root: ExpressionId) -> bool {
        let mut current = expression;
        for _ in 0..=MAXIMUM_EXPRESSION_DEPTH {
            if current == scope_root {
                return true;
            }
            match self.expression_parents.get(&current).map(Vec::as_slice) {
                Some([ExpressionParent::Expression(parent)]) => current = *parent,
                Some([ExpressionParent::Root(OwnerKey::Binding(binding))]) => {
                    let Some([container]) = self.binding_containers.get(binding).map(Vec::as_slice)
                    else {
                        return false;
                    };
                    current = container.expression;
                }
                _ => return false,
            }
        }
        false
    }

    fn compare_map_count(&mut self, label: &str, recorded: u64, logical: usize) {
        if recorded != logical as u64 {
            self.error(
                "kernel_full_map_count",
                format!(
                    "{label} map records {recorded} entries but logical authority has {logical}"
                ),
            );
        }
    }

    fn capture(&mut self, result: Result<(), Diagnostic>) {
        if let Err(diagnostic) = result {
            self.diagnostics.push(diagnostic);
        }
    }

    fn consume_work(&mut self) -> bool {
        if self.interrupted {
            return false;
        }
        if let Err(error) = (self.read.checkpoint)() {
            self.diagnostics.push(error);
            self.interrupted = true;
            return false;
        }
        self.work = self.work.saturating_add(1);
        if self.work > self.maximum_work {
            if !self
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "kernel_full_work")
            {
                self.diagnostics.push(Diagnostic::new(
                    DiagnosticClass::Resource,
                    "kernel_full_work",
                    "full validation exhausted its explicit work budget",
                ));
            }
            return false;
        }
        true
    }

    fn parameter_owns_storage(
        &mut self,
        key: OwnerKey,
        record: &OwnerRecord,
    ) -> Result<bool, Diagnostic> {
        let OwnerRecord::Parameter(parameter) = record else {
            return Err(Diagnostic::new(
                DiagnosticClass::Corrupt,
                "kernel_full_parameter_shape",
                "parameter storage admission requires an exact parameter owner",
            ));
        };
        if record.header().owner != key
            || !matches!(key, OwnerKey::Parameter(_))
            || record.header().kind != OwnerKind::Parameter
        {
            return Err(Diagnostic::new(
                DiagnosticClass::Corrupt,
                "kernel_full_parameter_shape",
                "parameter storage admission has a mismatched canonical owner",
            ));
        }
        let work = std::cell::Cell::new(self.work);
        let interrupted = std::cell::Cell::new(false);
        let result = (|| {
            let checkpoint = || {
                if let Err(error) = (self.read.checkpoint)() {
                    interrupted.set(true);
                    return Err(error);
                }
                let next = work.get().checked_add(1).ok_or_else(full_work_exhausted)?;
                work.set(next);
                if next > self.maximum_work {
                    return Err(full_work_exhausted());
                }
                Ok(())
            };
            let read = super::infer::CheckedExpressionRead {
                read: self.snapshot,
                checkpoint: &checkpoint,
            };
            let occurrence =
                super::memory::owner_occurrence(&read, key, record)?.ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticClass::Corrupt,
                        "kernel_full_parameter_shape",
                        "parameter storage admission has no canonical occurrence context",
                    )
                })?;
            Ok(super::memory::direct_at(
                &read,
                parameter.ty,
                occurrence,
                &self.associated_admission,
            )? || super::memory::view_at(
                &read,
                parameter.ty,
                occurrence,
                &self.associated_admission,
            )?)
        })();
        self.work = work.get();
        self.interrupted |= interrupted.get();
        result
    }

    fn exhausted(&self) -> bool {
        self.work > self.maximum_work || self.interrupted
    }

    fn error(&mut self, code: &str, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::new(DiagnosticClass::Semantic, code, message));
    }
}

fn full_work_exhausted() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "kernel_full_work",
        "full validation exhausted its explicit work budget",
    )
}

#[cfg(test)]
#[path = "associated_parameter_shape_tests.rs"]
mod associated_parameter_shape_tests;

#[cfg(test)]
mod read_view_glue_tests {
    use super::*;
    use crate::platform::kernel::{
        CallableResult, DeclarationRecord, DeclarationVisibility, ExpressionRecord,
        FunctionDeclaration, Name, OwnerHeader, ReadScopeParameter, ReadScopeReference,
        ReadScopeSource, ReadViewDeclaration,
    };
    use crate::platform::semantic_id::{DeclarationId, ParameterId, ReadScopeParameterId};

    const SEED: &[u8] = b"read-view-full-kernel-glue";

    struct Fixture {
        source: KernelSnapshot,
        function: DeclarationReference,
        view: DeclarationReference,
        parameter: ParameterId,
        scope: ReadScopeArgument,
        view_type: TypeObjectDigest,
    }

    fn intern(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
        let object = TypeObject::new(form).unwrap();
        let digest = encode_type_object(&object).unwrap().0;
        source.types.insert(digest, object);
        digest
    }

    fn fixture() -> Fixture {
        let mut source = super::super::tests::transport_snapshot();
        let (module, module_record) = source
            .owners
            .iter()
            .find_map(|(key, record)| match key {
                OwnerKey::Module(module) => Some((*module, record.clone())),
                _ => None,
            })
            .unwrap();
        source.owners.clear();
        source
            .owners
            .insert(OwnerKey::Module(module), module_record);
        source.types.clear();
        let function = DeclarationReference {
            package: source.root.package_id,
            declaration: DeclarationId::migrate(SEED, 0),
        };
        let view = DeclarationReference {
            package: source.root.package_id,
            declaration: DeclarationId::migrate(SEED, 1),
        };
        let scope_parameter = ReadScopeParameter {
            id: ReadScopeParameterId::migrate(SEED, 0),
            name: Name::new("s").unwrap(),
        };
        let scope = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(function),
            parameter: scope_parameter.id,
        });
        let unit = intern(&mut source, TypeForm::Unit);
        source.owners.insert(
            OwnerKey::Declaration(view.declaration),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(view.declaration),
                    OwnerKind::ReadView,
                ),
                module,
                name: Name::new("View").unwrap(),
                visibility: DeclarationVisibility::Public,
                payload: DeclarationPayload::ReadView(ReadViewDeclaration {
                    type_parameters: vec![],
                    scope_parameter: ReadScopeParameter {
                        id: ReadScopeParameterId::migrate(SEED, 1),
                        name: Name::new("v").unwrap(),
                    },
                    fields: vec![],
                }),
            }),
        );
        let view_type = intern(
            &mut source,
            TypeForm::ReadViewApplied {
                declaration: view,
                arguments: vec![],
                read_scope: scope,
            },
        );
        let parameter = ParameterId::migrate(SEED, 0);
        source.owners.insert(
            OwnerKey::Parameter(parameter),
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
                parent: ParameterParent::Function(function.declaration),
                name: Name::new("source").unwrap(),
                ty: view_type,
                use_mode: ParameterUse::Borrow,
                resource_requirement: None,
            }),
        );
        let body = ExpressionId::migrate(SEED, 0);
        source.owners.insert(
            OwnerKey::Expression(body),
            OwnerRecord::Expression(
                ExpressionRecord::new(body, ExpressionOperation::Unit {}).unwrap(),
            ),
        );
        source.owners.insert(
            OwnerKey::Declaration(function.declaration),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(function.declaration),
                    OwnerKind::PureFunction,
                ),
                module,
                name: Name::new("read").unwrap(),
                visibility: DeclarationVisibility::Public,
                payload: DeclarationPayload::Function(FunctionDeclaration {
                    implementation_parameters: vec![],
                    requirement_parameters: vec![],
                    effect_parameters: vec![],
                    type_parameters: vec![],
                    parameters: vec![parameter],
                    result: CallableResult {
                        ty: unit,
                        relation: ResultRelation::Value,
                    },
                    read_scope_parameters: vec![scope_parameter],
                    read_scope_sources: vec![ReadScopeSource {
                        source: parameter,
                        scope,
                    }],
                    effect: FunctionEffect::Pure,
                    body,
                }),
            }),
        );
        Fixture {
            source,
            function,
            view,
            parameter,
            scope,
            view_type,
        }
    }

    fn healthy() -> Result<(), Diagnostic> {
        Ok(())
    }

    fn validator(source: &KernelSnapshot) -> FullValidator<'_> {
        FullValidator {
            snapshot: source,
            read: super::super::infer::CheckedExpressionRead {
                read: source,
                checkpoint: &healthy,
            },
            interrupted: false,
            diagnostics: vec![],
            work: 0,
            maximum_work: MAXIMUM_VALIDATION_WORK,
            associated_admission: super::super::owned_associated::OwnedAssociatedAdmission::new(),
            relations: vec![],
            expression_parents: BTreeMap::new(),
            expression_root_owners: BTreeMap::new(),
            binding_containers: BTreeMap::new(),
        }
    }

    #[test]
    fn borrowed_view_parameter_is_a_positive_scoped_category() {
        let f = fixture();
        let mut check = validator(&f.source);
        check.validate_resource_shapes();
        assert!(check.diagnostics.is_empty(), "{:?}", check.diagnostics);
        let owner = OwnerKey::Parameter(f.parameter);
        assert!(
            check
                .parameter_owns_storage(owner, &f.source.owners[&owner])
                .unwrap()
        );
    }

    #[test]
    fn explicit_owned_view_actual_is_not_an_ordinary_buffer_container() {
        use crate::platform::kernel::{TypeParameterConstraints, TypeParameterRecord};
        use crate::platform::semantic_id::TypeParameterId;
        let mut f = fixture();
        let formal = TypeParameterId::migrate(SEED, 0);
        let OwnerRecord::Declaration(view) = f
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(f.view.declaration))
            .unwrap()
        else {
            panic!("view");
        };
        let DeclarationPayload::ReadView(view) = &mut view.payload else {
            panic!("view");
        };
        view.type_parameters.push(formal);
        f.source.owners.insert(
            OwnerKey::TypeParameter(formal),
            OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(OwnerKey::TypeParameter(formal), OwnerKind::TypeParameter),
                declaration: f.view.declaration,
                name: Name::new("Self").unwrap(),
                constraints: TypeParameterConstraints::Owned,
            }),
        );
        let actual = intern(&mut f.source, TypeForm::ByteBuffer);
        let view_type = intern(
            &mut f.source,
            TypeForm::ReadViewApplied {
                declaration: f.view,
                arguments: vec![actual],
                read_scope: f.scope,
            },
        );
        let OwnerRecord::Parameter(parameter) = f
            .source
            .owners
            .get_mut(&OwnerKey::Parameter(f.parameter))
            .unwrap()
        else {
            panic!("parameter");
        };
        parameter.ty = view_type;
        f.source.types.remove(&f.view_type);
        let mut check = validator(&f.source);
        check.validate_resource_shapes();
        assert!(check.diagnostics.is_empty(), "{:?}", check.diagnostics);
        super::super::owned_associated::read_view_admission::validate_nominal_at(
            &f.source,
            view_type,
            super::super::owned_associated::OwnedTypeOccurrence::authored(
                Some(f.function),
                super::super::contract::GRAPH_CONTRACT_VERSION,
            ),
            &check.associated_admission,
        )
        .unwrap();
    }

    #[test]
    fn unrestricted_view_parameter_and_value_result_are_refused() {
        let mut f = fixture();
        let OwnerRecord::Parameter(parameter) = f
            .source
            .owners
            .get_mut(&OwnerKey::Parameter(f.parameter))
            .unwrap()
        else {
            panic!("parameter");
        };
        parameter.use_mode = ParameterUse::Unrestricted;
        let OwnerRecord::Declaration(function) = f
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(f.function.declaration))
            .unwrap()
        else {
            panic!("function");
        };
        let DeclarationPayload::Function(function) = &mut function.payload else {
            panic!("function");
        };
        function.result.ty = f.view_type;
        let mut check = validator(&f.source);
        check.validate_resource_shapes();
        assert!(
            check
                .diagnostics
                .iter()
                .any(|error| error.code == "kernel_read_view_parameter")
        );
        assert!(
            check
                .diagnostics
                .iter()
                .any(|error| error.code == "kernel_read_view_container")
        );
    }

    #[test]
    fn inactive_nested_view_container_is_refused() {
        let mut f = fixture();
        let option = intern(&mut f.source, TypeForm::Option { item: f.view_type });
        let mut check = validator(&f.source);
        check.validate_resource_shapes();
        assert!(
            check
                .diagnostics
                .iter()
                .any(|error| error.code == "kernel_read_view_container"
                    && error.message.contains(&option.to_string()))
        );
    }

    #[test]
    fn scoped_view_call_requires_the_owning_view_binding_role() {
        let mut f = fixture();
        let OwnerRecord::Declaration(function) = f
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(f.function.declaration))
            .unwrap()
        else {
            panic!("function");
        };
        let DeclarationPayload::Function(function) = &mut function.payload else {
            panic!("function");
        };
        function.result = CallableResult {
            ty: f.view_type,
            relation: ResultRelation::ViewFrom {
                source: f.parameter,
                scope: f.scope,
            },
        };
        let call = ExpressionId::migrate(SEED, 1);
        let binding = BindingId::migrate(SEED, 0);
        f.source.owners.insert(
            OwnerKey::Expression(call),
            OwnerRecord::Expression(
                ExpressionRecord::new(
                    call,
                    ExpressionOperation::Call {
                        function: f.function,
                        arguments: vec![],
                        type_arguments: vec![],
                        effect_arguments: vec![],
                        requirement_arguments: vec![],
                        read_scope_arguments: vec![f.scope],
                    },
                )
                .unwrap(),
            ),
        );
        f.source.owners.insert(
            OwnerKey::Binding(binding),
            OwnerRecord::Binding(super::super::BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
                name: Name::new("view").unwrap(),
                kind: BindingKind::ReadView,
                value: None,
                declared_type: Some(f.view_type),
            }),
        );
        let operation = ExpressionOperation::BorrowCall {
            call,
            binding,
            body: ExpressionId::migrate(SEED, 0),
        };
        let mut check = validator(&f.source);
        check.collect_binding_containers(ExpressionId::migrate(SEED, 2), &operation);
        check.validate_binding_ownership();
        assert!(check.diagnostics.is_empty(), "{:?}", check.diagnostics);
        assert_eq!(
            check.binding_containers[&binding][0].kind,
            BindingContainerKind::ReadView
        );
        let OwnerRecord::Binding(record) = f
            .source
            .owners
            .get_mut(&OwnerKey::Binding(binding))
            .unwrap()
        else {
            panic!("binding");
        };
        record.kind = BindingKind::OwnedBorrow;
        let mut check = validator(&f.source);
        check.collect_binding_containers(ExpressionId::migrate(SEED, 2), &operation);
        check.validate_binding_ownership();
        assert!(
            check
                .diagnostics
                .iter()
                .any(|error| error.code == "kernel_full_binding_kind")
        );
    }

    #[test]
    fn equal_scope_parameter_ids_do_not_import_a_foreign_binder() {
        let f = fixture();
        let ReadScopeArgument::Parameter(mut foreign) = f.scope else {
            panic!("formal");
        };
        foreign.binder = ReadScopeBinder::Declaration(f.view);
        let mut check = validator(&f.source);
        check.validate_read_scope_argument(
            OwnerKey::Declaration(f.function.declaration),
            ReadScopeArgument::Parameter(foreign),
            Some(ReadScopeBinder::Declaration(f.function)),
        );
        assert!(
            check
                .diagnostics
                .iter()
                .any(|error| error.code == "kernel_read_view")
        );
        let mut f = fixture();
        let constant = DeclarationReference {
            package: f.function.package,
            declaration: DeclarationId::migrate(SEED, 10),
        };
        let body = ExpressionId::migrate(SEED, 0);
        let unit = f
            .source
            .types
            .iter()
            .find_map(|(ty, object)| matches!(object.form, TypeForm::Unit).then_some(*ty))
            .unwrap();
        let module = match &f.source.owners[&OwnerKey::Declaration(f.function.declaration)] {
            OwnerRecord::Declaration(record) => record.module,
            _ => panic!("function"),
        };
        f.source.owners.insert(
            OwnerKey::Declaration(constant.declaration),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(constant.declaration),
                    OwnerKind::Constant,
                ),
                module,
                name: Name::new("constant").unwrap(),
                visibility: DeclarationVisibility::Private,
                payload: DeclarationPayload::Constant {
                    ty: unit,
                    value: body,
                },
            }),
        );
        let binding = BindingId::migrate(SEED, 10);
        f.source.owners.insert(
            OwnerKey::Binding(binding),
            OwnerRecord::Binding(super::super::BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
                name: Name::new("loan").unwrap(),
                kind: BindingKind::OwnedBorrow,
                value: None,
                declared_type: Some(unit),
            }),
        );
        let mut check = validator(&f.source);
        check
            .expression_root_owners
            .insert(body, OwnerKey::Declaration(constant.declaration));
        check.binding_containers.insert(
            binding,
            vec![BindingContainer {
                expression: ExpressionId::migrate(SEED, 10),
                scope_roots: vec![body],
                kind: BindingContainerKind::OwnedBorrow,
            }],
        );
        check.validate_read_scope_argument(
            OwnerKey::Expression(body),
            ReadScopeArgument::Lexical {
                callable: constant,
                binding,
            },
            Some(ReadScopeBinder::Declaration(constant)),
        );
        assert!(
            check
                .diagnostics
                .iter()
                .any(|error| error.message.contains("named Function"))
        );
    }

    #[test]
    fn owning_view_let_preserves_role_but_requires_exact_initializer_scope() {
        let mut f = fixture();
        let initializer = ExpressionId::migrate(SEED, 3);
        let parent = ExpressionId::migrate(SEED, 4);
        let binding = BindingId::migrate(SEED, 1);
        f.source.owners.insert(
            OwnerKey::Expression(initializer),
            OwnerRecord::Expression(
                ExpressionRecord::new(
                    initializer,
                    ExpressionOperation::CreateReadView {
                        declaration: f.view,
                        type_arguments: vec![],
                        read_scope: f.scope,
                        source: LocalValueReference::FunctionParameter(f.parameter),
                        fields: vec![],
                    },
                )
                .unwrap(),
            ),
        );
        f.source.owners.insert(
            OwnerKey::Binding(binding),
            OwnerRecord::Binding(super::super::BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
                name: Name::new("view").unwrap(),
                kind: BindingKind::ReadView,
                value: Some(initializer),
                declared_type: Some(f.view_type),
            }),
        );
        let mut check = validator(&f.source);
        check.collect_binding_containers(
            parent,
            &ExpressionOperation::Let {
                bindings: vec![binding],
                body: ExpressionId::migrate(SEED, 0),
            },
        );
        check.validate_binding_ownership();
        assert!(check.diagnostics.is_empty(), "{:?}", check.diagnostics);
        assert_eq!(
            check.binding_containers[&binding][0].kind,
            BindingContainerKind::ReadViewLet
        );
        check
            .expression_root_owners
            .insert(parent, OwnerKey::Declaration(f.function.declaration));
        check.validate_read_scope_argument(
            OwnerKey::Binding(binding),
            ReadScopeArgument::Lexical {
                callable: f.function,
                binding,
            },
            Some(ReadScopeBinder::Declaration(f.function)),
        );
        assert!(
            check
                .diagnostics
                .iter()
                .any(|error| error.code == "kernel_read_view")
        );
    }

    #[test]
    fn exact_view_initializer_scope_protects_source_without_self_readable_value() {
        let mut f = fixture();
        let initializer = ExpressionId::migrate(SEED, 5);
        let parent = ExpressionId::migrate(SEED, 6);
        let binding = BindingId::migrate(SEED, 2);
        let scope = ReadScopeArgument::Lexical {
            callable: f.function,
            binding,
        };
        let annotation = intern(
            &mut f.source,
            TypeForm::ReadViewApplied {
                declaration: f.view,
                arguments: vec![],
                read_scope: scope,
            },
        );
        f.source.owners.insert(
            OwnerKey::Expression(initializer),
            OwnerRecord::Expression(
                ExpressionRecord::new(
                    initializer,
                    ExpressionOperation::CreateReadView {
                        declaration: f.view,
                        type_arguments: vec![],
                        read_scope: scope,
                        source: LocalValueReference::FunctionParameter(f.parameter),
                        fields: vec![],
                    },
                )
                .unwrap(),
            ),
        );
        f.source.owners.insert(
            OwnerKey::Binding(binding),
            OwnerRecord::Binding(super::super::BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
                name: Name::new("view").unwrap(),
                kind: BindingKind::ReadView,
                value: Some(initializer),
                declared_type: Some(annotation),
            }),
        );
        f.source.owners.insert(
            OwnerKey::Expression(parent),
            OwnerRecord::Expression(
                ExpressionRecord::new(
                    parent,
                    ExpressionOperation::Let {
                        bindings: vec![binding],
                        body: ExpressionId::migrate(SEED, 0),
                    },
                )
                .unwrap(),
            ),
        );
        let OwnerRecord::Declaration(function) = f
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(f.function.declaration))
            .unwrap()
        else {
            panic!("function");
        };
        let DeclarationPayload::Function(function) = &mut function.payload else {
            panic!("function");
        };
        function.body = parent;
        let mut check = validator(&f.source);
        check.validate_expressions();
        check.validate_read_scope_argument(
            OwnerKey::Expression(initializer),
            scope,
            Some(ReadScopeBinder::Declaration(f.function)),
        );
        assert!(check.diagnostics.is_empty(), "{:?}", check.diagnostics);
        check.validate_local_reference(initializer, LocalValueReference::LexicalBinding(binding));
        assert!(
            check
                .diagnostics
                .iter()
                .any(|error| error.code == "kernel_full_lexical_scope")
        );
    }

    #[test]
    fn prospective_call_scope_is_metadata_permission_without_argument_or_value_rights() {
        let mut f = fixture();
        let argument = ExpressionId::migrate(SEED, 7);
        let call = ExpressionId::migrate(SEED, 8);
        let parent = ExpressionId::migrate(SEED, 9);
        let binding = BindingId::migrate(SEED, 3);
        let scope = ReadScopeArgument::Lexical {
            callable: f.function,
            binding,
        };
        f.source.owners.insert(
            OwnerKey::Expression(argument),
            OwnerRecord::Expression(
                ExpressionRecord::new(
                    argument,
                    ExpressionOperation::Local {
                        value: LocalValueReference::FunctionParameter(f.parameter),
                    },
                )
                .unwrap(),
            ),
        );
        f.source.owners.insert(
            OwnerKey::Expression(call),
            OwnerRecord::Expression(
                ExpressionRecord::new(
                    call,
                    ExpressionOperation::Call {
                        function: f.function,
                        arguments: vec![argument],
                        type_arguments: vec![],
                        effect_arguments: vec![],
                        requirement_arguments: vec![],
                        read_scope_arguments: vec![scope],
                    },
                )
                .unwrap(),
            ),
        );
        f.source.owners.insert(
            OwnerKey::Binding(binding),
            OwnerRecord::Binding(super::super::BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
                name: Name::new("view").unwrap(),
                kind: BindingKind::ReadView,
                value: None,
                declared_type: Some(f.view_type),
            }),
        );
        f.source.owners.insert(
            OwnerKey::Expression(parent),
            OwnerRecord::Expression(
                ExpressionRecord::new(
                    parent,
                    ExpressionOperation::BorrowCall {
                        call,
                        binding,
                        body: ExpressionId::migrate(SEED, 0),
                    },
                )
                .unwrap(),
            ),
        );
        let OwnerRecord::Declaration(function) = f
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(f.function.declaration))
            .unwrap()
        else {
            panic!("function");
        };
        let DeclarationPayload::Function(function) = &mut function.payload else {
            panic!("function");
        };
        function.body = parent;
        function.result = CallableResult {
            ty: f.view_type,
            relation: ResultRelation::ViewFrom {
                source: f.parameter,
                scope: f.scope,
            },
        };
        let mut check = validator(&f.source);
        check.validate_expressions();
        check.validate_read_scope_argument(
            OwnerKey::Expression(call),
            scope,
            Some(ReadScopeBinder::Declaration(f.function)),
        );
        assert!(check.diagnostics.is_empty(), "{:?}", check.diagnostics);
        check.validate_read_scope_argument(
            OwnerKey::Expression(argument),
            scope,
            Some(ReadScopeBinder::Declaration(f.function)),
        );
        assert!(
            check
                .diagnostics
                .iter()
                .any(|error| error.code == "kernel_read_view")
        );
        check.validate_local_reference(argument, LocalValueReference::LexicalBinding(binding));
        assert!(
            check
                .diagnostics
                .iter()
                .any(|error| error.code == "kernel_full_lexical_scope")
        );
    }
}
