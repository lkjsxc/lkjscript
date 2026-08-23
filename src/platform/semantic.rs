//! Package resolution, nominal identity, type checking, and effect discovery.

use super::contract::registry::PACKAGE_REVISION_DIGEST_DOMAIN;
use super::diagnostic::{Diagnostic, SourceLocation};
use super::graph::GraphRoot;
use super::language::{
    Component, Declaration, Effect, Expression, ExternalFunction, Function, Idempotency, Interface,
    MatchArm, Module, Parameter, TaskCapability, Type, Visibility,
};
use super::meaning::{
    DeclarationIdentity, DeclarationReference, MeaningModule, MemberIdentity, RelationRole,
    RelationSource, RelationTarget, SemanticRelation,
};
use super::package::{
    Dependency, ModuleLocator, PackageDescriptor, PackageId, Target, semantic_dependency_bytes,
};
use super::semantic_digest::{ArtifactDigest, RootObjectDigest};
use super::semantic_id::{DeclarationId, ExpressionId, ModuleId, RevisionId, TypeParameterId};
use super::syntax::SourceSpan;
#[cfg(test)]
use super::{meaning::MigrationIdentityAllocator, syntax::SourceDocument};
use bincode::{Decode, Encode};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Decode, Encode, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerId {
    pub package: PackageId,
    pub module_id: ModuleId,
    pub declaration_id: DeclarationId,
    pub module: String,
    pub declaration: String,
}

impl PartialEq for OwnerId {
    fn eq(&self, other: &Self) -> bool {
        self.package == other.package
            && self.module_id == other.module_id
            && self.declaration_id == other.declaration_id
    }
}

impl Eq for OwnerId {}

impl Ord for OwnerId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (&self.package, self.module_id, self.declaration_id).cmp(&(
            &other.package,
            other.module_id,
            other.declaration_id,
        ))
    }
}

impl PartialOrd for OwnerId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl OwnerId {
    pub fn diagnostic_name(&self) -> String {
        format!(
            "{}::{}::{}",
            self.package.as_str(),
            self.module,
            self.declaration
        )
    }

    #[cfg(test)]
    pub(crate) fn deterministic_for_test(
        package: PackageId,
        module: &str,
        declaration: &str,
    ) -> Self {
        let mut seed = package.bytes().to_vec();
        seed.extend_from_slice(&(module.len() as u64).to_be_bytes());
        seed.extend_from_slice(module.as_bytes());
        seed.extend_from_slice(&(declaration.len() as u64).to_be_bytes());
        seed.extend_from_slice(declaration.as_bytes());
        Self {
            package,
            module_id: ModuleId::migrate(&seed, 0),
            declaration_id: DeclarationId::migrate(&seed, 0),
            module: module.to_owned(),
            declaration: declaration.to_owned(),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ResolvedType {
    #[default]
    Unit,
    Bool,
    I64,
    Bytes,
    Text,
    StaticText,
    Secret,
    Parameter(TypeParameterId),
    Nominal(OwnerId),
    Record(Vec<ResolvedField>),
    List(Box<ResolvedType>),
    Map(Box<ResolvedType>, Box<ResolvedType>),
    Option(Box<ResolvedType>),
    Result(Box<ResolvedType>, Box<ResolvedType>),
    Stream(Box<ResolvedType>),
    Function(Vec<ResolvedType>, Box<ResolvedType>),
}

impl ResolvedType {
    pub fn is_durable(&self) -> bool {
        match self {
            Self::Secret | Self::Stream(_) | Self::Function(_, _) | Self::Parameter(_) => false,
            Self::Record(fields) => fields.iter().all(|field| field.ty.is_durable()),
            Self::List(item) | Self::Option(item) => item.is_durable(),
            Self::Map(key, value) | Self::Result(key, value) => {
                key.is_durable() && value.is_durable()
            }
            Self::Unit
            | Self::Bool
            | Self::I64
            | Self::Bytes
            | Self::Text
            | Self::StaticText
            | Self::Nominal(_) => true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedField {
    pub name: String,
    pub ty: ResolvedType,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionSignature {
    pub owner: OwnerId,
    pub type_parameters: Vec<ResolvedTypeParameter>,
    pub parameters: Vec<ResolvedType>,
    pub result: ResolvedType,
    pub task_capabilities: Vec<ResolvedTaskCapability>,
    pub external_implementation: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedTypeParameter {
    pub id: TypeParameterId,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedTaskCapability {
    pub alias: String,
    pub interface: OwnerId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CapabilityFacts {
    pub interface: OwnerId,
    pub operations: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionFacts {
    pub signature: FunctionSignature,
    pub capabilities: BTreeMap<String, CapabilityFacts>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum NominalShape {
    Record(Vec<ResolvedField>),
    Variant(BTreeMap<String, Option<ResolvedType>>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedInterface {
    pub owner: OwnerId,
    pub operations: BTreeMap<String, ResolvedOperation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedOperation {
    pub parameters: Vec<ResolvedType>,
    pub result: ResolvedType,
    pub idempotency: Idempotency,
    pub visibility: Visibility,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedModule {
    pub path: String,
    pub module_id: ModuleId,
    pub module: Module,
    pub declaration_identities: Vec<DeclarationIdentity>,
    pub relations: Vec<SemanticRelation>,
    pub semantic_bytes: Vec<u8>,
}

impl ValidatedModule {
    pub fn owner(&self, package: &PackageId, declaration: &str) -> Result<OwnerId, Diagnostic> {
        validated_owner(package, self, declaration)
    }

    pub fn expression_id(
        &self,
        declaration: &str,
        path: &[u32],
    ) -> Result<ExpressionId, Diagnostic> {
        self.declaration_identities
            .iter()
            .find(|identity| identity.name == declaration)
            .and_then(|identity| {
                identity
                    .expressions
                    .iter()
                    .find(|expression| expression.path == path)
            })
            .map(|expression| expression.id)
            .ok_or_else(|| {
                semantic_without_location(
                    "semantic_expression_identity_missing",
                    format!(
                        "declaration '{}.{}' has no expression identity at path {:?}",
                        self.module.name, declaration, path
                    ),
                )
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedPackage {
    pub descriptor: PackageDescriptor,
    pub modules: Vec<ValidatedModule>,
    pub revision_digest: String,
    pub accepted_revision: Option<RevisionId>,
    pub graph_root_digest: Option<RootObjectDigest>,
    pub function_facts: BTreeMap<OwnerId, FunctionFacts>,
    pub nominal_shapes: BTreeMap<OwnerId, NominalShape>,
    pub interfaces: BTreeMap<OwnerId, ResolvedInterface>,
    pub constant_types: BTreeMap<OwnerId, ResolvedType>,
}

pub struct ExactDependency<'a> {
    pub alias: &'a str,
    pub package: &'a ValidatedPackage,
    pub artifact_digest: &'a str,
}

pub struct ExactGraphDependency<'a> {
    pub alias: &'a str,
    pub package: &'a ValidatedPackage,
    pub artifact: ArtifactDigest,
}

#[cfg(test)]
pub(crate) fn validate_package_documents(
    descriptor: PackageDescriptor,
    documents: Vec<SourceDocument>,
    dependencies: &[ExactDependency<'_>],
) -> Result<ValidatedPackage, Diagnostic> {
    if documents.len() != descriptor.modules.len() {
        return Err(semantic_without_location(
            "package_module_document_count",
            format!(
                "package declares {} modules but {} source documents were supplied",
                descriptor.modules.len(),
                documents.len()
            ),
        ));
    }

    let mut by_path = BTreeMap::new();
    for document in documents {
        let path = document.path().to_owned();
        if by_path.insert(path.clone(), document).is_some() {
            return Err(semantic_without_location(
                "package_module_document_duplicate",
                format!("source document path '{path}' was supplied twice"),
            ));
        }
    }

    let mut parsed = Vec::new();
    for locator in &descriptor.modules {
        let document = by_path.remove(&locator.path).ok_or_else(|| {
            semantic_without_location(
                "package_module_document_missing",
                format!("declared module path '{}' was not supplied", locator.path),
            )
        })?;
        let module = super::language::parse_module(&document)?;
        if module.name != locator.name {
            return Err(semantic_at(
                &locator.path,
                module
                    .declarations
                    .first()
                    .map(Declaration::span)
                    .cloned()
                    .unwrap_or(SourceSpan {
                        byte_start: 0,
                        byte_end: 0,
                        line: 1,
                        column: 1,
                    }),
                "package_module_name_mismatch",
                format!(
                    "descriptor names module '{}' but source declares '{}'",
                    locator.name, module.name
                ),
            ));
        }
        parsed.push((
            locator.path.clone(),
            module,
            document.semantic_bytes().to_vec(),
        ));
    }
    if let Some((path, _)) = by_path.into_iter().next() {
        return Err(semantic_without_location(
            "package_module_document_foreign",
            format!("source document '{path}' is not declared by the package"),
        ));
    }

    // The independent source oracle retains unresolved locator text only while parsing. Resolve
    // it to the same exact package/module identities required by canonical graph meaning before
    // any MeaningModule is constructed.
    let seed = descriptor.package_id.bytes().to_vec();
    let mut import_targets = BTreeMap::new();
    let mut local_module_targets = BTreeMap::new();
    for (index, locator) in descriptor.modules.iter().enumerate() {
        let ordinal = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| {
                semantic_without_location(
                    "package_module_identity_limit",
                    "source-oracle module identity ordinal was exhausted",
                )
            })?;
        let target = super::language::ModuleReference {
            package: descriptor.package_id.clone(),
            module: ModuleId::migrate(&seed, ordinal),
        };
        import_targets.insert(
            super::language::unresolved_import_reference(&locator.name).module,
            target.clone(),
        );
        local_module_targets.insert(locator.name.clone(), target);
    }
    for dependency in dependencies {
        for module in &dependency.package.modules {
            let locator = format!("{}.{}", dependency.alias, module.module.name);
            import_targets.insert(
                super::language::unresolved_import_reference(&locator).module,
                super::language::ModuleReference {
                    package: dependency.package.descriptor.package_id.clone(),
                    module: module.module_id,
                },
            );
        }
    }

    let mut declarations_by_module = BTreeMap::new();
    let mut declaration_ordinal = 0_u64;
    for (_, module, _) in &parsed {
        let target = local_module_targets.get(&module.name).ok_or_else(|| {
            semantic_without_location(
                "source_oracle_module_identity_missing",
                format!(
                    "source-oracle module '{}' has no exact identity",
                    module.name
                ),
            )
        })?;
        let mut declarations = Vec::new();
        for declaration in &module.declarations {
            declaration_ordinal = declaration_ordinal.checked_add(1).ok_or_else(|| {
                semantic_without_location(
                    "source_oracle_declaration_identity_limit",
                    "source-oracle declaration identity ordinal was exhausted",
                )
            })?;
            declarations.push((
                declaration.name().to_owned(),
                DeclarationReference {
                    package: descriptor.package_id.clone(),
                    module: target.module,
                    declaration: DeclarationId::migrate(&seed, declaration_ordinal),
                },
            ));
        }
        declarations_by_module.insert((descriptor.package_id.clone(), target.module), declarations);
    }
    for dependency in dependencies {
        for module in &dependency.package.modules {
            declarations_by_module.insert(
                (
                    dependency.package.descriptor.package_id.clone(),
                    module.module_id,
                ),
                module
                    .declaration_identities
                    .iter()
                    .map(|identity| {
                        (
                            identity.name.clone(),
                            DeclarationReference {
                                package: dependency.package.descriptor.package_id.clone(),
                                module: module.module_id,
                                declaration: identity.id,
                            },
                        )
                    })
                    .collect(),
            );
        }
    }

    let mut modules = Vec::new();
    let mut identity_allocator = MigrationIdentityAllocator::new(seed);
    for (path, mut module, semantic_bytes) in parsed {
        for import in &mut module.imports {
            let target = import_targets.get(&import.target.module).ok_or_else(|| {
                semantic_at(
                    &path,
                    import.span.clone(),
                    "source_oracle_import_missing",
                    format!(
                        "source import placeholder '{}' has no exact module binding",
                        import.target.module
                    ),
                )
            })?;
            import.target.clone_from(target);
        }
        resolve_source_module_references(
            &path,
            &descriptor.package_id,
            local_module_targets
                .get(&module.name)
                .ok_or_else(|| {
                    semantic_without_location(
                        "source_oracle_module_identity_missing",
                        format!(
                            "source-oracle module '{}' has no exact identity",
                            module.name
                        ),
                    )
                })?
                .module,
            &declarations_by_module,
            &mut module,
        )?;
        let meaning = MeaningModule::import(module.clone(), &mut identity_allocator)?;
        modules.push(ValidatedModule {
            path,
            module_id: meaning.module_id,
            module,
            declaration_identities: meaning.declarations,
            relations: meaning.relations,
            semantic_bytes,
        });
    }

    validate_package_modules(
        descriptor,
        modules,
        dependencies,
        None,
        None,
        RelationPolicy::Populate,
    )
}

#[cfg(test)]
fn resolve_source_module_references(
    path: &str,
    package: &PackageId,
    module_id: ModuleId,
    declarations_by_module: &BTreeMap<(PackageId, ModuleId), Vec<(String, DeclarationReference)>>,
    module: &mut Module,
) -> Result<(), Diagnostic> {
    let local_declarations = declarations_by_module
        .get(&(package.clone(), module_id))
        .ok_or_else(|| {
            semantic_without_location(
                "source_oracle_module_catalog_missing",
                format!(
                    "source-oracle module '{}' has no declaration catalog",
                    module.name
                ),
            )
        })?;
    let mut references = BTreeMap::new();
    let mut exports = BTreeMap::new();
    for (name, reference) in local_declarations {
        insert_source_reference(path, &mut references, name, reference.clone())?;
        exports.insert(
            super::language::unresolved_declaration_reference(name).declaration,
            reference.declaration,
        );
    }
    for import in &module.imports {
        let declarations = declarations_by_module
            .get(&(import.target.package.clone(), import.target.module))
            .ok_or_else(|| {
                semantic_at(
                    path,
                    import.span.clone(),
                    "source_oracle_import_catalog_missing",
                    format!(
                        "exact import target '{}:{}' has no declaration catalog",
                        import.target.package.as_str(),
                        import.target.module
                    ),
                )
            })?;
        for (name, reference) in declarations {
            insert_source_reference(
                path,
                &mut references,
                &format!("{}.{}", import.alias, name),
                reference.clone(),
            )?;
        }
    }
    for export in &mut module.exports {
        *export = exports.get(export).copied().ok_or_else(|| {
            semantic_without_location(
                "source_oracle_export_missing",
                format!(
                    "module '{}' exports an unresolved declaration placeholder '{}'",
                    module.name, export
                ),
            )
        })?;
    }
    for declaration in &mut module.declarations {
        resolve_source_declaration(path, &references, declaration)?;
    }
    Ok(())
}

#[cfg(test)]
fn insert_source_reference(
    path: &str,
    references: &mut BTreeMap<DeclarationReference, DeclarationReference>,
    locator: &str,
    reference: DeclarationReference,
) -> Result<(), Diagnostic> {
    let placeholder = super::language::unresolved_declaration_reference(locator);
    if let Some(previous) = references.insert(placeholder, reference.clone())
        && previous != reference
    {
        return Err(semantic_without_location(
            "source_oracle_reference_collision",
            format!("source-oracle declaration locator collision while resolving '{path}'"),
        ));
    }
    Ok(())
}

#[cfg(test)]
fn resolve_source_reference(
    path: &str,
    span: &SourceSpan,
    references: &BTreeMap<DeclarationReference, DeclarationReference>,
    reference: &mut DeclarationReference,
) -> Result<(), Diagnostic> {
    let exact = references.get(reference).ok_or_else(|| {
        semantic_at(
            path,
            span.clone(),
            "source_oracle_declaration_missing",
            format!(
                "source declaration placeholder '{}' has no exact binding",
                reference.declaration
            ),
        )
    })?;
    reference.clone_from(exact);
    Ok(())
}

#[cfg(test)]
fn resolve_source_type(
    path: &str,
    span: &SourceSpan,
    references: &BTreeMap<DeclarationReference, DeclarationReference>,
    ty: &mut Type,
) -> Result<(), Diagnostic> {
    match ty {
        Type::Named(reference) => resolve_source_reference(path, span, references, reference),
        Type::Record(fields) => {
            for field in fields {
                resolve_source_type(path, span, references, &mut field.ty)?;
            }
            Ok(())
        }
        Type::List(item) | Type::Option(item) | Type::Stream(item) => {
            resolve_source_type(path, span, references, item)
        }
        Type::Map(key, value) | Type::Result(key, value) => {
            resolve_source_type(path, span, references, key)?;
            resolve_source_type(path, span, references, value)
        }
        Type::Function(parameters, result) => {
            for parameter in parameters {
                resolve_source_type(path, span, references, parameter)?;
            }
            resolve_source_type(path, span, references, result)
        }
        Type::Unit
        | Type::Bool
        | Type::I64
        | Type::Bytes
        | Type::Text
        | Type::StaticText
        | Type::Secret
        | Type::Parameter(_) => Ok(()),
    }
}

#[cfg(test)]
fn resolve_source_declaration(
    path: &str,
    references: &BTreeMap<DeclarationReference, DeclarationReference>,
    declaration: &mut Declaration,
) -> Result<(), Diagnostic> {
    match declaration {
        Declaration::Record(record) => {
            for field in &mut record.fields {
                resolve_source_type(path, &field.span, references, &mut field.ty)?;
            }
        }
        Declaration::Variant(variant) => {
            for case in &mut variant.cases {
                if let Some(payload) = &mut case.payload {
                    resolve_source_type(path, &case.span, references, payload)?;
                }
            }
        }
        Declaration::Interface(interface) => {
            for operation in &mut interface.operations {
                for parameter in &mut operation.parameters {
                    resolve_source_type(path, &parameter.span, references, &mut parameter.ty)?;
                }
                resolve_source_type(path, &operation.span, references, &mut operation.result)?;
            }
        }
        Declaration::External(external) => {
            for parameter in &mut external.parameters {
                resolve_source_type(path, &parameter.span, references, &mut parameter.ty)?;
            }
            resolve_source_type(path, &external.span, references, &mut external.result)?;
        }
        Declaration::Function(function) => {
            for parameter in &mut function.parameters {
                resolve_source_type(path, &parameter.span, references, &mut parameter.ty)?;
            }
            resolve_source_type(path, &function.span, references, &mut function.result)?;
            if let Effect::Task { capabilities } = &mut function.effect {
                for capability in capabilities {
                    resolve_source_reference(
                        path,
                        &capability.span,
                        references,
                        &mut capability.interface,
                    )?;
                }
            }
            let mut variables = function
                .parameters
                .iter()
                .map(|parameter| parameter.name.clone())
                .collect();
            resolve_source_expression(path, references, &mut variables, &mut function.body)?;
        }
        Declaration::Constant(constant) => {
            resolve_source_type(path, &constant.span, references, &mut constant.ty)?;
            resolve_source_expression(path, references, &mut BTreeSet::new(), &mut constant.value)?;
        }
        Declaration::Component(component) => {
            for requirement in &mut component.requirements {
                resolve_source_reference(
                    path,
                    &requirement.span,
                    references,
                    &mut requirement.interface,
                )?;
            }
            for port in &mut component.ports {
                resolve_source_type(path, &port.span, references, &mut port.ty)?;
                resolve_source_expression(path, references, &mut BTreeSet::new(), &mut port.value)?;
            }
        }
        Declaration::Test(test) => {
            resolve_source_expression(path, references, &mut BTreeSet::new(), &mut test.actual)?;
            resolve_source_expression(path, references, &mut BTreeSet::new(), &mut test.expected)?;
        }
    }
    Ok(())
}

#[cfg(test)]
fn resolve_source_expression(
    path: &str,
    references: &BTreeMap<DeclarationReference, DeclarationReference>,
    variables: &mut BTreeSet<String>,
    expression: &mut Expression,
) -> Result<(), Diagnostic> {
    match expression {
        Expression::Variable(name, span) if !variables.contains(name) => {
            let mut reference = super::language::unresolved_declaration_reference(name);
            resolve_source_reference(path, span, references, &mut reference)?;
            *expression = Expression::Constant(reference, span.clone());
        }
        Expression::If {
            condition,
            when_true,
            when_false,
            ..
        } => {
            resolve_source_expression(path, references, variables, condition)?;
            resolve_source_expression(path, references, variables, when_true)?;
            resolve_source_expression(path, references, variables, when_false)?;
        }
        Expression::Let { bindings, body, .. } => {
            let mut local = variables.clone();
            for binding in bindings {
                resolve_source_expression(path, references, &mut local, &mut binding.value)?;
                local.insert(binding.name.clone());
            }
            resolve_source_expression(path, references, &mut local, body)?;
        }
        Expression::Do { expressions, .. } => {
            for expression in expressions {
                resolve_source_expression(path, references, variables, expression)?;
            }
        }
        Expression::Call {
            function,
            type_arguments,
            arguments,
            span,
        } => {
            resolve_source_reference(path, span, references, function)?;
            for ty in type_arguments {
                resolve_source_type(path, span, references, ty)?;
            }
            for argument in arguments {
                resolve_source_expression(path, references, variables, argument)?;
            }
        }
        Expression::Invoke {
            callee, arguments, ..
        } => {
            resolve_source_expression(path, references, variables, callee)?;
            for argument in arguments {
                resolve_source_expression(path, references, variables, argument)?;
            }
        }
        Expression::Record { ty, fields, span } => {
            if let Some(reference) = ty {
                resolve_source_reference(path, span, references, reference)?;
            }
            for field in fields {
                resolve_source_expression(path, references, variables, &mut field.value)?;
            }
        }
        Expression::Variant {
            ty, payload, span, ..
        } => {
            resolve_source_reference(path, span, references, ty)?;
            if let Some(payload) = payload {
                resolve_source_expression(path, references, variables, payload)?;
            }
        }
        Expression::Field { value, .. } => {
            resolve_source_expression(path, references, variables, value)?;
        }
        Expression::List {
            item_type,
            items,
            span,
        } => {
            resolve_source_type(path, span, references, item_type)?;
            for item in items {
                resolve_source_expression(path, references, variables, item)?;
            }
        }
        Expression::Map {
            key_type,
            value_type,
            entries,
            span,
        } => {
            resolve_source_type(path, span, references, key_type)?;
            resolve_source_type(path, span, references, value_type)?;
            for entry in entries {
                resolve_source_expression(path, references, variables, &mut entry.key)?;
                resolve_source_expression(path, references, variables, &mut entry.value)?;
            }
        }
        Expression::Match { value, arms, .. } => {
            resolve_source_expression(path, references, variables, value)?;
            for arm in arms {
                let mut local = variables.clone();
                if let Some(binding) = &arm.binding {
                    local.insert(binding.clone());
                }
                resolve_source_expression(path, references, &mut local, &mut arm.body)?;
            }
        }
        Expression::FunctionRef {
            function,
            type_arguments,
            span,
        } => {
            resolve_source_reference(path, span, references, function)?;
            for ty in type_arguments {
                resolve_source_type(path, span, references, ty)?;
            }
        }
        Expression::Perform { arguments, .. } => {
            for argument in arguments {
                resolve_source_expression(path, references, variables, argument)?;
            }
        }
        Expression::Transaction { body, .. } => {
            resolve_source_expression(path, references, variables, body)?;
        }
        Expression::Unit(_)
        | Expression::Bool(_, _)
        | Expression::I64(_, _)
        | Expression::Text(_, _)
        | Expression::StaticText(_, _)
        | Expression::Variable(_, _)
        | Expression::Constant(_, _) => {}
    }
    Ok(())
}

pub fn validate_graph_package(
    root: &GraphRoot,
    meanings: Vec<MeaningModule>,
    dependencies: &[ExactGraphDependency<'_>],
    accepted_revision: Option<RevisionId>,
) -> Result<ValidatedPackage, Diagnostic> {
    validate_graph_package_with_relations(
        root,
        meanings,
        dependencies,
        accepted_revision,
        RelationPolicy::Verify,
    )
}

/// Reconstructs every canonical semantic relation from owner meaning, refreshes packed module
/// bindings, and then verifies the result through the normal graph validator. This is used by the
/// transaction engine; callers never supply derived relation tables as authored input.
pub fn canonicalize_graph_package(
    root: &mut GraphRoot,
    meanings: &mut [MeaningModule],
    dependencies: &[ExactGraphDependency<'_>],
) -> Result<ValidatedPackage, Diagnostic> {
    for meaning in meanings.iter_mut() {
        super::meaning::normalize_module_spans(&mut meaning.module);
    }
    refresh_graph_module_objects(root, meanings)?;
    let populated = validate_graph_package_with_relations(
        root,
        meanings.to_vec(),
        dependencies,
        None,
        RelationPolicy::Populate,
    )?;
    for meaning in meanings.iter_mut() {
        let validated = populated
            .modules
            .iter()
            .find(|module| module.module_id == meaning.module_id)
            .ok_or_else(|| {
                semantic_without_location(
                    "graph_relation_module_missing",
                    "relation reconstruction lost a meaning module",
                )
            })?;
        meaning.relations.clone_from(&validated.relations);
    }
    refresh_graph_module_objects(root, meanings)?;
    validate_graph_package(root, meanings.to_vec(), dependencies, None)
}

pub fn refresh_graph_module_objects(
    root: &mut GraphRoot,
    meanings: &[MeaningModule],
) -> Result<(), Diagnostic> {
    if root.modules.len() != meanings.len() {
        return Err(semantic_without_location(
            "graph_module_binding_count",
            "graph root and meaning module counts differ",
        ));
    }
    for reference in &mut root.modules {
        let meaning = meanings
            .iter()
            .find(|meaning| meaning.module_id == reference.id)
            .ok_or_else(|| {
                semantic_without_location(
                    "graph_module_binding_missing",
                    format!(
                        "graph root module '{}' has no meaning object",
                        reference.name
                    ),
                )
            })?;
        reference.name.clone_from(&meaning.module.name);
        reference.object = meaning.digest()?;
    }
    root.modules.sort();
    root.validate_modules(meanings)
}

fn validate_graph_package_with_relations(
    root: &GraphRoot,
    meanings: Vec<MeaningModule>,
    dependencies: &[ExactGraphDependency<'_>],
    accepted_revision: Option<RevisionId>,
    relation_policy: RelationPolicy,
) -> Result<ValidatedPackage, Diagnostic> {
    root.validate_modules(&meanings)?;
    if dependencies.len() != root.dependencies.len() {
        return Err(semantic_without_location(
            "graph_dependency_count",
            "loaded graph dependency count does not match the canonical root",
        ));
    }
    let mut descriptor_dependencies = Vec::with_capacity(root.dependencies.len());
    let mut artifact_digests = Vec::with_capacity(root.dependencies.len());
    let mut matched_dependencies = Vec::with_capacity(root.dependencies.len());
    for binding in &root.dependencies {
        let dependency = dependencies
            .iter()
            .find(|dependency| dependency.alias == binding.alias)
            .ok_or_else(|| {
                semantic_without_location(
                    "graph_dependency_missing",
                    format!("exact dependency '{}' is unavailable", binding.alias),
                )
            })?;
        if dependency.package.descriptor.package_id != binding.package_id
            || dependency.package.accepted_revision != Some(binding.semantic_revision)
            || dependency.artifact != binding.artifact
        {
            return Err(semantic_without_location(
                "graph_dependency_mismatch",
                format!(
                    "exact dependency '{}' does not match its canonical binding",
                    binding.alias
                ),
            ));
        }
        let artifact = super::semantic_id::encode_hex(&dependency.artifact.bytes());
        artifact_digests.push(artifact.clone());
        matched_dependencies.push(dependency);
        descriptor_dependencies.push(Dependency {
            alias: binding.alias.clone(),
            package_id: binding.package_id.clone(),
            revision_digest: dependency.package.revision_digest.clone(),
            artifact_digest: artifact,
            artifact: "canonical-object".to_owned(),
        });
    }
    let mut exact_dependencies = Vec::with_capacity(dependencies.len());
    for (dependency, artifact_digest) in matched_dependencies.iter().zip(&artifact_digests) {
        exact_dependencies.push(ExactDependency {
            alias: dependency.alias,
            package: dependency.package,
            artifact_digest,
        });
    }
    let projected_targets = root
        .targets
        .iter()
        .map(|target| {
            let module = meanings
                .iter()
                .find(|module| module.module_id == target.component_module)
                .ok_or_else(|| {
                    semantic_without_location(
                        "graph_target_component_missing",
                        "validated target lost its exact component module",
                    )
                })?;
            let (identity, declaration) =
                module.declaration(target.component).ok_or_else(|| {
                    semantic_without_location(
                        "graph_target_component_missing",
                        "validated target lost its exact component declaration",
                    )
                })?;
            let Declaration::Component(component) = declaration else {
                return Err(semantic_without_location(
                    "graph_target_component_kind",
                    "validated target does not bind a component declaration",
                ));
            };
            let port_name = identity
                .members
                .iter()
                .find_map(|member| match member {
                    MemberIdentity::Port { id, name } if *id == target.port => Some(name.clone()),
                    _ => None,
                })
                .ok_or_else(|| {
                    semantic_without_location(
                        "graph_target_port_missing",
                        "validated target lost its exact component port",
                    )
                })?;
            Ok(Target {
                name: target.name.clone(),
                component: format!("{}.{}", module.module.name, component.name),
                port: port_name,
                runner: target.runner,
            })
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    let descriptor = PackageDescriptor {
        contract_version: super::package::PACKAGE_CONTRACT_VERSION,
        package_id: root.package_id.clone(),
        name: root.package_name.clone(),
        modules: root
            .modules
            .iter()
            .map(|module| ModuleLocator {
                name: module.name.clone(),
                path: format!("semantic:{}", module.id),
            })
            .collect(),
        dependencies: descriptor_dependencies,
        targets: projected_targets,
    };
    let mut by_id = meanings
        .into_iter()
        .map(|meaning| (meaning.module_id, meaning))
        .collect::<BTreeMap<_, _>>();
    let mut modules = Vec::with_capacity(root.modules.len());
    for reference in &root.modules {
        let meaning = by_id.remove(&reference.id).ok_or_else(|| {
            semantic_without_location(
                "graph_module_missing",
                format!("module object '{}' is missing", reference.name),
            )
        })?;
        let semantic_bytes = meaning.encode()?;
        modules.push(ValidatedModule {
            path: format!("semantic:{}", meaning.module_id),
            module_id: meaning.module_id,
            module: meaning.module,
            declaration_identities: meaning.declarations,
            relations: meaning.relations,
            semantic_bytes,
        });
    }
    validate_package_modules(
        descriptor,
        modules,
        &exact_dependencies,
        accepted_revision,
        matches!(relation_policy, RelationPolicy::Verify)
            .then(|| root.digest())
            .transpose()?,
        relation_policy,
    )
}

#[derive(Clone, Copy)]
enum RelationPolicy {
    Populate,
    Verify,
}

fn validate_package_modules(
    descriptor: PackageDescriptor,
    mut modules: Vec<ValidatedModule>,
    dependencies: &[ExactDependency<'_>],
    accepted_revision: Option<RevisionId>,
    graph_root_digest: Option<RootObjectDigest>,
    relation_policy: RelationPolicy,
) -> Result<ValidatedPackage, Diagnostic> {
    validate_exact_dependencies(&descriptor, dependencies)?;
    let (nominal_shapes, interfaces, constant_types, function_facts, derived_relations) = {
        let context = PackageContext {
            descriptor: &descriptor,
            modules: &modules,
            dependencies,
        };
        validate_imports_and_exports(&context)?;
        validate_declared_types(&context)?;
        let (nominal_shapes, interfaces) = collect_type_shapes(&context)?;
        let constant_types = collect_constant_types(&context)?;
        let signatures = collect_function_signatures(&context)?;
        for signature in signatures.values() {
            if let Some(implementation) = &signature.external_implementation {
                super::intrinsic_contract::validate_intrinsic(implementation, signature)?;
            }
        }
        let function_facts = validate_expressions_and_effects(
            &context,
            &signatures,
            &nominal_shapes,
            &interfaces,
            &constant_types,
        )?;
        let derived_relations = derive_semantic_relations(
            &context,
            &signatures,
            &nominal_shapes,
            &interfaces,
            &constant_types,
        )?;
        (
            nominal_shapes,
            interfaces,
            constant_types,
            function_facts,
            derived_relations,
        )
    };
    for module in &mut modules {
        let derived = derived_relations
            .get(&module.module_id)
            .cloned()
            .unwrap_or_default();
        match relation_policy {
            RelationPolicy::Populate => module.relations = derived,
            RelationPolicy::Verify if module.relations != derived => {
                return Err(semantic_without_location(
                    "semantic_relation_mismatch",
                    format!(
                        "module '{}' canonical relations do not match reconstructed meaning",
                        module.module.name
                    ),
                ));
            }
            RelationPolicy::Verify => {}
        }
    }
    let revision_digest = package_revision_digest(&descriptor, &modules)?;
    Ok(ValidatedPackage {
        descriptor,
        modules,
        revision_digest,
        accepted_revision,
        graph_root_digest,
        function_facts,
        nominal_shapes,
        interfaces,
        constant_types,
    })
}

struct PackageContext<'a> {
    descriptor: &'a PackageDescriptor,
    modules: &'a [ValidatedModule],
    dependencies: &'a [ExactDependency<'a>],
}

impl PackageContext<'_> {
    fn module(&self, name: &str) -> Option<&ValidatedModule> {
        self.modules
            .iter()
            .find(|module| module.module.name == name)
    }

    fn module_by_id(&self, id: ModuleId) -> Option<&ValidatedModule> {
        self.modules.iter().find(|module| module.module_id == id)
    }

    fn dependency_by_package(&self, package: &PackageId) -> Option<&ExactDependency<'_>> {
        self.dependencies
            .iter()
            .find(|dependency| dependency.package.descriptor.package_id == *package)
    }

    fn owner(&self, module: &str, declaration: &str) -> Result<OwnerId, Diagnostic> {
        let module = self.module(module).ok_or_else(|| {
            semantic_without_location(
                "semantic_owner_module_missing",
                format!("semantic owner module '{module}' is absent"),
            )
        })?;
        validated_owner(&self.descriptor.package_id, module, declaration)
    }

    fn resolve_reference<'a>(
        &'a self,
        from: &'a ValidatedModule,
        reference: &DeclarationReference,
    ) -> Result<ResolvedDeclaration<'a>, Diagnostic> {
        let same_module =
            reference.package == self.descriptor.package_id && reference.module == from.module_id;
        let (package, module) = if same_module {
            let module = self.module_by_id(reference.module).ok_or_else(|| {
                semantic_at(
                    &from.path,
                    semantic_reference_span(),
                    "semantic_reference_module_missing",
                    format!(
                        "exact declaration reference names absent local module '{}'",
                        reference.module
                    ),
                )
            })?;
            (&self.descriptor.package_id, module)
        } else {
            let import = from
                .module
                .imports
                .iter()
                .find(|import| {
                    import.target.package == reference.package
                        && import.target.module == reference.module
                })
                .ok_or_else(|| {
                    semantic_at(
                        &from.path,
                        semantic_reference_span(),
                        "semantic_reference_import_missing",
                        format!(
                            "exact declaration reference '{}:{}:{}' is outside the module's imports",
                            reference.package.as_str(),
                            reference.module,
                            reference.declaration
                        ),
                    )
                })?;
            self.imported_module(from, import)?
        };
        let (identity, declaration) = module
            .declaration_identities
            .iter()
            .zip(&module.module.declarations)
            .find(|(identity, _)| identity.id == reference.declaration)
            .ok_or_else(|| {
                semantic_at(
                    &from.path,
                    semantic_reference_span(),
                    "semantic_declaration_missing",
                    format!(
                        "module '{}' has no declaration identity '{}'",
                        module.module.name, reference.declaration
                    ),
                )
            })?;
        if !same_module && !module.module.exports.contains(&identity.id) {
            return Err(semantic_at(
                &from.path,
                semantic_reference_span(),
                "semantic_import_private",
                format!(
                    "module '{}' does not export declaration '{}'",
                    module.module.name, identity.name
                ),
            ));
        }
        Ok(ResolvedDeclaration {
            owner: OwnerId {
                package: package.clone(),
                module_id: module.module_id,
                declaration_id: identity.id,
                module: module.module.name.clone(),
                declaration: identity.name.clone(),
            },
            declaration,
        })
    }

    fn imported_module<'a>(
        &'a self,
        from: &ValidatedModule,
        import: &super::language::Import,
    ) -> Result<(&'a PackageId, &'a ValidatedModule), Diagnostic> {
        if import.target.package == self.descriptor.package_id {
            let module = self.module_by_id(import.target.module).ok_or_else(|| {
                semantic_at(
                    &from.path,
                    import.span.clone(),
                    "semantic_local_module_missing",
                    format!("local module '{}' does not exist", import.target.module),
                )
            })?;
            return Ok((&self.descriptor.package_id, module));
        }
        if let Some(dependency) = self.dependency_by_package(&import.target.package) {
            let module = dependency
                .package
                .modules
                .iter()
                .find(|module| module.module_id == import.target.module)
                .ok_or_else(|| {
                    semantic_at(
                        &from.path,
                        import.span.clone(),
                        "semantic_dependency_module_missing",
                        format!(
                            "dependency package '{}' has no module '{}'",
                            import.target.package.as_str(),
                            import.target.module
                        ),
                    )
                })?;
            return Ok((&dependency.package.descriptor.package_id, module));
        }
        Err(semantic_at(
            &from.path,
            import.span.clone(),
            "semantic_import_package_missing",
            format!(
                "import target package '{}' is neither this package nor an exact dependency",
                import.target.package.as_str()
            ),
        ))
    }

    fn owner_module<'a>(&'a self, owner: &OwnerId) -> Option<&'a ValidatedModule> {
        if owner.package == self.descriptor.package_id {
            return self
                .modules
                .iter()
                .find(|module| module.module_id == owner.module_id);
        }
        self.dependencies
            .iter()
            .find(|dependency| dependency.package.descriptor.package_id == owner.package)
            .and_then(|dependency| {
                dependency
                    .package
                    .modules
                    .iter()
                    .find(|module| module.module_id == owner.module_id)
            })
    }

    fn declaration_identity<'a>(
        &'a self,
        owner: &OwnerId,
    ) -> Result<&'a DeclarationIdentity, Diagnostic> {
        self.owner_module(owner)
            .and_then(|module| {
                module
                    .declaration_identities
                    .iter()
                    .find(|identity| identity.id == owner.declaration_id)
            })
            .ok_or_else(|| {
                semantic_without_location(
                    "semantic_relation_owner_missing",
                    format!(
                        "semantic relation target '{}' is absent from the exact package closure",
                        owner.diagnostic_name()
                    ),
                )
            })
    }
}

type RelationSets = BTreeMap<ModuleId, BTreeSet<SemanticRelation>>;

fn derive_semantic_relations(
    context: &PackageContext<'_>,
    signatures: &BTreeMap<OwnerId, FunctionSignature>,
    nominal_shapes: &BTreeMap<OwnerId, NominalShape>,
    interfaces: &BTreeMap<OwnerId, ResolvedInterface>,
    constant_types: &BTreeMap<OwnerId, ResolvedType>,
) -> Result<BTreeMap<ModuleId, Vec<SemanticRelation>>, Diagnostic> {
    let inference = InferContext {
        package: context,
        signatures,
        nominal_shapes,
        interfaces,
        constant_types,
        type_parameters: BTreeMap::new(),
        allow_task_function_values: false,
    };
    let site_types = collect_expression_site_types(&inference)?;
    let mut relations = RelationSets::new();
    for module in context.modules {
        relations.entry(module.module_id).or_default();
        collect_module_relations(context, module, &site_types, &mut relations)?;
    }
    Ok(relations
        .into_iter()
        .map(|(module, relations)| (module, relations.into_iter().collect()))
        .collect())
}

fn collect_module_relations(
    context: &PackageContext<'_>,
    module: &ValidatedModule,
    site_types: &BTreeMap<usize, ResolvedType>,
    relations: &mut RelationSets,
) -> Result<(), Diagnostic> {
    for import in &module.module.imports {
        context.imported_module(module, import)?;
        insert_relation(
            relations,
            module.module_id,
            RelationSource::Module(module.module_id),
            RelationTarget::Module(import.target.clone()),
            RelationRole::Import,
        );
    }
    for export in &module.module.exports {
        let owner = validated_owner_by_id(&context.descriptor.package_id, module, *export)?;
        insert_relation(
            relations,
            module.module_id,
            RelationSource::Module(module.module_id),
            declaration_target(&owner),
            RelationRole::Export,
        );
    }
    for (declaration, identity) in module
        .module
        .declarations
        .iter()
        .zip(&module.declaration_identities)
    {
        collect_declaration_relations(
            context,
            module,
            declaration,
            identity,
            site_types,
            relations,
        )?;
    }
    Ok(())
}

fn collect_declaration_relations(
    context: &PackageContext<'_>,
    module: &ValidatedModule,
    declaration: &Declaration,
    identity: &DeclarationIdentity,
    site_types: &BTreeMap<usize, ResolvedType>,
    relations: &mut RelationSets,
) -> Result<(), Diagnostic> {
    let owner = context.owner(&module.module.name, declaration.name())?;
    let owner_reference = declaration_reference(&owner);
    let catalog = IdentityCatalog::new(identity)?;
    let mut members = identity.members.iter();
    match declaration {
        Declaration::Record(record) => {
            for field in &record.fields {
                let MemberIdentity::Field { id, .. } = next_member(&mut members, identity)? else {
                    return Err(relation_shape(identity));
                };
                collect_type_relations(
                    context,
                    module,
                    RelationSource::Field(*id),
                    &field.ty,
                    relations,
                )?;
            }
        }
        Declaration::Variant(variant) => {
            for case in &variant.cases {
                let MemberIdentity::Case { id, .. } = next_member(&mut members, identity)? else {
                    return Err(relation_shape(identity));
                };
                if let Some(payload) = &case.payload {
                    collect_type_relations(
                        context,
                        module,
                        RelationSource::Case(*id),
                        payload,
                        relations,
                    )?;
                }
            }
        }
        Declaration::Interface(interface) => {
            for operation in &interface.operations {
                let MemberIdentity::Operation { id, .. } = next_member(&mut members, identity)?
                else {
                    return Err(relation_shape(identity));
                };
                for parameter in &operation.parameters {
                    let MemberIdentity::Parameter { id, .. } = next_member(&mut members, identity)?
                    else {
                        return Err(relation_shape(identity));
                    };
                    collect_type_relations(
                        context,
                        module,
                        RelationSource::Parameter(*id),
                        &parameter.ty,
                        relations,
                    )?;
                }
                collect_type_relations(
                    context,
                    module,
                    RelationSource::Operation(*id),
                    &operation.result,
                    relations,
                )?;
            }
        }
        Declaration::External(external) => {
            let mut type_parameters = BTreeMap::new();
            for parameter in &external.type_parameters {
                let MemberIdentity::TypeParameter { id, .. } = next_member(&mut members, identity)?
                else {
                    return Err(relation_shape(identity));
                };
                type_parameters.insert(
                    parameter.name.clone(),
                    RelationTarget::TypeParameter {
                        owner: owner_reference.clone(),
                        type_parameter: *id,
                    },
                );
            }
            for parameter in &external.parameters {
                let MemberIdentity::Parameter { id, .. } = next_member(&mut members, identity)?
                else {
                    return Err(relation_shape(identity));
                };
                collect_type_relations_in_scope(
                    context,
                    module,
                    RelationSource::Parameter(*id),
                    &parameter.ty,
                    &type_parameters,
                    relations,
                )?;
            }
            collect_type_relations_in_scope(
                context,
                module,
                RelationSource::Declaration(identity.id),
                &external.result,
                &type_parameters,
                relations,
            )?;
        }
        Declaration::Function(function) => {
            let mut type_parameters = BTreeMap::new();
            for parameter in &function.type_parameters {
                let MemberIdentity::TypeParameter { id, .. } = next_member(&mut members, identity)?
                else {
                    return Err(relation_shape(identity));
                };
                type_parameters.insert(
                    parameter.name.clone(),
                    RelationTarget::TypeParameter {
                        owner: owner_reference.clone(),
                        type_parameter: *id,
                    },
                );
            }
            let mut variables = BTreeMap::new();
            for parameter in &function.parameters {
                let MemberIdentity::Parameter { id, .. } = next_member(&mut members, identity)?
                else {
                    return Err(relation_shape(identity));
                };
                let target = RelationTarget::Parameter {
                    owner: owner_reference.clone(),
                    parameter: *id,
                };
                variables.insert(parameter.name.clone(), target);
                collect_type_relations_in_scope(
                    context,
                    module,
                    RelationSource::Parameter(*id),
                    &parameter.ty,
                    &type_parameters,
                    relations,
                )?;
            }
            let mut capabilities = BTreeMap::new();
            if let Effect::Task {
                capabilities: declared,
            } = &function.effect
            {
                for capability in declared {
                    let MemberIdentity::TaskRequirement { id, .. } =
                        next_member(&mut members, identity)?
                    else {
                        return Err(relation_shape(identity));
                    };
                    let resolved = context.resolve_reference(module, &capability.interface)?;
                    let target = RelationTarget::Requirement {
                        owner: owner_reference.clone(),
                        requirement: *id,
                    };
                    insert_relation(
                        relations,
                        module.module_id,
                        RelationSource::Requirement(*id),
                        declaration_target(&resolved.owner),
                        RelationRole::CapabilityInterface,
                    );
                    capabilities.insert(
                        capability.alias.clone(),
                        RelationCapability {
                            binding: target,
                            interface: resolved.owner,
                        },
                    );
                }
            }
            collect_type_relations_in_scope(
                context,
                module,
                RelationSource::Declaration(identity.id),
                &function.result,
                &type_parameters,
                relations,
            )?;
            collect_expression_relations(
                context,
                module,
                &catalog,
                &owner_reference,
                &function.body,
                vec![0],
                &variables,
                &capabilities,
                &type_parameters,
                site_types,
                None,
                relations,
            )?;
        }
        Declaration::Constant(constant) => {
            collect_type_relations(
                context,
                module,
                RelationSource::Declaration(identity.id),
                &constant.ty,
                relations,
            )?;
            collect_expression_relations(
                context,
                module,
                &catalog,
                &owner_reference,
                &constant.value,
                vec![0],
                &BTreeMap::new(),
                &BTreeMap::new(),
                &BTreeMap::new(),
                site_types,
                None,
                relations,
            )?;
        }
        Declaration::Component(component) => {
            for requirement in &component.requirements {
                let MemberIdentity::ComponentRequirement { id, .. } =
                    next_member(&mut members, identity)?
                else {
                    return Err(relation_shape(identity));
                };
                let resolved = context.resolve_reference(module, &requirement.interface)?;
                insert_relation(
                    relations,
                    module.module_id,
                    RelationSource::Requirement(*id),
                    declaration_target(&resolved.owner),
                    RelationRole::CapabilityInterface,
                );
                for operation in &requirement.operations {
                    insert_relation(
                        relations,
                        module.module_id,
                        RelationSource::Requirement(*id),
                        operation_target(context, &resolved.owner, operation)?,
                        RelationRole::CapabilityOperation,
                    );
                }
            }
            for (index, port) in component.ports.iter().enumerate() {
                let MemberIdentity::Port { id, .. } = next_member(&mut members, identity)? else {
                    return Err(relation_shape(identity));
                };
                collect_type_relations(
                    context,
                    module,
                    RelationSource::Port(*id),
                    &port.ty,
                    relations,
                )?;
                if let Expression::FunctionRef { function, .. } = &port.value {
                    let resolved = context.resolve_reference(module, function)?;
                    insert_relation(
                        relations,
                        module.module_id,
                        RelationSource::Port(*id),
                        declaration_target(&resolved.owner),
                        RelationRole::ComponentPortFunction,
                    );
                }
                collect_expression_relations(
                    context,
                    module,
                    &catalog,
                    &owner_reference,
                    &port.value,
                    vec![u32::try_from(index).map_err(|_| relation_work_limit())?],
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    site_types,
                    None,
                    relations,
                )?;
            }
        }
        Declaration::Test(test) => {
            for (ordinal, expression) in [(0, &test.actual), (1, &test.expected)] {
                collect_expression_relations(
                    context,
                    module,
                    &catalog,
                    &owner_reference,
                    expression,
                    vec![ordinal],
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    &BTreeMap::new(),
                    site_types,
                    Some(identity.id),
                    relations,
                )?;
            }
        }
    }
    if members.next().is_some() {
        return Err(relation_shape(identity));
    }
    Ok(())
}

fn collect_type_relations(
    context: &PackageContext<'_>,
    module: &ValidatedModule,
    source: RelationSource,
    ty: &Type,
    relations: &mut RelationSets,
) -> Result<(), Diagnostic> {
    collect_type_relations_in_scope(context, module, source, ty, &BTreeMap::new(), relations)
}

fn collect_type_relations_in_scope(
    context: &PackageContext<'_>,
    module: &ValidatedModule,
    source: RelationSource,
    ty: &Type,
    type_parameters: &BTreeMap<String, RelationTarget>,
    relations: &mut RelationSets,
) -> Result<(), Diagnostic> {
    match ty {
        Type::Parameter(name) => {
            let target = type_parameters.get(name).cloned().ok_or_else(|| {
                semantic_without_location(
                    "semantic_relation_type_parameter_missing",
                    format!("type parameter '{name}' has no stable identity in this declaration"),
                )
            })?;
            insert_relation(
                relations,
                module.module_id,
                source,
                target,
                RelationRole::TypeUse,
            );
        }
        Type::Named(reference) => {
            let resolved = context.resolve_reference(module, reference)?;
            insert_relation(
                relations,
                module.module_id,
                source,
                declaration_target(&resolved.owner),
                RelationRole::TypeUse,
            );
        }
        Type::Record(fields) => {
            for field in fields {
                collect_type_relations_in_scope(
                    context,
                    module,
                    source.clone(),
                    &field.ty,
                    type_parameters,
                    relations,
                )?;
            }
        }
        Type::List(item) | Type::Option(item) | Type::Stream(item) => {
            collect_type_relations_in_scope(
                context,
                module,
                source,
                item,
                type_parameters,
                relations,
            )?;
        }
        Type::Map(key, value) | Type::Result(key, value) => {
            collect_type_relations_in_scope(
                context,
                module,
                source.clone(),
                key,
                type_parameters,
                relations,
            )?;
            collect_type_relations_in_scope(
                context,
                module,
                source,
                value,
                type_parameters,
                relations,
            )?;
        }
        Type::Function(parameters, result) => {
            for parameter in parameters {
                collect_type_relations_in_scope(
                    context,
                    module,
                    source.clone(),
                    parameter,
                    type_parameters,
                    relations,
                )?;
            }
            collect_type_relations_in_scope(
                context,
                module,
                source,
                result,
                type_parameters,
                relations,
            )?;
        }
        Type::Unit
        | Type::Bool
        | Type::I64
        | Type::Bytes
        | Type::Text
        | Type::StaticText
        | Type::Secret => {}
    }
    Ok(())
}

fn next_member<'a>(
    members: &mut impl Iterator<Item = &'a MemberIdentity>,
    identity: &DeclarationIdentity,
) -> Result<&'a MemberIdentity, Diagnostic> {
    members.next().ok_or_else(|| relation_shape(identity))
}

fn relation_shape(identity: &DeclarationIdentity) -> Diagnostic {
    semantic_without_location(
        "semantic_relation_identity_shape",
        format!(
            "declaration '{}' identity catalog cannot reconstruct canonical relations",
            identity.name
        ),
    )
}

fn relation_work_limit() -> Diagnostic {
    semantic_without_location(
        "semantic_relation_work_limit",
        "semantic relation traversal exceeded its exact ordinal range",
    )
}

fn declaration_reference(owner: &OwnerId) -> DeclarationReference {
    DeclarationReference {
        package: owner.package.clone(),
        module: owner.module_id,
        declaration: owner.declaration_id,
    }
}

fn declaration_target(owner: &OwnerId) -> RelationTarget {
    RelationTarget::Declaration(declaration_reference(owner))
}

fn insert_relation(
    relations: &mut RelationSets,
    module: ModuleId,
    source: RelationSource,
    target: RelationTarget,
    role: RelationRole,
) {
    relations
        .entry(module)
        .or_default()
        .insert(SemanticRelation {
            source,
            target,
            role,
        });
}

fn operation_target(
    context: &PackageContext<'_>,
    owner: &OwnerId,
    name: &str,
) -> Result<RelationTarget, Diagnostic> {
    let identity = context.declaration_identity(owner)?;
    identity
        .members
        .iter()
        .find_map(|member| match member {
            MemberIdentity::Operation {
                id,
                name: candidate,
            } if candidate == name => Some(RelationTarget::Operation {
                owner: declaration_reference(owner),
                operation: *id,
            }),
            _ => None,
        })
        .ok_or_else(|| {
            semantic_without_location(
                "semantic_relation_operation_missing",
                format!(
                    "interface '{}' has no stable operation identity for '{name}'",
                    owner.diagnostic_name()
                ),
            )
        })
}

fn field_target(
    context: &PackageContext<'_>,
    owner: &OwnerId,
    name: &str,
) -> Result<RelationTarget, Diagnostic> {
    let identity = context.declaration_identity(owner)?;
    identity
        .members
        .iter()
        .find_map(|member| match member {
            MemberIdentity::Field {
                id,
                name: candidate,
            } if candidate == name => Some(RelationTarget::Field {
                owner: declaration_reference(owner),
                field: *id,
            }),
            _ => None,
        })
        .ok_or_else(|| {
            semantic_without_location(
                "semantic_relation_field_missing",
                format!(
                    "record '{}' has no stable field identity for '{name}'",
                    owner.diagnostic_name()
                ),
            )
        })
}

fn case_target(
    context: &PackageContext<'_>,
    owner: &OwnerId,
    name: &str,
) -> Result<RelationTarget, Diagnostic> {
    let identity = context.declaration_identity(owner)?;
    identity
        .members
        .iter()
        .find_map(|member| match member {
            MemberIdentity::Case {
                id,
                name: candidate,
            } if candidate == name => Some(RelationTarget::Case {
                owner: declaration_reference(owner),
                case: *id,
            }),
            _ => None,
        })
        .ok_or_else(|| {
            semantic_without_location(
                "semantic_relation_case_missing",
                format!(
                    "variant '{}' has no stable case identity for '{name}'",
                    owner.diagnostic_name()
                ),
            )
        })
}

#[derive(Clone)]
struct RelationCapability {
    binding: RelationTarget,
    interface: OwnerId,
}

struct IdentityCatalog {
    expressions: BTreeMap<Vec<u32>, ExpressionId>,
    bindings: BTreeMap<(Vec<u32>, u32), super::semantic_id::BindingId>,
}

impl IdentityCatalog {
    fn new(identity: &DeclarationIdentity) -> Result<Self, Diagnostic> {
        let expressions = identity
            .expressions
            .iter()
            .map(|expression| (expression.path.clone(), expression.id))
            .collect::<BTreeMap<_, _>>();
        let bindings = identity
            .bindings
            .iter()
            .map(|binding| ((binding.expression_path.clone(), binding.slot), binding.id))
            .collect::<BTreeMap<_, _>>();
        if expressions.len() != identity.expressions.len()
            || bindings.len() != identity.bindings.len()
        {
            return Err(relation_shape(identity));
        }
        Ok(Self {
            expressions,
            bindings,
        })
    }

    fn expression(&self, path: &[u32]) -> Result<ExpressionId, Diagnostic> {
        self.expressions.get(path).copied().ok_or_else(|| {
            semantic_without_location(
                "semantic_relation_expression_missing",
                format!("semantic expression site {path:?} has no stable identity"),
            )
        })
    }

    fn binding(
        &self,
        path: &[u32],
        slot: u32,
    ) -> Result<super::semantic_id::BindingId, Diagnostic> {
        self.bindings
            .get(&(path.to_vec(), slot))
            .copied()
            .ok_or_else(|| {
                semantic_without_location(
                    "semantic_relation_binding_missing",
                    format!(
                        "semantic binding at expression site {path:?} slot {slot} has no stable identity"
                    ),
                )
            })
    }
}

fn collect_expression_site_types(
    inference: &InferContext<'_>,
) -> Result<BTreeMap<usize, ResolvedType>, Diagnostic> {
    let mut site_types = BTreeMap::new();
    for module in inference.package.modules {
        for declaration in &module.module.declarations {
            match declaration {
                Declaration::Function(function) => {
                    let owner = inference
                        .package
                        .owner(&module.module.name, &function.name)?;
                    let signature = inference.signatures.get(&owner).ok_or_else(|| {
                        semantic_without_location(
                            "semantic_relation_signature_missing",
                            format!("function '{}' has no exact signature", function.name),
                        )
                    })?;
                    let variables = function
                        .parameters
                        .iter()
                        .zip(&signature.parameters)
                        .map(|(parameter, ty)| (parameter.name.clone(), ty.clone()))
                        .collect();
                    let capabilities = signature
                        .task_capabilities
                        .iter()
                        .map(|capability| {
                            (
                                capability.alias.clone(),
                                CapabilityBinding {
                                    interface: capability.interface.clone(),
                                    report_alias: capability.alias.clone(),
                                },
                            )
                        })
                        .collect();
                    let function_inference = inference.for_signature(signature);
                    let facts = infer_expression(
                        &function_inference,
                        module,
                        &function.body,
                        &variables,
                        &capabilities,
                        matches!(function.effect, Effect::Pure),
                    )?;
                    extend_site_types(&mut site_types, facts.site_types)?;
                }
                Declaration::Constant(constant) => {
                    let facts = infer_expression(
                        inference,
                        module,
                        &constant.value,
                        &BTreeMap::new(),
                        &BTreeMap::new(),
                        true,
                    )?;
                    extend_site_types(&mut site_types, facts.site_types)?;
                }
                Declaration::Component(component) => {
                    let component_inference = inference.for_component_port();
                    for port in &component.ports {
                        let facts = infer_expression(
                            &component_inference,
                            module,
                            &port.value,
                            &BTreeMap::new(),
                            &BTreeMap::new(),
                            true,
                        )?;
                        extend_site_types(&mut site_types, facts.site_types)?;
                    }
                }
                Declaration::Test(test) => {
                    for expression in [&test.actual, &test.expected] {
                        let facts = infer_expression(
                            inference,
                            module,
                            expression,
                            &BTreeMap::new(),
                            &BTreeMap::new(),
                            true,
                        )?;
                        extend_site_types(&mut site_types, facts.site_types)?;
                    }
                }
                Declaration::Record(_)
                | Declaration::Variant(_)
                | Declaration::Interface(_)
                | Declaration::External(_) => {}
            }
        }
    }
    Ok(site_types)
}

fn extend_site_types(
    target: &mut BTreeMap<usize, ResolvedType>,
    source: BTreeMap<usize, ResolvedType>,
) -> Result<(), Diagnostic> {
    for (site, ty) in source {
        if target.insert(site, ty).is_some() {
            return Err(semantic_without_location(
                "semantic_relation_site_duplicate",
                "one semantic expression site was inferred more than once",
            ));
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn collect_expression_relations(
    context: &PackageContext<'_>,
    module: &ValidatedModule,
    catalog: &IdentityCatalog,
    declaration_owner: &DeclarationReference,
    expression: &Expression,
    path: Vec<u32>,
    variables: &BTreeMap<String, RelationTarget>,
    capabilities: &BTreeMap<String, RelationCapability>,
    type_parameters: &BTreeMap<String, RelationTarget>,
    site_types: &BTreeMap<usize, ResolvedType>,
    test_owner: Option<DeclarationId>,
    relations: &mut RelationSets,
) -> Result<(), Diagnostic> {
    let expression_id = catalog.expression(&path)?;
    let source = RelationSource::Expression(expression_id);
    match expression {
        Expression::Unit(_)
        | Expression::Bool(_, _)
        | Expression::I64(_, _)
        | Expression::Text(_, _)
        | Expression::StaticText(_, _) => {}
        Expression::Variable(name, _) => {
            let target = variables.get(name).cloned().ok_or_else(|| {
                semantic_without_location(
                    "semantic_relation_variable_missing",
                    format!("lexical variable '{name}' has no exact binding"),
                )
            })?;
            insert_expression_relation(
                relations,
                module.module_id,
                source,
                target,
                RelationRole::ValueReference,
                test_owner,
            );
        }
        Expression::Constant(reference, _) => {
            let resolved = context.resolve_reference(module, reference)?;
            insert_expression_relation(
                relations,
                module.module_id,
                source,
                declaration_target(&resolved.owner),
                RelationRole::ValueReference,
                test_owner,
            );
        }
        Expression::If {
            condition,
            when_true,
            when_false,
            ..
        } => {
            for (ordinal, child) in [
                (0usize, condition.as_ref()),
                (1usize, when_true.as_ref()),
                (2usize, when_false.as_ref()),
            ] {
                collect_expression_relations(
                    context,
                    module,
                    catalog,
                    declaration_owner,
                    child,
                    relation_child_path(&path, ordinal)?,
                    variables,
                    capabilities,
                    type_parameters,
                    site_types,
                    test_owner,
                    relations,
                )?;
            }
        }
        Expression::Let { bindings, body, .. } => {
            let mut local = variables.clone();
            for (index, binding) in bindings.iter().enumerate() {
                collect_expression_relations(
                    context,
                    module,
                    catalog,
                    declaration_owner,
                    &binding.value,
                    relation_child_path(&path, index)?,
                    &local,
                    capabilities,
                    type_parameters,
                    site_types,
                    test_owner,
                    relations,
                )?;
                let slot = u32::try_from(index).map_err(|_| relation_work_limit())?;
                local.insert(
                    binding.name.clone(),
                    RelationTarget::Binding {
                        owner: declaration_owner.clone(),
                        binding: catalog.binding(&path, slot)?,
                    },
                );
            }
            collect_expression_relations(
                context,
                module,
                catalog,
                declaration_owner,
                body,
                relation_child_path(&path, bindings.len())?,
                &local,
                capabilities,
                type_parameters,
                site_types,
                test_owner,
                relations,
            )?;
        }
        Expression::Do { expressions, .. }
        | Expression::List {
            items: expressions, ..
        } => {
            if let Expression::List { item_type, .. } = expression {
                collect_type_relations_in_scope(
                    context,
                    module,
                    source.clone(),
                    item_type,
                    type_parameters,
                    relations,
                )?;
            }
            for (index, child) in expressions.iter().enumerate() {
                collect_expression_relations(
                    context,
                    module,
                    catalog,
                    declaration_owner,
                    child,
                    relation_child_path(&path, index)?,
                    variables,
                    capabilities,
                    type_parameters,
                    site_types,
                    test_owner,
                    relations,
                )?;
            }
        }
        Expression::Call {
            function,
            type_arguments,
            arguments,
            ..
        } => {
            for ty in type_arguments {
                collect_type_relations_in_scope(
                    context,
                    module,
                    source.clone(),
                    ty,
                    type_parameters,
                    relations,
                )?;
            }
            let resolved = context.resolve_reference(module, function)?;
            insert_expression_relation(
                relations,
                module.module_id,
                source.clone(),
                declaration_target(&resolved.owner),
                RelationRole::Call,
                test_owner,
            );
            for (index, argument) in arguments.iter().enumerate() {
                collect_expression_relations(
                    context,
                    module,
                    catalog,
                    declaration_owner,
                    argument,
                    relation_child_path(&path, index)?,
                    variables,
                    capabilities,
                    type_parameters,
                    site_types,
                    test_owner,
                    relations,
                )?;
            }
        }
        Expression::Invoke {
            callee, arguments, ..
        } => {
            collect_expression_relations(
                context,
                module,
                catalog,
                declaration_owner,
                callee,
                relation_child_path(&path, 0)?,
                variables,
                capabilities,
                type_parameters,
                site_types,
                test_owner,
                relations,
            )?;
            for (index, argument) in arguments.iter().enumerate() {
                collect_expression_relations(
                    context,
                    module,
                    catalog,
                    declaration_owner,
                    argument,
                    relation_child_path(
                        &path,
                        index.checked_add(1).ok_or_else(relation_work_limit)?,
                    )?,
                    variables,
                    capabilities,
                    type_parameters,
                    site_types,
                    test_owner,
                    relations,
                )?;
            }
        }
        Expression::Record { ty, fields, .. } => {
            if let Some(reference) = ty {
                let resolved = context.resolve_reference(module, reference)?;
                insert_expression_relation(
                    relations,
                    module.module_id,
                    source.clone(),
                    declaration_target(&resolved.owner),
                    RelationRole::TypeUse,
                    test_owner,
                );
                for field in fields {
                    insert_expression_relation(
                        relations,
                        module.module_id,
                        source.clone(),
                        field_target(context, &resolved.owner, &field.name)?,
                        RelationRole::FieldUse,
                        test_owner,
                    );
                }
            }
            for (index, field) in fields.iter().enumerate() {
                collect_expression_relations(
                    context,
                    module,
                    catalog,
                    declaration_owner,
                    &field.value,
                    relation_child_path(&path, index)?,
                    variables,
                    capabilities,
                    type_parameters,
                    site_types,
                    test_owner,
                    relations,
                )?;
            }
        }
        Expression::Variant {
            ty, case, payload, ..
        } => {
            let resolved = context.resolve_reference(module, ty)?;
            insert_expression_relation(
                relations,
                module.module_id,
                source.clone(),
                declaration_target(&resolved.owner),
                RelationRole::VariantConstruction,
                test_owner,
            );
            insert_expression_relation(
                relations,
                module.module_id,
                source,
                case_target(context, &resolved.owner, case)?,
                RelationRole::VariantConstruction,
                test_owner,
            );
            if let Some(payload) = payload {
                collect_expression_relations(
                    context,
                    module,
                    catalog,
                    declaration_owner,
                    payload,
                    relation_child_path(&path, 0)?,
                    variables,
                    capabilities,
                    type_parameters,
                    site_types,
                    test_owner,
                    relations,
                )?;
            }
        }
        Expression::Field { value, field, .. } => {
            collect_expression_relations(
                context,
                module,
                catalog,
                declaration_owner,
                value,
                relation_child_path(&path, 0)?,
                variables,
                capabilities,
                type_parameters,
                site_types,
                test_owner,
                relations,
            )?;
            if let Some(ResolvedType::Nominal(owner)) = site_types.get(&expression_pointer(value)) {
                insert_expression_relation(
                    relations,
                    module.module_id,
                    source,
                    field_target(context, owner, field)?,
                    RelationRole::FieldUse,
                    test_owner,
                );
            }
        }
        Expression::Map {
            key_type,
            value_type,
            entries,
            ..
        } => {
            collect_type_relations_in_scope(
                context,
                module,
                source.clone(),
                key_type,
                type_parameters,
                relations,
            )?;
            collect_type_relations_in_scope(
                context,
                module,
                source.clone(),
                value_type,
                type_parameters,
                relations,
            )?;
            for (index, entry) in entries.iter().enumerate() {
                let key = index.checked_mul(2).ok_or_else(relation_work_limit)?;
                collect_expression_relations(
                    context,
                    module,
                    catalog,
                    declaration_owner,
                    &entry.key,
                    relation_child_path(&path, key)?,
                    variables,
                    capabilities,
                    type_parameters,
                    site_types,
                    test_owner,
                    relations,
                )?;
                collect_expression_relations(
                    context,
                    module,
                    catalog,
                    declaration_owner,
                    &entry.value,
                    relation_child_path(
                        &path,
                        key.checked_add(1).ok_or_else(relation_work_limit)?,
                    )?,
                    variables,
                    capabilities,
                    type_parameters,
                    site_types,
                    test_owner,
                    relations,
                )?;
            }
        }
        Expression::Match { value, arms, .. } => {
            collect_expression_relations(
                context,
                module,
                catalog,
                declaration_owner,
                value,
                relation_child_path(&path, 0)?,
                variables,
                capabilities,
                type_parameters,
                site_types,
                test_owner,
                relations,
            )?;
            let owner = match site_types.get(&expression_pointer(value)) {
                Some(ResolvedType::Nominal(owner)) => owner,
                _ => {
                    return Err(semantic_without_location(
                        "semantic_relation_match_type",
                        "validated variant match has no nominal expression type",
                    ));
                }
            };
            for (index, arm) in arms.iter().enumerate() {
                insert_expression_relation(
                    relations,
                    module.module_id,
                    source.clone(),
                    case_target(context, owner, &arm.case)?,
                    RelationRole::VariantPattern,
                    test_owner,
                );
                let mut local = variables.clone();
                if let Some(binding) = &arm.binding {
                    let slot = u32::try_from(index).map_err(|_| relation_work_limit())?;
                    local.insert(
                        binding.clone(),
                        RelationTarget::Binding {
                            owner: declaration_owner.clone(),
                            binding: catalog.binding(&path, slot)?,
                        },
                    );
                }
                collect_expression_relations(
                    context,
                    module,
                    catalog,
                    declaration_owner,
                    &arm.body,
                    relation_child_path(
                        &path,
                        index.checked_add(1).ok_or_else(relation_work_limit)?,
                    )?,
                    &local,
                    capabilities,
                    type_parameters,
                    site_types,
                    test_owner,
                    relations,
                )?;
            }
        }
        Expression::FunctionRef {
            function,
            type_arguments,
            ..
        } => {
            for ty in type_arguments {
                collect_type_relations_in_scope(
                    context,
                    module,
                    source.clone(),
                    ty,
                    type_parameters,
                    relations,
                )?;
            }
            let resolved = context.resolve_reference(module, function)?;
            insert_expression_relation(
                relations,
                module.module_id,
                source,
                declaration_target(&resolved.owner),
                RelationRole::ValueReference,
                test_owner,
            );
        }
        Expression::Perform {
            capability,
            operation,
            arguments,
            ..
        } => {
            let capability = capabilities.get(capability).ok_or_else(|| {
                semantic_without_location(
                    "semantic_relation_capability_missing",
                    "validated capability expression has no exact binding",
                )
            })?;
            insert_expression_relation(
                relations,
                module.module_id,
                source.clone(),
                capability.binding.clone(),
                RelationRole::CapabilityInterface,
                test_owner,
            );
            insert_expression_relation(
                relations,
                module.module_id,
                source,
                operation_target(context, &capability.interface, operation)?,
                RelationRole::CapabilityOperation,
                test_owner,
            );
            for (index, argument) in arguments.iter().enumerate() {
                collect_expression_relations(
                    context,
                    module,
                    catalog,
                    declaration_owner,
                    argument,
                    relation_child_path(&path, index)?,
                    variables,
                    capabilities,
                    type_parameters,
                    site_types,
                    test_owner,
                    relations,
                )?;
            }
        }
        Expression::Transaction {
            capability,
            binding,
            body,
            ..
        } => {
            let capability = capabilities.get(capability).ok_or_else(|| {
                semantic_without_location(
                    "semantic_relation_capability_missing",
                    "validated transaction expression has no exact capability binding",
                )
            })?;
            let binding_id = catalog.binding(&path, 0)?;
            let binding_target = RelationTarget::Binding {
                owner: declaration_owner.clone(),
                binding: binding_id,
            };
            insert_expression_relation(
                relations,
                module.module_id,
                source.clone(),
                capability.binding.clone(),
                RelationRole::CapabilityInterface,
                test_owner,
            );
            insert_expression_relation(
                relations,
                module.module_id,
                source,
                operation_target(context, &capability.interface, "transaction")?,
                RelationRole::CapabilityOperation,
                test_owner,
            );
            insert_relation(
                relations,
                module.module_id,
                RelationSource::Binding(binding_id),
                declaration_target(&capability.interface),
                RelationRole::CapabilityInterface,
            );
            let mut nested = capabilities.clone();
            nested.insert(
                binding.clone(),
                RelationCapability {
                    binding: binding_target,
                    interface: capability.interface.clone(),
                },
            );
            collect_expression_relations(
                context,
                module,
                catalog,
                declaration_owner,
                body,
                relation_child_path(&path, 0)?,
                variables,
                &nested,
                type_parameters,
                site_types,
                test_owner,
                relations,
            )?;
        }
    }
    Ok(())
}

fn relation_child_path(parent: &[u32], ordinal: usize) -> Result<Vec<u32>, Diagnostic> {
    if parent.len() >= super::meaning::MAXIMUM_EXPRESSION_DEPTH {
        return Err(relation_work_limit());
    }
    let mut path = parent.to_vec();
    path.push(u32::try_from(ordinal).map_err(|_| relation_work_limit())?);
    Ok(path)
}

fn insert_expression_relation(
    relations: &mut RelationSets,
    module: ModuleId,
    source: RelationSource,
    target: RelationTarget,
    role: RelationRole,
    test_owner: Option<DeclarationId>,
) {
    if let Some(test_owner) = test_owner
        && let Some(owner) = relation_target_owner(&target)
        && owner.declaration != test_owner
    {
        insert_relation(
            relations,
            module,
            RelationSource::Declaration(test_owner),
            RelationTarget::Declaration(owner),
            RelationRole::TestDependency,
        );
    }
    insert_relation(relations, module, source, target, role);
}

fn relation_target_owner(target: &RelationTarget) -> Option<DeclarationReference> {
    match target {
        RelationTarget::Module(_) => None,
        RelationTarget::Declaration(owner)
        | RelationTarget::Field { owner, .. }
        | RelationTarget::Case { owner, .. }
        | RelationTarget::Operation { owner, .. }
        | RelationTarget::TypeParameter { owner, .. }
        | RelationTarget::Parameter { owner, .. }
        | RelationTarget::Binding { owner, .. }
        | RelationTarget::Requirement { owner, .. }
        | RelationTarget::Port { owner, .. } => Some(owner.clone()),
    }
}

fn validated_owner(
    package: &PackageId,
    module: &ValidatedModule,
    declaration: &str,
) -> Result<OwnerId, Diagnostic> {
    let identity = module
        .declaration_identities
        .iter()
        .find(|identity| identity.name == declaration)
        .ok_or_else(|| {
            semantic_without_location(
                "semantic_owner_identity_missing",
                format!(
                    "declaration '{}.{}' has no stable semantic identity",
                    module.module.name, declaration
                ),
            )
        })?;
    Ok(OwnerId {
        package: package.clone(),
        module_id: module.module_id,
        declaration_id: identity.id,
        module: module.module.name.clone(),
        declaration: declaration.to_owned(),
    })
}

fn validated_owner_by_id(
    package: &PackageId,
    module: &ValidatedModule,
    declaration: DeclarationId,
) -> Result<OwnerId, Diagnostic> {
    let identity = module
        .declaration_identities
        .iter()
        .find(|identity| identity.id == declaration)
        .ok_or_else(|| {
            semantic_without_location(
                "semantic_owner_identity_missing",
                format!(
                    "module '{}' has no stable declaration identity '{}'",
                    module.module.name, declaration
                ),
            )
        })?;
    Ok(OwnerId {
        package: package.clone(),
        module_id: module.module_id,
        declaration_id: identity.id,
        module: module.module.name.clone(),
        declaration: identity.name.clone(),
    })
}

struct ResolvedDeclaration<'a> {
    owner: OwnerId,
    declaration: &'a Declaration,
}

fn validate_exact_dependencies(
    descriptor: &PackageDescriptor,
    dependencies: &[ExactDependency<'_>],
) -> Result<(), Diagnostic> {
    if descriptor.dependencies.len() != dependencies.len() {
        return Err(semantic_without_location(
            "semantic_dependency_count",
            format!(
                "package declares {} dependencies but {} exact dependencies were supplied",
                descriptor.dependencies.len(),
                dependencies.len()
            ),
        ));
    }
    let mut supplied = BTreeSet::new();
    for dependency in dependencies {
        if !supplied.insert(dependency.alias) {
            return Err(semantic_without_location(
                "semantic_dependency_duplicate",
                format!("dependency '{}' was supplied twice", dependency.alias),
            ));
        }
        let declared = descriptor
            .dependencies
            .iter()
            .find(|declared| declared.alias == dependency.alias)
            .ok_or_else(|| {
                semantic_without_location(
                    "semantic_dependency_foreign",
                    format!("dependency '{}' is not declared", dependency.alias),
                )
            })?;
        if declared.package_id != dependency.package.descriptor.package_id {
            return Err(semantic_without_location(
                "semantic_dependency_package",
                format!(
                    "dependency '{}' has a foreign package identity",
                    dependency.alias
                ),
            ));
        }
        if declared.revision_digest != dependency.package.revision_digest {
            return Err(semantic_without_location(
                "semantic_dependency_revision",
                format!(
                    "dependency '{}' has a foreign revision digest",
                    dependency.alias
                ),
            ));
        }
        if declared.artifact_digest != dependency.artifact_digest {
            return Err(semantic_without_location(
                "semantic_dependency_artifact",
                format!(
                    "dependency '{}' has a foreign artifact digest",
                    dependency.alias
                ),
            ));
        }
    }
    Ok(())
}

fn validate_imports_and_exports(context: &PackageContext<'_>) -> Result<(), Diagnostic> {
    for module in context.modules {
        let mut exported = BTreeSet::new();
        for export in &module.module.exports {
            if !exported.insert(*export) {
                return Err(semantic_at(
                    &module.path,
                    semantic_reference_span(),
                    "semantic_export_duplicate",
                    format!(
                        "module '{}' exports declaration identity '{}' more than once",
                        module.module.name, export
                    ),
                ));
            }
            let (identity, declaration) = module
                .declaration_identities
                .iter()
                .zip(&module.module.declarations)
                .find(|(identity, _)| identity.id == *export)
                .ok_or_else(|| {
                    semantic_at(
                        &module.path,
                        SourceSpan {
                            byte_start: 0,
                            byte_end: 0,
                            line: 1,
                            column: 1,
                        },
                        "semantic_export_missing",
                        format!(
                            "module '{}' exports absent declaration identity '{export}'",
                            module.module.name,
                        ),
                    )
                })?;
            if matches!(declaration, Declaration::Test(_)) {
                return Err(semantic_at(
                    &module.path,
                    declaration.span().clone(),
                    "semantic_export_test",
                    "tests cannot be exported as package meaning",
                ));
            }
            if identity.name != declaration.name() {
                return Err(semantic_at(
                    &module.path,
                    declaration.span().clone(),
                    "semantic_export_identity_shape",
                    "export identity and declaration meaning disagree",
                ));
            }
        }
        for import in &module.module.imports {
            context.imported_module(module, import)?;
        }
    }
    Ok(())
}

fn validate_declared_types(context: &PackageContext<'_>) -> Result<(), Diagnostic> {
    for module in context.modules {
        for declaration in &module.module.declarations {
            match declaration {
                Declaration::Record(record) => {
                    for field in &record.fields {
                        resolve_type(context, module, &field.ty, &field.span)?;
                    }
                }
                Declaration::Variant(variant) => {
                    for case in &variant.cases {
                        if let Some(payload) = &case.payload {
                            resolve_type(context, module, payload, &case.span)?;
                        }
                    }
                }
                Declaration::Interface(interface) => {
                    validate_interface_types(context, module, interface)?;
                }
                Declaration::External(external) => {
                    external_signature(context, module, external)?;
                }
                Declaration::Function(function) => {
                    function_signature(context, module, function, None)?;
                }
                Declaration::Constant(constant) => {
                    resolve_type(context, module, &constant.ty, &constant.span)?;
                }
                Declaration::Component(component) => {
                    for requirement in &component.requirements {
                        let resolved = context.resolve_reference(module, &requirement.interface)?;
                        let Declaration::Interface(interface) = resolved.declaration else {
                            return Err(semantic_at(
                                &module.path,
                                requirement.span.clone(),
                                "semantic_requirement_interface_kind",
                                format!(
                                    "'{}' is not a capability interface",
                                    resolved.owner.diagnostic_name()
                                ),
                            ));
                        };
                        for operation in &requirement.operations {
                            if !interface
                                .operations
                                .iter()
                                .any(|candidate| candidate.name == *operation)
                            {
                                return Err(semantic_at(
                                    &module.path,
                                    requirement.span.clone(),
                                    "semantic_requirement_operation",
                                    format!(
                                        "interface '{}' has no operation '{operation}'",
                                        resolved.owner.diagnostic_name()
                                    ),
                                ));
                            }
                        }
                    }
                    for port in &component.ports {
                        resolve_type(context, module, &port.ty, &port.span)?;
                    }
                }
                Declaration::Test(_) => {}
            }
        }
    }
    Ok(())
}

type ShapeMaps = (
    BTreeMap<OwnerId, NominalShape>,
    BTreeMap<OwnerId, ResolvedInterface>,
);

fn collect_type_shapes(context: &PackageContext<'_>) -> Result<ShapeMaps, Diagnostic> {
    let mut nominal = BTreeMap::new();
    let mut interfaces = BTreeMap::new();
    for module in context.modules {
        for declaration in &module.module.declarations {
            match declaration {
                Declaration::Record(record) => {
                    let owner = context.owner(&module.module.name, &record.name)?;
                    nominal.insert(
                        owner,
                        NominalShape::Record(
                            record
                                .fields
                                .iter()
                                .map(|field| {
                                    Ok(ResolvedField {
                                        name: field.name.clone(),
                                        ty: resolve_type(context, module, &field.ty, &field.span)?,
                                    })
                                })
                                .collect::<Result<Vec<_>, Diagnostic>>()?,
                        ),
                    );
                }
                Declaration::Variant(variant) => {
                    let owner = context.owner(&module.module.name, &variant.name)?;
                    nominal.insert(
                        owner,
                        NominalShape::Variant(
                            variant
                                .cases
                                .iter()
                                .map(|case| {
                                    Ok((
                                        case.name.clone(),
                                        case.payload
                                            .as_ref()
                                            .map(|payload| {
                                                resolve_type(context, module, payload, &case.span)
                                            })
                                            .transpose()?,
                                    ))
                                })
                                .collect::<Result<BTreeMap<_, _>, Diagnostic>>()?,
                        ),
                    );
                }
                Declaration::Interface(interface) => {
                    let owner = context.owner(&module.module.name, &interface.name)?;
                    interfaces.insert(
                        owner.clone(),
                        ResolvedInterface {
                            owner,
                            operations: interface
                                .operations
                                .iter()
                                .map(|operation| {
                                    Ok((
                                        operation.name.clone(),
                                        ResolvedOperation {
                                            parameters: operation
                                                .parameters
                                                .iter()
                                                .map(|parameter| {
                                                    resolve_type(
                                                        context,
                                                        module,
                                                        &parameter.ty,
                                                        &parameter.span,
                                                    )
                                                })
                                                .collect::<Result<Vec<_>, _>>()?,
                                            result: resolve_type(
                                                context,
                                                module,
                                                &operation.result,
                                                &operation.span,
                                            )?,
                                            idempotency: operation.idempotency,
                                            visibility: operation.visibility,
                                        },
                                    ))
                                })
                                .collect::<Result<BTreeMap<_, _>, Diagnostic>>()?,
                        },
                    );
                }
                _ => {}
            }
        }
    }
    Ok((nominal, interfaces))
}

fn collect_constant_types(
    context: &PackageContext<'_>,
) -> Result<BTreeMap<OwnerId, ResolvedType>, Diagnostic> {
    let mut constants = BTreeMap::new();
    for module in context.modules {
        for declaration in &module.module.declarations {
            if let Declaration::Constant(constant) = declaration {
                constants.insert(
                    context.owner(&module.module.name, &constant.name)?,
                    resolve_type(context, module, &constant.ty, &constant.span)?,
                );
            }
        }
    }
    Ok(constants)
}

fn validate_interface_types(
    context: &PackageContext<'_>,
    module: &ValidatedModule,
    interface: &Interface,
) -> Result<(), Diagnostic> {
    for operation in &interface.operations {
        validate_signature_types(
            context,
            module,
            &operation.parameters,
            &operation.result,
            &operation.span,
        )?;
        if operation.visibility == super::language::Visibility::Possible
            && operation.idempotency == Idempotency::Idempotent
        {
            return Err(semantic_at(
                &module.path,
                operation.span.clone(),
                "semantic_visibility_idempotency",
                format!(
                    "operation '{}.{}' declares possible visibility but no idempotency key",
                    interface.name, operation.name
                ),
            ));
        }
    }
    Ok(())
}

fn validate_signature_types(
    context: &PackageContext<'_>,
    module: &ValidatedModule,
    parameters: &[Parameter],
    result: &Type,
    span: &SourceSpan,
) -> Result<(), Diagnostic> {
    for parameter in parameters {
        resolve_type(context, module, &parameter.ty, &parameter.span)?;
    }
    resolve_type(context, module, result, span)?;
    Ok(())
}

fn resolve_interface(
    context: &PackageContext<'_>,
    module: &ValidatedModule,
    capability: &TaskCapability,
) -> Result<OwnerId, Diagnostic> {
    let resolved = context.resolve_reference(module, &capability.interface)?;
    if !matches!(resolved.declaration, Declaration::Interface(_)) {
        return Err(semantic_at(
            &module.path,
            capability.span.clone(),
            "semantic_task_interface_kind",
            format!(
                "'{}' is not a capability interface",
                resolved.owner.diagnostic_name()
            ),
        ));
    }
    Ok(resolved.owner)
}

fn resolve_type(
    context: &PackageContext<'_>,
    module: &ValidatedModule,
    ty: &Type,
    span: &SourceSpan,
) -> Result<ResolvedType, Diagnostic> {
    resolve_type_in_scope(context, module, ty, span, &BTreeMap::new())
}

fn resolve_type_in_scope(
    context: &PackageContext<'_>,
    module: &ValidatedModule,
    ty: &Type,
    span: &SourceSpan,
    type_parameters: &BTreeMap<String, TypeParameterId>,
) -> Result<ResolvedType, Diagnostic> {
    Ok(match ty {
        Type::Unit => ResolvedType::Unit,
        Type::Bool => ResolvedType::Bool,
        Type::I64 => ResolvedType::I64,
        Type::Bytes => ResolvedType::Bytes,
        Type::Text => ResolvedType::Text,
        Type::StaticText => ResolvedType::StaticText,
        Type::Secret => ResolvedType::Secret,
        Type::Parameter(name) => {
            ResolvedType::Parameter(type_parameters.get(name).copied().ok_or_else(|| {
                semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_type_parameter_scope",
                    format!("type parameter '{name}' is not declared by this function"),
                )
            })?)
        }
        Type::Named(reference) => {
            let resolved = context.resolve_reference(module, reference)?;
            if !matches!(
                resolved.declaration,
                Declaration::Record(_) | Declaration::Variant(_)
            ) {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_type_kind",
                    format!(
                        "'{}' is not a record or variant type",
                        resolved.owner.diagnostic_name()
                    ),
                ));
            }
            ResolvedType::Nominal(resolved.owner)
        }
        Type::Record(fields) => ResolvedType::Record(
            fields
                .iter()
                .map(|field| {
                    Ok(ResolvedField {
                        name: field.name.clone(),
                        ty: resolve_type_in_scope(
                            context,
                            module,
                            &field.ty,
                            span,
                            type_parameters,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?,
        ),
        Type::List(item) => ResolvedType::List(Box::new(resolve_type_in_scope(
            context,
            module,
            item,
            span,
            type_parameters,
        )?)),
        Type::Map(key, value) => ResolvedType::Map(
            Box::new(resolve_type_in_scope(
                context,
                module,
                key,
                span,
                type_parameters,
            )?),
            Box::new(resolve_type_in_scope(
                context,
                module,
                value,
                span,
                type_parameters,
            )?),
        ),
        Type::Option(item) => ResolvedType::Option(Box::new(resolve_type_in_scope(
            context,
            module,
            item,
            span,
            type_parameters,
        )?)),
        Type::Result(ok, error) => ResolvedType::Result(
            Box::new(resolve_type_in_scope(
                context,
                module,
                ok,
                span,
                type_parameters,
            )?),
            Box::new(resolve_type_in_scope(
                context,
                module,
                error,
                span,
                type_parameters,
            )?),
        ),
        Type::Stream(item) => ResolvedType::Stream(Box::new(resolve_type_in_scope(
            context,
            module,
            item,
            span,
            type_parameters,
        )?)),
        Type::Function(parameters, result) => ResolvedType::Function(
            parameters
                .iter()
                .map(|parameter| {
                    resolve_type_in_scope(context, module, parameter, span, type_parameters)
                })
                .collect::<Result<Vec<_>, _>>()?,
            Box::new(resolve_type_in_scope(
                context,
                module,
                result,
                span,
                type_parameters,
            )?),
        ),
    })
}

fn collect_function_signatures(
    context: &PackageContext<'_>,
) -> Result<BTreeMap<OwnerId, FunctionSignature>, Diagnostic> {
    let mut signatures = BTreeMap::new();
    for module in context.modules {
        for declaration in &module.module.declarations {
            let signature = match declaration {
                Declaration::Function(function) => {
                    Some(function_signature(context, module, function, None)?)
                }
                Declaration::External(external) => {
                    Some(external_signature(context, module, external)?)
                }
                _ => None,
            };
            if let Some(signature) = signature {
                signatures.insert(signature.owner.clone(), signature);
            }
        }
    }
    Ok(signatures)
}

fn function_signature(
    context: &PackageContext<'_>,
    module: &ValidatedModule,
    function: &Function,
    implementation: Option<String>,
) -> Result<FunctionSignature, Diagnostic> {
    if !function.type_parameters.is_empty() && !matches!(function.effect, Effect::Pure) {
        return Err(semantic_at(
            &module.path,
            function.span.clone(),
            "semantic_generic_task",
            format!(
                "task function '{}' cannot declare type parameters",
                function.name
            ),
        ));
    }
    let (type_parameters, type_parameter_scope) = declared_type_parameters(
        module,
        &function.name,
        &function.type_parameters,
        &function.parameters,
    )?;
    let task_capabilities = match &function.effect {
        Effect::Pure => Vec::new(),
        Effect::Task { capabilities } => capabilities
            .iter()
            .map(|capability| {
                Ok(ResolvedTaskCapability {
                    alias: capability.alias.clone(),
                    interface: resolve_interface(context, module, capability)?,
                })
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?,
    };
    Ok(FunctionSignature {
        owner: context.owner(&module.module.name, &function.name)?,
        type_parameters,
        parameters: function
            .parameters
            .iter()
            .map(|parameter| {
                resolve_type_in_scope(
                    context,
                    module,
                    &parameter.ty,
                    &parameter.span,
                    &type_parameter_scope,
                )
            })
            .collect::<Result<Vec<_>, _>>()?,
        result: resolve_type_in_scope(
            context,
            module,
            &function.result,
            &function.span,
            &type_parameter_scope,
        )?,
        task_capabilities,
        external_implementation: implementation,
    })
}

fn external_signature(
    context: &PackageContext<'_>,
    module: &ValidatedModule,
    external: &ExternalFunction,
) -> Result<FunctionSignature, Diagnostic> {
    let (type_parameters, type_parameter_scope) = declared_type_parameters(
        module,
        &external.name,
        &external.type_parameters,
        &external.parameters,
    )?;
    Ok(FunctionSignature {
        owner: context.owner(&module.module.name, &external.name)?,
        type_parameters,
        parameters: external
            .parameters
            .iter()
            .map(|parameter| {
                resolve_type_in_scope(
                    context,
                    module,
                    &parameter.ty,
                    &parameter.span,
                    &type_parameter_scope,
                )
            })
            .collect::<Result<Vec<_>, _>>()?,
        result: resolve_type_in_scope(
            context,
            module,
            &external.result,
            &external.span,
            &type_parameter_scope,
        )?,
        task_capabilities: Vec::new(),
        external_implementation: Some(external.implementation.clone()),
    })
}

fn declared_type_parameters(
    module: &ValidatedModule,
    declaration_name: &str,
    declared: &[super::language::TypeParameter],
    value_parameters: &[Parameter],
) -> Result<
    (
        Vec<ResolvedTypeParameter>,
        BTreeMap<String, TypeParameterId>,
    ),
    Diagnostic,
> {
    let identity = module
        .declaration_identities
        .iter()
        .find(|identity| identity.name == declaration_name)
        .ok_or_else(|| {
            semantic_without_location(
                "semantic_type_parameter_identity_owner",
                format!("function '{declaration_name}' has no stable identity"),
            )
        })?;
    let identities = identity
        .members
        .iter()
        .take(declared.len())
        .map(|member| match member {
            MemberIdentity::TypeParameter { id, name } => Ok((*id, name)),
            _ => Err(semantic_without_location(
                "semantic_type_parameter_identity_shape",
                format!(
                    "function '{declaration_name}' type parameters have a foreign identity shape"
                ),
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if identities.len() != declared.len() {
        return Err(semantic_without_location(
            "semantic_type_parameter_identity_count",
            format!("function '{declaration_name}' type parameter identities are incomplete"),
        ));
    }
    let mut names = BTreeSet::new();
    let mut scope = BTreeMap::new();
    let mut resolved = Vec::with_capacity(declared.len());
    for (parameter, (id, identity_name)) in declared.iter().zip(identities) {
        if parameter.name != *identity_name || !names.insert(parameter.name.as_str()) {
            return Err(semantic_at(
                &module.path,
                parameter.span.clone(),
                "semantic_type_parameter_duplicate",
                format!(
                    "function '{declaration_name}' has duplicate or inconsistent type parameter '{}'",
                    parameter.name
                ),
            ));
        }
        if value_parameters
            .iter()
            .any(|value| value.name == parameter.name)
        {
            return Err(semantic_at(
                &module.path,
                parameter.span.clone(),
                "semantic_type_parameter_value_collision",
                format!(
                    "function '{declaration_name}' uses '{}' as both a type and value parameter",
                    parameter.name
                ),
            ));
        }
        scope.insert(parameter.name.clone(), id);
        resolved.push(ResolvedTypeParameter {
            id,
            name: parameter.name.clone(),
        });
    }
    Ok((resolved, scope))
}

fn validate_expressions_and_effects(
    context: &PackageContext<'_>,
    signatures: &BTreeMap<OwnerId, FunctionSignature>,
    nominal_shapes: &BTreeMap<OwnerId, NominalShape>,
    interfaces: &BTreeMap<OwnerId, ResolvedInterface>,
    constant_types: &BTreeMap<OwnerId, ResolvedType>,
) -> Result<BTreeMap<OwnerId, FunctionFacts>, Diagnostic> {
    let mut direct = BTreeMap::new();
    let mut facts = BTreeMap::new();
    for (owner, signature) in signatures {
        if signature.external_implementation.is_some() {
            facts.insert(
                owner.clone(),
                FunctionFacts {
                    signature: signature.clone(),
                    capabilities: BTreeMap::new(),
                },
            );
        }
    }

    for module in context.modules {
        for declaration in &module.module.declarations {
            if let Declaration::Function(function) = declaration {
                let owner = context.owner(&module.module.name, &function.name)?;
                let signature = signatures.get(&owner).ok_or_else(|| {
                    semantic_at(
                        &module.path,
                        function.span.clone(),
                        "semantic_function_signature_missing",
                        format!("function '{}' has no collected signature", function.name),
                    )
                })?;
                let mut variables = BTreeMap::new();
                for (parameter, ty) in function.parameters.iter().zip(&signature.parameters) {
                    variables.insert(parameter.name.clone(), ty.clone());
                }
                let capabilities = signature
                    .task_capabilities
                    .iter()
                    .map(|capability| {
                        (
                            capability.alias.clone(),
                            CapabilityBinding {
                                interface: capability.interface.clone(),
                                report_alias: capability.alias.clone(),
                            },
                        )
                    })
                    .collect();
                let inference = InferContext {
                    package: context,
                    signatures,
                    nominal_shapes,
                    interfaces,
                    constant_types,
                    type_parameters: signature
                        .type_parameters
                        .iter()
                        .map(|parameter| (parameter.name.clone(), parameter.id))
                        .collect(),
                    allow_task_function_values: false,
                };
                let body = infer_expression(
                    &inference,
                    module,
                    &function.body,
                    &variables,
                    &capabilities,
                    matches!(function.effect, Effect::Pure),
                )?;
                require_same_type(
                    module,
                    function.body.span(),
                    &signature.result,
                    &body.ty,
                    "semantic_function_result",
                    &format!("function '{}' result", function.name),
                )?;
                if matches!(function.effect, Effect::Pure)
                    && (!body.capabilities.is_empty() || !body.called_tasks.is_empty())
                {
                    return Err(semantic_at(
                        &module.path,
                        function.body.span().clone(),
                        "semantic_pure_effect",
                        format!("pure function '{}' attempts effectful work", function.name),
                    ));
                }
                direct.insert(owner, body);
            }
        }
    }

    validate_generic_recursion(&direct, signatures)?;

    let maximum_iterations = direct.len().saturating_add(1);
    let mut closed: BTreeMap<OwnerId, BTreeMap<String, CapabilityFacts>> = direct
        .iter()
        .map(|(owner, body)| (owner.clone(), body.capabilities.clone()))
        .collect();
    for iteration in 0..maximum_iterations {
        let snapshot = closed.clone();
        let mut changed = false;
        for (owner, body) in &direct {
            let mut capabilities = snapshot.get(owner).cloned().unwrap_or_default();
            for called in &body.called_tasks {
                let called_capabilities = if let Some(local) = snapshot.get(called) {
                    local
                } else {
                    &dependency_function_facts(context, called)
                        .ok_or_else(|| {
                            semantic_without_location(
                                "semantic_called_task_facts",
                                format!(
                                    "called task '{}' has no validated effect facts",
                                    called.diagnostic_name()
                                ),
                            )
                        })?
                        .capabilities
                };
                merge_capability_maps(&mut capabilities, called_capabilities, None)?;
            }
            if snapshot.get(owner) != Some(&capabilities) {
                closed.insert(owner.clone(), capabilities);
                changed = true;
            }
        }
        if !changed {
            break;
        }
        if iteration + 1 == maximum_iterations {
            return Err(semantic_without_location(
                "semantic_effect_closure",
                "effect closure did not converge within the finite function graph",
            ));
        }
    }

    for module in context.modules {
        for declaration in &module.module.declarations {
            if let Declaration::Function(function) = declaration {
                let owner = context.owner(&module.module.name, &function.name)?;
                let signature = signatures.get(&owner).ok_or_else(|| {
                    semantic_without_location(
                        "semantic_function_signature_missing",
                        format!("function '{}' has no signature", function.name),
                    )
                })?;
                let capabilities = closed.get(&owner).cloned().unwrap_or_default();
                let declared: BTreeMap<_, _> = signature
                    .task_capabilities
                    .iter()
                    .map(|capability| (capability.alias.clone(), capability.interface.clone()))
                    .collect();
                if matches!(function.effect, Effect::Pure) && !capabilities.is_empty() {
                    return Err(semantic_at(
                        &module.path,
                        function.span.clone(),
                        "semantic_pure_transitive_effect",
                        format!(
                            "pure function '{}' reaches an effectful task",
                            function.name
                        ),
                    ));
                }
                for (alias, capability) in &capabilities {
                    match declared.get(alias) {
                        Some(interface) if interface == &capability.interface => {}
                        Some(_) => {
                            return Err(semantic_at(
                                &module.path,
                                function.span.clone(),
                                "semantic_task_interface_mismatch",
                                format!(
                                    "task '{}' uses alias '{alias}' with a different interface",
                                    function.name
                                ),
                            ));
                        }
                        None => {
                            return Err(semantic_at(
                                &module.path,
                                function.span.clone(),
                                "semantic_task_capability_missing",
                                format!(
                                    "task '{}' uses undeclared capability alias '{alias}'",
                                    function.name
                                ),
                            ));
                        }
                    }
                }
                for alias in declared.keys() {
                    if !capabilities.contains_key(alias) {
                        return Err(semantic_at(
                            &module.path,
                            function.span.clone(),
                            "semantic_task_capability_unused",
                            format!(
                                "task '{}' declares unused capability alias '{alias}'",
                                function.name
                            ),
                        ));
                    }
                }
                facts.insert(
                    owner,
                    FunctionFacts {
                        signature: signature.clone(),
                        capabilities,
                    },
                );
            }
        }
    }

    let inference = InferContext {
        package: context,
        signatures,
        nominal_shapes,
        interfaces,
        constant_types,
        type_parameters: BTreeMap::new(),
        allow_task_function_values: false,
    };
    for module in context.modules {
        for declaration in &module.module.declarations {
            match declaration {
                Declaration::Constant(constant) => {
                    let value = infer_expression(
                        &inference,
                        module,
                        &constant.value,
                        &BTreeMap::new(),
                        &BTreeMap::new(),
                        true,
                    )?;
                    let owner = context.owner(&module.module.name, &constant.name)?;
                    let expected = constant_types.get(&owner).ok_or_else(|| {
                        semantic_without_location(
                            "semantic_constant_type_missing",
                            format!("constant '{}' has no resolved type", constant.name),
                        )
                    })?;
                    require_same_type(
                        module,
                        constant.value.span(),
                        expected,
                        &value.ty,
                        "semantic_constant_type",
                        &format!("constant '{}'.", constant.name),
                    )?;
                }
                Declaration::Component(component) => {
                    validate_component(&inference, module, component, &facts)?
                }
                Declaration::Test(test) => {
                    let actual = infer_expression(
                        &inference,
                        module,
                        &test.actual,
                        &BTreeMap::new(),
                        &BTreeMap::new(),
                        true,
                    )?;
                    let expected = infer_expression(
                        &inference,
                        module,
                        &test.expected,
                        &BTreeMap::new(),
                        &BTreeMap::new(),
                        true,
                    )?;
                    require_same_type(
                        module,
                        test.expected.span(),
                        &actual.ty,
                        &expected.ty,
                        "semantic_test_type",
                        &format!("test '{}'.", test.name),
                    )?;
                }
                _ => {}
            }
        }
    }
    validate_targets(context)?;
    Ok(facts)
}

fn validate_generic_recursion(
    functions: &BTreeMap<OwnerId, ExpressionFacts>,
    signatures: &BTreeMap<OwnerId, FunctionSignature>,
) -> Result<(), Diagnostic> {
    for (caller, facts) in functions {
        let Some(caller_signature) = signatures.get(caller) else {
            continue;
        };
        if caller_signature.type_parameters.is_empty() {
            continue;
        }
        let identity_arguments = caller_signature
            .type_parameters
            .iter()
            .map(|parameter| ResolvedType::Parameter(parameter.id))
            .collect::<Vec<_>>();
        for application in &facts.generic_calls {
            if generic_path_exists(functions, &application.function, caller)
                && application.type_arguments != identity_arguments
            {
                return Err(semantic_without_location(
                    "semantic_polymorphic_recursion",
                    format!(
                        "generic recursion from '{}' to '{}' changes its ordered type arguments",
                        caller.diagnostic_name(),
                        application.function.diagnostic_name()
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn generic_path_exists(
    functions: &BTreeMap<OwnerId, ExpressionFacts>,
    start: &OwnerId,
    target: &OwnerId,
) -> bool {
    let mut pending = vec![start.clone()];
    let mut visited = BTreeSet::new();
    while let Some(owner) = pending.pop() {
        if owner == *target {
            return true;
        }
        if !visited.insert(owner.clone()) {
            continue;
        }
        if let Some(facts) = functions.get(&owner) {
            pending.extend(
                facts
                    .generic_calls
                    .iter()
                    .map(|application| application.function.clone()),
            );
        }
    }
    false
}

#[derive(Clone)]
struct CapabilityBinding {
    interface: OwnerId,
    report_alias: String,
}

#[derive(Clone, Default)]
struct ExpressionFacts {
    ty: ResolvedType,
    capabilities: BTreeMap<String, CapabilityFacts>,
    called_tasks: BTreeSet<OwnerId>,
    function_refs: BTreeSet<OwnerId>,
    generic_calls: Vec<GenericApplication>,
    site_types: BTreeMap<usize, ResolvedType>,
}

#[derive(Clone)]
struct GenericApplication {
    function: OwnerId,
    type_arguments: Vec<ResolvedType>,
}

impl ExpressionFacts {
    fn typed(ty: ResolvedType) -> Self {
        Self {
            ty,
            ..Self::default()
        }
    }

    fn merge_effects(&mut self, other: &Self) -> Result<(), Diagnostic> {
        merge_capability_maps(&mut self.capabilities, &other.capabilities, None)?;
        self.called_tasks.extend(other.called_tasks.iter().cloned());
        self.function_refs
            .extend(other.function_refs.iter().cloned());
        self.generic_calls
            .extend(other.generic_calls.iter().cloned());
        self.site_types.extend(other.site_types.clone());
        Ok(())
    }
}

struct InferContext<'a> {
    package: &'a PackageContext<'a>,
    signatures: &'a BTreeMap<OwnerId, FunctionSignature>,
    nominal_shapes: &'a BTreeMap<OwnerId, NominalShape>,
    interfaces: &'a BTreeMap<OwnerId, ResolvedInterface>,
    constant_types: &'a BTreeMap<OwnerId, ResolvedType>,
    type_parameters: BTreeMap<String, TypeParameterId>,
    allow_task_function_values: bool,
}

impl InferContext<'_> {
    fn for_signature(&self, signature: &FunctionSignature) -> Self {
        Self {
            package: self.package,
            signatures: self.signatures,
            nominal_shapes: self.nominal_shapes,
            interfaces: self.interfaces,
            constant_types: self.constant_types,
            type_parameters: signature
                .type_parameters
                .iter()
                .map(|parameter| (parameter.name.clone(), parameter.id))
                .collect(),
            allow_task_function_values: self.allow_task_function_values,
        }
    }

    fn for_component_port(&self) -> Self {
        Self {
            package: self.package,
            signatures: self.signatures,
            nominal_shapes: self.nominal_shapes,
            interfaces: self.interfaces,
            constant_types: self.constant_types,
            type_parameters: BTreeMap::new(),
            allow_task_function_values: true,
        }
    }
}

fn infer_expression(
    context: &InferContext<'_>,
    module: &ValidatedModule,
    expression: &Expression,
    variables: &BTreeMap<String, ResolvedType>,
    capabilities: &BTreeMap<String, CapabilityBinding>,
    pure: bool,
) -> Result<ExpressionFacts, Diagnostic> {
    let mut facts =
        infer_expression_inner(context, module, expression, variables, capabilities, pure)?;
    facts
        .site_types
        .insert(expression_pointer(expression), facts.ty.clone());
    Ok(facts)
}

fn infer_expression_inner(
    context: &InferContext<'_>,
    module: &ValidatedModule,
    expression: &Expression,
    variables: &BTreeMap<String, ResolvedType>,
    capabilities: &BTreeMap<String, CapabilityBinding>,
    pure: bool,
) -> Result<ExpressionFacts, Diagnostic> {
    match expression {
        Expression::Unit(_) => Ok(ExpressionFacts::typed(ResolvedType::Unit)),
        Expression::Bool(_, _) => Ok(ExpressionFacts::typed(ResolvedType::Bool)),
        Expression::I64(_, _) => Ok(ExpressionFacts::typed(ResolvedType::I64)),
        Expression::Text(_, _) => Ok(ExpressionFacts::typed(ResolvedType::Text)),
        Expression::StaticText(_, _) => Ok(ExpressionFacts::typed(ResolvedType::StaticText)),
        Expression::Variable(name, span) => {
            let ty = variables.get(name).ok_or_else(|| {
                semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_variable_missing",
                    format!("lexical variable '{name}' is not bound"),
                )
            })?;
            Ok(ExpressionFacts::typed(ty.clone()))
        }
        Expression::Constant(reference, span) => {
            let resolved = context.package.resolve_reference(module, reference)?;
            if !matches!(resolved.declaration, Declaration::Constant(_)) {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_constant_kind",
                    format!("'{}' is not a constant", resolved.owner.diagnostic_name()),
                ));
            }
            let ty = lookup_constant_type(context, &resolved.owner).ok_or_else(|| {
                semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_constant_type_missing",
                    format!(
                        "constant '{}' has no validated type",
                        resolved.owner.diagnostic_name()
                    ),
                )
            })?;
            Ok(ExpressionFacts::typed(ty.clone()))
        }
        Expression::If {
            condition,
            when_true,
            when_false,
            span,
        } => {
            let condition =
                infer_expression(context, module, condition, variables, capabilities, pure)?;
            require_same_type(
                module,
                span,
                &ResolvedType::Bool,
                &condition.ty,
                "semantic_if_condition",
                "if condition",
            )?;
            let when_true =
                infer_expression(context, module, when_true, variables, capabilities, pure)?;
            let when_false =
                infer_expression(context, module, when_false, variables, capabilities, pure)?;
            require_same_type(
                module,
                span,
                &when_true.ty,
                &when_false.ty,
                "semantic_if_branches",
                "if branches",
            )?;
            let mut result = ExpressionFacts::typed(when_true.ty.clone());
            result.merge_effects(&condition)?;
            result.merge_effects(&when_true)?;
            result.merge_effects(&when_false)?;
            Ok(result)
        }
        Expression::Let { bindings, body, .. } => {
            let mut local = variables.clone();
            let mut result = ExpressionFacts::default();
            for binding in bindings {
                if local.contains_key(&binding.name) {
                    return Err(semantic_at(
                        &module.path,
                        binding.span.clone(),
                        "semantic_binding_shadow",
                        format!("binding '{}' shadows an existing local", binding.name),
                    ));
                }
                let value =
                    infer_expression(context, module, &binding.value, &local, capabilities, pure)?;
                local.insert(binding.name.clone(), value.ty.clone());
                result.merge_effects(&value)?;
            }
            let body = infer_expression(context, module, body, &local, capabilities, pure)?;
            result.ty = body.ty.clone();
            result.merge_effects(&body)?;
            Ok(result)
        }
        Expression::Do { expressions, span } => {
            let mut result = ExpressionFacts::default();
            for item in expressions {
                let value = infer_expression(context, module, item, variables, capabilities, pure)?;
                result.ty = value.ty.clone();
                result.merge_effects(&value)?;
            }
            if expressions.is_empty() {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_do_empty",
                    "do expression has no value",
                ));
            }
            Ok(result)
        }
        Expression::Call {
            function,
            type_arguments,
            arguments,
            span,
        } => {
            let generic_signature = resolve_function_signature(context, module, function)?;
            let (signature, resolved_type_arguments) =
                instantiate_signature(context, module, &generic_signature, type_arguments, span)?;
            if signature.parameters.len() != arguments.len() {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_call_arity",
                    format!(
                        "function '{}' requires {} arguments; {} were supplied",
                        generic_signature.owner.diagnostic_name(),
                        signature.parameters.len(),
                        arguments.len()
                    ),
                ));
            }
            let mut result = ExpressionFacts::typed(signature.result.clone());
            if !generic_signature.type_parameters.is_empty()
                && generic_signature.external_implementation.is_none()
            {
                result.generic_calls.push(GenericApplication {
                    function: generic_signature.owner.clone(),
                    type_arguments: resolved_type_arguments,
                });
            }
            for (index, (argument, expected)) in
                arguments.iter().zip(&signature.parameters).enumerate()
            {
                let value =
                    infer_expression(context, module, argument, variables, capabilities, pure)?;
                require_same_type(
                    module,
                    argument.span(),
                    expected,
                    &value.ty,
                    "semantic_call_argument",
                    &format!(
                        "argument {} of '{}'",
                        index + 1,
                        generic_signature.owner.diagnostic_name()
                    ),
                )?;
                result.merge_effects(&value)?;
            }
            if !signature.task_capabilities.is_empty() {
                if pure {
                    return Err(semantic_at(
                        &module.path,
                        span.clone(),
                        "semantic_pure_task_call",
                        format!(
                            "pure expression calls task '{}'",
                            generic_signature.owner.diagnostic_name()
                        ),
                    ));
                }
                for required in &signature.task_capabilities {
                    match capabilities.get(&required.alias) {
                        Some(binding) if binding.interface == required.interface => {}
                        Some(_) => {
                            return Err(semantic_at(
                                &module.path,
                                span.clone(),
                                "semantic_call_capability_interface",
                                format!(
                                    "task '{}' requires alias '{}' with a different interface",
                                    generic_signature.owner.diagnostic_name(),
                                    required.alias
                                ),
                            ));
                        }
                        None => {
                            return Err(semantic_at(
                                &module.path,
                                span.clone(),
                                "semantic_call_capability_missing",
                                format!(
                                    "task '{}' requires unavailable capability alias '{}'",
                                    generic_signature.owner.diagnostic_name(),
                                    required.alias
                                ),
                            ));
                        }
                    }
                }
                result.called_tasks.insert(signature.owner.clone());
            }
            Ok(result)
        }
        Expression::Invoke {
            callee,
            arguments,
            span,
        } => {
            let callee = infer_expression(context, module, callee, variables, capabilities, pure)?;
            let ResolvedType::Function(parameters, result_type) = &callee.ty else {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_invoke_type",
                    "invoke requires a pure named function value",
                ));
            };
            if parameters.len() != arguments.len() {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_invoke_arity",
                    format!(
                        "function value requires {} arguments; {} were supplied",
                        parameters.len(),
                        arguments.len()
                    ),
                ));
            }
            let mut result = ExpressionFacts::typed((**result_type).clone());
            result.merge_effects(&callee)?;
            for (index, (argument, expected)) in arguments.iter().zip(parameters).enumerate() {
                let value =
                    infer_expression(context, module, argument, variables, capabilities, pure)?;
                require_same_type(
                    module,
                    argument.span(),
                    expected,
                    &value.ty,
                    "semantic_invoke_argument",
                    &format!("function-value argument {}", index + 1),
                )?;
                result.merge_effects(&value)?;
            }
            Ok(result)
        }
        Expression::Record { ty, fields, span } => {
            let mut result = ExpressionFacts::default();
            if let Some(reference) = ty {
                let resolved = context.package.resolve_reference(module, reference)?;
                let shape = lookup_nominal_shape(context, &resolved.owner).ok_or_else(|| {
                    semantic_at(
                        &module.path,
                        span.clone(),
                        "semantic_record_type",
                        format!(
                            "'{}' is not a validated nominal type",
                            resolved.owner.diagnostic_name()
                        ),
                    )
                })?;
                let NominalShape::Record(expected_fields) = shape else {
                    return Err(semantic_at(
                        &module.path,
                        span.clone(),
                        "semantic_record_variant",
                        format!(
                            "'{}' is a variant, not a record",
                            resolved.owner.diagnostic_name()
                        ),
                    ));
                };
                validate_record_values(
                    context,
                    module,
                    fields,
                    expected_fields,
                    variables,
                    capabilities,
                    pure,
                    &mut result,
                )?;
                result.ty = ResolvedType::Nominal(resolved.owner);
            } else {
                let mut inferred = Vec::new();
                for field in fields {
                    let value = infer_expression(
                        context,
                        module,
                        &field.value,
                        variables,
                        capabilities,
                        pure,
                    )?;
                    inferred.push(ResolvedField {
                        name: field.name.clone(),
                        ty: value.ty.clone(),
                    });
                    result.merge_effects(&value)?;
                }
                result.ty = ResolvedType::Record(inferred);
            }
            Ok(result)
        }
        Expression::Variant {
            ty,
            case,
            payload,
            span,
        } => {
            let resolved = context.package.resolve_reference(module, ty)?;
            let shape = lookup_nominal_shape(context, &resolved.owner).ok_or_else(|| {
                semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_variant_type",
                    format!(
                        "'{}' is not a validated nominal type",
                        resolved.owner.diagnostic_name()
                    ),
                )
            })?;
            let NominalShape::Variant(cases) = shape else {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_variant_record",
                    format!(
                        "'{}' is a record, not a variant",
                        resolved.owner.diagnostic_name()
                    ),
                ));
            };
            let expected = cases.get(case).ok_or_else(|| {
                semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_variant_case",
                    format!(
                        "variant '{}' has no case '{case}'",
                        resolved.owner.diagnostic_name()
                    ),
                )
            })?;
            let owner = resolved.owner.clone();
            let mut result = ExpressionFacts::typed(ResolvedType::Nominal(owner.clone()));
            match (expected, payload) {
                (None, None) => {}
                (Some(expected), Some(payload)) => {
                    let value =
                        infer_expression(context, module, payload, variables, capabilities, pure)?;
                    require_same_type(
                        module,
                        payload.span(),
                        expected,
                        &value.ty,
                        "semantic_variant_payload",
                        &format!("variant '{}.{case}' payload", owner.diagnostic_name()),
                    )?;
                    result.merge_effects(&value)?;
                }
                (None, Some(_)) => {
                    return Err(semantic_at(
                        &module.path,
                        span.clone(),
                        "semantic_variant_unexpected_payload",
                        format!(
                            "variant '{}.{case}' has no payload",
                            owner.diagnostic_name()
                        ),
                    ));
                }
                (Some(_), None) => {
                    return Err(semantic_at(
                        &module.path,
                        span.clone(),
                        "semantic_variant_missing_payload",
                        format!(
                            "variant '{}.{case}' requires a payload",
                            owner.diagnostic_name()
                        ),
                    ));
                }
            }
            Ok(result)
        }
        Expression::Field { value, field, span } => {
            let value = infer_expression(context, module, value, variables, capabilities, pure)?;
            let fields = match &value.ty {
                ResolvedType::Record(fields) => fields.clone(),
                ResolvedType::Nominal(owner) => match lookup_nominal_shape(context, owner) {
                    Some(NominalShape::Record(fields)) => fields.clone(),
                    _ => {
                        return Err(semantic_at(
                            &module.path,
                            span.clone(),
                            "semantic_field_non_record",
                            "field selection requires a record value",
                        ));
                    }
                },
                _ => {
                    return Err(semantic_at(
                        &module.path,
                        span.clone(),
                        "semantic_field_non_record",
                        "field selection requires a record value",
                    ));
                }
            };
            let ty = fields
                .iter()
                .find(|candidate| candidate.name == *field)
                .map(|candidate| candidate.ty.clone())
                .ok_or_else(|| {
                    semantic_at(
                        &module.path,
                        span.clone(),
                        "semantic_field_missing",
                        format!("record has no field '{field}'"),
                    )
                })?;
            let mut result = ExpressionFacts::typed(ty);
            result.merge_effects(&value)?;
            Ok(result)
        }
        Expression::List {
            item_type,
            items,
            span,
        } => {
            let item_type = resolve_type(context.package, module, item_type, span)?;
            let mut result =
                ExpressionFacts::typed(ResolvedType::List(Box::new(item_type.clone())));
            for item in items {
                let value = infer_expression(context, module, item, variables, capabilities, pure)?;
                require_same_type(
                    module,
                    item.span(),
                    &item_type,
                    &value.ty,
                    "semantic_list_item",
                    "list item",
                )?;
                result.merge_effects(&value)?;
            }
            Ok(result)
        }
        Expression::Map {
            key_type,
            value_type,
            entries,
            span,
        } => {
            let key_type = resolve_type(context.package, module, key_type, span)?;
            if !matches!(
                key_type,
                ResolvedType::Bool
                    | ResolvedType::I64
                    | ResolvedType::Bytes
                    | ResolvedType::Text
                    | ResolvedType::StaticText
            ) {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_map_key_order",
                    "map keys must have a closed deterministic primitive ordering",
                ));
            }
            let value_type = resolve_type(context.package, module, value_type, span)?;
            let mut result = ExpressionFacts::typed(ResolvedType::Map(
                Box::new(key_type.clone()),
                Box::new(value_type.clone()),
            ));
            for entry in entries {
                let key =
                    infer_expression(context, module, &entry.key, variables, capabilities, pure)?;
                let value =
                    infer_expression(context, module, &entry.value, variables, capabilities, pure)?;
                require_same_type(
                    module,
                    entry.key.span(),
                    &key_type,
                    &key.ty,
                    "semantic_map_key",
                    "map key",
                )?;
                require_same_type(
                    module,
                    entry.value.span(),
                    &value_type,
                    &value.ty,
                    "semantic_map_value",
                    "map value",
                )?;
                result.merge_effects(&key)?;
                result.merge_effects(&value)?;
            }
            Ok(result)
        }
        Expression::Match { value, arms, span } => infer_match(
            context,
            module,
            value,
            arms,
            span,
            variables,
            capabilities,
            pure,
        ),
        Expression::FunctionRef {
            function,
            type_arguments,
            span,
        } => {
            let generic_signature = resolve_function_signature(context, module, function)?;
            let (signature, resolved_type_arguments) =
                instantiate_signature(context, module, &generic_signature, type_arguments, span)?;
            if !signature.task_capabilities.is_empty() && !context.allow_task_function_values {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_task_function_value",
                    format!(
                        "task '{}' cannot be used as a function value",
                        generic_signature.owner.diagnostic_name()
                    ),
                ));
            }
            let mut result = ExpressionFacts::typed(ResolvedType::Function(
                signature.parameters.clone(),
                Box::new(signature.result.clone()),
            ));
            if signature.external_implementation.is_none() {
                result.function_refs.insert(signature.owner.clone());
                if !generic_signature.type_parameters.is_empty() {
                    result.generic_calls.push(GenericApplication {
                        function: signature.owner.clone(),
                        type_arguments: resolved_type_arguments,
                    });
                }
            }
            Ok(result)
        }
        Expression::Perform {
            capability,
            operation,
            arguments,
            span,
        } => {
            if pure {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_pure_perform",
                    format!("pure expression performs '{capability}.{operation}'"),
                ));
            }
            let binding = capabilities.get(capability).ok_or_else(|| {
                semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_capability_missing",
                    format!("capability alias '{capability}' is not declared by this task"),
                )
            })?;
            let interface = lookup_interface(context, &binding.interface).ok_or_else(|| {
                semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_interface_missing",
                    format!(
                        "capability interface '{}' is unavailable",
                        binding.interface.diagnostic_name()
                    ),
                )
            })?;
            let contract = interface.operations.get(operation).ok_or_else(|| {
                semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_capability_operation",
                    format!("interface has no operation '{operation}'"),
                )
            })?;
            if contract.parameters.len() != arguments.len() {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_capability_arity",
                    format!(
                        "operation '{capability}.{operation}' requires {} arguments; {} were supplied",
                        contract.parameters.len(),
                        arguments.len()
                    ),
                ));
            }
            let mut result = ExpressionFacts::typed(contract.result.clone());
            for (argument, expected) in arguments.iter().zip(&contract.parameters) {
                let value =
                    infer_expression(context, module, argument, variables, capabilities, pure)?;
                require_same_type(
                    module,
                    argument.span(),
                    expected,
                    &value.ty,
                    "semantic_capability_argument",
                    &format!("operation '{capability}.{operation}' argument"),
                )?;
                result.merge_effects(&value)?;
            }
            result
                .capabilities
                .entry(binding.report_alias.clone())
                .or_insert_with(|| CapabilityFacts {
                    interface: binding.interface.clone(),
                    operations: BTreeSet::new(),
                })
                .operations
                .insert(operation.clone());
            Ok(result)
        }
        Expression::Transaction {
            capability,
            binding,
            body,
            span,
        } => {
            if pure {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_pure_transaction",
                    "pure expression opens a live transaction",
                ));
            }
            if capabilities.contains_key(binding) || variables.contains_key(binding) {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_transaction_binding",
                    format!("transaction binding '{binding}' shadows an existing owner"),
                ));
            }
            let parent = capabilities.get(capability).ok_or_else(|| {
                semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_transaction_capability",
                    format!("transaction capability alias '{capability}' is not declared"),
                )
            })?;
            let interface = lookup_interface(context, &parent.interface).ok_or_else(|| {
                semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_interface_missing",
                    "transaction interface is unavailable",
                )
            })?;
            let Some(transaction) = interface.operations.get("transaction") else {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_transaction_operation",
                    "capability interface does not declare transaction scope",
                ));
            };
            if !transaction.parameters.is_empty() || transaction.result != ResolvedType::Unit {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_transaction_signature",
                    "transaction scope operation must have signature (() -> Unit)",
                ));
            }
            let mut nested = capabilities.clone();
            nested.insert(binding.clone(), parent.clone());
            let mut result = infer_expression(context, module, body, variables, &nested, false)?;
            result
                .capabilities
                .entry(parent.report_alias.clone())
                .or_insert_with(|| CapabilityFacts {
                    interface: parent.interface.clone(),
                    operations: BTreeSet::new(),
                })
                .operations
                .insert("transaction".to_owned());
            Ok(result)
        }
    }
}

fn expression_pointer(expression: &Expression) -> usize {
    std::ptr::from_ref(expression).addr()
}

#[allow(clippy::too_many_arguments)]
fn validate_record_values(
    context: &InferContext<'_>,
    module: &ValidatedModule,
    fields: &[super::language::RecordField],
    expected_fields: &[ResolvedField],
    variables: &BTreeMap<String, ResolvedType>,
    capabilities: &BTreeMap<String, CapabilityBinding>,
    pure: bool,
    result: &mut ExpressionFacts,
) -> Result<(), Diagnostic> {
    if fields.len() != expected_fields.len() {
        return Err(semantic_at(
            &module.path,
            fields
                .first()
                .map(|field| field.span.clone())
                .unwrap_or(SourceSpan {
                    byte_start: 0,
                    byte_end: 0,
                    line: 1,
                    column: 1,
                }),
            "semantic_record_field_count",
            format!(
                "record requires {} fields; {} were supplied",
                expected_fields.len(),
                fields.len()
            ),
        ));
    }
    for expected in expected_fields {
        let field = fields
            .iter()
            .find(|field| field.name == expected.name)
            .ok_or_else(|| {
                semantic_at(
                    &module.path,
                    fields[0].span.clone(),
                    "semantic_record_field_missing",
                    format!("record omits field '{}'", expected.name),
                )
            })?;
        let value = infer_expression(context, module, &field.value, variables, capabilities, pure)?;
        require_same_type(
            module,
            field.value.span(),
            &expected.ty,
            &value.ty,
            "semantic_record_field_type",
            &format!("record field '{}'.", expected.name),
        )?;
        result.merge_effects(&value)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn infer_match(
    context: &InferContext<'_>,
    module: &ValidatedModule,
    value_expression: &Expression,
    arms: &[MatchArm],
    span: &SourceSpan,
    variables: &BTreeMap<String, ResolvedType>,
    capabilities: &BTreeMap<String, CapabilityBinding>,
    pure: bool,
) -> Result<ExpressionFacts, Diagnostic> {
    let value = infer_expression(
        context,
        module,
        value_expression,
        variables,
        capabilities,
        pure,
    )?;
    let ResolvedType::Nominal(owner) = &value.ty else {
        return Err(semantic_at(
            &module.path,
            span.clone(),
            "semantic_match_type",
            "match currently requires a nominal variant",
        ));
    };
    let Some(NominalShape::Variant(cases)) = lookup_nominal_shape(context, owner) else {
        return Err(semantic_at(
            &module.path,
            span.clone(),
            "semantic_match_record",
            "match requires a nominal variant, not a record",
        ));
    };
    let arm_names: BTreeSet<_> = arms.iter().map(|arm| arm.case.as_str()).collect();
    let case_names: BTreeSet<_> = cases.keys().map(String::as_str).collect();
    if arm_names != case_names {
        return Err(semantic_at(
            &module.path,
            span.clone(),
            "semantic_match_exhaustive",
            "match arms must cover every variant case exactly",
        ));
    }
    let mut result = ExpressionFacts::default();
    result.merge_effects(&value)?;
    let mut result_type = None;
    for arm in arms {
        let payload = cases.get(&arm.case).ok_or_else(|| {
            semantic_at(
                &module.path,
                arm.span.clone(),
                "semantic_match_case",
                format!("variant has no case '{}'", arm.case),
            )
        })?;
        let mut local = variables.clone();
        match (payload, &arm.binding) {
            (Some(payload), Some(binding)) => {
                local.insert(binding.clone(), payload.clone());
            }
            (None, None) => {}
            (Some(_), None) => {
                return Err(semantic_at(
                    &module.path,
                    arm.span.clone(),
                    "semantic_match_binding_missing",
                    format!("case '{}' requires a payload binding", arm.case),
                ));
            }
            (None, Some(_)) => {
                return Err(semantic_at(
                    &module.path,
                    arm.span.clone(),
                    "semantic_match_binding_unexpected",
                    format!("case '{}' has no payload to bind", arm.case),
                ));
            }
        }
        let body = infer_expression(context, module, &arm.body, &local, capabilities, pure)?;
        if let Some(expected) = &result_type {
            require_same_type(
                module,
                arm.body.span(),
                expected,
                &body.ty,
                "semantic_match_arm_type",
                "match arm",
            )?;
        } else {
            result_type = Some(body.ty.clone());
        }
        result.merge_effects(&body)?;
    }
    result.ty = result_type.unwrap_or(ResolvedType::Unit);
    Ok(result)
}

fn validate_component(
    context: &InferContext<'_>,
    module: &ValidatedModule,
    component: &Component,
    function_facts: &BTreeMap<OwnerId, FunctionFacts>,
) -> Result<(), Diagnostic> {
    let mut required = BTreeMap::new();
    let component_inference = context.for_component_port();
    for port in &component.ports {
        let expected = resolve_type(context.package, module, &port.ty, &port.span)?;
        let value = infer_expression(
            &component_inference,
            module,
            &port.value,
            &BTreeMap::new(),
            &BTreeMap::new(),
            true,
        )?;
        require_same_type(
            module,
            port.value.span(),
            &expected,
            &value.ty,
            "semantic_port_type",
            &format!("component '{}.{}' port", component.name, port.name),
        )?;
        for function in value.function_refs {
            let facts = function_facts
                .get(&function)
                .or_else(|| dependency_function_facts(context.package, &function))
                .ok_or_else(|| {
                    semantic_at(
                        &module.path,
                        port.span.clone(),
                        "semantic_port_function_facts",
                        format!(
                            "port references function '{}' without validated facts",
                            function.diagnostic_name()
                        ),
                    )
                })?;
            merge_capability_maps(&mut required, &facts.capabilities, Some(module))?;
        }
    }
    let declared: BTreeMap<_, _> = component
        .requirements
        .iter()
        .map(|requirement| (requirement.alias.as_str(), requirement))
        .collect();
    for (alias, capability) in required {
        let requirement = declared.get(alias.as_str()).ok_or_else(|| {
            semantic_at(
                &module.path,
                component.span.clone(),
                "semantic_component_requirement_missing",
                format!(
                    "component '{}' does not declare task capability alias '{alias}'",
                    component.name
                ),
            )
        })?;
        let resolved = context
            .package
            .resolve_reference(module, &requirement.interface)?;
        if resolved.owner != capability.interface {
            return Err(semantic_at(
                &module.path,
                requirement.span.clone(),
                "semantic_component_requirement_interface",
                format!("component capability alias '{alias}' binds a different interface"),
            ));
        }
        let allowed: BTreeSet<_> = requirement.operations.iter().cloned().collect();
        if !capability.operations.is_subset(&allowed) {
            return Err(semantic_at(
                &module.path,
                requirement.span.clone(),
                "semantic_component_requirement_operation",
                format!("component capability alias '{alias}' omits a used operation"),
            ));
        }
    }
    Ok(())
}

fn validate_targets(context: &PackageContext<'_>) -> Result<(), Diagnostic> {
    for target in &context.descriptor.targets {
        let Some((module_name, component_name)) = target.component.rsplit_once('.') else {
            return Err(semantic_without_location(
                "semantic_target_component",
                format!("target '{}' component is not qualified", target.name),
            ));
        };
        let module = context.module(module_name).ok_or_else(|| {
            semantic_without_location(
                "semantic_target_module",
                format!(
                    "target '{}' names absent module '{module_name}'",
                    target.name
                ),
            )
        })?;
        let component = module
            .module
            .declarations
            .iter()
            .find_map(|declaration| match declaration {
                Declaration::Component(component) if component.name == component_name => {
                    Some(component)
                }
                _ => None,
            })
            .ok_or_else(|| {
                semantic_without_location(
                    "semantic_target_component",
                    format!(
                        "target '{}' names absent component '{}'",
                        target.name, target.component
                    ),
                )
            })?;
        if !component.ports.iter().any(|port| port.name == target.port) {
            return Err(semantic_without_location(
                "semantic_target_port",
                format!(
                    "target '{}' names absent port '{}.{}'",
                    target.name, target.component, target.port
                ),
            ));
        }
    }
    Ok(())
}

fn resolve_function_signature(
    context: &InferContext<'_>,
    module: &ValidatedModule,
    reference: &DeclarationReference,
) -> Result<FunctionSignature, Diagnostic> {
    let resolved = context.package.resolve_reference(module, reference)?;
    if !matches!(
        resolved.declaration,
        Declaration::Function(_) | Declaration::External(_)
    ) {
        return Err(semantic_at(
            &module.path,
            resolved.declaration.span().clone(),
            "semantic_call_kind",
            format!("'{}' is not a function", resolved.owner.diagnostic_name()),
        ));
    }
    lookup_function_signature(context, &resolved.owner)
        .cloned()
        .ok_or_else(|| {
            semantic_at(
                &module.path,
                resolved.declaration.span().clone(),
                "semantic_function_signature_missing",
                format!(
                    "function '{}' has no validated signature",
                    resolved.owner.diagnostic_name()
                ),
            )
        })
}

fn instantiate_signature(
    context: &InferContext<'_>,
    module: &ValidatedModule,
    signature: &FunctionSignature,
    type_arguments: &[Type],
    span: &SourceSpan,
) -> Result<(FunctionSignature, Vec<ResolvedType>), Diagnostic> {
    if signature.type_parameters.len() != type_arguments.len() {
        return Err(semantic_at(
            &module.path,
            span.clone(),
            "semantic_type_argument_arity",
            format!(
                "function '{}' requires {} explicit type arguments; {} were supplied",
                signature.owner.diagnostic_name(),
                signature.type_parameters.len(),
                type_arguments.len()
            ),
        ));
    }
    let resolved = type_arguments
        .iter()
        .map(|argument| {
            resolve_type_in_scope(
                context.package,
                module,
                argument,
                span,
                &context.type_parameters,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let substitutions = signature
        .type_parameters
        .iter()
        .zip(&resolved)
        .map(|(parameter, argument)| (parameter.id, argument.clone()))
        .collect::<BTreeMap<_, _>>();
    let parameters = signature
        .parameters
        .iter()
        .map(|parameter| substitute_type(parameter, &substitutions))
        .collect::<Result<Vec<_>, _>>()?;
    let result = substitute_type(&signature.result, &substitutions)?;
    Ok((
        FunctionSignature {
            owner: signature.owner.clone(),
            type_parameters: Vec::new(),
            parameters,
            result,
            task_capabilities: signature.task_capabilities.clone(),
            external_implementation: signature.external_implementation.clone(),
        },
        resolved,
    ))
}

fn substitute_type(
    ty: &ResolvedType,
    substitutions: &BTreeMap<TypeParameterId, ResolvedType>,
) -> Result<ResolvedType, Diagnostic> {
    Ok(match ty {
        ResolvedType::Parameter(parameter) => {
            substitutions.get(parameter).cloned().ok_or_else(|| {
                semantic_without_location(
                    "semantic_type_substitution_missing",
                    format!("type parameter '{parameter}' has no explicit argument"),
                )
            })?
        }
        ResolvedType::Record(fields) => ResolvedType::Record(
            fields
                .iter()
                .map(|field| {
                    Ok(ResolvedField {
                        name: field.name.clone(),
                        ty: substitute_type(&field.ty, substitutions)?,
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?,
        ),
        ResolvedType::List(item) => {
            ResolvedType::List(Box::new(substitute_type(item, substitutions)?))
        }
        ResolvedType::Map(key, value) => ResolvedType::Map(
            Box::new(substitute_type(key, substitutions)?),
            Box::new(substitute_type(value, substitutions)?),
        ),
        ResolvedType::Option(item) => {
            ResolvedType::Option(Box::new(substitute_type(item, substitutions)?))
        }
        ResolvedType::Result(ok, error) => ResolvedType::Result(
            Box::new(substitute_type(ok, substitutions)?),
            Box::new(substitute_type(error, substitutions)?),
        ),
        ResolvedType::Stream(item) => {
            ResolvedType::Stream(Box::new(substitute_type(item, substitutions)?))
        }
        ResolvedType::Function(parameters, result) => ResolvedType::Function(
            parameters
                .iter()
                .map(|parameter| substitute_type(parameter, substitutions))
                .collect::<Result<Vec<_>, _>>()?,
            Box::new(substitute_type(result, substitutions)?),
        ),
        ResolvedType::Unit => ResolvedType::Unit,
        ResolvedType::Bool => ResolvedType::Bool,
        ResolvedType::I64 => ResolvedType::I64,
        ResolvedType::Bytes => ResolvedType::Bytes,
        ResolvedType::Text => ResolvedType::Text,
        ResolvedType::StaticText => ResolvedType::StaticText,
        ResolvedType::Secret => ResolvedType::Secret,
        ResolvedType::Nominal(owner) => ResolvedType::Nominal(owner.clone()),
    })
}

fn lookup_function_signature<'a>(
    context: &'a InferContext<'_>,
    owner: &OwnerId,
) -> Option<&'a FunctionSignature> {
    context
        .signatures
        .get(owner)
        .or_else(|| dependency_function_facts(context.package, owner).map(|facts| &facts.signature))
}

fn dependency_function_facts<'a>(
    context: &'a PackageContext<'_>,
    owner: &OwnerId,
) -> Option<&'a FunctionFacts> {
    context
        .dependencies
        .iter()
        .find(|dependency| dependency.package.descriptor.package_id == owner.package)
        .and_then(|dependency| dependency.package.function_facts.get(owner))
}

fn lookup_nominal_shape<'a>(
    context: &'a InferContext<'_>,
    owner: &OwnerId,
) -> Option<&'a NominalShape> {
    context.nominal_shapes.get(owner).or_else(|| {
        context
            .package
            .dependencies
            .iter()
            .find(|dependency| dependency.package.descriptor.package_id == owner.package)
            .and_then(|dependency| dependency.package.nominal_shapes.get(owner))
    })
}

fn lookup_interface<'a>(
    context: &'a InferContext<'_>,
    owner: &OwnerId,
) -> Option<&'a ResolvedInterface> {
    context.interfaces.get(owner).or_else(|| {
        context
            .package
            .dependencies
            .iter()
            .find(|dependency| dependency.package.descriptor.package_id == owner.package)
            .and_then(|dependency| dependency.package.interfaces.get(owner))
    })
}

fn lookup_constant_type<'a>(
    context: &'a InferContext<'_>,
    owner: &OwnerId,
) -> Option<&'a ResolvedType> {
    context.constant_types.get(owner).or_else(|| {
        context
            .package
            .dependencies
            .iter()
            .find(|dependency| dependency.package.descriptor.package_id == owner.package)
            .and_then(|dependency| dependency.package.constant_types.get(owner))
    })
}

fn merge_capability_maps(
    target: &mut BTreeMap<String, CapabilityFacts>,
    source: &BTreeMap<String, CapabilityFacts>,
    module: Option<&ValidatedModule>,
) -> Result<(), Diagnostic> {
    for (alias, facts) in source {
        match target.get_mut(alias) {
            Some(existing) if existing.interface == facts.interface => {
                existing.operations.extend(facts.operations.iter().cloned());
            }
            Some(_) => {
                return Err(if let Some(module) = module {
                    semantic_at(
                        &module.path,
                        SourceSpan {
                            byte_start: 0,
                            byte_end: 0,
                            line: 1,
                            column: 1,
                        },
                        "semantic_capability_alias_conflict",
                        format!("capability alias '{alias}' refers to two interfaces"),
                    )
                } else {
                    semantic_without_location(
                        "semantic_capability_alias_conflict",
                        format!("capability alias '{alias}' refers to two interfaces"),
                    )
                });
            }
            None => {
                target.insert(alias.clone(), facts.clone());
            }
        }
    }
    Ok(())
}

fn require_same_type(
    module: &ValidatedModule,
    span: &SourceSpan,
    expected: &ResolvedType,
    actual: &ResolvedType,
    code: &str,
    label: &str,
) -> Result<(), Diagnostic> {
    if expected != actual {
        return Err(semantic_at(
            &module.path,
            span.clone(),
            code,
            format!("{label} has type {actual:?}; expected {expected:?}"),
        ));
    }
    Ok(())
}

fn package_revision_digest(
    descriptor: &PackageDescriptor,
    modules: &[ValidatedModule],
) -> Result<String, Diagnostic> {
    let mut hasher = blake3::Hasher::new_derive_key(PACKAGE_REVISION_DIGEST_DOMAIN);
    let metadata = semantic_dependency_bytes(descriptor)?;
    hasher.update(&(metadata.len() as u64).to_be_bytes());
    hasher.update(&metadata);
    let mut ordered: Vec<_> = modules.iter().collect();
    ordered.sort_by(|left, right| left.module.name.cmp(&right.module.name));
    for module in ordered {
        hasher.update(&(module.module.name.len() as u64).to_be_bytes());
        hasher.update(module.module.name.as_bytes());
        hasher.update(&(module.semantic_bytes.len() as u64).to_be_bytes());
        hasher.update(&module.semantic_bytes);
    }
    Ok(hex(hasher.finalize().as_bytes()))
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    result
}

fn semantic_at(path: &str, span: SourceSpan, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::semantic(
        code,
        message,
        SourceLocation {
            path: path.to_owned(),
            byte_offset: span.byte_start,
            line: span.line,
            column: span.column,
        },
    )
}

fn semantic_reference_span() -> SourceSpan {
    SourceSpan {
        byte_start: 0,
        byte_end: 0,
        line: 1,
        column: 1,
    }
}

fn semantic_without_location(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(super::diagnostic::DiagnosticClass::Semantic, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::package::decode_package;
    use crate::platform::syntax::{SourceLimits, parse_source};

    fn source(path: &str, text: &str) -> SourceDocument {
        parse_source(path, text.as_bytes(), SourceLimits::default()).expect("source parses")
    }

    fn standard_package() -> ValidatedPackage {
        let descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","name":"standard","modules":[{"name":"core","path":"src/core.lkj"},{"name":"clock","path":"src/clock.lkj"},{"name":"random","path":"src/random.lkj"},{"name":"relational","path":"src/relational.lkj"},{"name":"http","path":"src/http.lkj"}],"dependencies":[],"targets":[]}"#,
        )
        .expect("standard descriptor");
        validate_package_documents(
            descriptor,
            vec![
                source(
                    "src/core.lkj",
                    "(module core (export equal-text) (extern equal-text ((left Text) (right Text)) Bool core.text.equal))",
                ),
                source(
                    "src/clock.lkj",
                    "(module clock (export Clock) (interface Clock (operation utc-now () I64 idempotent no-visibility)))",
                ),
                source(
                    "src/random.lkj",
                    "(module random (export SecureRandom) (interface SecureRandom (operation bytes ((length I64)) Bytes non-idempotent no-visibility)))",
                ),
                source(
                    "src/relational.lkj",
                    "(module relational
                       (export Command Row Database)
                       (record Command (statement Text) (parameters (List Text)))
                       (record Row (values (Map Text Text)))
                       (interface Database
                         (operation transaction () Unit idempotent no-visibility)
                         (operation execute ((command Command)) Row idempotent-with-key possible-visibility)))",
                ),
                source(
                    "src/http.lkj",
                    "(module http
                       (export Request Response Service service)
                       (record Request (body Text))
                       (record Response (status I64) (body Text))
                       (record Service (handler (Function (Request) Response)))
                       (fn service ((handler (Function (Request) Response))) Service
                         (record Service (handler handler))))",
                ),
            ],
            &[],
        )
        .expect("standard package validates")
    }

    fn service_descriptor(
        standard: &ValidatedPackage,
        database_operations: &str,
    ) -> PackageDescriptor {
        let json = format!(
            "{{\"contract_version\":1,\"package_id\":\"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\",\"name\":\"resource-service\",\"modules\":[{{\"name\":\"service\",\"path\":\"src/service.lkj\"}}],\"dependencies\":[{{\"alias\":\"std\",\"package_id\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"revision_digest\":\"{}\",\"artifact_digest\":\"{}\",\"artifact\":\"dependencies/standard.lkpackage\"}}],\"targets\":[{{\"name\":\"serve\",\"component\":\"service.Web\",\"port\":\"service\",\"runner\":\"http\"}}]}}",
            standard.revision_digest,
            "c".repeat(64)
        );
        let mut descriptor = decode_package(json.as_bytes()).expect("service descriptor");
        if database_operations != "transaction execute" {
            // The argument is consumed by the source helper; retain a single exact descriptor.
            descriptor.name = "resource-service".to_owned();
        }
        descriptor
    }

    fn service_source(database_operations: &str) -> String {
        format!(
            "(module service
               (import core std.core)
               (import clock std.clock)
               (import random std.random)
               (import db std.relational)
               (import http std.http)
               (export Web create)
               (task create ((request http.Request)) http.Response
                 (requires (database db.Database) (clock clock.Clock) (random random.SecureRandom))
                 (let ((now (perform clock utc-now))
                       (nonce (perform random bytes 16)))
                   (transaction database tx
                     (let ((row (perform tx execute
                       (record db.Command
                         (statement \"insert resource\")
                         (parameters (list Text (field request body)))))))
                       (record http.Response (status 201) (body (field request body)))))))
               (component Web
                 (require database db.Database (operations {database_operations})
                   (limit maximum-rows 1000))
                 (require clock clock.Clock (operations utc-now))
                 (require random random.SecureRandom (operations bytes)
                   (limit maximum-bytes 64))
                 (port service http.Service (call http.service (function create)))))"
        )
    }

    #[test]
    fn nominal_identity_is_not_a_path_or_position() {
        let descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"sample","modules":[{"name":"domain","path":"src/moved/domain.lkj"}],"dependencies":[],"targets":[]}"#,
        )
        .expect("descriptor");
        let package = validate_package_documents(
            descriptor,
            vec![source(
                "src/moved/domain.lkj",
                "(module domain (export Item) (record Item (name Text)))",
            )],
            &[],
        )
        .expect("package validates");
        let owner = package.modules[0]
            .owner(&package.descriptor.package_id, "Item")
            .expect("owner identity");
        assert_eq!(
            owner.diagnostic_name(),
            "1234567890abcdef1234567890abcdef::domain::Item"
        );
        assert!(!owner.diagnostic_name().contains("src/"));
    }

    #[test]
    fn source_oracle_resolves_exports_and_globals_but_keeps_lexical_variables() {
        let descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"references","modules":[{"name":"main","path":"src/main.lkj"}],"dependencies":[],"targets":[]}"#,
        )
        .expect("descriptor");
        let package = validate_package_documents(
            descriptor,
            vec![source(
                "src/main.lkj",
                "(module main
                   (export answer global local)
                   (const answer I64 42)
                   (fn global () I64 answer)
                   (fn local ((answer I64)) I64 answer))",
            )],
            &[],
        )
        .expect("source references resolve");
        let module = &package.modules[0];
        let answer = module
            .declaration_identities
            .iter()
            .find(|identity| identity.name == "answer")
            .expect("answer identity");
        assert!(module.module.exports.contains(&answer.id));
        let global = module
            .module
            .declarations
            .iter()
            .find_map(|declaration| match declaration {
                Declaration::Function(function) if function.name == "global" => Some(function),
                _ => None,
            })
            .expect("global function");
        let Expression::Constant(reference, _) = &global.body else {
            panic!("global constant reference was not resolved");
        };
        assert_eq!(reference.package, package.descriptor.package_id);
        assert_eq!(reference.module, module.module_id);
        assert_eq!(reference.declaration, answer.id);
        let local = module
            .module
            .declarations
            .iter()
            .find_map(|declaration| match declaration {
                Declaration::Function(function) if function.name == "local" => Some(function),
                _ => None,
            })
            .expect("local function");
        assert!(matches!(&local.body, Expression::Variable(name, _) if name == "answer"));
    }

    #[test]
    fn canonical_relations_reconstruct_calls_types_fields_and_tests() {
        let descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"relations","modules":[{"name":"main","path":"src/main.lkj"}],"dependencies":[],"targets":[]}"#,
        )
        .expect("descriptor");
        let package = validate_package_documents(
            descriptor,
            vec![source(
                "src/main.lkj",
                "(module main
                   (export Item make read)
                   (record Item (value Text))
                   (fn make ((value Text)) Item (record Item (value value)))
                   (fn read ((item Item)) Text (field item value))
                   (test round-trip (call read (call make \"value\")) \"value\"))",
            )],
            &[],
        )
        .expect("source migration validates");
        let relations = &package.modules[0].relations;
        for role in [
            RelationRole::Export,
            RelationRole::TypeUse,
            RelationRole::Call,
            RelationRole::FieldUse,
            RelationRole::ValueReference,
            RelationRole::TestDependency,
        ] {
            assert!(
                relations.iter().any(|relation| relation.role == role),
                "missing {role:?} relation"
            );
        }

        let mut meaning = MeaningModule {
            graph_contract_version: super::super::meaning::GRAPH_CONTRACT_VERSION,
            module_id: package.modules[0].module_id,
            module: package.modules[0].module.clone(),
            declarations: package.modules[0].declaration_identities.clone(),
            relations: relations.clone(),
            documentation: Vec::new(),
            annotations: Vec::new(),
        };
        super::super::meaning::normalize_module_spans(&mut meaning.module);
        let root = GraphRoot {
            graph_contract_version: super::super::meaning::GRAPH_CONTRACT_VERSION,
            repository_id: super::super::semantic_id::RepositoryId::migrate(b"relations", 1),
            package_id: package.descriptor.package_id.clone(),
            package_name: package.descriptor.name.clone(),
            modules: vec![super::super::graph::ModuleObjectRef {
                id: meaning.module_id,
                name: meaning.module.name.clone(),
                object: meaning.digest().expect("module digest"),
            }],
            dependencies: Vec::new(),
            targets: Vec::new(),
            tombstones: Vec::new(),
        };
        validate_graph_package(&root, vec![meaning.clone()], &[], None)
            .expect("canonical relations reconstruct");

        let mut missing = meaning;
        missing.relations.pop();
        let mut missing_root = root;
        missing_root.modules[0].object = missing.digest().expect("tampered module digest");
        let error = validate_graph_package(&missing_root, vec![missing], &[], None)
            .expect_err("missing canonical relation rejects");
        assert_eq!(error.code, "semantic_relation_mismatch");
    }

    #[test]
    fn unresolved_and_private_imports_reject() {
        let descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"sample","modules":[{"name":"a","path":"src/a.lkj"},{"name":"b","path":"src/b.lkj"}],"dependencies":[],"targets":[]}"#,
        )
        .expect("descriptor");
        let error = validate_package_documents(
            descriptor,
            vec![
                source(
                    "src/a.lkj",
                    "(module a (import other b) (record Uses (value other.Private)))",
                ),
                source("src/b.lkj", "(module b (record Private))"),
            ],
            &[],
        )
        .expect_err("private import rejects");
        assert_eq!(error.code, "semantic_import_private");
    }

    #[test]
    fn possible_visibility_requires_an_idempotency_key_contract() {
        let descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"sample","modules":[{"name":"effects","path":"src/effects.lkj"}],"dependencies":[],"targets":[]}"#,
        )
        .expect("descriptor");
        let error = validate_package_documents(
            descriptor,
            vec![source(
                "src/effects.lkj",
                "(module effects (interface Store (operation put ((value Bytes)) Unit idempotent possible-visibility)))",
            )],
            &[],
        )
        .expect_err("unsafe visibility declaration rejects");
        assert_eq!(error.code, "semantic_visibility_idempotency");
    }

    #[test]
    fn unknown_and_foreign_intrinsic_contracts_reject_during_semantic_validation() {
        let descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"sample","modules":[{"name":"core","path":"src/core.lkj"}],"dependencies":[],"targets":[]}"#,
        )
        .expect("descriptor");
        let unknown = validate_package_documents(
            descriptor.clone(),
            vec![source(
                "src/core.lkj",
                "(module core (extern forged ((value Text)) Text native.forged))",
            )],
            &[],
        )
        .expect_err("unknown intrinsic rejects before preparation");
        assert_eq!(unknown.code, "intrinsic_unknown");

        let foreign = validate_package_documents(
            descriptor,
            vec![source(
                "src/core.lkj",
                "(module core (extern forged ((value Text)) Text core.i64.add))",
            )],
            &[],
        )
        .expect_err("foreign signature rejects before preparation");
        assert_eq!(foreign.code, "intrinsic_signature");
    }

    #[test]
    fn service_component_resolves_exact_types_effects_and_operations() {
        let standard = standard_package();
        let descriptor = service_descriptor(&standard, "transaction execute");
        let dependency = ExactDependency {
            alias: "std",
            package: &standard,
            artifact_digest: &"c".repeat(64),
        };
        let package = validate_package_documents(
            descriptor,
            vec![source(
                "src/service.lkj",
                &service_source("transaction execute"),
            )],
            &[dependency],
        )
        .expect("service package validates");
        let owner = package.modules[0]
            .owner(&package.descriptor.package_id, "create")
            .expect("owner identity");
        let facts = package.function_facts.get(&owner).expect("task facts");
        assert_eq!(
            facts.capabilities["database"].operations,
            BTreeSet::from(["execute".to_owned(), "transaction".to_owned()])
        );
        assert_eq!(
            facts.capabilities["clock"].operations,
            BTreeSet::from(["utc-now".to_owned()])
        );
        assert_eq!(
            facts.capabilities["random"].operations,
            BTreeSet::from(["bytes".to_owned()])
        );
    }

    #[test]
    fn pure_effect_and_undergranted_component_reject() {
        let descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"sample","modules":[{"name":"bad","path":"src/bad.lkj"}],"dependencies":[],"targets":[]}"#,
        )
        .expect("descriptor");
        let error = validate_package_documents(
            descriptor,
            vec![source(
                "src/bad.lkj",
                "(module bad
                   (interface Clock (operation now () I64 idempotent no-visibility))
                   (fn observe () I64 (perform clock now)))",
            )],
            &[],
        )
        .expect_err("pure perform rejects");
        assert_eq!(error.code, "semantic_pure_perform");

        let standard = standard_package();
        let descriptor = service_descriptor(&standard, "execute");
        let digest = "c".repeat(64);
        let error = validate_package_documents(
            descriptor,
            vec![source("src/service.lkj", &service_source("execute"))],
            &[ExactDependency {
                alias: "std",
                package: &standard,
                artifact_digest: &digest,
            }],
        )
        .expect_err("undergrant rejects");
        assert_eq!(error.code, "semantic_component_requirement_operation");
    }
}
