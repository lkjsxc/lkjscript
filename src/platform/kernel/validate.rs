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
    BindingKind, DeclarationPayload, DeclarationVisibility, FunctionEffect, OwnerRecord,
    ParameterParent, ParameterRecord, ParameterUse, PortImplementation, PortRecord,
};
use super::owner_namespace;
use super::relation::{RelationEdge, extract_relations};
use super::root::{DependencyRecord, RetirementRecord, SemanticRoot};
use super::type_object::{TypeForm, TypeObject};
use super::{
    BlobObjectDigest, DeclarationReference, HttpRouteRecord, PackageInterfaceDeclarationPayload,
    PackageInterfaceRecord, PackageRevisionDigest, RequirementRecord, RequirementReference,
    TypeObjectDigest, TypeObjectInterner, encode_type_object, requirement_is_covered_by,
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
    pub dependency_interfaces:
        BTreeMap<PackageRevisionDigest, BTreeMap<OwnerKey, PackageInterfaceRecord>>,
    /// Structural types reachable from the bound dependency interfaces.
    pub dependency_types: BTreeMap<TypeObjectDigest, TypeObject>,
    pub blobs: BTreeMap<BlobObjectDigest, u64>,
    pub dependencies: BTreeMap<PackageId, DependencyRecord>,
    pub retirements: BTreeMap<OwnerKey, RetirementRecord>,
}

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
    let mut validator = FullValidator {
        snapshot,
        diagnostics: Vec::new(),
        work: 0,
        relations: Vec::new(),
        expression_parents: BTreeMap::new(),
        expression_root_owners: BTreeMap::new(),
        binding_containers: BTreeMap::new(),
    };
    validator.validate();
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
    diagnostics: Vec<Diagnostic>,
    work: usize,
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

fn snapshot_type_contains_resource(snapshot: &KernelSnapshot, digest: TypeObjectDigest) -> bool {
    fn visit(
        snapshot: &KernelSnapshot,
        digest: TypeObjectDigest,
        active_types: &mut BTreeSet<TypeObjectDigest>,
        active_declarations: &mut BTreeSet<(
            PackageId,
            crate::platform::semantic_id::DeclarationId,
        )>,
    ) -> bool {
        if !active_types.insert(digest) {
            return false;
        }
        let result = match snapshot
            .types
            .get(&digest)
            .or_else(|| snapshot.dependency_types.get(&digest))
            .map(|object| &object.form)
        {
            Some(TypeForm::CapabilityResource { .. }) => true,
            Some(TypeForm::Named { declaration }) => {
                let key = (declaration.package, declaration.declaration);
                if !active_declarations.insert(key) {
                    false
                } else {
                    let result = snapshot_named_member_types(snapshot, *declaration)
                        .into_iter()
                        .any(|member| visit(snapshot, member, active_types, active_declarations));
                    active_declarations.remove(&key);
                    result
                }
            }
            Some(object) => object_child_digests(object)
                .into_iter()
                .any(|child| visit(snapshot, child, active_types, active_declarations)),
            None => false,
        };
        active_types.remove(&digest);
        result
    }

    visit(snapshot, digest, &mut BTreeSet::new(), &mut BTreeSet::new())
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
        TypeForm::StructuralRecord { fields } => fields.iter().map(|field| field.ty).collect(),
        TypeForm::List { item } | TypeForm::Option { item } | TypeForm::Stream { item } => {
            vec![*item]
        }
        TypeForm::Map { key, value }
        | TypeForm::Result {
            ok: key,
            error: value,
        } => vec![*key, *value],
        TypeForm::Function { parameters, result } => {
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
            DeclarationPayload::Record { fields } => fields
                .iter()
                .filter_map(
                    |field| match snapshot.owners.get(&OwnerKey::Field(*field)) {
                        Some(OwnerRecord::Field(record)) => Some(record.ty),
                        _ => None,
                    },
                )
                .collect(),
            DeclarationPayload::Variant { cases } => cases
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
    let Some(PackageInterfaceRecord::Declaration(record)) =
        owners.get(&OwnerKey::Declaration(declaration.declaration))
    else {
        return Vec::new();
    };
    match &record.payload {
        super::PackageInterfaceDeclarationPayload::Record { fields } => fields
            .iter()
            .filter_map(|field| match owners.get(&OwnerKey::Field(*field)) {
                Some(PackageInterfaceRecord::Field(record)) => Some(record.ty),
                _ => None,
            })
            .collect(),
        super::PackageInterfaceDeclarationPayload::Variant { cases } => cases
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
    fn validate(&mut self) {
        self.validate_root_and_records();
        if self.exhausted() {
            return;
        }
        self.validate_namespaces();
        self.validate_owner_structure();
        self.validate_http_topology();
        self.validate_expressions();
        self.validate_types();
        self.validate_resource_shapes();
        self.validate_references();
        if self.diagnostics.is_empty() {
            validate_expression_meaning(self.snapshot, &mut self.diagnostics, &mut self.work);
        }
        if self.diagnostics.is_empty() {
            validate_affine_meaning(self.snapshot, &mut self.diagnostics, &mut self.work);
        }
        self.validate_relations();
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
                }
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
        let mut types = TypeObjectInterner::default();
        let expected_type = match crate::platform::http::semantic_http_route_function_type(
            &mut types,
            route.selector.capture_count(),
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
        let PortImplementation::Function(function) = port.implementation else {
            return;
        };
        let Some((type_parameters, parameter_ids, result, requirements)) =
            self.http_route_function_contract(function)
        else {
            return;
        };
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
        self.validate_http_route_requirement_closure(route, &requirements);
    }

    fn http_route_function_contract(
        &mut self,
        function: DeclarationReference,
    ) -> Option<(
        usize,
        Vec<crate::platform::semantic_id::ParameterId>,
        TypeObjectDigest,
        Vec<RequirementReference>,
    )> {
        if function.package == self.snapshot.root.package_id {
            return match self
                .snapshot
                .owners
                .get(&OwnerKey::Declaration(function.declaration))
            {
                Some(OwnerRecord::Declaration(record)) => match &record.payload {
                    DeclarationPayload::Function(signature) => Some((
                        signature.type_parameters.len(),
                        signature.parameters.clone(),
                        signature.result,
                        match &signature.effect {
                            FunctionEffect::Pure => Vec::new(),
                            FunctionEffect::Task { requirements } => requirements.clone(),
                        },
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
                    signature.type_parameters.len(),
                    signature.parameters.clone(),
                    signature.result,
                    match &signature.effect {
                        FunctionEffect::Pure => Vec::new(),
                        FunctionEffect::Task { requirements } => requirements.clone(),
                    },
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
        for requirement in requirements {
            if !self.consume_work() {
                return;
            }
            let Some(candidate) = self.http_route_requirement(*requirement) else {
                continue;
            };
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
            DeclarationPayload::Record { fields } => {
                for field in fields {
                    self.require_local_kind(OwnerKey::Field(*field), &[OwnerKind::Field], "field");
                }
            }
            DeclarationPayload::Variant { cases } => {
                for case in cases {
                    self.require_local_kind(OwnerKey::Case(*case), &[OwnerKind::Case], "case");
                }
            }
            DeclarationPayload::Interface { operations } => {
                for operation in operations {
                    self.require_local_kind(
                        OwnerKey::Operation(*operation),
                        &[OwnerKind::Operation],
                        "operation",
                    );
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
                if let FunctionEffect::Task { requirements } = &function.effect {
                    for requirement in requirements {
                        self.require_exact_kind(
                            requirement.package,
                            OwnerKey::Requirement(requirement.requirement),
                            &[OwnerKind::Requirement],
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
        let roots = self
            .snapshot
            .owners
            .iter()
            .flat_map(|(owner, record)| {
                record
                    .type_roots()
                    .into_iter()
                    .map(|digest| (*owner, digest))
            })
            .collect::<Vec<_>>();
        for (source, root) in roots {
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
                if !checked.insert((source, digest)) {
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
                self.validate_type_reference(source, &object.form);
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

    fn validate_type_reference(&mut self, source: OwnerKey, form: &TypeForm) {
        match form {
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
            TypeForm::Named { declaration } => self.require_exact_kind(
                declaration.package,
                OwnerKey::Declaration(declaration.declaration),
                &[OwnerKind::Record, OwnerKind::Variant],
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
            let forbidden = match &object.form {
                TypeForm::StructuralRecord { fields } => fields
                    .iter()
                    .any(|field| snapshot_type_contains_resource(self.snapshot, field.ty)),
                TypeForm::List { item } | TypeForm::Option { item } | TypeForm::Stream { item } => {
                    snapshot_type_contains_resource(self.snapshot, *item)
                }
                TypeForm::Map { key, value }
                | TypeForm::Result {
                    ok: key,
                    error: value,
                } => {
                    snapshot_type_contains_resource(self.snapshot, *key)
                        || snapshot_type_contains_resource(self.snapshot, *value)
                }
                TypeForm::Function { parameters, result } => {
                    parameters
                        .iter()
                        .any(|parameter| snapshot_type_contains_resource(self.snapshot, *parameter))
                        || snapshot_type_contains_resource(self.snapshot, *result)
                }
                _ => false,
            };
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
            match record {
                OwnerRecord::Parameter(parameter) => {
                    let contains = snapshot_type_contains_resource(self.snapshot, parameter.ty);
                    let direct = snapshot_type_is_direct_resource(self.snapshot, parameter.ty);
                    match parameter.parent {
                        ParameterParent::Function(_) => {
                            if direct {
                                if parameter.use_mode != ParameterUse::Consume {
                                    self.error(
                                        "kernel_affine_function_resource_use",
                                        format!(
                                            "direct resource function parameter {owner:?} must consume"
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
                    if snapshot_type_contains_resource(self.snapshot, field.ty) {
                        self.error(
                            "kernel_affine_record_field",
                            format!("record field {owner:?} cannot contain a capability resource"),
                        );
                    }
                }
                OwnerRecord::Case(case) => {
                    if let Some(payload) = case.payload
                        && snapshot_type_contains_resource(self.snapshot, payload)
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
                    DeclarationPayload::Variant { cases } => {
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
                        if snapshot_type_contains_resource(self.snapshot, signature.result) {
                            self.error(
                                "kernel_affine_external_result",
                                format!(
                                    "external declaration {owner:?} cannot return a capability resource"
                                ),
                            );
                        }
                    }
                    DeclarationPayload::Function(function) => {
                        if snapshot_type_contains_resource(self.snapshot, function.result) {
                            self.error(
                                "kernel_affine_function_result",
                                format!(
                                    "function declaration {owner:?} cannot return a capability resource"
                                ),
                            );
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
                            if resource_parameters.len() != 1 {
                                self.error(
                                    "kernel_affine_function_resource_count",
                                    format!(
                                        "function declaration {owner:?} must have exactly one direct resource parameter"
                                    ),
                                );
                            }
                            let (index, parameter, record) = resource_parameters[0];
                            if index.saturating_add(1) != function.parameters.len() {
                                self.error(
                                    "kernel_affine_function_resource_order",
                                    format!(
                                        "resource parameter {parameter} must be final in its function signature"
                                    ),
                                );
                            }
                            if declaration.visibility != DeclarationVisibility::Private {
                                self.error(
                                    "kernel_affine_function_resource_visibility",
                                    format!(
                                        "resource-bearing function declaration {owner:?} must be private"
                                    ),
                                );
                            }
                            if !function.type_parameters.is_empty() {
                                self.error(
                                    "kernel_affine_function_resource_generic",
                                    format!(
                                        "resource-bearing function declaration {owner:?} cannot be generic"
                                    ),
                                );
                            }
                            let FunctionEffect::Task { requirements } = &function.effect else {
                                self.error(
                                    "kernel_affine_function_resource_effect",
                                    format!(
                                        "resource-bearing function declaration {owner:?} must be a task"
                                    ),
                                );
                                continue;
                            };
                            let Some(requirement) = record.resource_requirement else {
                                continue;
                            };
                            if requirement.package != self.snapshot.root.package_id {
                                self.error(
                                    "kernel_affine_function_resource_package",
                                    format!(
                                        "resource parameter {parameter} must bind a same-package requirement"
                                    ),
                                );
                            }
                            if !requirements.contains(&requirement) {
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
                            match self
                                .snapshot
                                .owners
                                .get(&OwnerKey::Requirement(requirement.requirement))
                            {
                                Some(OwnerRecord::Requirement(bound))
                                    if requirement.package == self.snapshot.root.package_id
                                        && bound.interface == interface => {}
                                Some(OwnerRecord::Requirement(_)) => self.error(
                                    "kernel_affine_function_resource_interface",
                                    format!(
                                        "resource parameter {parameter} type disagrees with its exact requirement interface"
                                    ),
                                ),
                                _ => {}
                            }
                        }
                    }
                    DeclarationPayload::Constant { ty, .. }
                        if snapshot_type_contains_resource(self.snapshot, ty) =>
                    {
                        self.error(
                            "kernel_affine_constant",
                            format!(
                                "constant declaration {owner:?} cannot contain a capability resource"
                            ),
                        );
                    }
                    _ => {}
                },
                OwnerRecord::Port(port)
                    if snapshot_type_contains_resource(self.snapshot, port.function_type) =>
                {
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
        self.validate_expression_cycles_and_depth();
        self.assign_expression_roots();
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
            ExpressionOperation::Let { bindings, body } => {
                for (index, binding) in bindings.iter().enumerate() {
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
                            kind: BindingContainerKind::Let,
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
            ExpressionOperation::Transaction { binding, body, .. } => {
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
            let mut pending = vec![root];
            while let Some(expression) = pending.pop() {
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

    fn validate_expression_references(
        &mut self,
        expression: ExpressionId,
        operation: &ExpressionOperation,
    ) {
        match operation {
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
            ExpressionOperation::Call { function, .. }
            | ExpressionOperation::FunctionValue { function, .. } => self.require_exact_kind(
                function.package,
                OwnerKey::Declaration(function.declaration),
                &[
                    OwnerKind::PureFunction,
                    OwnerKind::TaskFunction,
                    OwnerKind::External,
                ],
                "function reference",
            ),
            ExpressionOperation::Record {
                nominal_type: Some(declaration),
                fields,
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
                self.require_exact_kind(
                    requirement.package,
                    OwnerKey::Requirement(requirement.requirement),
                    &[OwnerKind::Requirement],
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
                requirement.package,
                OwnerKey::Requirement(requirement.requirement),
                &[OwnerKind::Requirement],
                "transaction requirement",
            ),
            _ => {}
        }
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
            (OwnerKey::TypeParameter(id), OwnerRecord::Declaration(declaration)) => {
                match &declaration.payload {
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
                DeclarationPayload::Record { fields } if fields.contains(&id)
            ),
            (OwnerKey::Case(id), OwnerRecord::Declaration(declaration)) => matches!(
                &declaration.payload,
                DeclarationPayload::Variant { cases } if cases.contains(&id)
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
                        FunctionEffect::Task { requirements } => {
                            requirements.iter().any(|reference| {
                                reference.package == self.snapshot.root.package_id
                                    && reference.requirement == id
                            })
                        }
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
        requirement: crate::platform::kernel::RequirementReference,
        operation: crate::platform::kernel::OperationReference,
    ) -> bool {
        let record = if requirement.package == self.snapshot.root.package_id {
            match self
                .snapshot
                .owners
                .get(&OwnerKey::Requirement(requirement.requirement))
            {
                Some(OwnerRecord::Requirement(record)) => Some(record),
                _ => None,
            }
        } else {
            let dependency = self.snapshot.dependencies.get(&requirement.package);
            dependency
                .and_then(|dependency| {
                    self.snapshot
                        .dependency_interfaces
                        .get(&dependency.package_revision)
                })
                .and_then(|owners| owners.get(&OwnerKey::Requirement(requirement.requirement)))
                .and_then(|record| match record {
                    PackageInterfaceRecord::Requirement(record) => Some(record),
                    _ => None,
                })
        };
        record.is_some_and(|record| {
            record.interface.package == operation.package && record.operations.contains(&operation)
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
        self.work = self.work.saturating_add(1);
        if self.work > MAXIMUM_VALIDATION_WORK {
            if !self
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "kernel_full_work")
            {
                self.error(
                    "kernel_full_work",
                    "full validation exhausted its explicit work budget",
                );
            }
            return false;
        }
        true
    }

    fn exhausted(&self) -> bool {
        self.work > MAXIMUM_VALIDATION_WORK
    }

    fn error(&mut self, code: &str, message: impl Into<String>) {
        self.diagnostics
            .push(Diagnostic::new(DiagnosticClass::Semantic, code, message));
    }
}
