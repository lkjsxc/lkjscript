//! Independent exact-ID expression type and effect oracle for Graph 14.

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
    requirements: BTreeSet<super::RequirementOperand>,
    effect_parameters: BTreeSet<super::EffectParameterReference>,
    bindings: BTreeMap<BindingId, (BindingKind, TypeObjectDigest)>,
}

#[derive(Clone, Debug)]
struct FunctionSignature {
    target: Option<DeclarationReference>,
    parameters: Vec<TypeObjectDigest>,
    result: TypeObjectDigest,
    requirements: BTreeSet<super::RequirementOperand>,
    task: bool,
    effect_parameters: BTreeSet<super::EffectParameterReference>,
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

    /// Admit a metadata traversal step before scanning or growing derived state.
    /// Cancellation-only checkpoints remain distinct from proof-work admission.
    fn validation_work(&self) -> Result<(), Diagnostic> {
        self.validation_checkpoint()
    }
}

/// Adds an owning operation's cancellation checkpoint to immutable, possibly cached reads.
pub(crate) struct CheckedExpressionRead<'a, R: ?Sized> {
    pub read: &'a R,
    pub checkpoint: &'a dyn Fn() -> Result<(), Diagnostic>,
}

impl<R: ExpressionRead + ?Sized> ExpressionRead for CheckedExpressionRead<'_, R> {
    fn package_id(&self) -> PackageId {
        self.read.package_id()
    }
    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.validation_checkpoint()?;
        self.read.owner(owner)
    }
    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        self.validation_checkpoint()?;
        self.read.type_object(digest)
    }
    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.validation_checkpoint()?;
        self.read.package_interface_owner(package, owner)
    }
    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.validation_checkpoint()?;
        self.read.has_dependency(package)
    }
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        (self.checkpoint)()?;
        self.read.validation_checkpoint()
    }
}

/// Derived inference types remain visible to contract checks without becoming
/// accepted graph state. Persisted objects retain the same precedence as the
/// validator's own type reads.
struct InferredTypeRead<'a, R: ?Sized> {
    read: &'a R,
    types: &'a BTreeMap<TypeObjectDigest, TypeObject>,
}

impl<R: ExpressionRead + ?Sized> ExpressionRead for InferredTypeRead<'_, R> {
    fn package_id(&self) -> PackageId {
        self.read.package_id()
    }
    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.read.owner(owner)
    }
    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        Ok(self
            .read
            .type_object(digest)?
            .or_else(|| self.types.get(&digest).cloned()))
    }
    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.read.package_interface_owner(package, owner)
    }
    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.read.has_dependency(package)
    }
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        self.read.validation_checkpoint()
    }
    fn validation_work(&self) -> Result<(), Diagnostic> {
        self.read.validation_work()
    }
}

/// Request-local deterministic admissions owned by expression validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ExpressionValidationLimits {
    /// Shared maximum for inference/substitution, affine visits and affine metadata reads.
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
    read: &impl ExpressionRead,
    diagnostics: &mut Vec<Diagnostic>,
    work: &mut usize,
    maximum_steps: usize,
) {
    let roots = snapshot.owners.keys().copied().collect::<Vec<_>>();
    if validate_expression_roots_with_limits(
        read,
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
        selected_expression: None,
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
    infer_function_expression(
        read,
        declaration,
        expression,
        effect,
        work,
        maximum_steps,
        |_, digest| Ok(digest),
    )
}

/// Returns the inferred root shape, including recomputable callable types that
/// have no persisted type object. This does not add objects to accepted meaning.
pub(crate) fn infer_function_expression_type_object<R: ExpressionRead>(
    read: &R,
    declaration: DeclarationId,
    expression: ExpressionId,
    effect: &FunctionEffect,
    work: &mut usize,
    maximum_steps: usize,
) -> Result<TypeObject, Diagnostic> {
    infer_function_expression(
        read,
        declaration,
        expression,
        effect,
        work,
        maximum_steps,
        |validator, digest| validator.type_object(digest),
    )
}

fn infer_function_expression<R: ExpressionRead, T>(
    read: &R,
    declaration: DeclarationId,
    expression: ExpressionId,
    effect: &FunctionEffect,
    work: &mut usize,
    maximum_steps: usize,
    result: impl FnOnce(&ExpressionValidator<'_, '_, R>, TypeObjectDigest) -> Result<T, Diagnostic>,
) -> Result<T, Diagnostic> {
    let (pure, requirements) = match effect {
        FunctionEffect::Pure => (true, BTreeSet::new()),
        FunctionEffect::Task {
            effect_parameters: _,
            requirements,
        } => (false, requirements.iter().copied().collect::<BTreeSet<_>>()),
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
        selected_expression: Some((expression, None)),
    };
    validator.consume_work()?;
    let body = match read.owner(OwnerKey::Declaration(declaration))? {
        Some(OwnerRecord::Declaration(record)) => match record.payload {
            DeclarationPayload::Function(function) => function.body,
            _ => {
                return Err(type_error(
                    "kernel_type_parameter_scope",
                    "inference requires an exact graph function context",
                ));
            }
        },
        _ => {
            return Err(type_error(
                "kernel_type_parameter_scope",
                "inference function context is missing",
            ));
        }
    };
    validator.infer(
        body,
        &ExecutionContext {
            declaration: Some(declaration),
            pure,
            requirements,
            effect_parameters: effect.row().parameters.into_iter().collect(),
            bindings: BTreeMap::new(),
        },
        0,
    )?;
    let inferred = validator
        .selected_expression
        .and_then(|(_, ty)| ty)
        .ok_or_else(|| {
            type_error(
                "kernel_full_lexical_scope",
                "selected expression is outside the exact function body",
            )
        })?;
    result(&validator, inferred)
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
    selected_expression: Option<(ExpressionId, Option<TypeObjectDigest>)>,
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
            if let Err(mut diagnostic) = self.validate_owner_nominals(&owner) {
                diagnostic
                    .notes
                    .push(format!("semantic owner: {}", owner.owner()));
                self.push_diagnostic(diagnostic);
                continue;
            }
            match owner {
                OwnerRecord::Declaration(declaration) => match declaration.payload {
                    DeclarationPayload::Function(function) => {
                        let symbolic = function.effect.row().parameters.into_iter().collect();
                        let (pure, requirements) = match function.effect {
                            FunctionEffect::Pure => (true, BTreeSet::new()),
                            FunctionEffect::Task {
                                effect_parameters: _,
                                requirements,
                            } => {
                                for requirement in &requirements {
                                    if let Err(diagnostic) =
                                        self.requirement_constraint(*requirement)
                                    {
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
                            effect_parameters: symbolic,
                            bindings: BTreeMap::new(),
                        };
                        self.compare_root_type(
                            function.body,
                            function.result,
                            &context,
                            "function",
                        );
                    }
                    DeclarationPayload::External(external) => {
                        if let Err(mut diagnostic) =
                            crate::platform::intrinsic_contract::validate_kernel_intrinsic(
                                self.read,
                                &external,
                                self.work,
                                self.limits.maximum_steps,
                            )
                        {
                            if diagnostic.code == "kernel_full_work" {
                                self.exhaustion = Some(ExpressionValidationExhaustion::Steps);
                            } else {
                                diagnostic
                                    .notes
                                    .push(format!("semantic owner: {}", declaration.header.owner));
                                self.push_diagnostic(diagnostic);
                            }
                        }
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
                    let mut requirements = match self.component_requirements(port.declaration) {
                        Ok(requirements) => requirements,
                        Err(diagnostic) => {
                            self.push_diagnostic(diagnostic);
                            continue;
                        }
                    };
                    let signature = match self.callable_signature(port.function_type) {
                        Ok(signature) => signature,
                        Err(diagnostic) => {
                            self.push_diagnostic(diagnostic);
                            continue;
                        }
                    };
                    if !signature.effect_parameters.is_empty() {
                        self.error(
                            "kernel_type_port_effect_scope",
                            "port task effects must be closed exact requirements",
                        );
                        continue;
                    }
                    // An imported concrete effect named by the public port type
                    // is an explicit deployment obligation, not an inferred grant
                    // for every requirement in a dependency's implementation.
                    requirements.extend(signature.requirements.iter().copied().filter(
                        |requirement| {
                            requirement.concrete().is_some_and(|reference| {
                                reference.package != self.read.package_id()
                            })
                        },
                    ));
                    let component_context = ExecutionContext {
                        declaration: None,
                        pure: false,
                        requirements,
                        effect_parameters: BTreeSet::new(),
                        bindings: BTreeMap::new(),
                    };
                    if let Err(diagnostic) =
                        self.validate_call_effect(&signature, &component_context)
                    {
                        self.push_diagnostic(diagnostic);
                        continue;
                    }
                    let context = ExecutionContext {
                        declaration: None,
                        pure: !signature.task,
                        requirements: signature.requirements,
                        effect_parameters: BTreeSet::new(),
                        bindings: BTreeMap::new(),
                    };
                    match port.implementation {
                        PortImplementation::Expression(expression) => self.compare_root_type(
                            expression,
                            port.function_type,
                            &context,
                            "port expression",
                        ),
                        PortImplementation::Function(function) => {
                            match self.function_signature(function, &[], &[], &[], &[], &context) {
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
                    if let Err(diagnostic) =
                        self.validate_target_port(target.component, target_port)
                    {
                        self.push_diagnostic(diagnostic);
                        continue;
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
                    let matches = self.type_object(port.function_type).and_then(|ty| {
                        crate::platform::http::has_semantic_http_route_shape(
                            &ty.form,
                            route.selector.capture_count(),
                        )
                    });
                    let matches = match matches {
                        Ok(matches) => matches,
                        Err(error) => {
                            self.push_diagnostic(error);
                            continue;
                        }
                    };
                    if !matches {
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
                    let text_type = match crate::platform::http::semantic_http_types(
                        &mut TypeObjectInterner::default(),
                    ) {
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
            Ok(actual) => {
                let mut error = type_error(
                    "kernel_type_root",
                    format!(
                        "{label} {} expects {} but its root has {}",
                        context
                            .declaration
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "expression scope".into()),
                        self.describe_type(expected),
                        self.describe_type(actual)
                    ),
                );
                error.notes.push(format!("expression owner: {root}"));
                self.push_diagnostic(error);
            }
            Err(diagnostic) => self.push_diagnostic(diagnostic),
        }
    }

    fn describe_type(&self, ty: TypeObjectDigest) -> String {
        match self.type_object(ty).map(|object| object.form) {
            Ok(TypeForm::TaskFunction {
                parameters,
                result,
                effect,
            }) => format!(
                "{ty} task-function with {} parameters -> {result} ! {}",
                parameters.len(),
                effect.diagnostic()
            ),
            Ok(TypeForm::Function { parameters, result }) => format!(
                "{ty} pure-function with {} parameters -> {result}",
                parameters.len()
            ),
            _ => ty.to_string(),
        }
    }

    fn infer(
        &mut self,
        expression: ExpressionId,
        context: &ExecutionContext,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        let inferred = self
            .infer_at(expression, context, depth)
            .map_err(|mut error| {
                if !error
                    .notes
                    .iter()
                    .any(|note| note.starts_with("expression owner: "))
                {
                    error.notes.push(format!("expression owner: {expression}"));
                }
                error
            })?;
        if let Some((selected, ty)) = &mut self.selected_expression
            && *selected == expression
        {
            *ty = Some(inferred);
        }
        Ok(inferred)
    }

    fn scoped_context(
        &mut self,
        context: &ExecutionContext,
    ) -> Result<ExecutionContext, Diagnostic> {
        self.consume_work()?;
        for _ in context.bindings.keys() {
            self.consume_work()?;
        }
        for _ in &context.requirements {
            self.consume_work()?;
        }
        for _ in &context.effect_parameters {
            self.consume_work()?;
        }
        Ok(context.clone())
    }

    fn validate_target_port(
        &mut self,
        component: DeclarationReference,
        port: super::PortReference,
    ) -> Result<(), Diagnostic> {
        self.consume_work()?;
        if component.package != port.package {
            return Err(type_error(
                "kernel_full_target_package",
                "target component and port must belong to one package",
            ));
        }
        let ports = if component.package == self.read.package_id() {
            match self
                .read
                .owner(OwnerKey::Declaration(component.declaration))?
            {
                Some(OwnerRecord::Declaration(record)) => match record.payload {
                    DeclarationPayload::Component { ports, .. } => ports,
                    _ => Vec::new(),
                },
                _ => Vec::new(),
            }
        } else {
            match self.dependency_owner(
                component.package,
                OwnerKey::Declaration(component.declaration),
                "target component",
            )? {
                PackageInterfaceRecord::Declaration(record) => match record.payload {
                    PackageInterfaceDeclarationPayload::Component { ports, .. } => ports,
                    _ => Vec::new(),
                },
                _ => Vec::new(),
            }
        };
        self.consume_work()?;
        let parent = if port.package == self.read.package_id() {
            match self.read.owner(OwnerKey::Port(port.port))? {
                Some(OwnerRecord::Port(record)) => Some(record.declaration),
                _ => None,
            }
        } else {
            match self.dependency_owner(port.package, OwnerKey::Port(port.port), "target port")? {
                PackageInterfaceRecord::Port(record) => Some(record.declaration),
                _ => None,
            }
        };
        let mut listed = false;
        for candidate in ports {
            self.consume_work()?;
            listed |= candidate == port.port;
        }
        if !listed || parent != Some(component.declaration) {
            return Err(type_error(
                "kernel_full_target_port_owner",
                "target port does not belong to its component",
            ));
        }
        Ok(())
    }

    fn infer_at(
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
            ExpressionOperation::SequenceEmpty { sequence_type } => {
                self.sequence_item_type(sequence_type)?;
                Ok(sequence_type)
            }
            ExpressionOperation::SequenceLength {
                sequence_type,
                source,
            } => {
                self.sequence_item_type(sequence_type)?;
                let actual = self.sequence_source_type(source, context, next)?;
                require_same(
                    sequence_type,
                    actual,
                    "kernel_owned_sequence",
                    "sequence length source",
                )?;
                self.canonical_type(TypeForm::I64)
            }
            ExpressionOperation::SequencePush {
                sequence_type,
                value,
                source,
            } => {
                let item = self.sequence_item_type(sequence_type)?;
                let actual = self.infer(value, context, next)?;
                require_same(
                    item,
                    actual,
                    "kernel_owned_sequence",
                    "pushed sequence item",
                )?;
                let actual = self.sequence_source_type(source, context, next)?;
                require_same(
                    sequence_type,
                    actual,
                    "kernel_owned_sequence",
                    "sequence push source",
                )?;
                Ok(sequence_type)
            }
            ExpressionOperation::SequencePop {
                sequence_type,
                result_type,
                source,
            } => {
                let item = self.sequence_item_type(sequence_type)?;
                let actual = self.sequence_source_type(source, context, next)?;
                require_same(
                    sequence_type,
                    actual,
                    "kernel_owned_sequence",
                    "sequence pop source",
                )?;
                let item_result = self.canonical_type(TypeForm::OwnedProduct {
                    fields: vec![
                        StructuralTypeField {
                            name: super::Name::new("rest")?,
                            ty: sequence_type,
                        },
                        StructuralTypeField {
                            name: super::Name::new("value")?,
                            ty: item,
                        },
                    ],
                })?;
                let expected = self.canonical_type(TypeForm::OwnedChoice {
                    cases: vec![
                        StructuralTypeField {
                            name: super::Name::new("empty")?,
                            ty: sequence_type,
                        },
                        StructuralTypeField {
                            name: super::Name::new("item")?,
                            ty: item_result,
                        },
                    ],
                })?;
                require_same(
                    expected,
                    result_type,
                    "kernel_owned_sequence",
                    "sequence pop result annotation",
                )?;
                Ok(result_type)
            }
            ExpressionOperation::SequenceGet {
                sequence_type,
                source,
                index,
            } => {
                let item = self.sequence_item_type(sequence_type)?;
                if !self
                    .owned_read(|read| super::transfer::ordinary(read, item, context.declaration))?
                {
                    return Err(type_error(
                        "kernel_owned_sequence",
                        "sequence get requires ordinary first-order elements",
                    ));
                }
                let actual = self.infer(index, context, next)?;
                let i64_type = self.canonical_type(TypeForm::I64)?;
                require_same(
                    i64_type,
                    actual,
                    "kernel_owned_sequence",
                    "sequence get index",
                )?;
                let actual = self.sequence_source_type(source, context, next)?;
                require_same(
                    sequence_type,
                    actual,
                    "kernel_owned_sequence",
                    "sequence get source",
                )?;
                Ok(item)
            }
            ExpressionOperation::SequenceReplace {
                sequence_type,
                result_type,
                index,
                value,
                source,
            } => {
                let item = self.sequence_item_type(sequence_type)?;
                let actual = self.infer(index, context, next)?;
                let i64_type = self.canonical_type(TypeForm::I64)?;
                require_same(
                    i64_type,
                    actual,
                    "kernel_owned_sequence",
                    "sequence replace index",
                )?;
                let actual = self.infer(value, context, next)?;
                require_same(
                    item,
                    actual,
                    "kernel_owned_sequence",
                    "sequence replacement item",
                )?;
                let actual = self.sequence_source_type(source, context, next)?;
                require_same(
                    sequence_type,
                    actual,
                    "kernel_owned_sequence",
                    "sequence replace source",
                )?;
                let expected = self.canonical_type(TypeForm::OwnedProduct {
                    fields: vec![
                        StructuralTypeField {
                            name: super::Name::new("rest")?,
                            ty: sequence_type,
                        },
                        StructuralTypeField {
                            name: super::Name::new("value")?,
                            ty: item,
                        },
                    ],
                })?;
                require_same(
                    expected,
                    result_type,
                    "kernel_owned_sequence",
                    "sequence replace result annotation",
                )?;
                Ok(result_type)
            }
            ExpressionOperation::BorrowCall {
                call,
                binding,
                body,
            } => {
                let invocation = self.owned_read(|read| {
                    super::memory::borrow_invocation(read, call, context.declaration)
                })?;
                self.adopt_applied_types(invocation.types)?;
                // The wrapped invocation retains its complete ordinary type/effect checks.
                let actual = self.infer(call, context, next)?;
                require_same(
                    invocation.result,
                    actual,
                    "kernel_owned_borrow",
                    "borrowed invocation result",
                )?;
                let source = self.borrow_source_type(invocation.source, context, next)?;
                require_same(
                    invocation.source_type,
                    source,
                    "kernel_owned_borrow",
                    "borrowed invocation source",
                )?;
                self.require_borrow_binding(binding, invocation.result)?;
                let mut scoped = self.scoped_context(context)?;
                self.consume_work()?;
                scoped
                    .bindings
                    .insert(binding, (BindingKind::OwnedBorrow, invocation.result));
                self.infer(body, &scoped, next)
            }
            ExpressionOperation::BorrowOwnedItem {
                sequence_type,
                source,
                index,
                binding,
                body,
            } => {
                let item = self.sequence_item_type(sequence_type)?;
                if !self.owned_read(|read| super::memory::direct(read, item))? {
                    return Err(type_error(
                        "kernel_owned_borrow",
                        "owned item borrowing requires an owned element type",
                    ));
                }
                let actual = self.infer(index, context, next)?;
                let i64_type = self.canonical_type(TypeForm::I64)?;
                require_same(
                    i64_type,
                    actual,
                    "kernel_owned_borrow",
                    "borrowed item index",
                )?;
                let actual = self.borrow_source_type(source, context, next)?;
                require_same(
                    sequence_type,
                    actual,
                    "kernel_owned_borrow",
                    "borrowed sequence source",
                )?;
                self.require_borrow_binding(binding, item)?;
                let mut scoped = self.scoped_context(context)?;
                self.consume_work()?;
                scoped
                    .bindings
                    .insert(binding, (BindingKind::OwnedBorrow, item));
                self.infer(body, &scoped, next)
            }
            ExpressionOperation::BorrowOwnedField {
                product_type,
                source,
                field,
                binding,
                body,
            } => {
                let actual = self.borrow_source_type(source, context, next)?;
                require_same(
                    product_type,
                    actual,
                    "kernel_owned_borrow",
                    "borrowed product source",
                )?;
                let TypeForm::OwnedProduct { fields } = self.type_object(product_type)?.form else {
                    return Err(type_error(
                        "kernel_owned_borrow",
                        "field borrowing requires an owned product type",
                    ));
                };
                let mut selected = None;
                for candidate in fields {
                    self.consume_work()?;
                    if candidate.name == field {
                        selected = Some(candidate.ty);
                    }
                }
                let ty = selected.ok_or_else(|| {
                    type_error("kernel_owned_borrow", "unknown borrowed product field")
                })?;
                if !self.owned_read(|read| super::memory::direct(read, ty))? {
                    return Err(type_error(
                        "kernel_owned_borrow",
                        "borrowed product field must have a direct owned type",
                    ));
                }
                self.require_borrow_binding(binding, ty)?;
                let mut scoped = self.scoped_context(context)?;
                self.consume_work()?;
                scoped
                    .bindings
                    .insert(binding, (BindingKind::OwnedBorrow, ty));
                self.infer(body, &scoped, next)
            }
            ExpressionOperation::MatchBorrowedOwned {
                choice_type,
                source,
                arms,
            } => {
                let actual = self.borrow_source_type(source, context, next)?;
                require_same(
                    choice_type,
                    actual,
                    "kernel_owned_borrow",
                    "borrowed choice source",
                )?;
                let TypeForm::OwnedChoice { cases } = self.type_object(choice_type)?.form else {
                    return Err(type_error(
                        "kernel_owned_borrow",
                        "borrowed match requires an owned choice type",
                    ));
                };
                if cases.len() != arms.len() || arms.is_empty() {
                    return Err(type_error(
                        "kernel_owned_borrow",
                        "borrowed match must cover every case exactly once",
                    ));
                }
                let mut result = None;
                for (arm, case) in arms.into_iter().zip(cases) {
                    self.consume_work()?;
                    if arm.name != case.name {
                        return Err(type_error(
                            "kernel_owned_borrow",
                            "borrowed match cases must match the exact choice cases",
                        ));
                    }
                    self.require_borrow_binding(arm.binding, case.ty)?;
                    let mut scoped = self.scoped_context(context)?;
                    self.consume_work()?;
                    scoped
                        .bindings
                        .insert(arm.binding, (BindingKind::OwnedBorrow, case.ty));
                    let actual = self.infer(arm.body, &scoped, next)?;
                    if let Some(expected) = result {
                        require_same(
                            expected,
                            actual,
                            "kernel_owned_borrow",
                            "borrowed choice arm result",
                        )?;
                    } else {
                        result = Some(actual);
                    }
                }
                result.ok_or_else(|| {
                    type_error("kernel_owned_borrow", "borrowed match has no result")
                })
            }
            ExpressionOperation::ChooseOwned {
                choice_type,
                case,
                value,
            } => {
                let TypeForm::OwnedChoice { cases } = self.type_object(choice_type)?.form else {
                    return Err(type_error(
                        "kernel_owned_choice",
                        "choice construction requires an owned choice type",
                    ));
                };
                let mut payload = None;
                for candidate in cases {
                    self.consume_work()?;
                    if candidate.name == case {
                        payload = Some(candidate.ty);
                    }
                }
                let payload = payload.ok_or_else(|| {
                    type_error("kernel_owned_choice", "unknown owned choice case")
                })?;
                let actual = self.infer(value, context, next)?;
                require_same(payload, actual, "kernel_owned_choice", "selected payload")?;
                Ok(choice_type)
            }
            ExpressionOperation::MatchOwned {
                choice_type,
                source,
                arms,
            } => {
                let actual = self.infer(source, context, next)?;
                require_same(choice_type, actual, "kernel_owned_choice", "choice source")?;
                let TypeForm::OwnedChoice { cases } = self.type_object(choice_type)?.form else {
                    return Err(type_error(
                        "kernel_owned_choice",
                        "owned match requires an owned choice type",
                    ));
                };
                if cases.len() != arms.len() || arms.is_empty() {
                    return Err(type_error(
                        "kernel_owned_choice",
                        "owned match must cover every case exactly once",
                    ));
                }
                let mut result = None;
                for (arm, case) in arms.into_iter().zip(cases) {
                    self.consume_work()?;
                    let Some(OwnerRecord::Binding(binding)) =
                        self.read.owner(OwnerKey::Binding(arm.binding))?
                    else {
                        return Err(type_error(
                            "kernel_owned_choice",
                            "missing owned choice binding",
                        ));
                    };
                    if arm.name != case.name
                        || binding.kind != BindingKind::OwnedChoicePayload
                        || binding.value.is_some()
                        || binding.declared_type != Some(case.ty)
                    {
                        return Err(type_error(
                            "kernel_owned_choice",
                            "owned choice case, binding kind or payload type mismatch",
                        ));
                    }
                    let mut scoped = self.scoped_context(context)?;
                    self.consume_work()?;
                    scoped.bindings.insert(arm.binding, (binding.kind, case.ty));
                    let actual = self.infer(arm.body, &scoped, next)?;
                    if let Some(expected) = result {
                        require_same(
                            expected,
                            actual,
                            "kernel_owned_choice",
                            "owned choice arm result",
                        )?;
                    } else {
                        result = Some(actual);
                    }
                }
                result.ok_or_else(|| type_error("kernel_owned_choice", "owned match has no result"))
            }
            ExpressionOperation::PackOwned {
                product_type,
                fields,
            } => {
                let TypeForm::OwnedProduct { fields: expected } =
                    self.type_object(product_type)?.form
                else {
                    return Err(super::owned_product::reject(
                        "pack requires an owned product",
                    ));
                };
                if fields.len() != expected.len() {
                    return Err(super::owned_product::reject(
                        "pack must supply every field exactly once",
                    ));
                }
                let mut seen = BTreeSet::new();
                for field in fields {
                    let Some(contract) = expected.iter().find(|f| f.name == field.name) else {
                        return Err(super::owned_product::reject("unknown pack field"));
                    };
                    if !seen.insert(field.name) {
                        return Err(super::owned_product::reject("duplicate pack field"));
                    }
                    let actual = self.infer(field.value, context, next)?;
                    require_same(contract.ty, actual, "kernel_owned_product", "pack field")?;
                }
                Ok(product_type)
            }
            ExpressionOperation::UnpackOwned {
                product_type,
                source,
                fields,
                body,
            } => {
                let actual = self.infer(source, context, next)?;
                require_same(
                    product_type,
                    actual,
                    "kernel_owned_product",
                    "unpack source",
                )?;
                let TypeForm::OwnedProduct { fields: expected } =
                    self.type_object(product_type)?.form
                else {
                    return Err(super::owned_product::reject(
                        "unpack requires an owned product",
                    ));
                };
                if fields.len() != expected.len() {
                    return Err(super::owned_product::reject("unpack must bind every field"));
                }
                let mut scoped = self.scoped_context(context)?;
                for (field, contract) in fields.iter().zip(&expected) {
                    self.consume_work()?;
                    let Some(OwnerRecord::Binding(binding)) =
                        self.read.owner(OwnerKey::Binding(field.binding))?
                    else {
                        return Err(super::owned_product::reject("missing unpack binding"));
                    };
                    if field.name != contract.name
                        || binding.kind != BindingKind::OwnedUnpack
                        || binding.value.is_some()
                        || binding.declared_type != Some(contract.ty)
                    {
                        return Err(super::owned_product::reject(
                            "unpack field, binding kind or type mismatch",
                        ));
                    }
                    scoped
                        .bindings
                        .insert(field.binding, (binding.kind, contract.ty));
                }
                self.infer(body, &scoped, next)
            }
            ExpressionOperation::Unit {} => self.canonical_type(TypeForm::Unit),
            ExpressionOperation::Bool { .. } => self.canonical_type(TypeForm::Bool),
            ExpressionOperation::I64 { .. } => self.canonical_type(TypeForm::I64),
            ExpressionOperation::F64 { .. } => self.canonical_type(TypeForm::F64),
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
                let mut scoped = self.scoped_context(context)?;
                for binding in bindings {
                    self.consume_work()?;
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
                    let actual = self.infer(value, &scoped, next)?;
                    if let Some(expected) = binding_record.declared_type {
                        require_same(expected, actual, "kernel_type_binding", "let binding value")?;
                    }
                    self.consume_work()?;
                    scoped.bindings.insert(binding, (BindingKind::Let, actual));
                }
                self.infer(body, &scoped, next)
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
            ExpressionOperation::Parallel { left, right } => {
                if context.pure {
                    return Err(type_error(
                        "kernel_parallel_context",
                        "parallel requires a task context even when both effect rows are empty",
                    ));
                }
                let left_call = self.owned_read(|read| {
                    super::parallel::admit_call(read, left, context.declaration)
                })?;
                let right_call = self.owned_read(|read| {
                    super::parallel::admit_call(read, right, context.declaration)
                })?;
                // Both complete argument trees remain part of ordinary exact type,
                // scope and effect validation, including unreachable expressions.
                let left_type = self.infer(left, context, next)?;
                let right_type = self.infer(right, context, next)?;
                require_same(
                    left_call.result,
                    left_type,
                    "kernel_parallel_result",
                    "left child result",
                )?;
                require_same(
                    right_call.result,
                    right_type,
                    "kernel_parallel_result",
                    "right child result",
                )?;
                let fields = vec![
                    StructuralTypeField {
                        name: super::Name::new("left")?,
                        ty: left_type,
                    },
                    StructuralTypeField {
                        name: super::Name::new("right")?,
                        ty: right_type,
                    },
                ];
                if left_call.result_owned || right_call.result_owned {
                    let result = self.canonical_type(TypeForm::OwnedProduct { fields })?;
                    self.owned_read(|read| {
                        super::owned_product::validate(read, result, context.declaration)
                    })?;
                    Ok(result)
                } else {
                    self.canonical_type(TypeForm::StructuralRecord { fields })
                }
            }
            ExpressionOperation::ImplementationCall {
                function,
                type_arguments,
                effect_arguments,
                requirement_arguments,
                implementations,
                arguments,
            } => {
                let signature = self.function_signature(
                    function,
                    &type_arguments,
                    &effect_arguments,
                    &requirement_arguments,
                    &implementations,
                    context,
                )?;
                self.validate_call_effect(&signature, context)?;
                self.validate_arguments(&arguments, &signature.parameters, context, next)?;
                Ok(signature.result)
            }
            ExpressionOperation::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => {
                let application = self.owned_read(|read| {
                    super::owned_contract::method_signature(
                        read,
                        &witness,
                        contract,
                        method,
                        context.declaration,
                    )
                })?;
                self.adopt_applied_types(application.types)?;
                let signature = application.method;
                let row = signature.effect.row();
                self.validate_call_effect(
                    &FunctionSignature {
                        target: Some(contract),
                        parameters: Vec::new(),
                        result: signature.result,
                        task: !matches!(signature.effect, FunctionEffect::Pure),
                        requirements: row.requirements.into_iter().collect(),
                        effect_parameters: row.parameters.into_iter().collect(),
                    },
                    context,
                )?;
                let parameters = signature
                    .parameters
                    .iter()
                    .map(|p| p.ty)
                    .collect::<Vec<_>>();
                for ty in parameters.iter().chain([&signature.result]) {
                    self.validate_nominal_type(*ty, context, 0)?;
                }
                self.validate_arguments(&arguments, &parameters, context, next)?;
                Ok(signature.result)
            }
            ExpressionOperation::Call {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
            } => {
                let signature = self.function_signature(
                    function,
                    &type_arguments,
                    &effect_arguments,
                    &requirement_arguments,
                    &[],
                    context,
                )?;
                self.validate_call_effect(&signature, context)?;
                self.validate_arguments(&arguments, &signature.parameters, context, next)?;
                Ok(signature.result)
            }
            ExpressionOperation::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
            } => {
                let signature = self.function_signature(
                    function,
                    &type_arguments,
                    &effect_arguments,
                    &requirement_arguments,
                    &[],
                    context,
                )?;
                self.function_type(&signature)
            }
            ExpressionOperation::Bind { callee, arguments } => {
                let ty = self.infer(callee, context, next)?;
                let mut signature = self.callable_signature(ty)?;
                if arguments.len() > signature.parameters.len() {
                    return Err(type_error(
                        "kernel_type_bind_arity",
                        "bound prefix exceeds the remaining callable parameter count",
                    ));
                }
                for parameter in signature.parameters.iter().take(arguments.len()) {
                    self.require_capture_safe(*parameter, context)?;
                }
                self.validate_arguments(
                    &arguments,
                    &signature.parameters[..arguments.len()],
                    context,
                    next,
                )?;
                signature.parameters.drain(..arguments.len());
                self.function_type(&signature)
            }
            ExpressionOperation::Invoke { callee, arguments } => {
                let ty = self.infer(callee, context, next)?;
                let signature = self.callable_signature(ty)?;
                self.validate_call_effect(&signature, context)?;
                self.validate_arguments(&arguments, &signature.parameters, context, next)?;
                Ok(signature.result)
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
                let requirement_record = self.requirement_constraint(requirement)?;
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
                self.requirement_constraint(requirement)?;
                let ty = self.transaction_binding_type(binding)?;
                let mut scoped = self.scoped_context(context)?;
                self.consume_work()?;
                scoped
                    .bindings
                    .insert(binding, (BindingKind::Transaction, ty));
                self.infer(body, &scoped, next)
            }
            ExpressionOperation::TransactionOutcome {
                requirement,
                binding,
                body,
                outcome,
                type_argument,
            } => {
                outcome.validate_identity()?;
                if context.pure || !context.requirements.contains(&requirement) {
                    return Err(type_error(
                        "kernel_type_transaction_requirement",
                        "transaction-outcome requires its exact task allowance",
                    ));
                }
                let constraint = self.requirement_constraint(requirement)?;
                let interface = DeclarationReference {
                    package: outcome.outcome.package,
                    declaration: "decl_640e96fa57dee1c09557eb4bc7b53398".parse()?,
                };
                let operation = super::OperationReference {
                    package: outcome.outcome.package,
                    operation: "op_1c083402875f8f088541c27751f61d22".parse()?,
                };
                if constraint.interface != interface || !constraint.operations.contains(&operation)
                {
                    return Err(type_error(
                        "kernel_type_transaction_requirement",
                        "transaction-outcome requires the exact DataStore transaction operation",
                    ));
                }
                let ty = self.transaction_binding_type(binding)?;
                let mut scoped = self.scoped_context(context)?;
                self.consume_work()?;
                scoped
                    .bindings
                    .insert(binding, (BindingKind::Transaction, ty));
                let actual = self.infer(body, &scoped, next)?;
                require_same(
                    type_argument,
                    actual,
                    "kernel_type_transaction_outcome_payload",
                    "transaction-outcome body",
                )?;
                self.transaction_outcome_type(outcome, type_argument, context)
            }
        }
    }

    fn transaction_outcome_type(
        &mut self,
        contract: super::TransactionOutcomeContract,
        body: TypeObjectDigest,
        context: &ExecutionContext,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        let invalid = || {
            type_error(
                "kernel_type_transaction_outcome_shape",
                "exact transaction outcome owners have an incompatible nominal contract",
            )
        };
        let parameters = self.nominal_parameters(contract.outcome)?;
        if parameters.len() != 1 || !self.nominal_parameters(contract.abort_reason)?.is_empty() {
            return Err(invalid());
        }
        let parameter = if contract.outcome.package == self.read.package_id() {
            match self.read.owner(OwnerKey::TypeParameter(parameters[0]))? {
                Some(OwnerRecord::TypeParameter(record)) => record,
                _ => return Err(invalid()),
            }
        } else {
            match self.dependency_owner(
                contract.outcome.package,
                OwnerKey::TypeParameter(parameters[0]),
                "transaction outcome parameter",
            )? {
                PackageInterfaceRecord::TypeParameter(record) => record,
                _ => return Err(invalid()),
            }
        };
        if parameter.declaration != contract.outcome.declaration
            || parameter.constraints != super::TypeParameterConstraints::None
        {
            return Err(invalid());
        }
        let outcome_cases: BTreeSet<_> =
            self.variant_cases(contract.outcome)?.into_iter().collect();
        let reason_cases: BTreeSet<_> = self
            .variant_cases(contract.abort_reason)?
            .into_iter()
            .collect();
        if outcome_cases != BTreeSet::from([contract.committed.case, contract.aborted.case])
            || reason_cases
                != BTreeSet::from([contract.condition_failed.case, contract.conflict.case])
        {
            return Err(invalid());
        }
        let committed = self.case_record(contract.committed.package, contract.committed.case)?;
        let aborted = self.case_record(contract.aborted.package, contract.aborted.case)?;
        let condition = self.case_record(
            contract.condition_failed.package,
            contract.condition_failed.case,
        )?;
        let conflict = self.case_record(contract.conflict.package, contract.conflict.case)?;
        if committed.declaration != contract.outcome.declaration
            || aborted.declaration != contract.outcome.declaration
            || condition.declaration != contract.abort_reason.declaration
            || conflict.declaration != contract.abort_reason.declaration
            || condition.payload.is_some()
            || conflict.payload.is_some()
        {
            return Err(invalid());
        }
        let payload = committed.payload.ok_or_else(invalid)?;
        if self.type_object(payload)?.form
            != (TypeForm::TypeParameter {
                parameter: parameters[0],
            })
        {
            return Err(invalid());
        }
        let reason = self.nominal_type(contract.abort_reason, &[])?;
        if aborted.payload != Some(reason) {
            return Err(invalid());
        }
        let bindings = self.nominal_bindings(contract.outcome, &[body])?;
        require_same(
            body,
            self.substitute(payload, &bindings, 0)?,
            "kernel_type_transaction_outcome_payload",
            "transaction outcome substitution",
        )?;
        let result = self.nominal_type(contract.outcome, &[body])?;
        self.validate_nominal_type(result, context, 0)?;
        Ok(result)
    }

    fn sequence_item_type(
        &self,
        sequence_type: TypeObjectDigest,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        let TypeForm::OwnedSequence { item } = self.type_object(sequence_type)?.form else {
            return Err(type_error(
                "kernel_owned_sequence",
                "sequence operation requires an owned sequence type",
            ));
        };
        Ok(item)
    }

    fn sequence_source_type(
        &mut self,
        expression: ExpressionId,
        context: &ExecutionContext,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        self.consume_work()?;
        if !matches!(
            self.read.owner(OwnerKey::Expression(expression))?,
            Some(OwnerRecord::Expression(record))
                if matches!(record.operation, ExpressionOperation::Local { .. })
        ) {
            return Err(type_error(
                "kernel_owned_sequence",
                "sequence operation requires an exact local source",
            ));
        }
        self.infer(expression, context, depth)
    }

    fn borrow_source_type(
        &mut self,
        expression: ExpressionId,
        context: &ExecutionContext,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        self.consume_work()?;
        if !matches!(
            self.read.owner(OwnerKey::Expression(expression))?,
            Some(OwnerRecord::Expression(record))
                if matches!(record.operation, ExpressionOperation::Local { .. })
        ) {
            return Err(type_error(
                "kernel_owned_borrow",
                "scoped borrowing requires an exact local source",
            ));
        }
        self.infer(expression, context, depth)
    }

    fn require_borrow_binding(
        &mut self,
        binding: BindingId,
        ty: TypeObjectDigest,
    ) -> Result<(), Diagnostic> {
        self.consume_work()?;
        let Some(OwnerRecord::Binding(record)) = self.read.owner(OwnerKey::Binding(binding))?
        else {
            return Err(type_error(
                "kernel_owned_borrow",
                "missing borrowed child binding",
            ));
        };
        if record.kind != BindingKind::OwnedBorrow
            || record.value.is_some()
            || record.declared_type != Some(ty)
        {
            return Err(type_error(
                "kernel_owned_borrow",
                "borrowed child binding kind or exact annotation mismatch",
            ));
        }
        Ok(())
    }

    fn local_type(
        &mut self,
        reference: LocalValueReference,
        context: &ExecutionContext,
        _depth: usize,
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
                Err(type_error(
                    "kernel_type_parameter_scope",
                    "operation parameter has no executable operation context",
                ))
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
                let Some((kind, ty)) = context.bindings.get(&binding).copied() else {
                    return Err(type_error(
                        "kernel_full_lexical_scope",
                        "local binding is outside its exact lexical scope",
                    ));
                };
                let valid_domain = match reference {
                    LocalValueReference::LexicalBinding(_) => matches!(
                        kind,
                        BindingKind::Let
                            | BindingKind::OwnedUnpack
                            | BindingKind::OwnedChoicePayload
                            | BindingKind::OwnedBorrow
                    ),
                    LocalValueReference::MatchPayload(_) => kind == BindingKind::MatchPayload,
                    _ => false,
                };
                if !valid_domain || record.kind != kind {
                    return Err(type_error(
                        "kernel_full_local_binding_domain",
                        "local reference uses the wrong binding domain",
                    ));
                }
                Ok(ty)
            }
            LocalValueReference::TransactionBinding(binding) => {
                if !matches!(
                    context.bindings.get(&binding),
                    Some((BindingKind::Transaction, _))
                ) {
                    return Err(type_error(
                        "kernel_full_lexical_scope",
                        "transaction binding is outside its exact lexical scope",
                    ));
                }
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
                // Ownership admission separately restricts products to an exact local
                // read loan and a closed ordinary metadata field.
                let (TypeForm::StructuralRecord { fields } | TypeForm::OwnedProduct { fields }) =
                    object.form
                else {
                    return Err(type_error(
                        "kernel_type_structural_field",
                        "structural field selection requires a structural record or owned product",
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
            let mut scoped = self.scoped_context(context)?;
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
                    self.consume_work()?;
                    scoped
                        .bindings
                        .insert(binding, (BindingKind::MatchPayload, expected));
                }
                (None, None) => {}
                _ => {
                    return Err(type_error(
                        "kernel_type_match_payload",
                        "match payload binding disagrees with the case payload",
                    ));
                }
            }
            let body = self.infer(arm.body, &scoped, depth)?;
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
                | TypeForm::F64
                | TypeForm::Bytes
                | TypeForm::Text
                | TypeForm::StaticText
                | TypeForm::Function { .. }
                | TypeForm::TaskFunction { .. } => Vec::new(),
                TypeForm::Secret
                | TypeForm::Stream { .. }
                | TypeForm::ByteBuffer
                | TypeForm::OwnedI64Cell
                | TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. }
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
                                && record.constraints.proves_capture_safe() =>
                        {
                            match self.read.owner(OwnerKey::Declaration(record.declaration))? {
                                Some(OwnerRecord::Declaration(declaration)) => {
                                    match declaration.payload {
                                        DeclarationPayload::Function(function) => {
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
        effect_arguments: &[super::EffectRow],
        requirement_arguments: &[super::RequirementOperand],
        implementations: &[super::ImplementationOperand],
        context: &ExecutionContext,
    ) -> Result<FunctionSignature, Diagnostic> {
        if let Some(f) = self
            .owned_read(|read| super::owned_contract::optional_function_contract(read, reference))?
        {
            if !f.implementation_parameters.is_empty() || !implementations.is_empty() {
                self.owned_read(|read| {
                    super::owned_contract::validate_application(
                        read,
                        reference,
                        type_arguments,
                        implementations,
                        context.declaration,
                    )
                })?;
            }
        } else if !implementations.is_empty() {
            return Err(type_error(
                "kernel_owned_contract",
                "external cannot accept implementation witnesses",
            ));
        }
        let foreign = reference.package != self.read.package_id();
        let (
            type_parameters,
            requirement_parameters,
            effect_parameters,
            parameters,
            result,
            requirements,
            task,
            symbolic,
        ) = if foreign {
            let record = self.dependency_owner(
                reference.package,
                OwnerKey::Declaration(reference.declaration),
                "function",
            )?;
            match record {
                PackageInterfaceRecord::Declaration(record) => match record.payload {
                    PackageInterfaceDeclarationPayload::External(function) => (
                        function.type_parameters,
                        Vec::new(),
                        Vec::new(),
                        function.parameters,
                        function.result,
                        BTreeSet::new(),
                        false,
                        Vec::new(),
                    ),
                    PackageInterfaceDeclarationPayload::Function(function) => {
                        let symbolic = function.effect.row().parameters;
                        let (requirements, task) = match function.effect {
                            FunctionEffect::Pure => (BTreeSet::new(), false),
                            FunctionEffect::Task {
                                effect_parameters: _,
                                requirements,
                            } => {
                                for requirement in &requirements {
                                    self.requirement_constraint(*requirement)?;
                                }
                                (requirements.into_iter().collect(), true)
                            }
                        };
                        (
                            function.type_parameters,
                            function.requirement_parameters,
                            function.effect_parameters,
                            function.parameters,
                            function.result,
                            requirements,
                            task,
                            symbolic,
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
                    Vec::new(),
                    Vec::new(),
                    function.parameters,
                    function.result,
                    BTreeSet::new(),
                    false,
                    Vec::new(),
                ),
                DeclarationPayload::Function(function) => {
                    let symbolic = function.effect.row().parameters;
                    let (requirements, task) = match function.effect {
                        FunctionEffect::Pure => (BTreeSet::new(), false),
                        FunctionEffect::Task {
                            effect_parameters: _,
                            requirements,
                        } => {
                            for requirement in &requirements {
                                self.requirement_constraint(*requirement)?;
                            }
                            (requirements.into_iter().collect(), true)
                        }
                    };
                    (
                        function.type_parameters,
                        function.requirement_parameters,
                        function.effect_parameters,
                        function.parameters,
                        function.result,
                        requirements,
                        task,
                        symbolic,
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
        if requirement_parameters.len() != requirement_arguments.len() {
            return Err(type_error(
                "kernel_requirement_argument_count",
                "function requires exactly one explicit argument per requirement parameter",
            ));
        }
        let mut requirements_substitution = super::RequirementSubstitution::new();
        for (parameter, argument) in requirement_parameters.iter().zip(requirement_arguments) {
            self.consume_work()?;
            let formal = super::RequirementParameterReference {
                package: reference.package,
                parameter: *parameter,
            };
            let record = self.requirement_parameter_record(formal)?;
            if record.declaration != reference.declaration {
                return Err(type_error(
                    "kernel_requirement_parameter_scope",
                    "formal requirement parameter has a different function owner",
                ));
            }
            self.validate_requirement_scope(*argument, context)?;
            if !self
                .requirement_constraint(*argument)?
                .entails(&record.constraint)?
            {
                return Err(type_error(
                    "kernel_requirement_argument_constraint",
                    "requirement argument does not entail the exact interface and minimum operations",
                ));
            }
            requirements_substitution.insert(formal, *argument);
        }
        if effect_parameters.len() != effect_arguments.len() {
            return Err(type_error(
                "kernel_effect_argument_count",
                format!(
                    "function {}/{} expects {} effect arguments; supplied {}",
                    reference.package,
                    reference.declaration,
                    effect_parameters.len(),
                    effect_arguments.len()
                ),
            ));
        }
        for row in effect_arguments {
            self.validate_effect_scope(row, context)?;
        }
        let effects = effect_parameters
            .iter()
            .zip(effect_arguments)
            .map(|(parameter, row)| {
                (
                    super::EffectParameterReference {
                        package: reference.package,
                        parameter: *parameter,
                    },
                    row.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let row = super::EffectRow {
            requirements: requirements.into_iter().collect(),
            parameters: symbolic,
        }
        .substitute_requirements(&requirements_substitution, |n| {
            for _ in 0..n {
                self.consume_work()?;
            }
            Ok(())
        })?
        .substitute(&effects, |n| {
            for _ in 0..n {
                self.consume_work()?;
            }
            Ok(())
        })?;
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
            if owner.constraints.has_owned() {
                if !super::memory::direct(self.read, *supplied)? {
                    return Err(type_error(
                        "kernel_owned_constraint",
                        "owned parameter requires a direct owned type or exact owned parameter",
                    ));
                }
            } else if self.type_contains_buffer(*supplied)? {
                return Err(type_error(
                    "kernel_buffer_generic",
                    "owned memory cannot substitute an ordinary generic parameter",
                ));
            }
            if owner.constraints.requires_capture_safe() {
                self.require_capture_safe(*supplied, context).map_err(|error| {
                    if error.code != "kernel_type_bind_capture" { return error; }
                    type_error("kernel_type_constraint", format!("callee {}/{} parameter {} ({}) requires capture-safe; supplied {}: {}", reference.package, reference.declaration, parameter, owner.name, supplied, error.message))
                })?;
            }
            if owner.constraints.requires_transfer() {
                self.owned_read(|read| {
                    super::transfer::admit(read, *supplied, context.declaration)
                })?;
            }
            if owner.constraints.requires_share() {
                self.owned_read(|read| super::share::admit(read, *supplied, context.declaration))?;
            }
        }
        let substitutions = type_parameters
            .into_iter()
            .zip(type_arguments.iter().copied())
            .collect::<BTreeMap<_, _>>();
        let mut parameter_types = self.parameter_types(reference.package, &parameters)?;
        for parameter in &mut parameter_types {
            *parameter =
                self.substitute_effects(*parameter, &effects, &requirements_substitution, 0)?;
            *parameter = self.substitute(*parameter, &substitutions, 0)?;
            self.validate_nominal_type(*parameter, context, 0)?;
        }
        let result = self.substitute_effects(result, &effects, &requirements_substitution, 0)?;
        let result = self.substitute(result, &substitutions, 0)?;
        // Substitution can form an application that has no persisted TypeObject,
        // including a discarded phantom result or a function-value signature.
        self.validate_nominal_type(result, context, 0)?;
        Ok(FunctionSignature {
            target: Some(reference),
            parameters: parameter_types,
            result,
            requirements: row.requirements.into_iter().collect(),
            effect_parameters: row.parameters.into_iter().collect(),
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
                format!(
                    "pure expression in {} calls task {}; task kind requires a task context even for an empty row",
                    context
                        .declaration
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "expression scope".into()),
                    signature
                        .target
                        .map(|target| format!("{}/{}", target.package, target.declaration))
                        .unwrap_or_else(|| "indirect callable".into())
                ),
            ));
        }
        let required = super::EffectRow {
            requirements: signature.requirements.iter().copied().collect(),
            parameters: signature.effect_parameters.iter().copied().collect(),
        };
        let available = super::EffectRow {
            requirements: context.requirements.iter().copied().collect(),
            parameters: context.effect_parameters.iter().copied().collect(),
        };
        if !required.is_contained_by(&available, |required, available| {
            Ok(super::requirement_is_covered_by(
                required.package,
                &self.requirement_record(required)?,
                available.package,
                &self.requirement_record(available)?,
            ))
        })? {
            return Err(type_error(
                "kernel_type_task_requirement",
                format!(
                    "task {} requires {}; activation {} allows {}: package, operation, limit, or symbolic scope coverage is missing",
                    signature
                        .target
                        .map(|target| format!("{}/{}", target.package, target.declaration))
                        .unwrap_or_else(|| "indirect callable".into()),
                    required.diagnostic(),
                    context
                        .declaration
                        .map(|id| id.to_string())
                        .unwrap_or_else(|| "expression scope".into()),
                    available.diagnostic()
                ),
            ));
        }
        Ok(())
    }

    fn callable_signature(&self, ty: TypeObjectDigest) -> Result<FunctionSignature, Diagnostic> {
        let (parameters, result, task, row) = match self.type_object(ty)?.form {
            TypeForm::Function { parameters, result } => {
                (parameters, result, false, super::EffectRow::default())
            }
            TypeForm::TaskFunction {
                parameters,
                result,
                effect,
            } => (parameters, result, true, effect),
            _ => {
                return Err(type_error(
                    "kernel_type_invoke",
                    "callee is not a pure or task callable value",
                ));
            }
        };
        Ok(FunctionSignature {
            target: None,
            parameters,
            result,
            task,
            requirements: row.requirements.into_iter().collect(),
            effect_parameters: row.parameters.into_iter().collect(),
        })
    }

    fn function_type(
        &mut self,
        signature: &FunctionSignature,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        let parameters = signature.parameters.clone();
        let result = signature.result;
        self.canonical_type(if signature.task {
            TypeForm::TaskFunction {
                parameters,
                result,
                effect: super::EffectRow {
                    requirements: signature.requirements.iter().copied().collect(),
                    parameters: signature.effect_parameters.iter().copied().collect(),
                },
            }
        } else {
            TypeForm::Function { parameters, result }
        })
    }

    fn validate_effect_scope(
        &mut self,
        row: &super::EffectRow,
        context: &ExecutionContext,
    ) -> Result<(), Diagnostic> {
        row.validate()?;
        for requirement in &row.requirements {
            self.consume_work()?;
            self.validate_requirement_scope(*requirement, context)?;
        }
        for parameter in &row.parameters {
            self.consume_work()?;
            if parameter.package != self.read.package_id()
                || !matches!(self.read.owner(OwnerKey::EffectParameter(parameter.parameter))?,
                Some(OwnerRecord::EffectParameter(record)) if Some(record.declaration) == context.declaration)
            {
                return Err(type_error(
                    "kernel_effect_parameter_scope",
                    format!(
                        "effect parameter {}/{} is outside the caller's exact function scope",
                        parameter.package, parameter.parameter
                    ),
                ));
            }
        }
        Ok(())
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
                | TypeForm::TaskFunction { .. }
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
        if let OwnerRecord::RequirementParameter(parameter) = owner {
            let OwnerKey::RequirementParameter(id) = parameter.header.owner else {
                return Err(type_error(
                    "kernel_requirement_parameter_scope",
                    "requirement parameter has a foreign identity kind",
                ));
            };
            self.validate_requirement_parameter_contract(super::RequirementParameterReference {
                package: self.read.package_id(),
                parameter: id,
            })?;
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
        let mut roots = owner
            .type_roots()
            .into_iter()
            .map(|ty| (ty, owner.owner()))
            .collect::<Vec<_>>();
        if let OwnerRecord::Declaration(record) = owner {
            if let DeclarationPayload::Function(function) = &record.payload {
                for parameter in &function.requirement_parameters {
                    self.validate_requirement_parameter_contract(
                        super::RequirementParameterReference {
                            package: self.read.package_id(),
                            parameter: *parameter,
                        },
                    )?;
                }
                self.validate_effect_scope(&function.effect.row(), &context)?;
            }
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
                    roots.push((record.ty, record.header.owner));
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
                roots.extend(
                    self.nominal_member_types(reference)?
                        .into_iter()
                        .map(|ty| (ty, owner.owner())),
                );
                self.validate_nominal_schema(reference)?;
            }
        }
        for (ty, source) in roots {
            self.validate_nominal_type(ty, &context, 0)
                .map_err(|mut diagnostic| {
                    diagnostic.notes.push(format!("semantic owner: {source}"));
                    diagnostic
                })?;
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
        if matches!(
            object.form,
            TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. }
        ) {
            self.owned_read(|read| super::owned_product::validate(read, ty, context.declaration))?;
        }
        for child in object.child_types() {
            if !matches!(
                object.form,
                TypeForm::OwnedProduct { .. }
                    | TypeForm::OwnedChoice { .. }
                    | TypeForm::OwnedSequence { .. }
            ) && self.type_contains_buffer(child)?
            {
                return Err(type_error(
                    "kernel_buffer_container",
                    "ByteBuffer cannot occur in a container or callable descriptor",
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
                if self.type_contains_buffer(*argument)? {
                    return Err(type_error(
                        "kernel_buffer_generic",
                        "ByteBuffer cannot be a nominal generic argument",
                    ));
                }
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
                if owner.constraints.requires_capture_safe() {
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

    fn type_contains_buffer(&mut self, ty: TypeObjectDigest) -> Result<bool, Diagnostic> {
        let mut seen = BTreeSet::new();
        let mut declarations = BTreeSet::new();
        let mut pending = vec![ty];
        while let Some(ty) = pending.pop() {
            self.consume_work()?;
            if !seen.insert(ty) {
                continue;
            }
            let object = self.type_object(ty)?;
            if matches!(
                object.form,
                TypeForm::ByteBuffer
                    | TypeForm::OwnedI64Cell
                    | TypeForm::OwnedProduct { .. }
                    | TypeForm::OwnedChoice { .. }
                    | TypeForm::OwnedSequence { .. }
            ) || matches!(object.form, TypeForm::TypeParameter { parameter } if matches!(self.read.owner(OwnerKey::TypeParameter(parameter))?, Some(OwnerRecord::TypeParameter(p)) if p.constraints.has_owned()))
            {
                return Ok(true);
            }
            pending.extend(object.child_types());
            if let Some((declaration, arguments)) = nominal_parts(&object.form)
                && declarations.insert(declaration)
            {
                pending.extend(self.nominal_children(declaration, arguments)?);
            }
        }
        Ok(false)
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
                TypeForm::ByteBuffer
                | TypeForm::OwnedI64Cell
                | TypeForm::OwnedProduct { .. }
                | TypeForm::OwnedChoice { .. }
                | TypeForm::OwnedSequence { .. }
                | TypeForm::CapabilityResource { .. }
                | TypeForm::Stream { .. } => {
                    return Err(type_error(
                        "kernel_type_nominal_resource",
                        "applied nominal data cannot contain live resources, including phantom arguments and absent cases",
                    ));
                }
                TypeForm::Function { .. } | TypeForm::TaskFunction { .. } => Vec::new(),
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
            TypeForm::OwnedChoice { cases } => TypeForm::OwnedChoice {
                cases: cases
                    .into_iter()
                    .map(|case| {
                        Ok(StructuralTypeField {
                            name: case.name,
                            ty: self.substitute(case.ty, substitutions, next)?,
                        })
                    })
                    .collect::<Result<_, Diagnostic>>()?,
            },
            TypeForm::OwnedProduct { fields } => TypeForm::OwnedProduct {
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
            TypeForm::OwnedSequence { item } => TypeForm::OwnedSequence {
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
            TypeForm::TaskFunction {
                parameters,
                result,
                effect,
            } => TypeForm::TaskFunction {
                parameters: parameters
                    .into_iter()
                    .map(|ty| self.substitute(ty, substitutions, next))
                    .collect::<Result<_, _>>()?,
                result: self.substitute(result, substitutions, next)?,
                effect,
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

    fn substitute_effects(
        &mut self,
        digest: TypeObjectDigest,
        bindings: &BTreeMap<super::EffectParameterReference, super::EffectRow>,
        requirements: &super::RequirementSubstitution,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        if bindings.is_empty() && requirements.is_empty() {
            return Ok(digest);
        }
        self.consume_work()?;
        if depth > MAXIMUM_TYPE_DEPTH {
            return Err(type_error(
                "kernel_type_substitution_depth",
                "effect substitution exceeded its structural depth bound",
            ));
        }
        let mut object = self.type_object(digest)?;
        if let TypeForm::TaskFunction { effect, .. } = &mut object.form {
            *effect = effect
                .substitute_requirements(requirements, |n| {
                    for _ in 0..n {
                        self.consume_work()?;
                    }
                    Ok(())
                })?
                .substitute(bindings, |n| {
                    for _ in 0..n {
                        self.consume_work()?;
                    }
                    Ok(())
                })?;
        }
        let mut replace = |ty: &mut TypeObjectDigest| -> Result<(), Diagnostic> {
            *ty = self.substitute_effects(*ty, bindings, requirements, depth + 1)?;
            Ok(())
        };
        match &mut object.form {
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                for field in fields {
                    replace(&mut field.ty)?;
                }
            }
            TypeForm::Applied { arguments, .. } => {
                for ty in arguments {
                    replace(ty)?;
                }
            }
            TypeForm::List { item }
            | TypeForm::Option { item }
            | TypeForm::Stream { item }
            | TypeForm::OwnedSequence { item } => replace(item)?,
            TypeForm::Map { key, value }
            | TypeForm::Result {
                ok: key,
                error: value,
            } => {
                replace(key)?;
                replace(value)?;
            }
            TypeForm::Function { parameters, result }
            | TypeForm::TaskFunction {
                parameters, result, ..
            } => {
                for ty in parameters {
                    replace(ty)?;
                }
                replace(result)?;
            }
            _ => {}
        }
        self.canonical_type(object.form)
    }

    fn adopt_applied_types(
        &mut self,
        types: BTreeMap<TypeObjectDigest, TypeObject>,
    ) -> Result<(), Diagnostic> {
        for (digest, object) in types {
            self.consume_work()?;
            if self.canonical_type(object.form)? != digest {
                return Err(type_error(
                    "kernel_owned_contract",
                    "derived application type is not canonical",
                ));
            }
        }
        Ok(())
    }

    fn canonical_type(&mut self, form: TypeForm) -> Result<TypeObjectDigest, Diagnostic> {
        let object = TypeObject::new(form)?;
        let (digest, bytes) = super::codec::encode_type_object(&object)?;
        if !self.ephemeral_types.contains_key(&digest) {
            let children = match &object.form {
                TypeForm::StructuralRecord { fields }
                | TypeForm::OwnedProduct { fields }
                | TypeForm::OwnedChoice { cases: fields } => fields.len(),
                TypeForm::Applied { arguments, .. } => arguments.len(),
                TypeForm::Function { parameters, .. } => parameters.len(),
                TypeForm::TaskFunction {
                    parameters, effect, ..
                } => parameters
                    .len()
                    .saturating_add(effect.requirements.len())
                    .saturating_add(effect.parameters.len()),
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
    ) -> Result<BTreeSet<super::RequirementOperand>, Diagnostic> {
        let package = self.read.package_id();
        Ok(match self.read.owner(OwnerKey::Declaration(declaration))? {
            Some(OwnerRecord::Declaration(record)) => match &record.payload {
                DeclarationPayload::Component { requirements, .. } => requirements
                    .iter()
                    .copied()
                    .map(|requirement| {
                        RequirementReference {
                            package,
                            requirement,
                        }
                        .into()
                    })
                    .collect(),
                _ => BTreeSet::new(),
            },
            _ => BTreeSet::new(),
        })
    }

    fn requirement_parameter_record(
        &self,
        reference: super::RequirementParameterReference,
    ) -> Result<super::RequirementParameterRecord, Diagnostic> {
        if reference.package != self.read.package_id() {
            return match self.dependency_owner(
                reference.package,
                OwnerKey::RequirementParameter(reference.parameter),
                "requirement parameter",
            )? {
                PackageInterfaceRecord::RequirementParameter(record) => Ok(record),
                _ => Err(type_error(
                    "kernel_requirement_parameter_scope",
                    "requirement parameter has a foreign owner kind",
                )),
            };
        }
        match self
            .read
            .owner(OwnerKey::RequirementParameter(reference.parameter))?
        {
            Some(OwnerRecord::RequirementParameter(record)) => Ok(record),
            _ => Err(type_error(
                "kernel_requirement_parameter_scope",
                "exact requirement parameter is missing",
            )),
        }
    }

    fn validate_requirement_parameter_contract(
        &mut self,
        reference: super::RequirementParameterReference,
    ) -> Result<(), Diagnostic> {
        self.consume_work()?;
        let parameter = self.requirement_parameter_record(reference)?;
        parameter.constraint.validate()?;
        let listed = if reference.package == self.read.package_id() {
            matches!(self.read.owner(OwnerKey::Declaration(parameter.declaration))?,
                Some(OwnerRecord::Declaration(record)) if matches!(&record.payload, DeclarationPayload::Function(function) if function.requirement_parameters.contains(&reference.parameter)))
        } else {
            matches!(self.dependency_owner(reference.package, OwnerKey::Declaration(parameter.declaration), "requirement parameter function")?,
                PackageInterfaceRecord::Declaration(record) if matches!(&record.payload, PackageInterfaceDeclarationPayload::Function(function) if function.requirement_parameters.contains(&reference.parameter)))
        };
        if !listed {
            return Err(type_error(
                "kernel_requirement_parameter_scope",
                "requirement parameter is not listed by its exact function",
            ));
        }
        let interface = parameter.constraint.interface;
        let is_interface = if interface.package == self.read.package_id() {
            matches!(self.read.owner(OwnerKey::Declaration(interface.declaration))?, Some(OwnerRecord::Declaration(record)) if matches!(record.payload, DeclarationPayload::Interface { .. }))
        } else {
            matches!(self.dependency_owner(interface.package, OwnerKey::Declaration(interface.declaration), "requirement interface")?, PackageInterfaceRecord::Declaration(record) if matches!(record.payload, PackageInterfaceDeclarationPayload::Interface { .. }))
        };
        if !is_interface {
            return Err(type_error(
                "kernel_requirement_constraint_interface",
                "requirement constraint must name an exact interface",
            ));
        }
        for operation in &parameter.constraint.operations {
            self.consume_work()?;
            if operation.package != interface.package
                || self
                    .operation_record(operation.package, operation.operation)?
                    .declaration
                    != interface.declaration
            {
                return Err(type_error(
                    "kernel_requirement_constraint_operation",
                    "minimum operation belongs to another exact interface",
                ));
            }
        }
        Ok(())
    }

    fn requirement_constraint(
        &self,
        operand: super::RequirementOperand,
    ) -> Result<super::RequirementConstraint, Diagnostic> {
        match operand {
            super::RequirementOperand::Concrete(reference) => {
                let record = self.requirement_record(reference)?;
                Ok(super::RequirementConstraint {
                    interface: record.interface,
                    operations: record.operations,
                })
            }
            super::RequirementOperand::Parameter(reference) => {
                Ok(self.requirement_parameter_record(reference)?.constraint)
            }
        }
    }

    fn validate_requirement_scope(
        &self,
        operand: super::RequirementOperand,
        context: &ExecutionContext,
    ) -> Result<(), Diagnostic> {
        if let super::RequirementOperand::Parameter(reference) = operand {
            let record = self.requirement_parameter_record(reference)?;
            if reference.package != self.read.package_id()
                || Some(record.declaration) != context.declaration
            {
                return Err(type_error(
                    "kernel_requirement_parameter_scope",
                    "requirement parameter is outside the exact caller function scope",
                ));
            }
        }
        self.requirement_constraint(operand)?.validate()
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

    fn owned_read<T>(
        &mut self,
        action: impl FnOnce(&dyn ExpressionRead) -> Result<T, Diagnostic>,
    ) -> Result<T, Diagnostic> {
        let used = std::cell::Cell::from_mut(self.work);
        let maximum = self.limits.maximum_steps;
        let checkpoint = || {
            if used.get() >= maximum {
                return Err(Diagnostic::new(
                    DiagnosticClass::Resource,
                    "kernel_type_work",
                    "owned contract validation exhausted inference work",
                ));
            }
            used.set(used.get() + 1);
            Ok(())
        };
        let inferred = InferredTypeRead {
            read: self.read,
            types: &self.ephemeral_types,
        };
        let read = CheckedExpressionRead {
            read: &inferred,
            checkpoint: &checkpoint,
        };
        let result = action(&read);
        if result.as_ref().is_err_and(|e| e.code == "kernel_type_work") {
            self.exhaustion = Some(ExpressionValidationExhaustion::Steps);
        }
        result
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
        effect_parameters: BTreeSet::new(),
        bindings: BTreeMap::new(),
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
