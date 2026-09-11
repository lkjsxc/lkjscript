//! Independent exact-ID expression type and effect oracle for Graph 13.

use super::contract::{MAXIMUM_EXPRESSION_DEPTH, MAXIMUM_TYPE_DEPTH, MAXIMUM_VALIDATION_WORK};
use super::digest::TypeObjectDigest;
use super::expression::{ExpressionOperation, FieldSelector, LocalValueReference};
use super::id::{OwnerKey, OwnerKind, PackageId};
use super::owner::{
    BindingKind, CaseRecord, DeclarationPayload, FieldRecord, FunctionEffect,
    HttpRoutePatternSegment, OperationRecord, OwnerRecord, ParameterParent, ParameterRecord,
    ParameterUse, PortImplementation, RequirementRecord,
};
use super::reference::{DeclarationReference, RequirementReference};
use super::type_object::{StructuralTypeField, TypeForm, TypeObject};
use super::validate::KernelSnapshot;
use super::{PackageInterfaceDeclarationPayload, PackageInterfaceRecord, TypeObjectInterner};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::package::RunnerKind;
use crate::platform::semantic_id::{
    BindingId, CaseId, DeclarationId, ExpressionId, FieldId, OperationId, TypeParameterId,
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "nominal_flow.rs"]
mod nominal_flow;

#[cfg(test)]
#[path = "nominal_flow_tests.rs"]
mod nominal_flow_tests;

#[derive(Clone, Debug)]
struct ExecutionContext {
    declaration: Option<DeclarationId>,
    pure: bool,
    requirements: BTreeSet<RequirementReference>,
    allow_task_function_value: bool,
}

#[derive(Clone, Debug)]
struct FunctionSignature {
    parameters: Vec<TypeObjectDigest>,
    result: TypeObjectDigest,
    requirements: BTreeSet<RequirementReference>,
    task: bool,
}

fn nominal_parts(form: &TypeForm) -> Option<(DeclarationReference, &[TypeObjectDigest])> {
    match form {
        TypeForm::Named { declaration } => Some((*declaration, &[])),
        TypeForm::Applied {
            declaration,
            arguments,
        } => Some((*declaration, arguments)),
        _ => None,
    }
}

/// Exact point-read surface required by declaration-local expression validation.
pub(crate) trait ExpressionRead {
    fn package_id(&self) -> PackageId;

    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic>;

    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic>;

    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic>;

    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic>;

    /// Owning operations may interrupt long in-memory analysis even when every read is cached.
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        Ok(())
    }
}

/// Request-local deterministic admissions owned by expression validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ExpressionValidationLimits {
    /// Maximum inference and substitution steps.
    pub maximum_steps: usize,
    /// Maximum semantic diagnostics inserted into the caller's bounded sink.
    pub maximum_diagnostics: usize,
}

/// Exact request-local admission exhausted before its next unit could be consumed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExpressionValidationExhaustion {
    Steps,
    Diagnostics,
}

impl ExpressionRead for KernelSnapshot {
    fn package_id(&self) -> PackageId {
        self.root.package_id
    }

    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        Ok(self.owners.get(&owner).cloned())
    }

    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        Ok(self
            .types
            .get(&digest)
            .or_else(|| self.dependency_types.get(&digest))
            .cloned())
    }

    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        let Some(dependency) = self.dependencies.get(&package) else {
            return Ok(None);
        };
        Ok(self
            .dependency_interfaces
            .get(&dependency.package_revision)
            .and_then(|owners| owners.get(&owner))
            .cloned())
    }

    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        Ok(self.dependencies.contains_key(&package))
    }
}

pub(super) fn validate_expression_meaning(
    snapshot: &KernelSnapshot,
    diagnostics: &mut Vec<Diagnostic>,
    work: &mut usize,
    maximum_steps: usize,
) {
    let roots = snapshot.owners.keys().copied().collect::<Vec<_>>();
    if validate_expression_roots_with_limits(
        snapshot,
        roots,
        diagnostics,
        work,
        ExpressionValidationLimits {
            maximum_steps,
            maximum_diagnostics: usize::MAX,
        },
    ) == Err(ExpressionValidationExhaustion::Steps)
    {
        diagnostics.push(type_error(
            "kernel_type_work",
            "expression type validation exhausted its explicit work budget",
        ));
    }
}

pub(crate) fn validate_expression_roots<R: ExpressionRead>(
    read: &R,
    roots: impl IntoIterator<Item = OwnerKey>,
    diagnostics: &mut Vec<Diagnostic>,
    work: &mut usize,
) {
    if validate_expression_roots_with_limits(
        read,
        roots,
        diagnostics,
        work,
        ExpressionValidationLimits {
            maximum_steps: MAXIMUM_VALIDATION_WORK,
            maximum_diagnostics: usize::MAX,
        },
    ) == Err(ExpressionValidationExhaustion::Steps)
    {
        diagnostics.push(type_error(
            "kernel_type_work",
            "expression type validation exhausted its explicit work budget",
        ));
    }
}

pub(crate) fn validate_expression_roots_with_limits<R: ExpressionRead>(
    read: &R,
    roots: impl IntoIterator<Item = OwnerKey>,
    diagnostics: &mut Vec<Diagnostic>,
    work: &mut usize,
    limits: ExpressionValidationLimits,
) -> Result<(), ExpressionValidationExhaustion> {
    let mut validator = ExpressionValidator {
        read,
        diagnostics,
        work,
        limits,
        exhaustion: None,
        ephemeral_types: BTreeMap::new(),
        admitted_nominals: BTreeSet::new(),
        type_metadata_bytes: 0,
    };
    validator.validate_roots(roots);
    validator.exhaustion.map_or(Ok(()), Err)
}

/// Infers one exact expression type in an existing function context without creating a second
/// graph authority. Callers supply the same bounded read surface used by normal validation.
pub(crate) fn infer_function_expression_type<R: ExpressionRead>(
    read: &R,
    declaration: DeclarationId,
    expression: ExpressionId,
    effect: &FunctionEffect,
    work: &mut usize,
    maximum_steps: usize,
) -> Result<TypeObjectDigest, Diagnostic> {
    let (pure, requirements) = match effect {
        FunctionEffect::Pure => (true, BTreeSet::new()),
        FunctionEffect::Task { requirements } => {
            (false, requirements.iter().copied().collect::<BTreeSet<_>>())
        }
    };
    let mut diagnostics = Vec::new();
    let mut validator = ExpressionValidator {
        read,
        diagnostics: &mut diagnostics,
        work,
        limits: ExpressionValidationLimits {
            maximum_steps,
            maximum_diagnostics: 1,
        },
        exhaustion: None,
        ephemeral_types: BTreeMap::new(),
        admitted_nominals: BTreeSet::new(),
        type_metadata_bytes: 0,
    };
    validator.infer(
        expression,
        &ExecutionContext {
            declaration: Some(declaration),
            pure,
            requirements,
            allow_task_function_value: false,
        },
        0,
    )
}

struct ExpressionValidator<'a, 'b, R> {
    read: &'a R,
    diagnostics: &'b mut Vec<Diagnostic>,
    work: &'b mut usize,
    limits: ExpressionValidationLimits,
    exhaustion: Option<ExpressionValidationExhaustion>,
    ephemeral_types: BTreeMap<TypeObjectDigest, TypeObject>,
    admitted_nominals: BTreeSet<DeclarationReference>,
    type_metadata_bytes: usize,
}

impl<R: ExpressionRead> ExpressionValidator<'_, '_, R> {
    fn validate_roots(&mut self, roots: impl IntoIterator<Item = OwnerKey>) {
        for owner in roots {
            if self.exhausted() {
                return;
            }
            let owner = match self.read.owner(owner) {
                Ok(Some(owner)) => owner,
                Ok(None) => {
                    self.error(
                        "kernel_type_frontier_owner_missing",
                        "expression-validation frontier names a missing owner",
                    );
                    continue;
                }
                Err(diagnostic) => {
                    self.push_diagnostic(diagnostic);
                    continue;
                }
            };
            if let Err(diagnostic) = self.validate_owner_nominals(&owner) {
                self.push_diagnostic(diagnostic);
                continue;
            }
            match owner {
                OwnerRecord::Declaration(declaration) => match declaration.payload {
                    DeclarationPayload::Function(function) => {
                        let (pure, requirements) = match function.effect {
                            FunctionEffect::Pure => (true, BTreeSet::new()),
                            FunctionEffect::Task { requirements } => {
                                for requirement in &requirements {
                                    if let Err(diagnostic) = self.requirement_record(*requirement) {
                                        self.push_diagnostic(diagnostic);
                                        if self.exhausted() {
                                            return;
                                        }
                                    }
                                }
                                (false, requirements.into_iter().collect())
                            }
                        };
                        let context = ExecutionContext {
                            declaration: match declaration.header.owner {
                                OwnerKey::Declaration(id) => Some(id),
                                _ => None,
                            },
                            pure,
                            requirements,
                            allow_task_function_value: false,
                        };
                        self.compare_root_type(
                            function.body,
                            function.result,
                            &context,
                            "function",
                        );
                    }
                    DeclarationPayload::Constant { ty, value } => {
                        self.compare_root_type(value, ty, &pure_context(None), "constant");
                    }
                    DeclarationPayload::Test {
                        actual, expected, ..
                    } => {
                        let context = pure_context(None);
                        let actual_type = self.infer(actual, &context, 0);
                        let expected_type = self.infer(expected, &context, 0);
                        match (actual_type, expected_type) {
                            (Ok(actual_type), Ok(expected_type))
                                if actual_type != expected_type =>
                            {
                                self.error(
                                    "kernel_type_test",
                                    "test actual and expected expressions have different exact types",
                                );
                            }
                            (Err(diagnostic), Ok(_)) | (Ok(_), Err(diagnostic)) => {
                                self.push_diagnostic(diagnostic);
                            }
                            (Err(actual), Err(expected)) => {
                                self.push_diagnostic(actual);
                                self.push_diagnostic(expected);
                            }
                            (Ok(ty), Ok(_)) => {
                                if let Err(error) = self.validate_application_equality(ty) {
                                    self.push_diagnostic(error);
                                }
                            }
                        }
                    }
                    _ => {}
                },
                OwnerRecord::Port(port) => {
                    let requirements = match self.component_requirements(port.declaration) {
                        Ok(requirements) => requirements,
                        Err(diagnostic) => {
                            self.push_diagnostic(diagnostic);
                            continue;
                        }
                    };
                    let context = ExecutionContext {
                        declaration: None,
                        pure: false,
                        requirements,
                        allow_task_function_value: true,
                    };
                    match port.implementation {
                        PortImplementation::Expression(expression) => self.compare_root_type(
                            expression,
                            port.function_type,
                            &context,
                            "port expression",
                        ),
                        PortImplementation::Function(function) => {
                            match self.function_signature(function, &[], &context) {
                                Ok(signature) => {
                                    if let Err(diagnostic) =
                                        self.validate_call_effect(&signature, &context)
                                    {
                                        self.push_diagnostic(diagnostic);
                                    }
                                    match self.function_type(&signature) {
                                        Ok(actual) if actual == port.function_type => {}
                                        Ok(_) => self.error(
                                            "kernel_type_port_function",
                                            "port function type disagrees with its exact declaration",
                                        ),
                                        Err(diagnostic) => self.push_diagnostic(diagnostic),
                                    }
                                }
                                Err(diagnostic) => self.push_diagnostic(diagnostic),
                            }
                        }
                    }
                }
                OwnerRecord::Target(target) => {
                    let Some(target_port) = target.port else {
                        continue;
                    };
                    if target.component.package == self.read.package_id()
                        && target_port.package == self.read.package_id()
                    {
                        match self
                            .read
                            .owner(OwnerKey::Declaration(target.component.declaration))
                        {
                            Ok(Some(OwnerRecord::Declaration(component))) => {
                                if !matches!(
                                    component.payload,
                                    DeclarationPayload::Component { ref ports, .. }
                                        if ports.contains(&target_port.port)
                                ) {
                                    self.error(
                                        "kernel_full_target_port_owner",
                                        "target port does not belong to its component",
                                    );
                                }
                            }
                            Ok(_) => {}
                            Err(diagnostic) => self.push_diagnostic(diagnostic),
                        }
                    }
                    if target_port.package != self.read.package_id() {
                        continue;
                    }
                    let port = match self.read.owner(OwnerKey::Port(target_port.port)) {
                        Ok(Some(OwnerRecord::Port(port))) => port,
                        Ok(_) => continue,
                        Err(diagnostic) => {
                            self.push_diagnostic(diagnostic);
                            continue;
                        }
                    };
                    let http_type = match crate::platform::http::semantic_http_types(
                        &mut TypeObjectInterner::default(),
                    ) {
                        Ok(types) => types.function_type,
                        Err(diagnostic) => {
                            self.push_diagnostic(diagnostic);
                            continue;
                        }
                    };
                    match target.runner {
                        RunnerKind::Interactive => {
                            let standard =
                                crate::platform::builtin_standard::BuiltinStandard::load()
                                    .and_then(|standard| standard.session_contract());
                            match standard.and_then(|standard| {
                                crate::platform::session::validate_session_function_type(
                                    &crate::platform::session::ExpressionSessionRead::new(
                                        self.read,
                                    ),
                                    standard,
                                    port.function_type,
                                )
                            }) {
                                Ok(_) => {}
                                Err(diagnostic) => self.push_diagnostic(diagnostic),
                            }
                        }
                        RunnerKind::Command if port.function_type == http_type => self.error(
                            "kernel_type_target_command_runner",
                            "command target cannot select the semantic HTTP port shape",
                        ),
                        _ => {}
                    }
                }
                OwnerRecord::HttpRoute(route) => {
                    if route.port.package != self.read.package_id() {
                        continue;
                    }
                    let port = match self.read.owner(OwnerKey::Port(route.port.port)) {
                        Ok(Some(OwnerRecord::Port(port))) => port,
                        Ok(_) => continue,
                        Err(diagnostic) => {
                            self.push_diagnostic(diagnostic);
                            continue;
                        }
                    };
                    let mut types = TypeObjectInterner::default();
                    let capture_count = route.selector.capture_count();
                    let http_type = match crate::platform::http::semantic_http_route_function_type(
                        &mut types,
                        capture_count,
                    ) {
                        Ok(function_type) => function_type,
                        Err(diagnostic) => {
                            self.push_diagnostic(diagnostic);
                            continue;
                        }
                    };
                    if port.function_type != http_type {
                        self.error(
                            "kernel_type_http_route_port",
                            "HTTP route requires the exact semantic HTTP function-backed port shape",
                        );
                    }
                    let PortImplementation::Function(function) = port.implementation else {
                        continue;
                    };
                    let parameters = match self.function_parameter_records(function) {
                        Ok(parameters) => parameters,
                        Err(diagnostic) => {
                            self.push_diagnostic(diagnostic);
                            continue;
                        }
                    };
                    let captures = route.selector.capture_names();
                    if parameters.len() != captures.len().saturating_add(1) {
                        self.error(
                            "kernel_type_http_route_parameters",
                            "HTTP route backing function parameter count disagrees with its selector",
                        );
                        continue;
                    }
                    let text_type = match crate::platform::http::semantic_http_types(&mut types) {
                        Ok(types) => types.text_type,
                        Err(diagnostic) => {
                            self.push_diagnostic(diagnostic);
                            continue;
                        }
                    };
                    for (parameter, capture) in parameters.iter().skip(1).zip(captures) {
                        if parameter.name.as_str() != capture.as_str()
                            || parameter.ty != text_type
                            || parameter.use_mode != ParameterUse::Unrestricted
                            || parameter.resource_requirement.is_some()
                        {
                            self.error(
                                "kernel_type_http_route_capture_parameter",
                                "HTTP route capture must index one same-named unrestricted Text parameter without a resource binding",
                            );
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn compare_root_type(
        &mut self,
        root: ExpressionId,
        expected: TypeObjectDigest,
        context: &ExecutionContext,
        label: &str,
    ) {
        match self.infer(root, context, 0) {
            Ok(actual) if actual == expected => {}
            Ok(actual) => self.error(
                "kernel_type_root",
                format!("{label} expects type {expected} but its root has type {actual}"),
            ),
            Err(diagnostic) => self.push_diagnostic(diagnostic),
        }
    }

    fn infer(
        &mut self,
        expression: ExpressionId,
        context: &ExecutionContext,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        self.consume_work()?;
        if depth > MAXIMUM_EXPRESSION_DEPTH {
            return Err(type_error(
                "kernel_type_expression_depth",
                "expression inference exceeded its structural depth bound",
            ));
        }
        let record = match self.read.owner(OwnerKey::Expression(expression))? {
            Some(OwnerRecord::Expression(record)) => record,
            _ => {
                return Err(type_error(
                    "kernel_type_expression_missing",
                    format!("expression {expression} is missing"),
                ));
            }
        };
        let next = depth.saturating_add(1);
        for ty in record.type_roots() {
            self.validate_nominal_type(ty, context, 0)?;
        }
        match record.operation {
            ExpressionOperation::Unit {} => self.canonical_type(TypeForm::Unit),
            ExpressionOperation::Bool { .. } => self.canonical_type(TypeForm::Bool),
            ExpressionOperation::I64 { .. } => self.canonical_type(TypeForm::I64),
            ExpressionOperation::Text { .. } => self.canonical_type(TypeForm::Text),
            ExpressionOperation::StaticText { .. } => self.canonical_type(TypeForm::StaticText),
            ExpressionOperation::Local { value } => self.local_type(value, context, next),
            ExpressionOperation::Constant { declaration } => self.constant_type(declaration),
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                let bool_type = self.canonical_type(TypeForm::Bool)?;
                let condition = self.infer(condition, context, next)?;
                require_same(
                    bool_type,
                    condition,
                    "kernel_type_if_condition",
                    "if condition",
                )?;
                let when_true = self.infer(when_true, context, next)?;
                let when_false = self.infer(when_false, context, next)?;
                require_same(
                    when_true,
                    when_false,
                    "kernel_type_if_branches",
                    "if branches",
                )?;
                Ok(when_true)
            }
            ExpressionOperation::Let { bindings, body } => {
                for binding in bindings {
                    let binding_record = match self.read.owner(OwnerKey::Binding(binding))? {
                        Some(OwnerRecord::Binding(record)) => record,
                        _ => {
                            return Err(type_error(
                                "kernel_type_binding_missing",
                                format!("let binding {binding} is missing"),
                            ));
                        }
                    };
                    if binding_record.kind != BindingKind::Let {
                        return Err(type_error(
                            "kernel_type_binding_kind",
                            "let expression references a non-let binding",
                        ));
                    }
                    let value = binding_record.value.ok_or_else(|| {
                        type_error("kernel_type_binding_value", "let binding has no value")
                    })?;
                    let actual = self.infer(value, context, next)?;
                    if let Some(expected) = binding_record.declared_type {
                        require_same(expected, actual, "kernel_type_binding", "let binding value")?;
                    }
                }
                self.infer(body, context, next)
            }
            ExpressionOperation::Sequence { items } => {
                let mut result = None;
                for item in items {
                    result = Some(self.infer(item, context, next)?);
                }
                result.ok_or_else(|| {
                    type_error(
                        "kernel_type_sequence_empty",
                        "sequence has no result expression",
                    )
                })
            }
            ExpressionOperation::Call {
                function,
                type_arguments,
                arguments,
            } => {
                let signature = self.function_signature(function, &type_arguments, context)?;
                self.validate_call_effect(&signature, context)?;
                let mut argument_context = context.clone();
                argument_context.allow_task_function_value = false;
                self.validate_arguments(
                    &arguments,
                    &signature.parameters,
                    &argument_context,
                    next,
                )?;
                Ok(signature.result)
            }
            ExpressionOperation::FunctionValue {
                function,
                type_arguments,
            } => {
                let signature = self.function_signature(function, &type_arguments, context)?;
                if signature.task && !context.allow_task_function_value {
                    return Err(type_error(
                        "kernel_type_task_function_value",
                        "task function value is unavailable in this expression context",
                    ));
                }
                self.function_type(&signature)
            }
            ExpressionOperation::Bind { callee, arguments } => {
                let mut pure_callable_context = context.clone();
                pure_callable_context.allow_task_function_value = false;
                let callee_type = self.infer(callee, &pure_callable_context, next)?;
                let TypeForm::Function { parameters, result } = self.type_object(callee_type)?.form
                else {
                    return Err(type_error(
                        "kernel_type_bind",
                        "bind callee must be a pure function value",
                    ));
                };
                if arguments.len() > parameters.len() {
                    return Err(type_error(
                        "kernel_type_bind_arity",
                        "bound prefix exceeds the callee's remaining parameter count",
                    ));
                }
                for parameter in parameters.iter().take(arguments.len()) {
                    self.require_capture_safe(*parameter, context)?;
                }
                self.validate_arguments(
                    &arguments,
                    &parameters[..arguments.len()],
                    &pure_callable_context,
                    next,
                )?;
                self.canonical_type(TypeForm::Function {
                    parameters: parameters[arguments.len()..].to_vec(),
                    result,
                })
            }
            ExpressionOperation::Invoke { callee, arguments } => {
                let mut callable_context = context.clone();
                callable_context.allow_task_function_value = false;
                let callee_type = self.infer(callee, &callable_context, next)?;
                let object = self.type_object(callee_type)?;
                let TypeForm::Function { parameters, result } = object.form else {
                    return Err(type_error(
                        "kernel_type_invoke",
                        "invoke callee is not a function value",
                    ));
                };
                self.validate_arguments(&arguments, &parameters, &callable_context, next)?;
                Ok(result)
            }
            ExpressionOperation::Record {
                nominal_type,
                type_arguments,
                fields,
            } => self.infer_record(nominal_type, &type_arguments, &fields, context, next),
            ExpressionOperation::Variant {
                case,
                type_arguments,
                payload,
            } => {
                let case_record = self.case_record(case.package, case.case)?;
                let declaration = DeclarationReference {
                    package: case.package,
                    declaration: case_record.declaration,
                };
                let bindings = self.nominal_bindings(declaration, &type_arguments)?;
                let ty = self.nominal_type(declaration, &type_arguments)?;
                self.validate_nominal_type(ty, context, 0)?;
                let expected = case_record
                    .payload
                    .map(|ty| self.substitute(ty, &bindings, 0))
                    .transpose()?;
                self.validate_optional_payload(payload, expected, context, next, "variant")?;
                Ok(ty)
            }
            ExpressionOperation::Field { value, selector } => {
                let value_type = self.infer(value, context, next)?;
                self.infer_field(value_type, selector)
            }
            ExpressionOperation::List { item_type, items } => {
                for item in items {
                    let actual = self.infer(item, context, next)?;
                    require_same(item_type, actual, "kernel_type_list_item", "list item")?;
                }
                self.canonical_type(TypeForm::List { item: item_type })
            }
            ExpressionOperation::Map {
                key_type,
                value_type,
                entries,
            } => {
                let key_object = self.type_object(key_type)?;
                if !matches!(
                    key_object.form,
                    TypeForm::Bool
                        | TypeForm::I64
                        | TypeForm::Bytes
                        | TypeForm::Text
                        | TypeForm::StaticText
                ) {
                    return Err(type_error(
                        "kernel_type_map_key_order",
                        "map key type lacks a closed deterministic primitive ordering",
                    ));
                }
                for entry in entries {
                    let key = self.infer(entry.key, context, next)?;
                    let value = self.infer(entry.value, context, next)?;
                    require_same(key_type, key, "kernel_type_map_key", "map key")?;
                    require_same(value_type, value, "kernel_type_map_value", "map value")?;
                }
                self.canonical_type(TypeForm::Map {
                    key: key_type,
                    value: value_type,
                })
            }
            ExpressionOperation::Match { value, arms } => {
                self.infer_match(value, &arms, context, next)
            }
            ExpressionOperation::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => {
                if context.pure {
                    return Err(type_error(
                        "kernel_type_pure_capability",
                        "pure expression performs a capability operation",
                    ));
                }
                if !context.requirements.contains(&requirement) {
                    return Err(type_error(
                        "kernel_type_capability_missing",
                        "capability requirement is unavailable in this task context",
                    ));
                }
                let requirement_record = self.requirement_record(requirement)?;
                if requirement_record.interface.package != operation.package
                    || !requirement_record.operations.contains(&operation)
                {
                    return Err(type_error(
                        "kernel_type_capability_operation",
                        "capability operation is not admitted by the exact requirement",
                    ));
                }
                let operation_record =
                    self.operation_record(operation.package, operation.operation)?;
                if operation_record.declaration != requirement_record.interface.declaration {
                    return Err(type_error(
                        "kernel_type_capability_operation_owner",
                        "capability operation does not belong to the requirement's exact interface",
                    ));
                }
                let parameters =
                    self.parameter_types(operation.package, &operation_record.parameters)?;
                self.validate_arguments(&arguments, &parameters, context, next)?;
                Ok(operation_record.result)
            }
            ExpressionOperation::Transaction {
                requirement,
                binding,
                body,
            } => {
                if context.pure {
                    return Err(type_error(
                        "kernel_type_pure_transaction",
                        "pure expression opens a live transaction",
                    ));
                }
                if !context.requirements.contains(&requirement) {
                    return Err(type_error(
                        "kernel_type_transaction_requirement",
                        "transaction requirement is unavailable in this task context",
                    ));
                }
                self.requirement_record(requirement)?;
                self.transaction_binding_type(binding)?;
                self.infer(body, context, next)
            }
        }
    }

    fn local_type(
        &mut self,
        reference: LocalValueReference,
        context: &ExecutionContext,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        match reference {
            LocalValueReference::FunctionParameter(parameter) => {
                let record = match self.read.owner(OwnerKey::Parameter(parameter))? {
                    Some(OwnerRecord::Parameter(record)) => record,
                    _ => {
                        return Err(type_error(
                            "kernel_type_parameter_missing",
                            "function parameter is missing",
                        ));
                    }
                };
                let ParameterParent::Function(parent) = record.parent else {
                    return Err(type_error(
                        "kernel_type_parameter_domain",
                        "function-parameter reference names an operation parameter",
                    ));
                };
                if context.declaration != Some(parent) {
                    return Err(type_error(
                        "kernel_type_parameter_scope",
                        "function parameter belongs to another declaration",
                    ));
                }
                Ok(record.ty)
            }
            LocalValueReference::OperationParameter(parameter) => {
                let record = match self.read.owner(OwnerKey::Parameter(parameter))? {
                    Some(OwnerRecord::Parameter(record)) => record,
                    _ => {
                        return Err(type_error(
                            "kernel_type_parameter_missing",
                            "operation parameter is missing",
                        ));
                    }
                };
                if !matches!(record.parent, ParameterParent::Operation(_)) {
                    return Err(type_error(
                        "kernel_type_parameter_domain",
                        "operation-parameter reference names a function parameter",
                    ));
                }
                Ok(record.ty)
            }
            LocalValueReference::LexicalBinding(binding)
            | LocalValueReference::MatchPayload(binding) => {
                let record = match self.read.owner(OwnerKey::Binding(binding))? {
                    Some(OwnerRecord::Binding(record)) => record,
                    _ => {
                        return Err(type_error(
                            "kernel_type_binding_missing",
                            "binding is missing",
                        ));
                    }
                };
                // A port's separately authorized task value can sit behind an annotated local.
                // Pure binding must inspect that source under its own context as well as its type.
                if !context.allow_task_function_value
                    && context.declaration.is_none()
                    && let Some(value) = record.value
                {
                    let actual = self.infer(value, context, depth)?;
                    if let Some(expected) = record.declared_type {
                        require_same(expected, actual, "kernel_type_binding", "binding value")?;
                    }
                    return Ok(actual);
                }
                if let Some(ty) = record.declared_type {
                    return Ok(ty);
                }
                if let Some(value) = record.value {
                    return self.infer(value, context, depth);
                }
                Err(type_error(
                    "kernel_type_binding_annotation",
                    "non-value binding lacks an exact inferred type",
                ))
            }
            LocalValueReference::TransactionBinding(binding) => {
                self.transaction_binding_type(binding)
            }
        }
    }

    fn transaction_binding_type(
        &mut self,
        binding: BindingId,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        let record = match self.read.owner(OwnerKey::Binding(binding))? {
            Some(OwnerRecord::Binding(record)) if record.kind == BindingKind::Transaction => record,
            _ => {
                return Err(type_error(
                    "kernel_type_transaction_binding",
                    "transaction scope names no exact transaction binding",
                ));
            }
        };
        let Some(ty) = record.declared_type else {
            return Err(type_error(
                "kernel_type_transaction_binding",
                "transaction binding must declare the unit marker type",
            ));
        };
        if !matches!(self.type_object(ty)?.form, TypeForm::Unit) {
            return Err(type_error(
                "kernel_type_transaction_binding",
                "transaction binding must use the unit marker type",
            ));
        }
        Ok(ty)
    }

    fn infer_record(
        &mut self,
        nominal_type: Option<DeclarationReference>,
        type_arguments: &[TypeObjectDigest],
        fields: &[super::expression::RecordExpressionField],
        context: &ExecutionContext,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        if let Some(declaration) = nominal_type {
            let bindings = self.nominal_bindings(declaration, type_arguments)?;
            let ty = self.nominal_type(declaration, type_arguments)?;
            self.validate_nominal_type(ty, context, 0)?;
            let expected = self.record_fields(declaration)?;
            if expected.len() != fields.len() {
                return Err(type_error(
                    "kernel_type_record_field_count",
                    "nominal record expression has the wrong field set",
                ));
            }
            for expected_field in expected {
                let field_record = self.field_record(declaration.package, expected_field)?;
                let value = fields
                    .iter()
                    .find_map(|field| match field.selector {
                        FieldSelector::Nominal(reference)
                            if reference.package == declaration.package
                                && reference.field == expected_field =>
                        {
                            Some(field.value)
                        }
                        _ => None,
                    })
                    .ok_or_else(|| {
                        type_error(
                            "kernel_type_record_field_missing",
                            "nominal record expression omits a field",
                        )
                    })?;
                let actual = self.infer(value, context, depth)?;
                require_same(
                    self.substitute(field_record.ty, &bindings, 0)?,
                    actual,
                    "kernel_type_record_field",
                    "record field value",
                )?;
            }
            Ok(ty)
        } else {
            if !type_arguments.is_empty() {
                return Err(type_error(
                    "kernel_type_nominal_arity",
                    "structural records cannot have nominal type arguments",
                ));
            }
            let mut structural = Vec::with_capacity(fields.len());
            for field in fields {
                let FieldSelector::Structural(name) = &field.selector else {
                    return Err(type_error(
                        "kernel_type_structural_selector",
                        "structural record contains a nominal field selector",
                    ));
                };
                structural.push(StructuralTypeField {
                    name: name.clone(),
                    ty: self.infer(field.value, context, depth)?,
                });
            }
            self.canonical_type(TypeForm::StructuralRecord { fields: structural })
        }
    }

    fn infer_field(
        &mut self,
        value_type: TypeObjectDigest,
        selector: FieldSelector,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        match selector {
            FieldSelector::Nominal(reference) => {
                let field = self.field_record(reference.package, reference.field)?;
                let object = self.type_object(value_type)?;
                let (declaration, arguments) = nominal_parts(&object.form).ok_or_else(|| {
                    type_error(
                        "kernel_type_field_owner",
                        "field selection requires a nominal record",
                    )
                })?;
                if declaration.package != reference.package
                    || declaration.declaration != field.declaration
                {
                    return Err(type_error(
                        "kernel_type_field_owner",
                        "nominal field does not belong to the selected value type",
                    ));
                }
                let bindings = self.nominal_bindings(declaration, arguments)?;
                self.substitute(field.ty, &bindings, 0)
            }
            FieldSelector::Structural(name) => {
                let object = self.type_object(value_type)?;
                let TypeForm::StructuralRecord { fields } = object.form else {
                    return Err(type_error(
                        "kernel_type_structural_field",
                        "structural field selection requires a structural record",
                    ));
                };
                fields
                    .into_iter()
                    .find(|field| field.name == name)
                    .map(|field| field.ty)
                    .ok_or_else(|| {
                        type_error(
                            "kernel_type_structural_field_missing",
                            "structural record lacks the selected field",
                        )
                    })
            }
        }
    }

    fn infer_match(
        &mut self,
        value: ExpressionId,
        arms: &[super::expression::MatchExpressionArm],
        context: &ExecutionContext,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        let value_type = self.infer(value, context, depth)?;
        let value_object = self.type_object(value_type)?;
        let Some((declaration, arguments)) = nominal_parts(&value_object.form) else {
            return Err(type_error(
                "kernel_type_match_value",
                "match value is not a nominal variant",
            ));
        };
        let bindings = self.nominal_bindings(declaration, arguments)?;
        let expected_cases = self.variant_cases(declaration)?;
        let expected_cases = expected_cases
            .iter()
            .map(|case| (declaration.package, *case))
            .collect::<Vec<_>>();
        let actual_cases = arms
            .iter()
            .map(|arm| (arm.case.package, arm.case.case))
            .collect::<Vec<_>>();
        if expected_cases != actual_cases {
            return Err(type_error(
                "kernel_type_match_exhaustive",
                "match arms do not exactly cover the variant cases",
            ));
        }
        let mut result = None;
        for arm in arms {
            let case = self.case_record(arm.case.package, arm.case.case)?;
            if case.declaration != declaration.declaration {
                return Err(type_error(
                    "kernel_type_match_case_owner",
                    "match arm case belongs to another variant",
                ));
            }
            match (case.payload, arm.payload_binding) {
                (Some(expected), Some(binding)) => {
                    let expected = self.substitute(expected, &bindings, 0)?;
                    let declared = match self.read.owner(OwnerKey::Binding(binding))? {
                        Some(OwnerRecord::Binding(record))
                            if record.kind == BindingKind::MatchPayload =>
                        {
                            record.declared_type
                        }
                        _ => None,
                    };
                    if declared != Some(expected) {
                        return Err(type_error(
                            "kernel_type_match_binding",
                            "match payload binding lacks the exact case payload type",
                        ));
                    }
                }
                (None, None) => {}
                _ => {
                    return Err(type_error(
                        "kernel_type_match_payload",
                        "match payload binding disagrees with the case payload",
                    ));
                }
            }
            let body = self.infer(arm.body, context, depth)?;
            if let Some(previous) = result {
                require_same(previous, body, "kernel_type_match_arms", "match arms")?;
            }
            result = Some(body);
        }
        result.ok_or_else(|| type_error("kernel_type_match_empty", "match has no arms"))
    }

    fn validate_optional_payload(
        &mut self,
        expression: Option<ExpressionId>,
        expected: Option<TypeObjectDigest>,
        context: &ExecutionContext,
        depth: usize,
        label: &str,
    ) -> Result<(), Diagnostic> {
        match (expression, expected) {
            (None, None) => Ok(()),
            (Some(expression), Some(expected)) => {
                let actual = self.infer(expression, context, depth)?;
                require_same(expected, actual, "kernel_type_payload", label)
            }
            (Some(_), None) => Err(type_error(
                "kernel_type_unexpected_payload",
                format!("{label} does not accept a payload"),
            )),
            (None, Some(_)) => Err(type_error(
                "kernel_type_missing_payload",
                format!("{label} requires a payload"),
            )),
        }
    }

    fn record_fields(&self, reference: DeclarationReference) -> Result<Vec<FieldId>, Diagnostic> {
        if reference.package == self.read.package_id() {
            return match self
                .read
                .owner(OwnerKey::Declaration(reference.declaration))?
            {
                Some(OwnerRecord::Declaration(record)) => match record.payload {
                    DeclarationPayload::Record { fields, .. } => Ok(fields),
                    _ => Err(type_error(
                        "kernel_type_record_kind",
                        "nominal record expression names a non-record declaration",
                    )),
                },
                _ => Err(type_error(
                    "kernel_type_record_missing",
                    "nominal record declaration is missing",
                )),
            };
        }
        match self.dependency_owner(
            reference.package,
            OwnerKey::Declaration(reference.declaration),
            "record",
        )? {
            PackageInterfaceRecord::Declaration(record) => match record.payload {
                PackageInterfaceDeclarationPayload::Record { fields, .. } => Ok(fields),
                _ => Err(type_error(
                    "kernel_type_record_kind",
                    "nominal record expression names a non-record dependency declaration",
                )),
            },
            _ => Err(type_error(
                "kernel_type_record_kind",
                "nominal record expression names a non-declaration dependency owner",
            )),
        }
    }

    fn variant_cases(&self, reference: DeclarationReference) -> Result<Vec<CaseId>, Diagnostic> {
        if reference.package == self.read.package_id() {
            return match self
                .read
                .owner(OwnerKey::Declaration(reference.declaration))?
            {
                Some(OwnerRecord::Declaration(record)) => match record.payload {
                    DeclarationPayload::Variant { cases, .. } => Ok(cases),
                    _ => Err(type_error(
                        "kernel_type_match_kind",
                        "match value names a non-variant declaration",
                    )),
                },
                _ => Err(type_error(
                    "kernel_type_match_variant_missing",
                    "match variant declaration is missing",
                )),
            };
        }
        match self.dependency_owner(
            reference.package,
            OwnerKey::Declaration(reference.declaration),
            "match variant",
        )? {
            PackageInterfaceRecord::Declaration(record) => match record.payload {
                PackageInterfaceDeclarationPayload::Variant { cases, .. } => Ok(cases),
                _ => Err(type_error(
                    "kernel_type_match_kind",
                    "match value names a non-variant dependency declaration",
                )),
            },
            _ => Err(type_error(
                "kernel_type_match_kind",
                "match value names a non-declaration dependency owner",
            )),
        }
    }

    fn require_capture_safe(
        &mut self,
        ty: TypeObjectDigest,
        context: &ExecutionContext,
    ) -> Result<(), Diagnostic> {
        let mut visited = BTreeSet::new();
        let mut pending = vec![ty];
        while let Some(ty) = pending.pop() {
            self.consume_work()?;
            if !visited.insert(ty) {
                continue;
            }
            let children = match self.type_object(ty)?.form {
                TypeForm::Unit
                | TypeForm::Bool
                | TypeForm::I64
                | TypeForm::Bytes
                | TypeForm::Text
                | TypeForm::StaticText
                | TypeForm::Function { .. } => Vec::new(),
                TypeForm::Secret
                | TypeForm::Stream { .. }
                | TypeForm::CapabilityResource { .. } => {
                    return Err(type_error(
                        "kernel_type_bind_capture",
                        "bound values require capture-safe stored types; remove secrets, streams, resources, and unconstrained stored type parameters",
                    ));
                }
                TypeForm::TypeParameter { parameter } => {
                    self.consume_work()?;
                    let proof = match self.read.owner(OwnerKey::TypeParameter(parameter))? {
                        Some(OwnerRecord::TypeParameter(record))
                            if Some(record.declaration) == context.declaration
                                && record.constraints
                                    == super::TypeParameterConstraints::CaptureSafe =>
                        {
                            match self.read.owner(OwnerKey::Declaration(record.declaration))? {
                                Some(OwnerRecord::Declaration(declaration)) => {
                                    match declaration.payload {
                                        DeclarationPayload::Function(function) => {
                                            if function.effect != FunctionEffect::Pure {
                                                return Err(type_error(
                                                    "kernel_type_bind_capture",
                                                    "capture assumptions require a pure graph declaration",
                                                ));
                                            }
                                            let mut present = false;
                                            for declared in function.type_parameters {
                                                self.consume_work()?;
                                                present |= declared == parameter;
                                            }
                                            present
                                        }
                                        DeclarationPayload::Record {
                                            type_parameters, ..
                                        }
                                        | DeclarationPayload::Variant {
                                            type_parameters, ..
                                        } => {
                                            let mut present = false;
                                            for declared in type_parameters {
                                                self.consume_work()?;
                                                present |= declared == parameter;
                                            }
                                            present
                                        }
                                        _ => false,
                                    }
                                }
                                _ => false,
                            }
                        }
                        _ => false,
                    };
                    if !proof {
                        return Err(type_error(
                            "kernel_type_bind_capture",
                            format!(
                                "stored type parameter {parameter} lacks an exact in-scope capture-safe assumption; declare its constraint or remove the capture"
                            ),
                        ));
                    }
                    Vec::new()
                }
                TypeForm::List { item } | TypeForm::Option { item } => {
                    self.consume_work()?;
                    vec![item]
                }
                TypeForm::Map { key, value }
                | TypeForm::Result {
                    ok: key,
                    error: value,
                } => {
                    self.consume_work()?;
                    self.consume_work()?;
                    vec![key, value]
                }
                TypeForm::StructuralRecord { fields } => {
                    let mut children = Vec::new();
                    for field in fields {
                        self.consume_work()?;
                        children.push(field.ty);
                    }
                    children
                }
                TypeForm::Applied {
                    declaration,
                    arguments,
                } => self.nominal_children(declaration, &arguments)?,
                TypeForm::Named { declaration } => {
                    let payload = if declaration.package == self.read.package_id() {
                        match self
                            .read
                            .owner(OwnerKey::Declaration(declaration.declaration))?
                        {
                            Some(OwnerRecord::Declaration(record)) => match record.payload {
                                DeclarationPayload::Record { fields, .. } => {
                                    Some((fields, Vec::new()))
                                }
                                DeclarationPayload::Variant { cases, .. } => {
                                    Some((Vec::new(), cases))
                                }
                                _ => None,
                            },
                            _ => None,
                        }
                    } else {
                        match self.dependency_owner(
                            declaration.package,
                            OwnerKey::Declaration(declaration.declaration),
                            "captured nominal type",
                        )? {
                            PackageInterfaceRecord::Declaration(record) => match record.payload {
                                PackageInterfaceDeclarationPayload::Record { fields, .. } => {
                                    Some((fields, Vec::new()))
                                }
                                PackageInterfaceDeclarationPayload::Variant { cases, .. } => {
                                    Some((Vec::new(), cases))
                                }
                                _ => None,
                            },
                            _ => None,
                        }
                    };
                    let (fields, cases) = payload.ok_or_else(|| {
                        type_error(
                            "kernel_type_bind_capture",
                            "capture type must name an exact record or variant",
                        )
                    })?;
                    let mut children = Vec::new();
                    for field in fields {
                        self.consume_work()?;
                        children.push(self.field_record(declaration.package, field)?.ty);
                    }
                    for case in cases {
                        self.consume_work()?;
                        if let Some(payload) = self.case_record(declaration.package, case)?.payload
                        {
                            children.push(payload);
                        }
                    }
                    children
                }
            };
            pending.extend(children);
        }
        Ok(())
    }

    fn field_record(&self, package: PackageId, field: FieldId) -> Result<FieldRecord, Diagnostic> {
        if package == self.read.package_id() {
            return match self.read.owner(OwnerKey::Field(field))? {
                Some(OwnerRecord::Field(record)) => Ok(record),
                _ => Err(type_error(
                    "kernel_type_field_missing",
                    "nominal field is missing",
                )),
            };
        }
        match self.dependency_owner(package, OwnerKey::Field(field), "field")? {
            PackageInterfaceRecord::Field(record) => Ok(record),
            _ => Err(type_error(
                "kernel_type_field_missing",
                "dependency field identity has another owner kind",
            )),
        }
    }

    fn case_record(&self, package: PackageId, case: CaseId) -> Result<CaseRecord, Diagnostic> {
        if package == self.read.package_id() {
            return match self.read.owner(OwnerKey::Case(case))? {
                Some(OwnerRecord::Case(record)) => Ok(record),
                _ => Err(type_error(
                    "kernel_type_case_missing",
                    "variant case is missing",
                )),
            };
        }
        match self.dependency_owner(package, OwnerKey::Case(case), "variant case")? {
            PackageInterfaceRecord::Case(record) => Ok(record),
            _ => Err(type_error(
                "kernel_type_case_missing",
                "dependency case identity has another owner kind",
            )),
        }
    }

    fn operation_record(
        &self,
        package: PackageId,
        operation: OperationId,
    ) -> Result<OperationRecord, Diagnostic> {
        if package == self.read.package_id() {
            return match self.read.owner(OwnerKey::Operation(operation))? {
                Some(OwnerRecord::Operation(record)) => Ok(record),
                _ => Err(type_error(
                    "kernel_type_operation_missing",
                    "capability operation is missing",
                )),
            };
        }
        match self.dependency_owner(package, OwnerKey::Operation(operation), "operation")? {
            PackageInterfaceRecord::Operation(record) => Ok(record),
            _ => Err(type_error(
                "kernel_type_operation_missing",
                "dependency operation identity has another owner kind",
            )),
        }
    }

    fn constant_type(
        &self,
        reference: DeclarationReference,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        if reference.package != self.read.package_id() {
            return match self.dependency_owner(
                reference.package,
                OwnerKey::Declaration(reference.declaration),
                "constant",
            )? {
                PackageInterfaceRecord::Declaration(record) => match record.payload {
                    PackageInterfaceDeclarationPayload::Constant { ty } => Ok(ty),
                    _ => Err(type_error(
                        "kernel_type_constant_kind",
                        "constant reference names another declaration kind",
                    )),
                },
                _ => Err(type_error(
                    "kernel_type_constant_kind",
                    "constant reference names another owner kind",
                )),
            };
        }
        match self
            .read
            .owner(OwnerKey::Declaration(reference.declaration))?
        {
            Some(OwnerRecord::Declaration(record)) => match record.payload {
                DeclarationPayload::Constant { ty, .. } => Ok(ty),
                _ => Err(type_error(
                    "kernel_type_constant_kind",
                    "constant reference names another declaration kind",
                )),
            },
            _ => Err(type_error(
                "kernel_type_constant_missing",
                "constant declaration is missing",
            )),
        }
    }

    fn function_signature(
        &mut self,
        reference: DeclarationReference,
        type_arguments: &[TypeObjectDigest],
        context: &ExecutionContext,
    ) -> Result<FunctionSignature, Diagnostic> {
        let foreign = reference.package != self.read.package_id();
        let (type_parameters, parameters, result, requirements, task) = if foreign {
            let record = self.dependency_owner(
                reference.package,
                OwnerKey::Declaration(reference.declaration),
                "function",
            )?;
            match record {
                PackageInterfaceRecord::Declaration(record) => match record.payload {
                    PackageInterfaceDeclarationPayload::External(function) => (
                        function.type_parameters,
                        function.parameters,
                        function.result,
                        BTreeSet::new(),
                        false,
                    ),
                    PackageInterfaceDeclarationPayload::Function(function) => {
                        let (requirements, task) = match function.effect {
                            FunctionEffect::Pure => (BTreeSet::new(), false),
                            FunctionEffect::Task { requirements } => {
                                for requirement in &requirements {
                                    self.requirement_record(*requirement)?;
                                }
                                (requirements.into_iter().collect(), true)
                            }
                        };
                        (
                            function.type_parameters,
                            function.parameters,
                            function.result,
                            requirements,
                            task,
                        )
                    }
                    _ => {
                        return Err(type_error(
                            "kernel_type_function_kind",
                            "function reference names another declaration kind",
                        ));
                    }
                },
                _ => {
                    return Err(type_error(
                        "kernel_type_function_kind",
                        "function reference names another owner kind",
                    ));
                }
            }
        } else {
            let record = match self
                .read
                .owner(OwnerKey::Declaration(reference.declaration))?
            {
                Some(OwnerRecord::Declaration(record)) => record,
                _ => {
                    return Err(type_error(
                        "kernel_type_function_missing",
                        "function declaration is missing",
                    ));
                }
            };
            match record.payload {
                DeclarationPayload::External(function) => (
                    function.type_parameters,
                    function.parameters,
                    function.result,
                    BTreeSet::new(),
                    false,
                ),
                DeclarationPayload::Function(function) => {
                    let (requirements, task) = match function.effect {
                        FunctionEffect::Pure => (BTreeSet::new(), false),
                        FunctionEffect::Task { requirements } => {
                            for requirement in &requirements {
                                self.requirement_record(*requirement)?;
                            }
                            (requirements.into_iter().collect(), true)
                        }
                    };
                    (
                        function.type_parameters,
                        function.parameters,
                        function.result,
                        requirements,
                        task,
                    )
                }
                _ => {
                    return Err(type_error(
                        "kernel_type_function_kind",
                        "function reference names another declaration kind",
                    ));
                }
            }
        };
        if type_parameters.len() != type_arguments.len() {
            return Err(type_error(
                "kernel_type_argument_count",
                "function type argument count disagrees with its declaration",
            ));
        }
        for (parameter, supplied) in type_parameters.iter().zip(type_arguments) {
            self.consume_work()?;
            let owner = if foreign {
                match self.dependency_owner(
                    reference.package,
                    OwnerKey::TypeParameter(*parameter),
                    "callee type parameter",
                )? {
                    PackageInterfaceRecord::TypeParameter(record) => Some(record),
                    _ => None,
                }
            } else {
                match self.read.owner(OwnerKey::TypeParameter(*parameter))? {
                    Some(OwnerRecord::TypeParameter(record)) => Some(record),
                    _ => None,
                }
            };
            let owner = owner
                .filter(|owner| owner.declaration == reference.declaration)
                .ok_or_else(|| {
                    type_error(
                        "kernel_type_parameter_scope",
                        "callee type parameter has no exact declaration owner",
                    )
                })?;
            if owner.constraints == super::TypeParameterConstraints::CaptureSafe {
                self.require_capture_safe(*supplied, context).map_err(|error| {
                    if error.code != "kernel_type_bind_capture" { return error; }
                    type_error("kernel_type_constraint", format!("callee {}/{} parameter {} ({}) requires capture-safe; supplied {}: {}", reference.package, reference.declaration, parameter, owner.name, supplied, error.message))
                })?;
            }
        }
        let substitutions = type_parameters
            .into_iter()
            .zip(type_arguments.iter().copied())
            .collect::<BTreeMap<_, _>>();
        let mut parameter_types = self.parameter_types(reference.package, &parameters)?;
        for parameter in &mut parameter_types {
            *parameter = self.substitute(*parameter, &substitutions, 0)?;
        }
        let result = self.substitute(result, &substitutions, 0)?;
        Ok(FunctionSignature {
            parameters: parameter_types,
            result,
            requirements,
            task,
        })
    }

    fn parameter_types(
        &self,
        package: PackageId,
        parameters: &[crate::platform::semantic_id::ParameterId],
    ) -> Result<Vec<TypeObjectDigest>, Diagnostic> {
        parameters
            .iter()
            .map(|parameter| {
                if package == self.read.package_id() {
                    return match self.read.owner(OwnerKey::Parameter(*parameter))? {
                        Some(OwnerRecord::Parameter(record)) => Ok(record.ty),
                        _ => Err(type_error(
                            "kernel_type_parameter_missing",
                            "signature parameter record is missing",
                        )),
                    };
                }
                match self.dependency_owner(
                    package,
                    OwnerKey::Parameter(*parameter),
                    "signature parameter",
                )? {
                    PackageInterfaceRecord::Parameter(record) => Ok(record.ty),
                    _ => Err(type_error(
                        "kernel_type_parameter_missing",
                        "dependency signature parameter has another owner kind",
                    )),
                }
            })
            .collect()
    }

    fn function_parameter_records(
        &self,
        reference: DeclarationReference,
    ) -> Result<Vec<ParameterRecord>, Diagnostic> {
        let parameters = if reference.package == self.read.package_id() {
            match self
                .read
                .owner(OwnerKey::Declaration(reference.declaration))?
            {
                Some(OwnerRecord::Declaration(record)) => match record.payload {
                    DeclarationPayload::Function(function) => function.parameters,
                    _ => {
                        return Err(type_error(
                            "kernel_type_http_route_function",
                            "HTTP route port must resolve to a function declaration",
                        ));
                    }
                },
                _ => {
                    return Err(type_error(
                        "kernel_type_http_route_function",
                        "HTTP route backing function is missing",
                    ));
                }
            }
        } else {
            match self.dependency_owner(
                reference.package,
                OwnerKey::Declaration(reference.declaration),
                "HTTP route function",
            )? {
                PackageInterfaceRecord::Declaration(record) => match record.payload {
                    PackageInterfaceDeclarationPayload::Function(function) => function.parameters,
                    _ => {
                        return Err(type_error(
                            "kernel_type_http_route_function",
                            "HTTP route port must resolve to a dependency function declaration",
                        ));
                    }
                },
                _ => {
                    return Err(type_error(
                        "kernel_type_http_route_function",
                        "HTTP route backing dependency owner is not a declaration",
                    ));
                }
            }
        };
        parameters
            .into_iter()
            .map(|parameter| {
                let record = if reference.package == self.read.package_id() {
                    match self.read.owner(OwnerKey::Parameter(parameter))? {
                        Some(OwnerRecord::Parameter(record)) => record,
                        _ => {
                            return Err(type_error(
                                "kernel_type_http_route_parameter",
                                "HTTP route backing function parameter is missing",
                            ));
                        }
                    }
                } else {
                    match self.dependency_owner(
                        reference.package,
                        OwnerKey::Parameter(parameter),
                        "HTTP route parameter",
                    )? {
                        PackageInterfaceRecord::Parameter(record) => record,
                        _ => {
                            return Err(type_error(
                                "kernel_type_http_route_parameter",
                                "HTTP route dependency parameter has another owner kind",
                            ));
                        }
                    }
                };
                if record.parent != ParameterParent::Function(reference.declaration) {
                    return Err(type_error(
                        "kernel_type_http_route_parameter_parent",
                        "HTTP route function parameter belongs to another semantic parent",
                    ));
                }
                Ok(record)
            })
            .collect()
    }

    fn validate_arguments(
        &mut self,
        arguments: &[ExpressionId],
        expected: &[TypeObjectDigest],
        context: &ExecutionContext,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        if arguments.len() != expected.len() {
            return Err(type_error(
                "kernel_type_call_arity",
                "argument count disagrees with the exact function signature",
            ));
        }
        for (argument, expected) in arguments.iter().zip(expected) {
            let actual = self.infer(*argument, context, depth)?;
            require_same(*expected, actual, "kernel_type_argument", "argument")?;
        }
        Ok(())
    }

    fn validate_call_effect(
        &self,
        signature: &FunctionSignature,
        context: &ExecutionContext,
    ) -> Result<(), Diagnostic> {
        if signature.task && context.pure {
            return Err(type_error(
                "kernel_type_pure_task_call",
                "pure expression calls a task function",
            ));
        }
        for required in &signature.requirements {
            if context.requirements.contains(required) {
                continue;
            }
            let required_record = self.requirement_record(*required)?;
            let mut covered = false;
            for available in &context.requirements {
                let available_record = self.requirement_record(*available)?;
                if available_record.name == required_record.name
                    && available_record.interface == required_record.interface
                    && required_record
                        .operations
                        .iter()
                        .all(|operation| available_record.operations.contains(operation))
                {
                    covered = true;
                    break;
                }
            }
            if !covered {
                return Err(type_error(
                    "kernel_type_task_requirement",
                    "task call requires an unavailable capability alias, interface, or operation",
                ));
            }
        }
        Ok(())
    }

    fn function_type(
        &mut self,
        signature: &FunctionSignature,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        self.canonical_type(TypeForm::Function {
            parameters: signature.parameters.clone(),
            result: signature.result,
        })
    }

    fn named_type(
        &mut self,
        declaration: DeclarationReference,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        self.canonical_type(TypeForm::Named { declaration })
    }

    fn nominal_type(
        &mut self,
        declaration: DeclarationReference,
        arguments: &[TypeObjectDigest],
    ) -> Result<TypeObjectDigest, Diagnostic> {
        if arguments.is_empty() {
            self.named_type(declaration)
        } else {
            self.canonical_type(TypeForm::Applied {
                declaration,
                arguments: arguments.to_vec(),
            })
        }
    }

    fn validate_application_equality(&mut self, ty: TypeObjectDigest) -> Result<(), Diagnostic> {
        let mut visited = BTreeSet::new();
        let mut pending = vec![(ty, false)];
        while let Some((ty, applied)) = pending.pop() {
            self.consume_work()?;
            if !visited.insert((ty, applied)) {
                continue;
            }
            let object = self.type_object(ty)?;
            let children = match &object.form {
                TypeForm::Applied {
                    declaration,
                    arguments,
                } => {
                    for child in self.nominal_children(*declaration, arguments)? {
                        pending.push((child, true));
                    }
                    continue;
                }
                TypeForm::Named { declaration } => self.nominal_children(*declaration, &[])?,
                TypeForm::Function { .. }
                | TypeForm::Secret
                | TypeForm::CapabilityResource { .. }
                | TypeForm::Stream { .. }
                | TypeForm::TypeParameter { .. } => {
                    if applied {
                        return Err(type_error(
                            "kernel_type_nominal_equality",
                            "applied data contains a noncomparable argument or member",
                        ));
                    }
                    continue;
                }
                _ => object.child_types(),
            };
            for child in children {
                pending.push((child, applied));
            }
        }
        Ok(())
    }

    fn nominal_parameters(
        &self,
        reference: DeclarationReference,
    ) -> Result<Vec<TypeParameterId>, Diagnostic> {
        let parameters = if reference.package == self.read.package_id() {
            match self
                .read
                .owner(OwnerKey::Declaration(reference.declaration))?
            {
                Some(OwnerRecord::Declaration(record)) => match record.payload {
                    DeclarationPayload::Record {
                        type_parameters, ..
                    }
                    | DeclarationPayload::Variant {
                        type_parameters, ..
                    } => Some(type_parameters),
                    _ => None,
                },
                _ => None,
            }
        } else {
            match self.dependency_owner(
                reference.package,
                OwnerKey::Declaration(reference.declaration),
                "nominal template",
            )? {
                PackageInterfaceRecord::Declaration(record) => match record.payload {
                    PackageInterfaceDeclarationPayload::Record {
                        type_parameters, ..
                    }
                    | PackageInterfaceDeclarationPayload::Variant {
                        type_parameters, ..
                    } => Some(type_parameters),
                    _ => None,
                },
                _ => None,
            }
        };
        parameters.ok_or_else(|| {
            type_error(
                "kernel_type_nominal_kind",
                "nominal type requires an exact record or variant declaration",
            )
        })
    }

    fn nominal_member_types(
        &mut self,
        declaration: DeclarationReference,
    ) -> Result<Vec<TypeObjectDigest>, Diagnostic> {
        let kind = if declaration.package == self.read.package_id() {
            self.read
                .owner(OwnerKey::Declaration(declaration.declaration))?
                .map(|record| record.kind())
        } else {
            Some(
                self.dependency_owner(
                    declaration.package,
                    OwnerKey::Declaration(declaration.declaration),
                    "nominal template",
                )?
                .header()
                .kind,
            )
        };
        match kind {
            Some(OwnerKind::Record) => {
                let mut types = Vec::new();
                for field in self.record_fields(declaration)? {
                    self.consume_work()?;
                    types.push(self.field_record(declaration.package, field)?.ty);
                }
                Ok(types)
            }
            Some(OwnerKind::Variant) => {
                let mut types = Vec::new();
                for case in self.variant_cases(declaration)? {
                    self.consume_work()?;
                    if let Some(ty) = self.case_record(declaration.package, case)?.payload {
                        types.push(ty);
                    }
                }
                Ok(types)
            }
            _ => Err(type_error(
                "kernel_type_nominal_kind",
                "nominal member traversal requires a record or variant",
            )),
        }
    }

    fn validate_owner_nominals(&mut self, owner: &OwnerRecord) -> Result<(), Diagnostic> {
        // Expression roots inherit their exact executable context through inference, including
        // dead branches. Signature and nominal templates also require admission when unused.
        if matches!(owner, OwnerRecord::Expression(_) | OwnerRecord::Binding(_)) {
            return Ok(());
        }
        let declaration = match owner {
            OwnerRecord::Declaration(record) => match record.header.owner {
                OwnerKey::Declaration(id) => Some(id),
                _ => None,
            },
            OwnerRecord::Field(record) => Some(record.declaration),
            OwnerRecord::Case(record) => Some(record.declaration),
            OwnerRecord::Parameter(record) => match record.parent {
                super::ParameterParent::Function(id) => Some(id),
                _ => None,
            },
            _ => None,
        };
        let context = pure_context(declaration);
        let mut roots = owner.type_roots();
        if let OwnerRecord::Declaration(record) = owner {
            let parameters = match &record.payload {
                DeclarationPayload::Function(function) => &function.parameters[..],
                DeclarationPayload::External(function) => &function.parameters[..],
                _ => &[],
            };
            for parameter in parameters {
                self.consume_work()?;
                if let Some(OwnerRecord::Parameter(record)) =
                    self.read.owner(OwnerKey::Parameter(*parameter))?
                {
                    roots.push(record.ty);
                }
            }
            if matches!(
                record.payload,
                DeclarationPayload::Record { .. } | DeclarationPayload::Variant { .. }
            ) {
                let reference = DeclarationReference {
                    package: self.read.package_id(),
                    declaration: declaration.ok_or_else(|| {
                        type_error("kernel_type_nominal_kind", "missing nominal identity")
                    })?,
                };
                roots.extend(self.nominal_member_types(reference)?);
                self.validate_nominal_schema(reference)?;
            }
        }
        for ty in roots {
            self.validate_nominal_type(ty, &context, 0)?;
        }
        Ok(())
    }

    fn nominal_bindings(
        &mut self,
        declaration: DeclarationReference,
        arguments: &[TypeObjectDigest],
    ) -> Result<BTreeMap<TypeParameterId, TypeObjectDigest>, Diagnostic> {
        let parameters = self.nominal_parameters(declaration)?;
        if parameters.len() != arguments.len() {
            return Err(type_error(
                "kernel_type_nominal_arity",
                "nominal type arguments must exactly match its ordered declaration parameters",
            ));
        }
        let mut bindings = BTreeMap::new();
        for (parameter, argument) in parameters.into_iter().zip(arguments) {
            self.consume_work()?;
            bindings.insert(parameter, *argument);
        }
        Ok(bindings)
    }

    fn nominal_children(
        &mut self,
        declaration: DeclarationReference,
        arguments: &[TypeObjectDigest],
    ) -> Result<Vec<TypeObjectDigest>, Diagnostic> {
        let bindings = self.nominal_bindings(declaration, arguments)?;
        for _ in arguments {
            self.consume_work()?;
        }
        let mut children = arguments.to_vec();
        for ty in self.nominal_member_types(declaration)? {
            self.consume_work()?;
            children.push(self.substitute(ty, &bindings, 0)?);
        }
        Ok(children)
    }

    fn validate_nominal_type(
        &mut self,
        ty: TypeObjectDigest,
        context: &ExecutionContext,
        depth: usize,
    ) -> Result<(), Diagnostic> {
        self.consume_work()?;
        if depth > MAXIMUM_TYPE_DEPTH {
            return Err(type_error(
                "kernel_type_nominal_depth",
                "nominal type exceeds the existing type-depth limit",
            ));
        }
        let object = self.type_object(ty)?;
        if let TypeForm::TypeParameter { parameter } = &object.form {
            let parameter = match self.read.owner(OwnerKey::TypeParameter(*parameter))? {
                Some(OwnerRecord::TypeParameter(parameter)) => parameter,
                _ => {
                    return Err(type_error(
                        "kernel_type_parameter_scope",
                        "type parameter has no exact local owner",
                    ));
                }
            };
            if context.declaration != Some(parameter.declaration) {
                return Err(type_error(
                    "kernel_type_parameter_scope",
                    "type parameter escapes its exact declaration scope",
                ));
            }
        }
        // Admit every syntactic argument before following substituted members. A closed or
        // phantom outer declaration can carry an expanding schema in one of its arguments.
        for child in object.child_types() {
            self.validate_nominal_type(child, context, depth + 1)?;
        }
        if let Some((declaration, arguments)) = nominal_parts(&object.form) {
            self.validate_nominal_schema(declaration)?;
            let parameters = self.nominal_parameters(declaration)?;
            if parameters.len() != arguments.len() {
                return Err(type_error(
                    "kernel_type_nominal_arity",
                    "generic nominal types require their complete explicit ordered arguments",
                ));
            }
            for (parameter, argument) in parameters.iter().zip(arguments) {
                self.consume_work()?;
                let owner = if declaration.package == self.read.package_id() {
                    match self.read.owner(OwnerKey::TypeParameter(*parameter))? {
                        Some(OwnerRecord::TypeParameter(record)) => Some(record),
                        _ => None,
                    }
                } else {
                    match self.dependency_owner(
                        declaration.package,
                        OwnerKey::TypeParameter(*parameter),
                        "nominal parameter",
                    )? {
                        PackageInterfaceRecord::TypeParameter(record) => Some(record),
                        _ => None,
                    }
                }
                .filter(|record| record.declaration == declaration.declaration)
                .ok_or_else(|| {
                    type_error(
                        "kernel_type_parameter_scope",
                        "nominal parameter has a foreign declaration owner",
                    )
                })?;
                if owner.constraints == super::TypeParameterConstraints::CaptureSafe {
                    self.require_capture_safe(*argument, context)
                        .map_err(|error| {
                            if error.code == "kernel_type_bind_capture" {
                                type_error("kernel_type_constraint", error.message)
                            } else {
                                error
                            }
                        })?;
                }
            }
            if !arguments.is_empty() {
                self.require_ordinary_application(ty)?;
            }
        }
        Ok(())
    }

    fn require_ordinary_application(&mut self, ty: TypeObjectDigest) -> Result<(), Diagnostic> {
        let mut pending = vec![ty];
        let mut visited = BTreeSet::new();
        while let Some(ty) = pending.pop() {
            self.consume_work()?;
            if !visited.insert(ty) {
                continue;
            }
            let object = self.type_object(ty)?;
            let children = match &object.form {
                TypeForm::CapabilityResource { .. } | TypeForm::Stream { .. } => {
                    return Err(type_error(
                        "kernel_type_nominal_resource",
                        "applied nominal data cannot contain live resources, including phantom arguments and absent cases",
                    ));
                }
                TypeForm::Function { .. } => Vec::new(),
                TypeForm::Named { declaration } => self.nominal_children(*declaration, &[])?,
                TypeForm::Applied {
                    declaration,
                    arguments,
                } => self.nominal_children(*declaration, arguments)?,
                _ => object.child_types(),
            };
            for child in children {
                self.consume_work()?;
                if !visited.contains(&child) {
                    pending.push(child);
                }
            }
        }
        Ok(())
    }

    fn substitute(
        &mut self,
        digest: TypeObjectDigest,
        substitutions: &BTreeMap<TypeParameterId, TypeObjectDigest>,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        if substitutions.is_empty() {
            return Ok(digest);
        }
        self.consume_work()?;
        if depth > MAXIMUM_TYPE_DEPTH {
            return Err(type_error(
                "kernel_type_substitution_depth",
                "type substitution exceeded its structural depth bound",
            ));
        }
        let object = self.type_object(digest)?;
        let next = depth.saturating_add(1);
        let form = match object.form {
            TypeForm::TypeParameter { parameter } => {
                return substitutions.get(&parameter).copied().ok_or_else(|| {
                    type_error(
                        "kernel_type_parameter_scope",
                        "type object names a parameter outside this signature",
                    )
                });
            }
            TypeForm::StructuralRecord { fields } => TypeForm::StructuralRecord {
                fields: fields
                    .into_iter()
                    .map(|field| {
                        Ok(StructuralTypeField {
                            name: field.name,
                            ty: self.substitute(field.ty, substitutions, next)?,
                        })
                    })
                    .collect::<Result<_, Diagnostic>>()?,
            },
            TypeForm::Applied {
                declaration,
                arguments,
            } => TypeForm::Applied {
                declaration,
                arguments: arguments
                    .into_iter()
                    .map(|argument| self.substitute(argument, substitutions, next))
                    .collect::<Result<_, _>>()?,
            },
            TypeForm::List { item } => TypeForm::List {
                item: self.substitute(item, substitutions, next)?,
            },
            TypeForm::Map { key, value } => TypeForm::Map {
                key: self.substitute(key, substitutions, next)?,
                value: self.substitute(value, substitutions, next)?,
            },
            TypeForm::Option { item } => TypeForm::Option {
                item: self.substitute(item, substitutions, next)?,
            },
            TypeForm::Result { ok, error } => TypeForm::Result {
                ok: self.substitute(ok, substitutions, next)?,
                error: self.substitute(error, substitutions, next)?,
            },
            TypeForm::Stream { item } => TypeForm::Stream {
                item: self.substitute(item, substitutions, next)?,
            },
            TypeForm::Function { parameters, result } => TypeForm::Function {
                parameters: parameters
                    .into_iter()
                    .map(|parameter| self.substitute(parameter, substitutions, next))
                    .collect::<Result<_, _>>()?,
                result: self.substitute(result, substitutions, next)?,
            },
            other => other,
        };
        self.canonical_type(form)
    }

    fn canonical_type(&mut self, form: TypeForm) -> Result<TypeObjectDigest, Diagnostic> {
        let object = TypeObject::new(form)?;
        let (digest, bytes) = super::codec::encode_type_object(&object)?;
        if !self.ephemeral_types.contains_key(&digest) {
            let children = match &object.form {
                TypeForm::StructuralRecord { fields } => fields.len(),
                TypeForm::Applied { arguments, .. } => arguments.len(),
                TypeForm::Function { parameters, .. } => parameters.len(),
                _ => 0,
            };
            self.type_metadata_bytes = children
                .checked_mul(std::mem::size_of::<StructuralTypeField>())
                .and_then(|size| size.checked_add(bytes.len()))
                .and_then(|size| {
                    size.checked_add(
                        std::mem::size_of::<(TypeObjectDigest, TypeObject)>()
                            + 4 * std::mem::size_of::<usize>(),
                    )
                })
                .and_then(|size| self.type_metadata_bytes.checked_add(size))
                .filter(|bytes| *bytes <= 64 * 1_048_576)
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticClass::Resource,
                        "kernel_type_nominal_storage",
                        "substituted type metadata exceeds its storage admission",
                    )
                })?;
            self.ephemeral_types.insert(digest, object);
        }
        Ok(digest)
    }

    fn type_object(&self, digest: TypeObjectDigest) -> Result<TypeObject, Diagnostic> {
        if let Some(object) = self.read.type_object(digest)? {
            return Ok(object);
        }
        self.ephemeral_types.get(&digest).cloned().ok_or_else(|| {
            type_error(
                "kernel_type_object_missing",
                format!("type object {digest} is unavailable for semantic inference"),
            )
        })
    }

    fn component_requirements(
        &self,
        declaration: DeclarationId,
    ) -> Result<BTreeSet<RequirementReference>, Diagnostic> {
        let package = self.read.package_id();
        Ok(match self.read.owner(OwnerKey::Declaration(declaration))? {
            Some(OwnerRecord::Declaration(record)) => match &record.payload {
                DeclarationPayload::Component { requirements, .. } => requirements
                    .iter()
                    .copied()
                    .map(|requirement| RequirementReference {
                        package,
                        requirement,
                    })
                    .collect(),
                _ => BTreeSet::new(),
            },
            _ => BTreeSet::new(),
        })
    }

    fn requirement_record(
        &self,
        reference: RequirementReference,
    ) -> Result<RequirementRecord, Diagnostic> {
        if reference.package != self.read.package_id() {
            return match self.dependency_owner(
                reference.package,
                OwnerKey::Requirement(reference.requirement),
                "capability requirement",
            )? {
                PackageInterfaceRecord::Requirement(record) => Ok(record),
                _ => Err(type_error(
                    "kernel_type_capability_requirement_kind",
                    "capability requirement names another dependency owner kind",
                )),
            };
        }
        match self
            .read
            .owner(OwnerKey::Requirement(reference.requirement))?
        {
            Some(OwnerRecord::Requirement(record)) => Ok(record),
            _ => Err(type_error(
                "kernel_type_capability_missing",
                "capability requirement record is missing",
            )),
        }
    }

    fn dependency_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
        label: &str,
    ) -> Result<PackageInterfaceRecord, Diagnostic> {
        if !self.read.has_dependency(package)? {
            return Err(type_error(
                "kernel_type_dependency_missing",
                format!("{label} names unbound package {package}"),
            ));
        }
        self.read
            .package_interface_owner(package, owner)?
            .ok_or_else(|| {
                type_error(
                    "kernel_type_dependency_owner_missing",
                    format!(
                        "{label} names owner {owner:?} absent from exact dependency interface {package}"
                    ),
                )
            })
    }

    fn consume_work(&mut self) -> Result<(), Diagnostic> {
        self.read.validation_checkpoint()?;
        if self.exhaustion.is_some() {
            return Err(type_error(
                "kernel_type_work",
                "expression type validation stopped after exhausting an admission",
            ));
        }
        if *self.work >= self.limits.maximum_steps {
            self.exhaustion = Some(ExpressionValidationExhaustion::Steps);
            return Err(type_error(
                "kernel_type_work",
                "expression type validation exhausted its explicit work budget",
            ));
        }
        *self.work = self.work.saturating_add(1);
        Ok(())
    }

    fn exhausted(&self) -> bool {
        self.exhaustion.is_some()
    }

    fn error(&mut self, code: &str, message: impl Into<String>) {
        self.push_diagnostic(type_error(code, message));
    }

    fn push_diagnostic(&mut self, diagnostic: Diagnostic) {
        if self.exhaustion.is_some() {
            return;
        }
        if self.diagnostics.len() >= self.limits.maximum_diagnostics {
            self.exhaustion = Some(ExpressionValidationExhaustion::Diagnostics);
            return;
        }
        self.diagnostics.push(diagnostic);
    }
}

fn pure_context(declaration: Option<DeclarationId>) -> ExecutionContext {
    ExecutionContext {
        declaration,
        pure: true,
        requirements: BTreeSet::new(),
        allow_task_function_value: false,
    }
}

fn require_same(
    expected: TypeObjectDigest,
    actual: TypeObjectDigest,
    code: &str,
    label: &str,
) -> Result<(), Diagnostic> {
    if expected != actual {
        return Err(type_error(
            code,
            format!("{label} expects type {expected} but has type {actual}"),
        ));
    }
    Ok(())
}

fn type_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        if code == "kernel_type_work" {
            DiagnosticClass::Resource
        } else {
            DiagnosticClass::Semantic
        },
        code,
        message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::semantic_id::ModuleId;

    #[test]
    fn expression_and_declared_type_steps_stop_before_zero_and_exact_limits() {
        let snapshot = super::super::tests::witness_snapshot();
        let constant = snapshot
            .owners
            .iter()
            .find_map(|(owner, record)| match record {
                OwnerRecord::Declaration(declaration)
                    if declaration.name.as_str() == "unit_constant" =>
                {
                    Some(*owner)
                }
                _ => None,
            })
            .expect("unit constant declaration");

        let mut diagnostics = Vec::new();
        let mut work = 0;
        let exhausted = validate_expression_roots_with_limits(
            &snapshot,
            [constant],
            &mut diagnostics,
            &mut work,
            ExpressionValidationLimits {
                maximum_steps: 0,
                maximum_diagnostics: 1,
            },
        );
        assert_eq!(exhausted, Err(ExpressionValidationExhaustion::Steps));
        assert_eq!(work, 0);
        assert!(diagnostics.is_empty());

        let mut diagnostics = Vec::new();
        let mut work = 0;
        let exhausted = validate_expression_roots_with_limits(
            &snapshot,
            [constant],
            &mut diagnostics,
            &mut work,
            ExpressionValidationLimits {
                maximum_steps: 1,
                maximum_diagnostics: 1,
            },
        );
        assert_eq!(exhausted, Err(ExpressionValidationExhaustion::Steps));
        assert_eq!(work, 1);
        assert!(diagnostics.is_empty());

        let mut work = 0;
        validate_expression_roots_with_limits(
            &snapshot,
            [constant],
            &mut diagnostics,
            &mut work,
            ExpressionValidationLimits {
                maximum_steps: 2,
                maximum_diagnostics: 1,
            },
        )
        .expect("one declared type and one expression fit exactly two steps");
        assert_eq!(work, 2);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn diagnostic_admission_stops_before_limits_zero_and_one_are_exceeded() {
        let snapshot = super::super::tests::witness_snapshot();
        let missing_a = OwnerKey::Module(ModuleId::migrate(b"expression-budget", 1));
        let missing_b = OwnerKey::Module(ModuleId::migrate(b"expression-budget", 2));

        let mut diagnostics = Vec::new();
        let mut work = 0;
        let exhausted = validate_expression_roots_with_limits(
            &snapshot,
            [missing_a],
            &mut diagnostics,
            &mut work,
            ExpressionValidationLimits {
                maximum_steps: 1,
                maximum_diagnostics: 0,
            },
        );
        assert_eq!(exhausted, Err(ExpressionValidationExhaustion::Diagnostics));
        assert_eq!(work, 0);
        assert!(diagnostics.is_empty());

        let mut diagnostics = Vec::new();
        let mut work = 0;
        let exhausted = validate_expression_roots_with_limits(
            &snapshot,
            [missing_a, missing_b],
            &mut diagnostics,
            &mut work,
            ExpressionValidationLimits {
                maximum_steps: 1,
                maximum_diagnostics: 1,
            },
        );
        assert_eq!(exhausted, Err(ExpressionValidationExhaustion::Diagnostics));
        assert_eq!(work, 0);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "kernel_type_frontier_owner_missing");
    }
}
