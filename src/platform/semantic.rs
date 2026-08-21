//! Package resolution, nominal identity, type checking, and effect discovery.

use super::diagnostic::{Diagnostic, SourceLocation};
use super::language::{
    Component, Declaration, Effect, Expression, ExternalFunction, Function, Idempotency, Interface,
    MatchArm, Module, Parameter, TaskCapability, Type, Visibility,
};
use super::package::{PackageDescriptor, PackageId, semantic_dependency_bytes};
use super::syntax::{SourceDocument, SourceSpan};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerId {
    pub package: PackageId,
    pub module: String,
    pub declaration: String,
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
            Self::Secret | Self::Stream(_) | Self::Function(_, _) => false,
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
    pub parameters: Vec<ResolvedType>,
    pub result: ResolvedType,
    pub task_capabilities: Vec<ResolvedTaskCapability>,
    pub external_implementation: Option<String>,
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
    pub module: Module,
    pub semantic_bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidatedPackage {
    pub descriptor: PackageDescriptor,
    pub modules: Vec<ValidatedModule>,
    pub revision_digest: String,
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

pub fn validate_package_documents(
    descriptor: PackageDescriptor,
    documents: Vec<SourceDocument>,
    dependencies: &[ExactDependency<'_>],
) -> Result<ValidatedPackage, Diagnostic> {
    validate_exact_dependencies(&descriptor, dependencies)?;
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

    let mut modules = Vec::new();
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
        modules.push(ValidatedModule {
            path: locator.path.clone(),
            module,
            semantic_bytes: document.semantic_bytes().to_vec(),
        });
    }
    if let Some((path, _)) = by_path.into_iter().next() {
        return Err(semantic_without_location(
            "package_module_document_foreign",
            format!("source document '{path}' is not declared by the package"),
        ));
    }

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
    let revision_digest = package_revision_digest(&descriptor, &modules)?;
    Ok(ValidatedPackage {
        descriptor,
        modules,
        revision_digest,
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

    fn dependency(&self, alias: &str) -> Option<&ExactDependency<'_>> {
        self.dependencies
            .iter()
            .find(|dependency| dependency.alias == alias)
    }

    fn owner(&self, module: &str, declaration: &str) -> OwnerId {
        OwnerId {
            package: self.descriptor.package_id.clone(),
            module: module.to_owned(),
            declaration: declaration.to_owned(),
        }
    }

    fn resolve<'a>(
        &'a self,
        from: &'a ValidatedModule,
        name: &str,
    ) -> Result<ResolvedDeclaration<'a>, Diagnostic> {
        if let Some((alias, declaration_name)) = name.split_once('.') {
            if declaration_name.contains('.') {
                return Err(semantic_at(
                    &from.path,
                    SourceSpan {
                        byte_start: 0,
                        byte_end: 0,
                        line: 1,
                        column: 1,
                    },
                    "semantic_qualified_name",
                    format!("reference '{name}' has more than one qualification level"),
                ));
            }
            let import = from
                .module
                .imports
                .iter()
                .find(|import| import.alias == alias)
                .ok_or_else(|| {
                    semantic_at(
                        &from.path,
                        SourceSpan {
                            byte_start: 0,
                            byte_end: 0,
                            line: 1,
                            column: 1,
                        },
                        "semantic_import_alias_missing",
                        format!("reference '{name}' uses undeclared import alias '{alias}'"),
                    )
                })?;
            return self.resolve_import(from, import, declaration_name);
        }
        let declaration = from
            .module
            .declarations
            .iter()
            .find(|declaration| declaration.name() == name)
            .ok_or_else(|| {
                semantic_at(
                    &from.path,
                    SourceSpan {
                        byte_start: 0,
                        byte_end: 0,
                        line: 1,
                        column: 1,
                    },
                    "semantic_declaration_missing",
                    format!("module '{}' has no declaration '{name}'", from.module.name),
                )
            })?;
        Ok(ResolvedDeclaration {
            owner: self.owner(&from.module.name, name),
            declaration,
        })
    }

    fn resolve_import<'a>(
        &'a self,
        from: &'a ValidatedModule,
        import: &super::language::Import,
        declaration_name: &str,
    ) -> Result<ResolvedDeclaration<'a>, Diagnostic> {
        if let Some((dependency_alias, module_name)) = import.module.split_once('.')
            && let Some(dependency) = self.dependency(dependency_alias)
        {
            let module = dependency
                .package
                .modules
                .iter()
                .find(|module| module.module.name == module_name)
                .ok_or_else(|| {
                    semantic_at(
                        &from.path,
                        import.span.clone(),
                        "semantic_dependency_module_missing",
                        format!("dependency '{dependency_alias}' has no module '{module_name}'"),
                    )
                })?;
            if !module
                .module
                .exports
                .iter()
                .any(|export| export == declaration_name)
            {
                return Err(semantic_at(
                    &from.path,
                    import.span.clone(),
                    "semantic_import_private",
                    format!(
                        "dependency module '{}' does not export '{declaration_name}'",
                        import.module
                    ),
                ));
            }
            let declaration = module
                    .module
                    .declarations
                    .iter()
                    .find(|declaration| declaration.name() == declaration_name)
                    .ok_or_else(|| {
                        semantic_at(
                            &from.path,
                            import.span.clone(),
                            "semantic_import_export_corrupt",
                            format!(
                                "dependency module '{}' exports absent declaration '{declaration_name}'",
                                import.module
                            ),
                        )
                    })?;
            return Ok(ResolvedDeclaration {
                owner: OwnerId {
                    package: dependency.package.descriptor.package_id.clone(),
                    module: module.module.name.clone(),
                    declaration: declaration_name.to_owned(),
                },
                declaration,
            });
        }
        let module = self.module(&import.module).ok_or_else(|| {
            semantic_at(
                &from.path,
                import.span.clone(),
                "semantic_local_module_missing",
                format!("local module '{}' does not exist", import.module),
            )
        })?;
        if !module
            .module
            .exports
            .iter()
            .any(|export| export == declaration_name)
        {
            return Err(semantic_at(
                &from.path,
                import.span.clone(),
                "semantic_import_private",
                format!(
                    "local module '{}' does not export '{declaration_name}'",
                    import.module
                ),
            ));
        }
        let declaration = module
            .module
            .declarations
            .iter()
            .find(|declaration| declaration.name() == declaration_name)
            .ok_or_else(|| {
                semantic_at(
                    &from.path,
                    import.span.clone(),
                    "semantic_import_export_corrupt",
                    format!(
                        "local module '{}' exports absent declaration '{declaration_name}'",
                        import.module
                    ),
                )
            })?;
        Ok(ResolvedDeclaration {
            owner: self.owner(&module.module.name, declaration_name),
            declaration,
        })
    }
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
        for export in &module.module.exports {
            let declaration = module
                .module
                .declarations
                .iter()
                .find(|declaration| declaration.name() == export)
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
                            "module '{}' exports absent declaration '{export}'",
                            module.module.name
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
        }
        for import in &module.module.imports {
            if let Some((dependency_alias, dependency_module)) = import.module.split_once('.')
                && let Some(dependency) = context.dependency(dependency_alias)
            {
                if !dependency
                    .package
                    .modules
                    .iter()
                    .any(|candidate| candidate.module.name == dependency_module)
                {
                    return Err(semantic_at(
                        &module.path,
                        import.span.clone(),
                        "semantic_dependency_module_missing",
                        format!("dependency module '{}' does not exist", import.module),
                    ));
                }
                continue;
            }
            if context.module(&import.module).is_none() {
                return Err(semantic_at(
                    &module.path,
                    import.span.clone(),
                    "semantic_local_module_missing",
                    format!("local module '{}' does not exist", import.module),
                ));
            }
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
                    validate_signature_types(
                        context,
                        module,
                        &external.parameters,
                        &external.result,
                        &external.span,
                    )?;
                }
                Declaration::Function(function) => {
                    validate_signature_types(
                        context,
                        module,
                        &function.parameters,
                        &function.result,
                        &function.span,
                    )?;
                    if let Effect::Task { capabilities } = &function.effect {
                        for capability in capabilities {
                            resolve_interface(context, module, capability)?;
                        }
                    }
                }
                Declaration::Constant(constant) => {
                    resolve_type(context, module, &constant.ty, &constant.span)?;
                }
                Declaration::Component(component) => {
                    for requirement in &component.requirements {
                        let resolved = context.resolve(module, &requirement.interface)?;
                        let Declaration::Interface(interface) = resolved.declaration else {
                            return Err(semantic_at(
                                &module.path,
                                requirement.span.clone(),
                                "semantic_requirement_interface_kind",
                                format!(
                                    "'{}' is not a capability interface",
                                    requirement.interface
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
                                        requirement.interface
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
                    let owner = context.owner(&module.module.name, &record.name);
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
                    let owner = context.owner(&module.module.name, &variant.name);
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
                    let owner = context.owner(&module.module.name, &interface.name);
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
                    context.owner(&module.module.name, &constant.name),
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
    let resolved = context.resolve(module, &capability.interface)?;
    if !matches!(resolved.declaration, Declaration::Interface(_)) {
        return Err(semantic_at(
            &module.path,
            capability.span.clone(),
            "semantic_task_interface_kind",
            format!("'{}' is not a capability interface", capability.interface),
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
    Ok(match ty {
        Type::Unit => ResolvedType::Unit,
        Type::Bool => ResolvedType::Bool,
        Type::I64 => ResolvedType::I64,
        Type::Bytes => ResolvedType::Bytes,
        Type::Text => ResolvedType::Text,
        Type::StaticText => ResolvedType::StaticText,
        Type::Secret => ResolvedType::Secret,
        Type::Named(name) => {
            let resolved = context.resolve(module, name)?;
            if !matches!(
                resolved.declaration,
                Declaration::Record(_) | Declaration::Variant(_)
            ) {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_type_kind",
                    format!("'{name}' is not a record or variant type"),
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
                        ty: resolve_type(context, module, &field.ty, span)?,
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?,
        ),
        Type::List(item) => {
            ResolvedType::List(Box::new(resolve_type(context, module, item, span)?))
        }
        Type::Map(key, value) => ResolvedType::Map(
            Box::new(resolve_type(context, module, key, span)?),
            Box::new(resolve_type(context, module, value, span)?),
        ),
        Type::Option(item) => {
            ResolvedType::Option(Box::new(resolve_type(context, module, item, span)?))
        }
        Type::Result(ok, error) => ResolvedType::Result(
            Box::new(resolve_type(context, module, ok, span)?),
            Box::new(resolve_type(context, module, error, span)?),
        ),
        Type::Stream(item) => {
            ResolvedType::Stream(Box::new(resolve_type(context, module, item, span)?))
        }
        Type::Function(parameters, result) => ResolvedType::Function(
            parameters
                .iter()
                .map(|parameter| resolve_type(context, module, parameter, span))
                .collect::<Result<Vec<_>, _>>()?,
            Box::new(resolve_type(context, module, result, span)?),
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
        owner: context.owner(&module.module.name, &function.name),
        parameters: function
            .parameters
            .iter()
            .map(|parameter| resolve_type(context, module, &parameter.ty, &parameter.span))
            .collect::<Result<Vec<_>, _>>()?,
        result: resolve_type(context, module, &function.result, &function.span)?,
        task_capabilities,
        external_implementation: implementation,
    })
}

fn external_signature(
    context: &PackageContext<'_>,
    module: &ValidatedModule,
    external: &ExternalFunction,
) -> Result<FunctionSignature, Diagnostic> {
    Ok(FunctionSignature {
        owner: context.owner(&module.module.name, &external.name),
        parameters: external
            .parameters
            .iter()
            .map(|parameter| resolve_type(context, module, &parameter.ty, &parameter.span))
            .collect::<Result<Vec<_>, _>>()?,
        result: resolve_type(context, module, &external.result, &external.span)?,
        task_capabilities: Vec::new(),
        external_implementation: Some(external.implementation.clone()),
    })
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
                let owner = context.owner(&module.module.name, &function.name);
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
                let owner = context.owner(&module.module.name, &function.name);
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
                    let owner = context.owner(&module.module.name, &constant.name);
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
        Ok(())
    }
}

struct InferContext<'a> {
    package: &'a PackageContext<'a>,
    signatures: &'a BTreeMap<OwnerId, FunctionSignature>,
    nominal_shapes: &'a BTreeMap<OwnerId, NominalShape>,
    interfaces: &'a BTreeMap<OwnerId, ResolvedInterface>,
    constant_types: &'a BTreeMap<OwnerId, ResolvedType>,
}

fn infer_expression(
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
            if let Some(ty) = variables.get(name) {
                return Ok(ExpressionFacts::typed(ty.clone()));
            }
            let resolved = context.package.resolve(module, name)?;
            if !matches!(resolved.declaration, Declaration::Constant(_)) {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_variable_kind",
                    format!("'{name}' is not a variable or constant"),
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
            arguments,
            span,
        } => {
            let signature = resolve_function_signature(context, module, function)?;
            if signature.parameters.len() != arguments.len() {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_call_arity",
                    format!(
                        "function '{function}' requires {} arguments; {} were supplied",
                        signature.parameters.len(),
                        arguments.len()
                    ),
                ));
            }
            let mut result = ExpressionFacts::typed(signature.result.clone());
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
                    &format!("argument {} of '{function}'", index + 1),
                )?;
                result.merge_effects(&value)?;
            }
            if !signature.task_capabilities.is_empty() {
                if pure {
                    return Err(semantic_at(
                        &module.path,
                        span.clone(),
                        "semantic_pure_task_call",
                        format!("pure expression calls task '{function}'"),
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
                                    "task '{function}' requires alias '{}' with a different interface",
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
                                    "task '{function}' requires unavailable capability alias '{}'",
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
        Expression::Record { ty, fields, span } => {
            let mut result = ExpressionFacts::default();
            if let Some(name) = ty {
                let resolved = context.package.resolve(module, name)?;
                let shape = lookup_nominal_shape(context, &resolved.owner).ok_or_else(|| {
                    semantic_at(
                        &module.path,
                        span.clone(),
                        "semantic_record_type",
                        format!("'{name}' is not a validated nominal type"),
                    )
                })?;
                let NominalShape::Record(expected_fields) = shape else {
                    return Err(semantic_at(
                        &module.path,
                        span.clone(),
                        "semantic_record_variant",
                        format!("'{name}' is a variant, not a record"),
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
            let resolved = context.package.resolve(module, ty)?;
            let shape = lookup_nominal_shape(context, &resolved.owner).ok_or_else(|| {
                semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_variant_type",
                    format!("'{ty}' is not a validated nominal type"),
                )
            })?;
            let NominalShape::Variant(cases) = shape else {
                return Err(semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_variant_record",
                    format!("'{ty}' is a record, not a variant"),
                ));
            };
            let expected = cases.get(case).ok_or_else(|| {
                semantic_at(
                    &module.path,
                    span.clone(),
                    "semantic_variant_case",
                    format!("variant '{ty}' has no case '{case}'"),
                )
            })?;
            let mut result = ExpressionFacts::typed(ResolvedType::Nominal(resolved.owner));
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
                        &format!("variant '{ty}.{case}' payload"),
                    )?;
                    result.merge_effects(&value)?;
                }
                (None, Some(_)) => {
                    return Err(semantic_at(
                        &module.path,
                        span.clone(),
                        "semantic_variant_unexpected_payload",
                        format!("variant '{ty}.{case}' has no payload"),
                    ));
                }
                (Some(_), None) => {
                    return Err(semantic_at(
                        &module.path,
                        span.clone(),
                        "semantic_variant_missing_payload",
                        format!("variant '{ty}.{case}' requires a payload"),
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
        Expression::FunctionRef { function, span } => {
            let signature = resolve_function_signature(context, module, function)?;
            let mut result = ExpressionFacts::typed(ResolvedType::Function(
                signature.parameters.clone(),
                Box::new(signature.result.clone()),
            ));
            if signature.external_implementation.is_none()
                || !signature.task_capabilities.is_empty()
            {
                result.function_refs.insert(signature.owner.clone());
            }
            if pure && !signature.task_capabilities.is_empty() {
                // Taking a task reference is pure. Execution remains owned by a component runner.
                let _ = span;
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
    for port in &component.ports {
        let expected = resolve_type(context.package, module, &port.ty, &port.span)?;
        let value = infer_expression(
            context,
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
        let resolved = context.package.resolve(module, &requirement.interface)?;
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
    name: &str,
) -> Result<FunctionSignature, Diagnostic> {
    let resolved = context.package.resolve(module, name)?;
    if !matches!(
        resolved.declaration,
        Declaration::Function(_) | Declaration::External(_)
    ) {
        return Err(semantic_at(
            &module.path,
            resolved.declaration.span().clone(),
            "semantic_call_kind",
            format!("'{name}' is not a function"),
        ));
    }
    lookup_function_signature(context, &resolved.owner)
        .cloned()
        .ok_or_else(|| {
            semantic_at(
                &module.path,
                resolved.declaration.span().clone(),
                "semantic_function_signature_missing",
                format!("function '{name}' has no validated signature"),
            )
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
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.package-revision.v1");
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
        let owner = OwnerId {
            package: package.descriptor.package_id.clone(),
            module: "domain".to_owned(),
            declaration: "Item".to_owned(),
        };
        assert_eq!(
            owner.diagnostic_name(),
            "1234567890abcdef1234567890abcdef::domain::Item"
        );
        assert!(!owner.diagnostic_name().contains("src/"));
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
        let owner = OwnerId {
            package: package.descriptor.package_id.clone(),
            module: "service".to_owned(),
            declaration: "create".to_owned(),
        };
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
