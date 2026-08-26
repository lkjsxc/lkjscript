//! Implementation-disjoint full validator for normalized Graph 5 authority.

use super::contract::{MAXIMUM_EXPRESSION_DEPTH, MAXIMUM_TYPE_DEPTH, MAXIMUM_VALIDATION_WORK};
use super::expression::{ExpressionOperation, FieldSelector, LocalValueReference, TextValue};
use super::id::{OwnerKey, OwnerKind, PackageId};
use super::infer::validate_expression_meaning;
use super::owner::{
    BindingKind, DeclarationPayload, FunctionEffect, OwnerRecord, ParameterParent,
    PortImplementation,
};
use super::owner_namespace;
use super::relation::{RelationEdge, extract_relations};
use super::root::{DependencyRecord, RetirementRecord, SemanticRoot};
use super::type_object::{TypeForm, TypeObject};
use super::{
    BlobObjectDigest, PackageInterfaceRecord, PackageRevisionDigest, TypeObjectDigest,
    encode_type_object,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{BindingId, ExpressionId};
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

impl FullValidator<'_> {
    fn validate(&mut self) {
        self.validate_root_and_records();
        if self.exhausted() {
            return;
        }
        self.validate_namespaces();
        self.validate_owner_structure();
        self.validate_expressions();
        self.validate_types();
        self.validate_references();
        if self.diagnostics.is_empty() {
            validate_expression_meaning(self.snapshot, &mut self.diagnostics, &mut self.work);
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
            _ => {}
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
                OwnerKey::Module(_) | OwnerKey::Target(_) => return None,
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
                    self.require_exact_kind(
                        target.port.package,
                        OwnerKey::Port(target.port.port),
                        &[OwnerKind::Port],
                        "target port",
                    );
                    if target.component.package != target.port.package {
                        self.error(
                            "kernel_full_target_package",
                            "target component and port must belong to one package",
                        );
                    }
                    if target.port.package == self.snapshot.root.package_id
                        && self
                            .port_parent(target.port.port)
                            .is_some_and(|parent| parent != target.component.declaration)
                    {
                        self.error(
                            "kernel_full_target_port_owner",
                            "target port does not belong to its component",
                        );
                    }
                }
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
