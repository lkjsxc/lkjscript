//! Prepared component execution shared by command, test, interactive, service, and worker runners.

mod capability;
mod compiler;
mod intrinsic;
mod reference;
mod vm;

use super::artifact::LoadedArtifact;
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::language::{Declaration, DeclarationReference, Expression};
use super::meaning::MemberIdentity;
use super::package::{PackageId, RunnerKind};
use super::semantic::{FunctionSignature, OwnerId, ResolvedOperation};
use super::semantic_id::{ExpressionId, PortId};
use std::collections::BTreeMap;
use std::sync::Arc;

pub use capability::{
    BoundCapabilities, CAPABILITY_GRANT_CONTRACT_VERSION, CallPolicy, CapabilityAdapter,
    CapabilityGrant, CapabilityGrantDescriptor, CapabilityTransaction, ScriptedAdapter,
    ScriptedCall,
};
pub use compiler::{CompiledFunction, Instruction, VariantJump};
pub use reference::ReferenceInterpreter;
pub use vm::{
    ExecutionControl, ExecutionError, ExecutionFailureClass, RunObservation, RunPolicy, Vm,
};

#[derive(Clone, Debug)]
pub struct PreparedFunction {
    pub signature: FunctionSignature,
    pub parameters: Vec<String>,
    pub compiled: Option<CompiledFunction>,
    pub external_implementation: Option<String>,
    pub source: Option<Expression>,
}

#[derive(Clone, Debug)]
pub struct PreparedPort {
    pub name: String,
    pub function: OwnerId,
    pub signature: FunctionSignature,
}

#[derive(Clone, Debug)]
pub struct PreparedRequirement {
    pub alias: String,
    pub interface: OwnerId,
    pub operations: BTreeMap<String, ResolvedOperation>,
    pub limits: BTreeMap<String, u64>,
}

#[derive(Clone, Debug)]
pub struct PreparedComponent {
    pub owner: OwnerId,
    pub requirements: BTreeMap<String, PreparedRequirement>,
    pub ports: BTreeMap<PortId, PreparedPort>,
}

#[derive(Clone, Debug)]
pub struct PreparedTarget {
    pub name: String,
    pub runner: RunnerKind,
    pub component: OwnerId,
    pub port: PreparedPort,
}

#[derive(Clone, Debug)]
pub struct PreparedTest {
    pub package: PackageId,
    pub module: String,
    pub name: String,
    pub actual: ExpressionId,
    pub expected: ExpressionId,
}

#[derive(Clone, Debug)]
pub struct PreparedProgram {
    artifact: Arc<LoadedArtifact>,
    functions: BTreeMap<OwnerId, PreparedFunction>,
    test_expressions: BTreeMap<ExpressionId, PreparedFunction>,
    components: BTreeMap<OwnerId, PreparedComponent>,
    targets: BTreeMap<String, PreparedTarget>,
    tests: Vec<PreparedTest>,
}

impl PreparedProgram {
    pub fn prepare(artifact: LoadedArtifact) -> Result<Self, Diagnostic> {
        let artifact = Arc::new(artifact);
        let mut functions = BTreeMap::new();
        let mut test_expressions = BTreeMap::new();
        let mut tests = Vec::new();
        for package in artifact.packages.values() {
            for module in &package.modules {
                for declaration in &module.module.declarations {
                    match declaration {
                        Declaration::Function(function) => {
                            let owner =
                                module.owner(&package.descriptor.package_id, &function.name)?;
                            let signature = package
                                .function_facts
                                .get(&owner)
                                .map(|facts| facts.signature.clone())
                                .ok_or_else(|| {
                                    execution_diagnostic(
                                        "prepare_function_facts",
                                        format!(
                                            "validated function '{}' has no function facts",
                                            owner.diagnostic_name()
                                        ),
                                    )
                                })?;
                            let compiled = compiler::compile_function(
                                &artifact,
                                &function.parameters,
                                &function.body,
                            )?;
                            functions.insert(
                                owner,
                                PreparedFunction {
                                    signature,
                                    parameters: function
                                        .parameters
                                        .iter()
                                        .map(|parameter| parameter.name.clone())
                                        .collect(),
                                    compiled: Some(compiled),
                                    external_implementation: None,
                                    source: Some(function.body.clone()),
                                },
                            );
                        }
                        Declaration::External(external) => {
                            let owner =
                                module.owner(&package.descriptor.package_id, &external.name)?;
                            let signature = package
                                .function_facts
                                .get(&owner)
                                .map(|facts| facts.signature.clone())
                                .ok_or_else(|| {
                                    execution_diagnostic(
                                        "prepare_external_facts",
                                        format!(
                                            "validated external '{}' has no function facts",
                                            owner.diagnostic_name()
                                        ),
                                    )
                                })?;
                            super::intrinsic_contract::validate_intrinsic(
                                &external.implementation,
                                &signature,
                            )?;
                            functions.insert(
                                owner,
                                PreparedFunction {
                                    signature,
                                    parameters: external
                                        .parameters
                                        .iter()
                                        .map(|parameter| parameter.name.clone())
                                        .collect(),
                                    compiled: None,
                                    external_implementation: Some(external.implementation.clone()),
                                    source: None,
                                },
                            );
                        }
                        Declaration::Constant(constant) => {
                            let owner =
                                module.owner(&package.descriptor.package_id, &constant.name)?;
                            let result =
                                package.constant_types.get(&owner).cloned().ok_or_else(|| {
                                    execution_diagnostic(
                                        "prepare_constant_type",
                                        format!(
                                            "validated constant '{}' has no type",
                                            owner.diagnostic_name()
                                        ),
                                    )
                                })?;
                            let compiled =
                                compiler::compile_function(&artifact, &[], &constant.value)?;
                            functions.insert(
                                owner.clone(),
                                PreparedFunction {
                                    signature: FunctionSignature {
                                        owner,
                                        type_parameters: Vec::new(),
                                        parameters: Vec::new(),
                                        result,
                                        task_capabilities: Vec::new(),
                                        external_implementation: None,
                                    },
                                    parameters: Vec::new(),
                                    compiled: Some(compiled),
                                    external_implementation: None,
                                    source: Some(constant.value.clone()),
                                },
                            );
                        }
                        Declaration::Test(test) => {
                            let test_owner =
                                module.owner(&package.descriptor.package_id, &test.name)?;
                            let actual = module.expression_id(&test.name, &[0])?;
                            let expected = module.expression_id(&test.name, &[1])?;
                            for (owner, expression) in
                                [(actual, &test.actual), (expected, &test.expected)]
                            {
                                let compiled =
                                    compiler::compile_function(&artifact, &[], expression)?;
                                test_expressions.insert(
                                    owner,
                                    PreparedFunction {
                                        signature: FunctionSignature {
                                            owner: test_owner.clone(),
                                            type_parameters: Vec::new(),
                                            parameters: Vec::new(),
                                            result: super::semantic::ResolvedType::Unit,
                                            task_capabilities: Vec::new(),
                                            external_implementation: None,
                                        },
                                        parameters: Vec::new(),
                                        compiled: Some(compiled),
                                        external_implementation: None,
                                        source: Some(expression.clone()),
                                    },
                                );
                            }
                            tests.push(PreparedTest {
                                package: package.descriptor.package_id.clone(),
                                module: module.module.name.clone(),
                                name: test.name.clone(),
                                actual,
                                expected,
                            });
                        }
                        Declaration::Record(_)
                        | Declaration::Variant(_)
                        | Declaration::Interface(_)
                        | Declaration::Component(_) => {}
                    }
                }
            }
        }

        let mut components = BTreeMap::new();
        for package in artifact.packages.values() {
            for module in &package.modules {
                for declaration in &module.module.declarations {
                    let Declaration::Component(component) = declaration else {
                        continue;
                    };
                    let owner = module.owner(&package.descriptor.package_id, &component.name)?;
                    let mut requirements = BTreeMap::new();
                    for requirement in &component.requirements {
                        let interface = resolve_reference_owner(&artifact, &requirement.interface)?;
                        let interface_contract = artifact
                            .packages
                            .get(&interface.package)
                            .and_then(|package| package.interfaces.get(&interface))
                            .ok_or_else(|| {
                                execution_diagnostic(
                                    "prepare_requirement_interface",
                                    format!(
                                        "component requirement '{}.{}' has no interface contract",
                                        owner.diagnostic_name(),
                                        requirement.alias
                                    ),
                                )
                            })?;
                        let selected_operations = requirement
                            .operations
                            .iter()
                            .map(|operation| {
                                let contract = interface_contract
                                    .operations
                                    .get(operation)
                                    .cloned()
                                    .ok_or_else(|| {
                                        execution_diagnostic(
                                            "prepare_requirement_operation",
                                            format!(
                                                "component requirement '{}.{}' names absent operation '{operation}'",
                                                owner.diagnostic_name(),
                                                requirement.alias
                                            ),
                                        )
                                    })?;
                                Ok((operation.clone(), contract))
                            })
                            .collect::<Result<BTreeMap<_, _>, Diagnostic>>()?;
                        let limits = requirement
                            .limits
                            .iter()
                            .map(|limit| (limit.name.clone(), limit.value))
                            .collect();
                        requirements.insert(
                            requirement.alias.clone(),
                            PreparedRequirement {
                                alias: requirement.alias.clone(),
                                interface,
                                operations: selected_operations,
                                limits,
                            },
                        );
                    }
                    let mut ports = BTreeMap::new();
                    for port in &component.ports {
                        let port_id = module
                            .declaration_identities
                            .iter()
                            .find(|identity| identity.id == owner.declaration_id)
                            .and_then(|identity| {
                                identity.members.iter().find_map(|member| match member {
                                    MemberIdentity::Port { id, name } if name == &port.name => {
                                        Some(*id)
                                    }
                                    _ => None,
                                })
                            })
                            .ok_or_else(|| {
                                execution_diagnostic(
                                    "prepare_port_identity",
                                    format!(
                                        "component port '{}.{}' has no stable identity",
                                        owner.diagnostic_name(),
                                        port.name
                                    ),
                                )
                            })?;
                        let Expression::FunctionRef { function, .. } = &port.value else {
                            return Err(execution_diagnostic(
                                "prepare_port_value",
                                format!(
                                    "component port '{}.{}' must currently be a direct function reference",
                                    owner.diagnostic_name(),
                                    port.name
                                ),
                            ));
                        };
                        let function = resolve_reference_owner(&artifact, function)?;
                        let prepared = functions.get(&function).ok_or_else(|| {
                            execution_diagnostic(
                                "prepare_port_function",
                                format!(
                                    "component port '{}.{}' references absent function '{}'",
                                    owner.diagnostic_name(),
                                    port.name,
                                    function.diagnostic_name()
                                ),
                            )
                        })?;
                        ports.insert(
                            port_id,
                            PreparedPort {
                                name: port.name.clone(),
                                function,
                                signature: prepared.signature.clone(),
                            },
                        );
                    }
                    components.insert(
                        owner.clone(),
                        PreparedComponent {
                            owner,
                            requirements,
                            ports,
                        },
                    );
                }
            }
        }

        let root = artifact.root()?;
        let graph_root = artifact.root_graph()?;
        let mut targets = BTreeMap::new();
        for target in &graph_root.targets {
            let owner = resolve_reference_owner(
                &artifact,
                &DeclarationReference {
                    package: root.descriptor.package_id.clone(),
                    module: target.component_module,
                    declaration: target.component,
                },
            )?;
            let prepared_component = components.get(&owner).ok_or_else(|| {
                execution_diagnostic(
                    "prepare_target_component",
                    format!(
                        "target '{}' references absent component '{}'",
                        target.name,
                        owner.diagnostic_name()
                    ),
                )
            })?;
            let port = prepared_component
                .ports
                .get(&target.port)
                .cloned()
                .ok_or_else(|| {
                    execution_diagnostic(
                        "prepare_target_port",
                        format!(
                            "target '{}' references absent port '{}.{}'",
                            target.name,
                            owner.diagnostic_name(),
                            target.port
                        ),
                    )
                })?;
            targets.insert(
                target.name.clone(),
                PreparedTarget {
                    name: target.name.clone(),
                    runner: target.runner,
                    component: owner,
                    port,
                },
            );
        }
        Ok(Self {
            artifact,
            functions,
            test_expressions,
            components,
            targets,
            tests,
        })
    }

    pub fn artifact(&self) -> &LoadedArtifact {
        &self.artifact
    }

    pub fn functions(&self) -> &BTreeMap<OwnerId, PreparedFunction> {
        &self.functions
    }

    pub fn components(&self) -> &BTreeMap<OwnerId, PreparedComponent> {
        &self.components
    }

    pub fn targets(&self) -> &BTreeMap<String, PreparedTarget> {
        &self.targets
    }

    pub fn tests(&self) -> &[PreparedTest] {
        &self.tests
    }

    pub fn target(&self, name: &str) -> Result<&PreparedTarget, Diagnostic> {
        self.targets.get(name).ok_or_else(|| {
            execution_diagnostic(
                "prepare_target_missing",
                format!("prepared program has no target '{name}'"),
            )
        })
    }

    pub(crate) fn function(&self, owner: &OwnerId) -> Option<&PreparedFunction> {
        self.functions.get(owner)
    }

    pub(crate) fn test_expression(&self, expression: &ExpressionId) -> Option<&PreparedFunction> {
        self.test_expressions.get(expression)
    }

    pub(crate) fn call_intrinsic(
        &self,
        implementation: &str,
        signature: &FunctionSignature,
        arguments: Vec<super::value::Value>,
    ) -> Result<super::value::Value, ExecutionError> {
        intrinsic::call_intrinsic(
            implementation,
            signature,
            arguments,
            &self.artifact.packages,
        )
    }

    pub fn resolve_name(
        &self,
        package_id: &PackageId,
        module_name: &str,
        name: &str,
    ) -> Result<OwnerId, Diagnostic> {
        let package = package_for(&self.artifact, package_id)?;
        let module = package
            .modules
            .iter()
            .find(|module| module.module.name == module_name)
            .ok_or_else(|| {
                execution_diagnostic(
                    "prepare_module_missing",
                    format!("module '{module_name}' is absent"),
                )
            })?;
        resolve_name_owner(&self.artifact, package, module, name)
    }
}

pub(crate) fn resolve_reference_owner(
    artifact: &LoadedArtifact,
    reference: &DeclarationReference,
) -> Result<OwnerId, Diagnostic> {
    let package = artifact.packages.get(&reference.package).ok_or_else(|| {
        execution_diagnostic(
            "prepare_reference_package_missing",
            format!(
                "exact declaration reference names absent package '{}'",
                reference.package.as_str()
            ),
        )
    })?;
    let module = package
        .modules
        .iter()
        .find(|module| module.module_id == reference.module)
        .ok_or_else(|| {
            execution_diagnostic(
                "prepare_reference_module_missing",
                format!(
                    "exact declaration reference names absent module '{}' in package '{}'",
                    reference.module,
                    reference.package.as_str()
                ),
            )
        })?;
    let identity = module
        .declaration_identities
        .iter()
        .zip(&module.module.declarations)
        .find(|(identity, _)| identity.id == reference.declaration)
        .map(|(identity, _)| identity)
        .ok_or_else(|| {
            execution_diagnostic(
                "prepare_reference_declaration_missing",
                format!(
                    "exact declaration reference names absent declaration '{}' in package '{}' module '{}'",
                    reference.declaration,
                    reference.package.as_str(),
                    reference.module
                ),
            )
        })?;
    Ok(OwnerId {
        package: reference.package.clone(),
        module_id: reference.module,
        declaration_id: reference.declaration,
        module: module.module.name.clone(),
        declaration: identity.name.clone(),
    })
}

fn resolve_name_owner(
    artifact: &LoadedArtifact,
    package: &super::semantic::ValidatedPackage,
    module: &super::semantic::ValidatedModule,
    name: &str,
) -> Result<OwnerId, Diagnostic> {
    let Some((alias, declaration)) = name.split_once('.') else {
        let reference =
            declaration_reference_by_name(&package.descriptor.package_id, module, name)?;
        return resolve_reference_owner(artifact, &reference);
    };
    let import = module
        .module
        .imports
        .iter()
        .find(|import| import.alias == alias)
        .ok_or_else(|| {
            execution_diagnostic(
                "prepare_import_alias",
                format!(
                    "module '{}' has no import alias '{alias}'",
                    module.module.name
                ),
            )
        })?;
    let imported_package = if import.target.package == package.descriptor.package_id {
        package
    } else {
        artifact
            .packages
            .get(&import.target.package)
            .ok_or_else(|| {
                execution_diagnostic(
                    "prepare_dependency_missing",
                    format!(
                        "exact imported package '{}' is absent",
                        import.target.package.as_str()
                    ),
                )
            })?
    };
    let imported_module = imported_package
        .modules
        .iter()
        .find(|module| module.module_id == import.target.module)
        .ok_or_else(|| {
            execution_diagnostic(
                "prepare_import_module_missing",
                format!("imported module '{}' is absent", import.target.module),
            )
        })?;
    let reference =
        declaration_reference_by_name(&import.target.package, imported_module, declaration)?;
    resolve_reference_owner(artifact, &reference)
}

fn declaration_reference_by_name(
    package: &PackageId,
    module: &super::semantic::ValidatedModule,
    name: &str,
) -> Result<DeclarationReference, Diagnostic> {
    let declaration = module
        .declaration_identities
        .iter()
        .zip(&module.module.declarations)
        .find(|(identity, _)| identity.name == name)
        .map(|(identity, _)| identity.id)
        .ok_or_else(|| {
            execution_diagnostic(
                "prepare_declaration_missing",
                format!(
                    "module '{}' has no declaration '{name}'",
                    module.module.name
                ),
            )
        })?;
    Ok(DeclarationReference {
        package: package.clone(),
        module: module.module_id,
        declaration,
    })
}

pub(crate) fn package_for<'a>(
    artifact: &'a LoadedArtifact,
    package: &PackageId,
) -> Result<&'a super::semantic::ValidatedPackage, Diagnostic> {
    artifact.packages.get(package).ok_or_else(|| {
        execution_diagnostic(
            "prepare_package_missing",
            format!("package '{}' is absent", package.as_str()),
        )
    })
}

fn execution_diagnostic(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::graph::{GraphRoot, ModuleObjectRef};
    use crate::platform::language::{
        Function, Parameter, TestCase, Type, TypeParameter, unresolved_declaration_reference,
    };
    use crate::platform::meaning::{MeaningModule, MigrationIdentityAllocator};
    use crate::platform::package::PackageId;
    use crate::platform::semantic::canonicalize_graph_package;
    use crate::platform::semantic_id::RepositoryId;
    use crate::platform::syntax::SourceSpan;
    use crate::platform::{
        SourceLimits, Value, build_artifact, decode_package, load_artifact, parse_source,
        validate_package_documents,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;

    fn prepared(source: &[u8]) -> PreparedProgram {
        let descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"execution","modules":[{"name":"main","path":"src/main.lkj"}],"dependencies":[],"targets":[{"name":"run","component":"main.App","port":"main","runner":"command"}]}"#,
        )
        .expect("descriptor");
        let document =
            parse_source("src/main.lkj", source, SourceLimits::default()).expect("parse source");
        let package =
            validate_package_documents(descriptor, vec![document], &[]).expect("validate package");
        let (bytes, _) = build_artifact(&package, &[&package]).expect("build artifact");
        PreparedProgram::prepare(load_artifact(&bytes).expect("load artifact"))
            .expect("prepare program")
    }

    fn graph_span() -> SourceSpan {
        SourceSpan {
            byte_start: 0,
            byte_end: 0,
            line: 1,
            column: 1,
        }
    }

    fn generic_program() -> PreparedProgram {
        let span = graph_span();
        let parameter_type = Type::Parameter("T".to_owned());
        let identity = Declaration::Function(Function {
            name: "identity".to_owned(),
            type_parameters: vec![TypeParameter {
                name: "T".to_owned(),
                span: span.clone(),
            }],
            parameters: vec![Parameter {
                name: "value".to_owned(),
                ty: parameter_type.clone(),
                span: span.clone(),
            }],
            result: parameter_type.clone(),
            effect: super::super::language::Effect::Pure,
            body: Expression::Variable("value".to_owned(), span.clone()),
            span: span.clone(),
        });
        let apply = Declaration::Function(Function {
            name: "apply".to_owned(),
            type_parameters: vec![TypeParameter {
                name: "T".to_owned(),
                span: span.clone(),
            }],
            parameters: vec![
                Parameter {
                    name: "mapper".to_owned(),
                    ty: Type::Function(
                        vec![parameter_type.clone()],
                        Box::new(parameter_type.clone()),
                    ),
                    span: span.clone(),
                },
                Parameter {
                    name: "value".to_owned(),
                    ty: parameter_type.clone(),
                    span: span.clone(),
                },
            ],
            result: parameter_type,
            effect: super::super::language::Effect::Pure,
            body: Expression::Invoke {
                callee: Box::new(Expression::Variable("mapper".to_owned(), span.clone())),
                arguments: vec![Expression::Variable("value".to_owned(), span.clone())],
                span: span.clone(),
            },
            span: span.clone(),
        });
        let text_main = generic_application_function(
            "text-main",
            Type::Text,
            Expression::Text("generic text".to_owned(), span.clone()),
            span.clone(),
        );
        let integer_main = generic_application_function(
            "integer-main",
            Type::I64,
            Expression::I64(41, span.clone()),
            span.clone(),
        );
        let tests = [
            (
                "text-application",
                "text-main",
                Expression::Text("generic text".to_owned(), span.clone()),
            ),
            (
                "integer-application",
                "integer-main",
                Expression::I64(41, span.clone()),
            ),
        ]
        .into_iter()
        .map(|(name, function, expected)| {
            Declaration::Test(TestCase {
                name: name.to_owned(),
                actual: Expression::Call {
                    function: unresolved_declaration_reference(function),
                    type_arguments: Vec::new(),
                    arguments: Vec::new(),
                    span: span.clone(),
                },
                expected,
                span: span.clone(),
            })
        });
        let module = super::super::language::Module {
            name: "main".to_owned(),
            imports: Vec::new(),
            exports: vec![
                unresolved_declaration_reference("identity").declaration,
                unresolved_declaration_reference("apply").declaration,
            ],
            declarations: [identity, apply, text_main, integer_main]
                .into_iter()
                .chain(tests)
                .collect(),
        };
        let mut allocator = MigrationIdentityAllocator::new(b"generic-execution".to_vec());
        let mut meaning = MeaningModule::import(module, &mut allocator).expect("generic meaning");
        let package_id = PackageId::parse("1234567890abcdef1234567890abcdef").expect("package id");
        let identity_reference = fixture_declaration_reference(&meaning, &package_id, "identity");
        let apply_reference = fixture_declaration_reference(&meaning, &package_id, "apply");
        let text_main_reference = fixture_declaration_reference(&meaning, &package_id, "text-main");
        let integer_main_reference =
            fixture_declaration_reference(&meaning, &package_id, "integer-main");
        meaning.module.exports = vec![identity_reference.declaration, apply_reference.declaration];
        for declaration in &mut meaning.module.declarations {
            match declaration {
                Declaration::Function(function)
                    if function.name == "text-main" || function.name == "integer-main" =>
                {
                    let Expression::Call {
                        function,
                        arguments,
                        ..
                    } = &mut function.body
                    else {
                        panic!("generic fixture application body must be a direct call");
                    };
                    *function = apply_reference.clone();
                    let Some(Expression::FunctionRef { function, .. }) = arguments.first_mut()
                    else {
                        panic!("generic fixture application must pass a function reference");
                    };
                    *function = identity_reference.clone();
                }
                Declaration::Test(test) => {
                    let Expression::Call { function, .. } = &mut test.actual else {
                        panic!("generic fixture test actual must be a direct call");
                    };
                    *function = match test.name.as_str() {
                        "text-application" => text_main_reference.clone(),
                        "integer-application" => integer_main_reference.clone(),
                        name => panic!("unexpected generic fixture test '{name}'"),
                    };
                }
                _ => {}
            }
        }
        let mut root = GraphRoot {
            graph_contract_version: super::super::meaning::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(b"generic-execution", 1),
            package_id,
            package_name: "generic-execution".to_owned(),
            modules: vec![ModuleObjectRef {
                id: meaning.module_id,
                name: meaning.module.name.clone(),
                object: meaning.digest().expect("meaning digest"),
            }],
            dependencies: Vec::new(),
            targets: Vec::new(),
            tombstones: Vec::new(),
        };
        let mut meanings = vec![meaning];
        let package = canonicalize_graph_package(&mut root, &mut meanings, &[])
            .expect("generic package validates");
        let encoded = meanings[0].encode().expect("generic meaning encodes");
        assert_eq!(
            MeaningModule::decode(&encoded).expect("generic meaning decodes"),
            meanings[0]
        );
        assert!(meanings[0].relations.iter().any(|relation| matches!(
            relation.target,
            super::super::meaning::RelationTarget::TypeParameter { .. }
        )));
        let (bytes, _) = build_artifact(&package, &[&package]).expect("build generic artifact");
        PreparedProgram::prepare(load_artifact(&bytes).expect("load generic artifact"))
            .expect("prepare generic program")
    }

    fn generic_application_function(
        name: &str,
        ty: Type,
        value: Expression,
        span: SourceSpan,
    ) -> Declaration {
        Declaration::Function(Function {
            name: name.to_owned(),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            result: ty.clone(),
            effect: super::super::language::Effect::Pure,
            body: Expression::Call {
                function: unresolved_declaration_reference("apply"),
                type_arguments: vec![ty.clone()],
                arguments: vec![
                    Expression::FunctionRef {
                        function: unresolved_declaration_reference("identity"),
                        type_arguments: vec![ty],
                        span: span.clone(),
                    },
                    value,
                ],
                span: span.clone(),
            },
            span,
        })
    }

    fn fixture_declaration_reference(
        meaning: &MeaningModule,
        package: &PackageId,
        name: &str,
    ) -> DeclarationReference {
        let declaration = meaning
            .declarations
            .iter()
            .find(|identity| identity.name == name)
            .map(|identity| identity.id)
            .unwrap_or_else(|| panic!("generic fixture declaration '{name}' is absent"));
        DeclarationReference {
            package: package.clone(),
            module: meaning.module_id,
            declaration,
        }
    }

    #[test]
    fn explicit_generics_and_named_higher_order_calls_match_both_execution_tiers() {
        let program = generic_program();
        let package = program.artifact().root().expect("root package");
        for (name, expected) in [
            ("text-main", Value::text("generic text")),
            ("integer-main", Value::I64(41)),
        ] {
            let owner = package.modules[0]
                .owner(&package.descriptor.package_id, name)
                .expect("generic consumer owner");
            let (actual, _) = Vm::new(&program, RunPolicy::default())
                .invoke(&owner, Vec::new())
                .expect("bytecode generic execution");
            let (reference, _) = ReferenceInterpreter::new(&program, RunPolicy::default())
                .invoke(&owner, Vec::new())
                .expect("reference generic execution");
            assert_eq!(actual.canonical_json(), expected.canonical_json());
            assert_eq!(actual.canonical_json(), reference.canonical_json());
        }
        assert_eq!(program.tests().len(), 2);
    }

    #[test]
    fn exact_constant_references_remain_distinct_from_lexical_variables() {
        let program = prepared(
            br#"(module main
  (export App)
  (const answer I64 42)
  (fn global () I64 answer)
  (fn local ((answer I64)) I64 answer)
  (component App (port main (Function () I64) (function global))))
"#,
        );
        let target = program.target("run").expect("target");
        let package = program.artifact().root().expect("root package");
        let local = package.modules[0]
            .owner(&package.descriptor.package_id, "local")
            .expect("local owner");
        for interpreter in ["bytecode", "reference"] {
            let global = match interpreter {
                "bytecode" => Vm::new(&program, RunPolicy::default())
                    .invoke(&target.port.function, Vec::new())
                    .expect("bytecode exact constant"),
                "reference" => ReferenceInterpreter::new(&program, RunPolicy::default())
                    .invoke(&target.port.function, Vec::new())
                    .expect("reference exact constant"),
                _ => unreachable!(),
            };
            assert_eq!(global.0.canonical_json(), Value::I64(42).canonical_json());

            let lexical = match interpreter {
                "bytecode" => Vm::new(&program, RunPolicy::default())
                    .invoke(&local, vec![Value::I64(7)])
                    .expect("bytecode lexical variable"),
                "reference" => ReferenceInterpreter::new(&program, RunPolicy::default())
                    .invoke(&local, vec![Value::I64(7)])
                    .expect("reference lexical variable"),
                _ => unreachable!(),
            };
            assert_eq!(lexical.0.canonical_json(), Value::I64(7).canonical_json());
        }
    }

    #[test]
    fn bytecode_executes_deep_calls_without_native_recursion() {
        let program = prepared(
            br#"(module main
  (export App)
  (extern add ((left I64) (right I64)) I64 core.i64.add)
  (extern subtract ((left I64) (right I64)) I64 core.i64.subtract)
  (extern less ((left I64) (right I64)) Bool core.i64.less)
  (fn sum ((remaining I64) (total I64)) I64
    (if (call less remaining 1)
        total
        (call sum (call subtract remaining 1) (call add total remaining))))
  (fn main ((value I64)) I64 (call sum value 0))
  (component App (port main (Function (I64) I64) (function main))))
"#,
        );
        let target = program.target("run").expect("target");
        let (value, observation) = Vm::new(
            &program,
            RunPolicy {
                maximum_call_depth: 2_000,
                ..RunPolicy::default()
            },
        )
        .invoke(&target.port.function, vec![Value::I64(1_000)])
        .expect("execute");
        assert!(matches!(value, Value::I64(500_500)));
        assert!(observation.maximum_call_depth > 1_000);
        assert_eq!(observation.production_tier, "bytecode_v1");
        let (reference, reference_observation) = ReferenceInterpreter::new(
            &program,
            RunPolicy {
                maximum_call_depth: 2_000,
                ..RunPolicy::default()
            },
        )
        .invoke(&target.port.function, vec![Value::I64(1_000)])
        .expect("reference execution");
        assert_eq!(value.canonical_json(), reference.canonical_json());
        assert_eq!(reference_observation.production_tier, "reference_ast_v1");
    }

    #[test]
    fn bytecode_preserves_lazy_branches_and_checked_arithmetic() {
        let program = prepared(
            br#"(module main
  (export App)
  (extern add ((left I64) (right I64)) I64 core.i64.add)
  (fn main ((condition Bool)) I64 (if condition 7 (call add 9223372036854775807 1)))
  (component App (port main (Function (Bool) I64) (function main))))
"#,
        );
        let target = program.target("run").expect("target");
        let vm = Vm::new(&program, RunPolicy::default());
        let (value, _) = vm
            .invoke(&target.port.function, vec![Value::Bool(true)])
            .expect("lazy branch");
        assert!(matches!(value, Value::I64(7)));
        let error = vm
            .invoke(&target.port.function, vec![Value::Bool(false)])
            .expect_err("overflow traps");
        assert_eq!(error.class, ExecutionFailureClass::Trap);
        assert_eq!(error.code, "integer_overflow");
        let reference_error = ReferenceInterpreter::new(&program, RunPolicy::default())
            .invoke(&target.port.function, vec![Value::Bool(false)])
            .expect_err("reference overflow traps");
        assert_eq!(error.class, reference_error.class);
        assert_eq!(error.code, reference_error.code);
    }

    #[test]
    fn unknown_or_forged_intrinsic_rejects_before_execution() {
        let descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"execution","modules":[{"name":"main","path":"src/main.lkj"}],"dependencies":[],"targets":[]}"#,
        )
        .expect("descriptor");
        let document = parse_source(
            "src/main.lkj",
            b"(module main (extern forged ((value Text)) Text core.i64.add))\n",
            SourceLimits::default(),
        )
        .expect("source");
        let error = validate_package_documents(descriptor, vec![document], &[])
            .expect_err("forged intrinsic must reject before artifact construction");
        assert_eq!(error.code, "intrinsic_signature");
    }

    #[test]
    fn effects_require_exact_bounded_grants_before_work() {
        let program = prepared(
            br#"(module main
  (export App)
  (interface Clock
    (operation now () I64 idempotent no-visibility))
  (task main () I64 (requires (clock Clock))
    (do (perform clock now) (perform clock now)))
  (component App
    (require clock Clock (operations now) (limit maximum_calls 1) (limit maximum_output_bytes 8))
    (port main (Function () I64) (function main))))
"#,
        );
        let target = program.target("run").expect("target");
        let component = program
            .components()
            .get(&target.component)
            .expect("component");
        let requirement = component.requirements.get("clock").expect("requirement");
        let adapter = Arc::new(ScriptedAdapter::new(
            requirement.interface.clone(),
            vec![
                ScriptedCall {
                    operation: "now".to_owned(),
                    result: Ok(Value::I64(10)),
                },
                ScriptedCall {
                    operation: "now".to_owned(),
                    result: Ok(Value::I64(11)),
                },
            ],
        ));
        let grant = CapabilityGrant {
            requirement: "clock".to_owned(),
            descriptor: CapabilityGrantDescriptor {
                contract_version: CAPABILITY_GRANT_CONTRACT_VERSION,
                interface: requirement.interface.clone(),
                adapter_kind: "deterministic-fake".to_owned(),
                sharing_domain: "test".to_owned(),
                authority_revision: "1".repeat(64),
                descriptor_digest: "2".repeat(64),
                operations: BTreeSet::from(["now".to_owned()]),
                limits: BTreeMap::from([
                    ("maximum_calls".to_owned(), 1),
                    ("maximum_output_bytes".to_owned(), 8),
                ]),
            },
            adapter: adapter.clone(),
        };
        let bound = BoundCapabilities::bind(component, vec![grant]).expect("bind grant");
        let error = Vm::new(&program, RunPolicy::default())
            .invoke_with_capabilities(&target.port.function, Vec::new(), &bound)
            .expect_err("second effect exceeds task grant");
        assert_eq!(error.class, ExecutionFailureClass::Resource);
        assert_eq!(error.code, "capability_call_limit");
        assert_eq!(adapter.observed(), vec!["now"]);
        assert_eq!(adapter.remaining(), 1);

        let missing = BoundCapabilities::bind(component, Vec::new())
            .expect_err("missing grant rejects before execution");
        assert_eq!(missing.code, "grant_requirement_missing");
    }

    #[test]
    fn transaction_scope_commits_once_and_rolls_back_on_failure() {
        let program = prepared(
            br#"(module main
  (export App)
  (interface Database
    (operation transaction () Unit idempotent-with-key possible-visibility)
    (operation write ((value I64)) I64 idempotent-with-key possible-visibility))
  (task main ((value I64)) I64 (requires (database Database))
    (transaction database transaction-scope
      (perform transaction-scope write value)))
  (component App
    (require database Database (operations transaction write) (limit maximum_calls 4) (limit maximum_input_bytes 32) (limit maximum_output_bytes 32))
    (port main (Function (I64) I64) (function main))))
"#,
        );
        let target = program.target("run").expect("target");
        let component = program
            .components()
            .get(&target.component)
            .expect("component");
        let requirement = component.requirements.get("database").expect("requirement");

        let grant_for = |adapter: Arc<ScriptedAdapter>| CapabilityGrant {
            requirement: "database".to_owned(),
            descriptor: CapabilityGrantDescriptor {
                contract_version: CAPABILITY_GRANT_CONTRACT_VERSION,
                interface: requirement.interface.clone(),
                adapter_kind: "deterministic-fake".to_owned(),
                sharing_domain: "test".to_owned(),
                authority_revision: "3".repeat(64),
                descriptor_digest: "4".repeat(64),
                operations: BTreeSet::from(["transaction".to_owned(), "write".to_owned()]),
                limits: BTreeMap::from([
                    ("maximum_calls".to_owned(), 4),
                    ("maximum_input_bytes".to_owned(), 32),
                    ("maximum_output_bytes".to_owned(), 32),
                ]),
            },
            adapter,
        };

        let success_adapter = Arc::new(ScriptedAdapter::with_transactions(
            requirement.interface.clone(),
            Vec::new(),
            vec![vec![ScriptedCall {
                operation: "write".to_owned(),
                result: Ok(Value::I64(9)),
            }]],
        ));
        let success = BoundCapabilities::bind(component, vec![grant_for(success_adapter.clone())])
            .expect("bind success grant");
        let (value, _) = Vm::new(&program, RunPolicy::default())
            .invoke_with_capabilities(&target.port.function, vec![Value::I64(9)], &success)
            .expect("transaction success");
        assert!(matches!(value, Value::I64(9)));
        assert_eq!(
            success_adapter.observed(),
            vec![
                "transaction.begin",
                "transaction.write",
                "transaction.commit"
            ]
        );

        let failure_adapter = Arc::new(ScriptedAdapter::with_transactions(
            requirement.interface.clone(),
            Vec::new(),
            vec![vec![ScriptedCall {
                operation: "write".to_owned(),
                result: Err(ExecutionError::new(
                    ExecutionFailureClass::Capability,
                    "database_constraint",
                    "deterministic constraint failure",
                )),
            }]],
        ));
        let failure = BoundCapabilities::bind(component, vec![grant_for(failure_adapter.clone())])
            .expect("bind failure grant");
        let error = Vm::new(&program, RunPolicy::default())
            .invoke_with_capabilities(&target.port.function, vec![Value::I64(9)], &failure)
            .expect_err("transaction call fails");
        assert_eq!(error.code, "database_constraint");
        assert_eq!(
            failure_adapter.observed(),
            vec![
                "transaction.begin",
                "transaction.write",
                "transaction.rollback"
            ]
        );
    }
}
