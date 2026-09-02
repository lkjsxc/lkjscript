//! Single deterministic relation extractor for Graph 7 records.

use super::TypeObjectDigest;
use super::contract::MAXIMUM_VALIDATION_WORK;
use super::expression::{ExpressionOperation, FieldSelector, LocalValueReference};
use super::id::{ExactOwnerKey, OwnerKey, PackageId};
use super::owner::{
    DeclarationPayload, FunctionEffect, OwnerRecord, ParameterParent, PortImplementation,
};
use super::root::DependencyRecord;
use super::type_object::{TypeForm, TypeObject};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RelationEdge {
    pub source: RelationEndpoint,
    pub kind: RelationKind,
    pub target: RelationEndpoint,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RelationEndpoint {
    Owner(ExactOwnerKey),
    Package(PackageId),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RelationKind {
    DeclarationModule,
    MemberDeclaration,
    ParameterOperation,
    ExpressionParent,
    ExpressionRoot,
    TypeParameterUse,
    NamedTypeUse,
    LocalValueReference,
    ConstantReference,
    FunctionCall,
    FunctionValue,
    FunctionRequirement,
    ParameterRequirement,
    NominalFieldConstruction,
    NominalFieldAccess,
    VariantConstruction,
    VariantMatch,
    CapabilityInterface,
    CapabilityOperation,
    ComponentRequirement,
    ComponentPort,
    TargetComponent,
    TargetPort,
    TestExecutionDependency,
    DocumentationOwnership,
    AnnotationOwnership,
    PackageDependency,
    VariantExhaustiveness,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PropagationClass {
    Ownership,
    Type,
    Value,
    Behavior,
    Capability,
    Target,
    Test,
    Presentation,
    Package,
}

impl RelationKind {
    pub const ALL: [Self; 28] = [
        Self::DeclarationModule,
        Self::MemberDeclaration,
        Self::ParameterOperation,
        Self::ExpressionParent,
        Self::ExpressionRoot,
        Self::TypeParameterUse,
        Self::NamedTypeUse,
        Self::LocalValueReference,
        Self::ConstantReference,
        Self::FunctionCall,
        Self::FunctionValue,
        Self::FunctionRequirement,
        Self::ParameterRequirement,
        Self::NominalFieldConstruction,
        Self::NominalFieldAccess,
        Self::VariantConstruction,
        Self::VariantMatch,
        Self::CapabilityInterface,
        Self::CapabilityOperation,
        Self::ComponentRequirement,
        Self::ComponentPort,
        Self::TargetComponent,
        Self::TargetPort,
        Self::TestExecutionDependency,
        Self::DocumentationOwnership,
        Self::AnnotationOwnership,
        Self::PackageDependency,
        Self::VariantExhaustiveness,
    ];

    pub const fn tag(self) -> u8 {
        match self {
            Self::DeclarationModule => 1,
            Self::MemberDeclaration => 2,
            Self::ParameterOperation => 3,
            Self::ExpressionParent => 4,
            Self::ExpressionRoot => 5,
            Self::TypeParameterUse => 6,
            Self::NamedTypeUse => 7,
            Self::LocalValueReference => 8,
            Self::ConstantReference => 9,
            Self::FunctionCall => 10,
            Self::FunctionValue => 11,
            Self::FunctionRequirement => 27,
            Self::ParameterRequirement => 28,
            Self::NominalFieldConstruction => 12,
            Self::NominalFieldAccess => 13,
            Self::VariantConstruction => 14,
            Self::VariantMatch => 15,
            Self::VariantExhaustiveness => 26,
            Self::CapabilityInterface => 16,
            Self::CapabilityOperation => 17,
            Self::ComponentRequirement => 18,
            Self::ComponentPort => 19,
            Self::TargetComponent => 20,
            Self::TargetPort => 21,
            Self::TestExecutionDependency => 22,
            Self::DocumentationOwnership => 23,
            Self::AnnotationOwnership => 24,
            Self::PackageDependency => 25,
        }
    }

    pub fn from_tag(tag: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.tag() == tag)
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::DeclarationModule => "declaration_module",
            Self::MemberDeclaration => "member_declaration",
            Self::ParameterOperation => "parameter_operation",
            Self::ExpressionParent => "expression_parent",
            Self::ExpressionRoot => "expression_root",
            Self::TypeParameterUse => "type_parameter_use",
            Self::NamedTypeUse => "named_type_use",
            Self::LocalValueReference => "local_value_reference",
            Self::ConstantReference => "constant_reference",
            Self::FunctionCall => "function_call",
            Self::FunctionValue => "function_value",
            Self::FunctionRequirement => "function_requirement",
            Self::ParameterRequirement => "parameter_requirement",
            Self::NominalFieldConstruction => "nominal_field_construction",
            Self::NominalFieldAccess => "nominal_field_access",
            Self::VariantConstruction => "variant_construction",
            Self::VariantMatch => "variant_match",
            Self::CapabilityInterface => "capability_interface",
            Self::CapabilityOperation => "capability_operation",
            Self::ComponentRequirement => "component_requirement",
            Self::ComponentPort => "component_port",
            Self::TargetComponent => "target_component",
            Self::TargetPort => "target_port",
            Self::TestExecutionDependency => "test_execution_dependency",
            Self::DocumentationOwnership => "documentation_ownership",
            Self::AnnotationOwnership => "annotation_ownership",
            Self::PackageDependency => "package_dependency",
            Self::VariantExhaustiveness => "variant_exhaustiveness",
        }
    }

    pub fn parse(value: &str) -> Result<Self, Diagnostic> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.name() == value)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Source,
                    "kernel_relation_kind",
                    format!("unknown semantic relation kind '{value}'"),
                )
            })
    }

    pub const fn propagation(self) -> PropagationClass {
        match self {
            Self::DeclarationModule
            | Self::MemberDeclaration
            | Self::ParameterOperation
            | Self::ExpressionParent
            | Self::ExpressionRoot => PropagationClass::Ownership,
            Self::TypeParameterUse | Self::NamedTypeUse => PropagationClass::Type,
            Self::LocalValueReference
            | Self::ConstantReference
            | Self::NominalFieldConstruction
            | Self::NominalFieldAccess
            | Self::VariantConstruction
            | Self::VariantMatch
            | Self::VariantExhaustiveness => PropagationClass::Value,
            Self::FunctionCall | Self::FunctionValue => PropagationClass::Behavior,
            Self::FunctionRequirement
            | Self::ParameterRequirement
            | Self::CapabilityInterface
            | Self::CapabilityOperation
            | Self::ComponentRequirement
            | Self::ComponentPort => PropagationClass::Capability,
            Self::TargetComponent | Self::TargetPort => PropagationClass::Target,
            Self::TestExecutionDependency => PropagationClass::Test,
            Self::DocumentationOwnership | Self::AnnotationOwnership => {
                PropagationClass::Presentation
            }
            Self::PackageDependency => PropagationClass::Package,
        }
    }
}

impl PropagationClass {
    pub const fn tag(self) -> u8 {
        match self {
            Self::Ownership => 1,
            Self::Type => 2,
            Self::Value => 3,
            Self::Behavior => 4,
            Self::Capability => 5,
            Self::Target => 6,
            Self::Test => 7,
            Self::Presentation => 8,
            Self::Package => 9,
        }
    }
}

pub fn extract_relations(
    package: PackageId,
    owners: &BTreeMap<OwnerKey, OwnerRecord>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
    dependencies: &BTreeMap<PackageId, DependencyRecord>,
) -> Result<Vec<RelationEdge>, Diagnostic> {
    let mut edges = RelationCollector::new(MAXIMUM_VALIDATION_WORK);
    let mut work = 0_usize;
    for (key, record) in owners {
        consume_work(&mut work)?;
        let source = exact(package, *key);
        extract_owner(
            source,
            record,
            package,
            &mut |digest| Ok(types.get(&digest).cloned()),
            &mut |target_package, case| {
                if target_package != package {
                    return Ok(None);
                }
                Ok(match owners.get(&OwnerKey::Case(case)) {
                    Some(OwnerRecord::Case(record)) => Some(record.declaration),
                    _ => None,
                })
            },
            &mut edges,
            &mut work,
        )?;
    }
    for dependency in dependencies.keys() {
        consume_work(&mut work)?;
        edges.insert(RelationEdge {
            source: RelationEndpoint::Package(package),
            kind: RelationKind::PackageDependency,
            target: RelationEndpoint::Package(*dependency),
        })?;
    }
    Ok(edges.into_edges())
}

/// Extracts the exact relation set contributed by one canonical owner record. Incremental witness
/// maintenance and the full oracle call this same switch; only traversal scheduling differs.
pub fn extract_owner_relations<F, C>(
    package: PackageId,
    owner: OwnerKey,
    record: &OwnerRecord,
    type_object: F,
    case_parent: C,
) -> Result<Vec<RelationEdge>, Diagnostic>
where
    F: FnMut(TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic>,
    C: FnMut(
        PackageId,
        crate::platform::semantic_id::CaseId,
    ) -> Result<Option<crate::platform::semantic_id::DeclarationId>, Diagnostic>,
{
    extract_owner_relations_with_limit(
        package,
        owner,
        record,
        type_object,
        case_parent,
        MAXIMUM_VALIDATION_WORK,
    )
    .map(|(edges, _)| edges)
}

pub fn extract_owner_relations_with_limit<F, C>(
    package: PackageId,
    owner: OwnerKey,
    record: &OwnerRecord,
    mut type_object: F,
    mut case_parent: C,
    maximum_edges: usize,
) -> Result<(Vec<RelationEdge>, u64), Diagnostic>
where
    F: FnMut(TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic>,
    C: FnMut(
        PackageId,
        crate::platform::semantic_id::CaseId,
    ) -> Result<Option<crate::platform::semantic_id::DeclarationId>, Diagnostic>,
{
    if record.owner() != owner {
        return Err(relation_error(
            "kernel_relation_owner_key",
            "relation extraction owner key disagrees with the canonical record header",
        ));
    }
    let mut edges = RelationCollector::new(maximum_edges);
    let mut work = 0;
    extract_owner(
        exact(package, owner),
        record,
        package,
        &mut type_object,
        &mut case_parent,
        &mut edges,
        &mut work,
    )?;
    let examined = edges.examined;
    Ok((edges.into_edges(), examined))
}

fn extract_owner<F, C>(
    source: ExactOwnerKey,
    record: &OwnerRecord,
    package: PackageId,
    type_object: &mut F,
    case_parent: &mut C,
    edges: &mut RelationCollector,
    work: &mut usize,
) -> Result<(), Diagnostic>
where
    F: FnMut(TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic>,
    C: FnMut(
        PackageId,
        crate::platform::semantic_id::CaseId,
    ) -> Result<Option<crate::platform::semantic_id::DeclarationId>, Diagnostic>,
{
    match record {
        OwnerRecord::Module(_) => {}
        OwnerRecord::Declaration(declaration) => {
            owner_edge(
                edges,
                source,
                RelationKind::DeclarationModule,
                package,
                OwnerKey::Module(declaration.module),
            )?;
            match &declaration.payload {
                DeclarationPayload::Record { fields } => {
                    for field in fields {
                        owner_edge(
                            edges,
                            exact(package, OwnerKey::Field(*field)),
                            RelationKind::MemberDeclaration,
                            package,
                            source.owner,
                        )?;
                    }
                }
                DeclarationPayload::Variant { cases } => {
                    for case in cases {
                        owner_edge(
                            edges,
                            exact(package, OwnerKey::Case(*case)),
                            RelationKind::MemberDeclaration,
                            package,
                            source.owner,
                        )?;
                    }
                }
                DeclarationPayload::Interface { operations } => {
                    for operation in operations {
                        owner_edge(
                            edges,
                            exact(package, OwnerKey::Operation(*operation)),
                            RelationKind::MemberDeclaration,
                            package,
                            source.owner,
                        )?;
                    }
                }
                DeclarationPayload::Function(function) => {
                    if let FunctionEffect::Task { requirements } = &function.effect {
                        for requirement in requirements {
                            exact_edge(
                                edges,
                                source,
                                RelationKind::FunctionRequirement,
                                requirement.package,
                                OwnerKey::Requirement(requirement.requirement),
                            )?;
                        }
                    }
                }
                DeclarationPayload::Component {
                    requirements,
                    ports,
                } => {
                    for requirement in requirements {
                        owner_edge(
                            edges,
                            source,
                            RelationKind::ComponentRequirement,
                            package,
                            OwnerKey::Requirement(*requirement),
                        )?;
                    }
                    for port in ports {
                        owner_edge(
                            edges,
                            source,
                            RelationKind::ComponentPort,
                            package,
                            OwnerKey::Port(*port),
                        )?;
                    }
                }
                DeclarationPayload::Test {
                    actual, expected, ..
                } => {
                    for root in [actual, expected] {
                        owner_edge(
                            edges,
                            source,
                            RelationKind::TestExecutionDependency,
                            package,
                            OwnerKey::Expression(*root),
                        )?;
                    }
                }
                DeclarationPayload::External(_) | DeclarationPayload::Constant { .. } => {}
            }
            for root in record.expression_roots() {
                owner_edge(
                    edges,
                    exact(package, OwnerKey::Expression(root)),
                    RelationKind::ExpressionRoot,
                    package,
                    source.owner,
                )?;
            }
        }
        OwnerRecord::TypeParameter(parameter) => owner_edge(
            edges,
            source,
            RelationKind::MemberDeclaration,
            package,
            OwnerKey::Declaration(parameter.declaration),
        )?,
        OwnerRecord::Field(field) => owner_edge(
            edges,
            source,
            RelationKind::MemberDeclaration,
            package,
            OwnerKey::Declaration(field.declaration),
        )?,
        OwnerRecord::Case(case) => owner_edge(
            edges,
            source,
            RelationKind::MemberDeclaration,
            package,
            OwnerKey::Declaration(case.declaration),
        )?,
        OwnerRecord::Operation(operation) => owner_edge(
            edges,
            source,
            RelationKind::MemberDeclaration,
            package,
            OwnerKey::Declaration(operation.declaration),
        )?,
        OwnerRecord::Parameter(parameter) => {
            match parameter.parent {
                ParameterParent::Function(declaration) => owner_edge(
                    edges,
                    source,
                    RelationKind::MemberDeclaration,
                    package,
                    OwnerKey::Declaration(declaration),
                )?,
                ParameterParent::Operation(operation) => owner_edge(
                    edges,
                    source,
                    RelationKind::ParameterOperation,
                    package,
                    OwnerKey::Operation(operation),
                )?,
            }
            if let Some(requirement) = parameter.resource_requirement {
                exact_edge(
                    edges,
                    source,
                    RelationKind::ParameterRequirement,
                    requirement.package,
                    OwnerKey::Requirement(requirement.requirement),
                )?;
            }
        }
        OwnerRecord::Binding(binding) => {
            if let Some(value) = binding.value {
                owner_edge(
                    edges,
                    exact(package, OwnerKey::Expression(value)),
                    RelationKind::ExpressionRoot,
                    package,
                    source.owner,
                )?;
            }
        }
        OwnerRecord::Expression(expression) => {
            for child in expression.children() {
                owner_edge(
                    edges,
                    exact(package, OwnerKey::Expression(child.expression)),
                    RelationKind::ExpressionParent,
                    package,
                    source.owner,
                )?;
            }
            extract_expression(source, &expression.operation, package, case_parent, edges)?;
        }
        OwnerRecord::Requirement(requirement) => {
            exact_edge(
                edges,
                source,
                RelationKind::CapabilityInterface,
                requirement.interface.package,
                OwnerKey::Declaration(requirement.interface.declaration),
            )?;
            for operation in &requirement.operations {
                exact_edge(
                    edges,
                    source,
                    RelationKind::CapabilityOperation,
                    operation.package,
                    OwnerKey::Operation(operation.operation),
                )?;
            }
        }
        OwnerRecord::Port(port) => {
            owner_edge(
                edges,
                source,
                RelationKind::MemberDeclaration,
                package,
                OwnerKey::Declaration(port.declaration),
            )?;
            if let PortImplementation::Function(function) = &port.implementation {
                exact_edge(
                    edges,
                    source,
                    RelationKind::FunctionValue,
                    function.package,
                    OwnerKey::Declaration(function.declaration),
                )?;
            }
        }
        OwnerRecord::Target(target) => {
            exact_edge(
                edges,
                source,
                RelationKind::TargetComponent,
                target.component.package,
                OwnerKey::Declaration(target.component.declaration),
            )?;
            exact_edge(
                edges,
                source,
                RelationKind::TargetPort,
                target.port.package,
                OwnerKey::Port(target.port.port),
            )?;
        }
        OwnerRecord::Documentation(documentation) => owner_edge(
            edges,
            source,
            RelationKind::DocumentationOwnership,
            package,
            documentation.owner,
        )?,
        OwnerRecord::Annotation(annotation) => owner_edge(
            edges,
            source,
            RelationKind::AnnotationOwnership,
            package,
            annotation.owner,
        )?,
    }

    for root in record.type_roots() {
        extract_type_relations(source, root, package, type_object, edges, work)?;
    }
    Ok(())
}

fn extract_expression<C>(
    source: ExactOwnerKey,
    operation: &ExpressionOperation,
    package: PackageId,
    case_parent: &mut C,
    edges: &mut RelationCollector,
) -> Result<(), Diagnostic>
where
    C: FnMut(
        PackageId,
        crate::platform::semantic_id::CaseId,
    ) -> Result<Option<crate::platform::semantic_id::DeclarationId>, Diagnostic>,
{
    match operation {
        ExpressionOperation::Local { value } => {
            let target = match value {
                LocalValueReference::FunctionParameter(id)
                | LocalValueReference::OperationParameter(id) => OwnerKey::Parameter(*id),
                LocalValueReference::LexicalBinding(id)
                | LocalValueReference::MatchPayload(id)
                | LocalValueReference::TransactionBinding(id) => OwnerKey::Binding(*id),
            };
            owner_edge(
                edges,
                source,
                RelationKind::LocalValueReference,
                package,
                target,
            )?;
        }
        ExpressionOperation::Constant { declaration } => exact_edge(
            edges,
            source,
            RelationKind::ConstantReference,
            declaration.package,
            OwnerKey::Declaration(declaration.declaration),
        )?,
        ExpressionOperation::Call { function, .. } => exact_edge(
            edges,
            source,
            RelationKind::FunctionCall,
            function.package,
            OwnerKey::Declaration(function.declaration),
        )?,
        ExpressionOperation::FunctionValue { function, .. } => exact_edge(
            edges,
            source,
            RelationKind::FunctionValue,
            function.package,
            OwnerKey::Declaration(function.declaration),
        )?,
        ExpressionOperation::Record {
            nominal_type,
            fields,
        } => {
            if let Some(declaration) = nominal_type {
                exact_edge(
                    edges,
                    source,
                    RelationKind::NamedTypeUse,
                    declaration.package,
                    OwnerKey::Declaration(declaration.declaration),
                )?;
            }
            for field in fields {
                if let FieldSelector::Nominal(field) = field.selector {
                    exact_edge(
                        edges,
                        source,
                        RelationKind::NominalFieldConstruction,
                        field.package,
                        OwnerKey::Field(field.field),
                    )?;
                }
            }
        }
        ExpressionOperation::Variant { case, .. } => exact_edge(
            edges,
            source,
            RelationKind::VariantConstruction,
            case.package,
            OwnerKey::Case(case.case),
        )?,
        ExpressionOperation::Field {
            selector: FieldSelector::Nominal(field),
            ..
        } => exact_edge(
            edges,
            source,
            RelationKind::NominalFieldAccess,
            field.package,
            OwnerKey::Field(field.field),
        )?,
        ExpressionOperation::Match { arms, .. } => {
            for arm in arms {
                exact_edge(
                    edges,
                    source,
                    RelationKind::VariantMatch,
                    arm.case.package,
                    OwnerKey::Case(arm.case.case),
                )?;
                if let Some(declaration) = case_parent(arm.case.package, arm.case.case)? {
                    exact_edge(
                        edges,
                        source,
                        RelationKind::VariantExhaustiveness,
                        arm.case.package,
                        OwnerKey::Declaration(declaration),
                    )?;
                }
            }
        }
        ExpressionOperation::CapabilityCall {
            requirement,
            operation,
            ..
        } => {
            exact_edge(
                edges,
                source,
                RelationKind::ComponentRequirement,
                requirement.package,
                OwnerKey::Requirement(requirement.requirement),
            )?;
            exact_edge(
                edges,
                source,
                RelationKind::CapabilityOperation,
                operation.package,
                OwnerKey::Operation(operation.operation),
            )?;
        }
        ExpressionOperation::Transaction { requirement, .. } => exact_edge(
            edges,
            source,
            RelationKind::ComponentRequirement,
            requirement.package,
            OwnerKey::Requirement(requirement.requirement),
        )?,
        ExpressionOperation::Unit {}
        | ExpressionOperation::Bool { .. }
        | ExpressionOperation::I64 { .. }
        | ExpressionOperation::Text { .. }
        | ExpressionOperation::StaticText { .. }
        | ExpressionOperation::If { .. }
        | ExpressionOperation::Let { .. }
        | ExpressionOperation::Sequence { .. }
        | ExpressionOperation::Invoke { .. }
        | ExpressionOperation::Field {
            selector: FieldSelector::Structural(_),
            ..
        }
        | ExpressionOperation::List { .. }
        | ExpressionOperation::Map { .. } => {}
    }
    Ok(())
}

fn extract_type_relations<F>(
    source: ExactOwnerKey,
    root: TypeObjectDigest,
    package: PackageId,
    type_object: &mut F,
    edges: &mut RelationCollector,
    work: &mut usize,
) -> Result<(), Diagnostic>
where
    F: FnMut(TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic>,
{
    let mut pending = vec![root];
    let mut observed = BTreeSet::new();
    while let Some(digest) = pending.pop() {
        consume_work(work)?;
        if !observed.insert(digest) {
            continue;
        }
        let object = type_object(digest)?.ok_or_else(|| {
            relation_error(
                "kernel_relation_missing_type",
                format!("type object {digest} is missing"),
            )
        })?;
        match &object.form {
            TypeForm::TypeParameter { parameter } => {
                owner_edge(
                    edges,
                    source,
                    RelationKind::TypeParameterUse,
                    package,
                    OwnerKey::TypeParameter(*parameter),
                )?;
            }
            TypeForm::Named { declaration } => {
                exact_edge(
                    edges,
                    source,
                    RelationKind::NamedTypeUse,
                    declaration.package,
                    OwnerKey::Declaration(declaration.declaration),
                )?;
            }
            TypeForm::CapabilityResource { interface } => {
                exact_edge(
                    edges,
                    source,
                    RelationKind::CapabilityInterface,
                    interface.package,
                    OwnerKey::Declaration(interface.declaration),
                )?;
            }
            _ => pending.extend(object.child_types()),
        }
    }
    Ok(())
}

fn exact(package: PackageId, owner: OwnerKey) -> ExactOwnerKey {
    ExactOwnerKey { package, owner }
}

fn owner_edge(
    edges: &mut RelationCollector,
    source: ExactOwnerKey,
    kind: RelationKind,
    package: PackageId,
    owner: OwnerKey,
) -> Result<(), Diagnostic> {
    exact_edge(edges, source, kind, package, owner)
}

fn exact_edge(
    edges: &mut RelationCollector,
    source: ExactOwnerKey,
    kind: RelationKind,
    package: PackageId,
    owner: OwnerKey,
) -> Result<(), Diagnostic> {
    edges.insert(RelationEdge {
        source: RelationEndpoint::Owner(source),
        kind,
        target: RelationEndpoint::Owner(exact(package, owner)),
    })
}

struct RelationCollector {
    edges: BTreeSet<RelationEdge>,
    examined: u64,
    maximum: u64,
}

impl RelationCollector {
    fn new(maximum: usize) -> Self {
        Self {
            edges: BTreeSet::new(),
            examined: 0,
            maximum: u64::try_from(maximum).unwrap_or(u64::MAX),
        }
    }

    fn insert(&mut self, edge: RelationEdge) -> Result<(), Diagnostic> {
        if self.examined >= self.maximum {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_relation_edge_budget",
                format!(
                    "relation extraction exceeds the declared {}-edge budget",
                    self.maximum
                ),
            ));
        }
        self.examined = self.examined.saturating_add(1);
        self.edges.insert(edge);
        Ok(())
    }

    fn into_edges(self) -> Vec<RelationEdge> {
        self.edges.into_iter().collect()
    }
}

fn consume_work(work: &mut usize) -> Result<(), Diagnostic> {
    *work = work.saturating_add(1);
    if *work > MAXIMUM_VALIDATION_WORK {
        return Err(relation_error(
            "kernel_relation_work",
            "relation extraction exhausted its work budget",
        ));
    }
    Ok(())
}

fn relation_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}
