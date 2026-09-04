//! Language-order affine capability-resource validation for Graph 10.

use super::contract::MAXIMUM_EXPRESSION_DEPTH;
use super::infer::{ExpressionRead, ExpressionValidationExhaustion, ExpressionValidationLimits};
use super::{
    BindingKind, BindingRecord, CaseRecord, DeclarationPayload, DeclarationReference,
    DeclarationVisibility, ExpressionOperation, FunctionDeclaration, FunctionEffect,
    LocalValueReference, OperationRecord, OwnerKey, OwnerRecord, PackageId,
    PackageInterfaceDeclarationPayload, PackageInterfaceRecord, ParameterRecord, ParameterUse,
    RequirementRecord, RequirementReference, TypeForm, TypeObjectDigest,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{BindingId, DeclarationId, ExpressionId, ParameterId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResourceShape {
    Direct,
    Variant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Provenance {
    requirement: RequirementReference,
    interface: DeclarationReference,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResourceValue {
    shape: ResourceShape,
    provenance: Provenance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EvaluatedValue {
    Unrestricted,
    Resource(ResourceValue),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResourceSlot {
    value: ResourceValue,
    live: bool,
}

type FlowState = BTreeMap<LocalValueReference, ResourceSlot>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResourceFunctionParameter {
    parameter: ParameterId,
    requirement: RequirementReference,
    interface: DeclarationReference,
    parameter_count: usize,
}

pub(super) fn validate_affine_meaning(
    snapshot: &super::KernelSnapshot,
    diagnostics: &mut Vec<Diagnostic>,
    work: &mut usize,
) {
    let roots = snapshot.owners.keys().copied().collect::<Vec<_>>();
    let _ = validate_affine_roots_with_limits(
        snapshot,
        roots,
        diagnostics,
        work,
        ExpressionValidationLimits {
            maximum_steps: super::contract::MAXIMUM_VALIDATION_WORK,
            maximum_diagnostics: usize::MAX,
        },
    );
}

pub(crate) fn validate_affine_roots_with_limits<R: ExpressionRead>(
    read: &R,
    roots: impl IntoIterator<Item = OwnerKey>,
    diagnostics: &mut Vec<Diagnostic>,
    work: &mut usize,
    limits: ExpressionValidationLimits,
) -> Result<(), ExpressionValidationExhaustion> {
    let mut validator = AffineValidator {
        read,
        work,
        maximum_steps: limits.maximum_steps,
        current_function: None,
    };
    for owner in roots {
        let record = match validator.read.owner(owner) {
            Ok(Some(record)) => record,
            Ok(None) => continue,
            Err(diagnostic) => {
                push_diagnostic(diagnostics, diagnostic, limits.maximum_diagnostics)?;
                continue;
            }
        };
        if let Err(diagnostic) = validator.validate_owner_shape(owner, &record) {
            push_diagnostic(diagnostics, diagnostic, limits.maximum_diagnostics)?;
            continue;
        }
        let OwnerRecord::Declaration(declaration) = record else {
            continue;
        };
        let DeclarationPayload::Function(function) = declaration.payload else {
            continue;
        };
        if !matches!(function.effect, FunctionEffect::Task { .. }) {
            continue;
        }
        let OwnerKey::Declaration(declaration) = owner else {
            continue;
        };
        validator.current_function = Some(declaration);
        let mut state = match validator.initial_state(declaration, &function) {
            Ok(state) => state,
            Err(diagnostic) => {
                push_diagnostic(diagnostics, diagnostic, limits.maximum_diagnostics)?;
                validator.current_function = None;
                continue;
            }
        };
        if let Err(diagnostic) = validator.evaluate(function.body, &mut state, 0) {
            if diagnostic.code == "kernel_affine_work" {
                return Err(ExpressionValidationExhaustion::Steps);
            }
            push_diagnostic(diagnostics, diagnostic, limits.maximum_diagnostics)?;
        }
        validator.current_function = None;
    }
    Ok(())
}

fn push_diagnostic(
    diagnostics: &mut Vec<Diagnostic>,
    diagnostic: Diagnostic,
    maximum: usize,
) -> Result<(), ExpressionValidationExhaustion> {
    if diagnostics.len() >= maximum {
        return Err(ExpressionValidationExhaustion::Diagnostics);
    }
    diagnostics.push(diagnostic);
    Ok(())
}

struct AffineValidator<'a, 'b, R: ?Sized> {
    read: &'a R,
    work: &'b mut usize,
    maximum_steps: usize,
    current_function: Option<DeclarationId>,
}

impl<R: ExpressionRead + ?Sized> AffineValidator<'_, '_, R> {
    fn initial_state(
        &mut self,
        declaration: DeclarationId,
        _function: &FunctionDeclaration,
    ) -> Result<FlowState, Diagnostic> {
        let mut state = FlowState::new();
        if let Some(parameter) = self.resource_function_parameter(DeclarationReference {
            package: self.read.package_id(),
            declaration,
        })? {
            state.insert(
                LocalValueReference::FunctionParameter(parameter.parameter),
                ResourceSlot {
                    value: ResourceValue {
                        shape: ResourceShape::Direct,
                        provenance: Provenance {
                            requirement: parameter.requirement,
                            interface: parameter.interface,
                        },
                    },
                    live: true,
                },
            );
        }
        Ok(state)
    }

    fn validate_owner_shape(
        &mut self,
        owner: OwnerKey,
        record: &OwnerRecord,
    ) -> Result<(), Diagnostic> {
        match record {
            OwnerRecord::Parameter(parameter) => {
                let resource = self.resource_type(parameter.ty)?;
                let contains = self.type_contains_resource(parameter.ty)?;
                match parameter.parent {
                    super::ParameterParent::Function(declaration) => {
                        if matches!(resource, Some((ResourceShape::Direct, _))) {
                            if parameter.use_mode != ParameterUse::Consume {
                                return Err(owner_affine_error(
                                    "kernel_affine_function_resource_use",
                                    owner,
                                    "direct resource function parameter must consume",
                                ));
                            }
                            if parameter.resource_requirement.is_none() {
                                return Err(owner_affine_error(
                                    "kernel_affine_function_resource_requirement",
                                    owner,
                                    "direct resource function parameter requires one exact requirement binding",
                                ));
                            }
                            self.resource_function_parameter(DeclarationReference {
                                package: self.read.package_id(),
                                declaration,
                            })?;
                        } else if contains {
                            return Err(owner_affine_error(
                                "kernel_affine_function_parameter_container",
                                owner,
                                "function parameter may contain a resource only as its direct type",
                            ));
                        } else if parameter.use_mode != ParameterUse::Unrestricted {
                            return Err(owner_affine_error(
                                "kernel_affine_function_parameter_use",
                                owner,
                                "nonresource function parameter must be unrestricted",
                            ));
                        } else if parameter.resource_requirement.is_some() {
                            return Err(owner_affine_error(
                                "kernel_affine_parameter_requirement_extra",
                                owner,
                                "nonresource function parameter cannot bind a resource requirement",
                            ));
                        }
                    }
                    super::ParameterParent::Operation(operation) => match resource {
                        Some((ResourceShape::Direct, interface)) => {
                            if parameter.resource_requirement.is_some() {
                                return Err(owner_affine_error(
                                    "kernel_affine_parameter_requirement_extra",
                                    owner,
                                    "operation parameter cannot bind a function resource requirement",
                                ));
                            }
                            let operation = self.operation(self.read.package_id(), operation)?;
                            if parameter.use_mode == ParameterUse::Unrestricted {
                                return Err(owner_affine_error(
                                    "kernel_affine_resource_parameter_use",
                                    owner,
                                    "direct resource operation parameter must borrow or consume",
                                ));
                            }
                            if interface.package != self.read.package_id()
                                || interface.declaration != operation.declaration
                            {
                                return Err(owner_affine_error(
                                    "kernel_affine_resource_parameter_interface",
                                    owner,
                                    "resource parameter is not bound to its operation's exact interface",
                                ));
                            }
                        }
                        _ if contains => {
                            return Err(owner_affine_error(
                                "kernel_affine_operation_parameter_container",
                                owner,
                                "operation resource parameter must use the direct resource type",
                            ));
                        }
                        _ if parameter.use_mode != ParameterUse::Unrestricted => {
                            return Err(owner_affine_error(
                                "kernel_affine_nonresource_parameter_use",
                                owner,
                                "nonresource operation parameter must be unrestricted",
                            ));
                        }
                        _ if parameter.resource_requirement.is_some() => {
                            return Err(owner_affine_error(
                                "kernel_affine_parameter_requirement_extra",
                                owner,
                                "operation parameter cannot bind a function resource requirement",
                            ));
                        }
                        _ => {}
                    },
                }
            }
            OwnerRecord::Field(field) => {
                if self.type_contains_resource(field.ty)? {
                    return Err(owner_affine_error(
                        "kernel_affine_record_field",
                        owner,
                        "record fields cannot contain capability resources",
                    ));
                }
            }
            OwnerRecord::Case(case) => {
                if let Some(payload) = case.payload
                    && self.type_contains_resource(payload)?
                    && !matches!(
                        self.resource_type(payload)?,
                        Some((ResourceShape::Direct, _))
                    )
                {
                    return Err(owner_affine_error(
                        "kernel_affine_variant_payload",
                        owner,
                        "variant cases may carry only one direct capability resource",
                    ));
                }
            }
            OwnerRecord::Operation(operation) => {
                let contains = self.type_contains_resource(operation.result)?;
                match self.resource_type(operation.result)? {
                    Some((_, interface))
                        if interface.package == self.read.package_id()
                            && interface.declaration == operation.declaration => {}
                    Some(_) => {
                        return Err(owner_affine_error(
                            "kernel_affine_operation_result_interface",
                            owner,
                            "resource result is not bound to its operation's exact interface",
                        ));
                    }
                    None if contains => {
                        return Err(owner_affine_error(
                            "kernel_affine_operation_result_container",
                            owner,
                            "operation result contains a forbidden resource container",
                        ));
                    }
                    None => {}
                }
            }
            OwnerRecord::Declaration(declaration) => match &declaration.payload {
                DeclarationPayload::External(signature) => {
                    if self.type_contains_resource(signature.result)? {
                        return Err(owner_affine_error(
                            "kernel_affine_external_result",
                            owner,
                            "external functions cannot return capability resources",
                        ));
                    }
                    for parameter in &signature.parameters {
                        let parameter = self.parameter(self.read.package_id(), *parameter)?;
                        if self.type_contains_resource(parameter.ty)?
                            || parameter.use_mode != ParameterUse::Unrestricted
                            || parameter.resource_requirement.is_some()
                        {
                            return Err(owner_affine_error(
                                "kernel_affine_external_parameter",
                                owner,
                                "external parameters cannot contain, borrow, consume, or bind capability resources",
                            ));
                        }
                    }
                }
                DeclarationPayload::Function(function) => {
                    if self.type_contains_resource(function.result)? {
                        return Err(owner_affine_error(
                            "kernel_affine_function_result",
                            owner,
                            "functions cannot return capability resources",
                        ));
                    }
                    let OwnerKey::Declaration(declaration) = owner else {
                        return Err(owner_affine_error(
                            "kernel_affine_function_identity",
                            owner,
                            "function has a foreign owner identity domain",
                        ));
                    };
                    self.resource_function_parameter(DeclarationReference {
                        package: self.read.package_id(),
                        declaration,
                    })?;
                }
                DeclarationPayload::Constant { ty, .. } => {
                    if self.type_contains_resource(*ty)? {
                        return Err(owner_affine_error(
                            "kernel_affine_constant",
                            owner,
                            "constants cannot contain capability resources",
                        ));
                    }
                }
                DeclarationPayload::Variant { cases } => {
                    let mut count = 0_usize;
                    for case in cases {
                        if let Some(OwnerRecord::Case(record)) =
                            self.read.owner(OwnerKey::Case(*case))?
                            && let Some(payload) = record.payload
                            && matches!(
                                self.resource_type(payload)?,
                                Some((ResourceShape::Direct, _))
                            )
                        {
                            count = count.saturating_add(1);
                        }
                    }
                    if count > 1 {
                        return Err(owner_affine_error(
                            "kernel_affine_variant_resource_count",
                            owner,
                            "nominal variant contains more than one direct resource payload",
                        ));
                    }
                }
                _ => {}
            },
            OwnerRecord::Port(port) if self.type_contains_resource(port.function_type)? => {
                return Err(owner_affine_error(
                    "kernel_affine_port_signature",
                    owner,
                    "ports cannot transfer capability resources",
                ));
            }
            _ => {}
        }
        Ok(())
    }

    fn evaluate(
        &mut self,
        expression: ExpressionId,
        state: &mut FlowState,
        depth: usize,
    ) -> Result<EvaluatedValue, Diagnostic> {
        self.step(expression, depth)?;
        let record = self.expression(expression)?;
        match record.operation {
            ExpressionOperation::Unit {}
            | ExpressionOperation::Bool { .. }
            | ExpressionOperation::I64 { .. }
            | ExpressionOperation::Text { .. }
            | ExpressionOperation::StaticText { .. }
            | ExpressionOperation::Constant { .. } => Ok(EvaluatedValue::Unrestricted),
            ExpressionOperation::FunctionValue { function, .. } => {
                if self.resource_function_parameter(function)?.is_some() {
                    return Err(affine_error(
                        "kernel_affine_resource_function_value",
                        expression,
                        "resource-bearing task functions can be used only by direct named call",
                    ));
                }
                Ok(EvaluatedValue::Unrestricted)
            }
            ExpressionOperation::Local { value } => {
                if let Some(owner) = resource_owner(value)
                    && state.contains_key(&owner)
                {
                    return Err(affine_error(
                        "kernel_affine_resource_copy",
                        expression,
                        format!(
                            "{} can be used only by an explicit borrow, consume, variant transfer, or match",
                            resource_owner_label(owner)
                        ),
                    ));
                }
                Ok(EvaluatedValue::Unrestricted)
            }
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                self.require_unrestricted(condition, state, depth + 1, "if condition")?;
                let before = state.clone();
                let mut true_state = before.clone();
                let true_value = self.evaluate(when_true, &mut true_state, depth + 1)?;
                let mut false_state = before.clone();
                let false_value = self.evaluate(when_false, &mut false_state, depth + 1)?;
                self.merge_branches(expression, &before, &true_state, &false_state, state)?;
                merge_values(expression, true_value, false_value)
            }
            ExpressionOperation::Let { bindings, body } => {
                let mut scoped = Vec::new();
                for binding in bindings {
                    let record = self.binding(binding)?;
                    let value_expression = record.value.ok_or_else(|| {
                        affine_error(
                            "kernel_affine_binding_value",
                            expression,
                            format!("let binding {binding} has no value"),
                        )
                    })?;
                    let value = self.evaluate(value_expression, state, depth + 1)?;
                    if let EvaluatedValue::Resource(resource) = value {
                        let declared = record.declared_type.ok_or_else(|| {
                            affine_error(
                                "kernel_affine_binding_annotation",
                                expression,
                                format!(
                                    "resource binding {binding} requires an exact declared type"
                                ),
                            )
                        })?;
                        let declared_shape = self.resource_type(declared)?;
                        if declared_shape != Some((resource.shape, resource.provenance.interface)) {
                            return Err(affine_error(
                                "kernel_affine_binding_shape",
                                expression,
                                format!(
                                    "resource binding {binding} annotation disagrees with its acquired shape or exact interface"
                                ),
                            ));
                        }
                        let owner = LocalValueReference::LexicalBinding(binding);
                        if state
                            .insert(
                                owner,
                                ResourceSlot {
                                    value: resource,
                                    live: true,
                                },
                            )
                            .is_some()
                        {
                            return Err(affine_error(
                                "kernel_affine_binding_alias",
                                expression,
                                format!("resource binding {binding} is introduced more than once"),
                            ));
                        }
                        scoped.push(owner);
                    }
                }
                let result = self.evaluate(body, state, depth + 1)?;
                for owner in scoped {
                    state.remove(&owner);
                }
                Ok(result)
            }
            ExpressionOperation::Sequence { items } => {
                let mut result = EvaluatedValue::Unrestricted;
                for item in items {
                    result = self.evaluate(item, state, depth + 1)?;
                }
                Ok(result)
            }
            ExpressionOperation::Call {
                function,
                arguments,
                ..
            } => self.evaluate_function_call(expression, function, &arguments, state, depth + 1),
            ExpressionOperation::Invoke { callee, arguments } => {
                self.require_unrestricted(callee, state, depth + 1, "invoked value")?;
                for argument in arguments {
                    self.require_unrestricted(argument, state, depth + 1, "function argument")?;
                }
                Ok(EvaluatedValue::Unrestricted)
            }
            ExpressionOperation::Record { fields, .. } => {
                for field in fields {
                    self.require_unrestricted(field.value, state, depth + 1, "record field")?;
                }
                Ok(EvaluatedValue::Unrestricted)
            }
            ExpressionOperation::Variant { case, payload } => {
                let case_record = self.case(case.package, case.case)?;
                let resource_payload = case_record
                    .payload
                    .map(|payload| self.resource_type(payload))
                    .transpose()?
                    .flatten();
                match (payload, resource_payload) {
                    (Some(payload), Some((ResourceShape::Direct, interface))) => {
                        let resource =
                            self.take_local_resource(payload, state, ParameterUse::Consume, None)?;
                        if resource.provenance.interface != interface {
                            return Err(affine_error(
                                "kernel_affine_variant_interface",
                                expression,
                                "variant payload resource has a foreign exact interface",
                            ));
                        }
                        Ok(EvaluatedValue::Resource(ResourceValue {
                            shape: ResourceShape::Variant,
                            provenance: resource.provenance,
                        }))
                    }
                    (Some(payload), None) => {
                        self.require_unrestricted(payload, state, depth + 1, "variant payload")?;
                        Ok(EvaluatedValue::Unrestricted)
                    }
                    (None, None) => Ok(EvaluatedValue::Unrestricted),
                    _ => Err(affine_error(
                        "kernel_affine_variant_payload",
                        expression,
                        "resource-bearing variant payload is malformed",
                    )),
                }
            }
            ExpressionOperation::Field { value, .. } => {
                self.require_unrestricted(value, state, depth + 1, "field value")?;
                Ok(EvaluatedValue::Unrestricted)
            }
            ExpressionOperation::List { items, .. } => {
                for item in items {
                    self.require_unrestricted(item, state, depth + 1, "list item")?;
                }
                Ok(EvaluatedValue::Unrestricted)
            }
            ExpressionOperation::Map { entries, .. } => {
                for entry in entries {
                    self.require_unrestricted(entry.key, state, depth + 1, "map key")?;
                    self.require_unrestricted(entry.value, state, depth + 1, "map value")?;
                }
                Ok(EvaluatedValue::Unrestricted)
            }
            ExpressionOperation::Match { value, arms } => {
                let matched = self.evaluate_match_value(value, state, depth + 1)?;
                let before = state.clone();
                let mut branch_states = Vec::with_capacity(arms.len());
                let mut branch_values = Vec::with_capacity(arms.len());
                for arm in arms {
                    let mut branch = before.clone();
                    let case = self.case(arm.case.package, arm.case.case)?;
                    let payload_resource = case
                        .payload
                        .map(|payload| self.resource_type(payload))
                        .transpose()?
                        .flatten();
                    let mut scoped = None;
                    if let Some((ResourceShape::Direct, interface)) = payload_resource {
                        let resource = matched.ok_or_else(|| {
                            affine_error(
                                "kernel_affine_match_provenance",
                                expression,
                                "resource-bearing match arm has no acquired outer resource",
                            )
                        })?;
                        if resource.provenance.interface != interface {
                            return Err(affine_error(
                                "kernel_affine_match_interface",
                                expression,
                                "match payload resource has a foreign exact interface",
                            ));
                        }
                        let binding = arm.payload_binding.ok_or_else(|| {
                            affine_error(
                                "kernel_affine_match_binding",
                                expression,
                                "resource-bearing match arm omits its payload binding",
                            )
                        })?;
                        let owner = LocalValueReference::MatchPayload(binding);
                        if branch
                            .insert(
                                owner,
                                ResourceSlot {
                                    value: ResourceValue {
                                        shape: ResourceShape::Direct,
                                        provenance: resource.provenance,
                                    },
                                    live: true,
                                },
                            )
                            .is_some()
                        {
                            return Err(affine_error(
                                "kernel_affine_binding_alias",
                                expression,
                                format!(
                                    "match payload binding {binding} aliases another live resource owner"
                                ),
                            ));
                        }
                        scoped = Some(owner);
                    }
                    let value = self.evaluate(arm.body, &mut branch, depth + 1)?;
                    if let Some(owner) = scoped {
                        branch.remove(&owner);
                    }
                    branch_states.push(branch);
                    branch_values.push(value);
                }
                self.merge_many_branches(expression, &before, &branch_states, state)?;
                merge_many_values(expression, &branch_values)
            }
            ExpressionOperation::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => self.evaluate_capability_call(
                expression,
                requirement,
                operation,
                &arguments,
                state,
                depth + 1,
            ),
            ExpressionOperation::Transaction { body, .. } => self.evaluate(body, state, depth + 1),
        }
    }

    fn evaluate_function_call(
        &mut self,
        expression: ExpressionId,
        function: DeclarationReference,
        arguments: &[ExpressionId],
        state: &mut FlowState,
        depth: usize,
    ) -> Result<EvaluatedValue, Diagnostic> {
        let Some(parameter) = self.resource_function_parameter(function)? else {
            for argument in arguments {
                self.require_unrestricted(*argument, state, depth, "function argument")?;
            }
            return Ok(EvaluatedValue::Unrestricted);
        };
        if function.package != self.read.package_id() {
            return Err(affine_error(
                "kernel_affine_resource_call_package",
                expression,
                "resource transfer requires a private same-package task function",
            ));
        }
        if arguments.len() != parameter.parameter_count {
            return Err(affine_error(
                "kernel_affine_resource_call_arguments",
                expression,
                "resource-bearing call argument count disagrees with its exact callee signature",
            ));
        }
        let Some((resource_argument, ordinary_arguments)) = arguments.split_last() else {
            return Err(affine_error(
                "kernel_affine_resource_call_arguments",
                expression,
                "resource-bearing call omits its final consume argument",
            ));
        };
        for argument in ordinary_arguments {
            self.require_unrestricted(*argument, state, depth, "function argument")?;
        }
        let resource = self.take_local_resource(
            *resource_argument,
            state,
            ParameterUse::Consume,
            Some(parameter.requirement),
        )?;
        if resource.provenance.interface != parameter.interface {
            return Err(affine_error(
                "kernel_affine_resource_call_interface",
                expression,
                "transferred resource has a foreign exact interface",
            ));
        }
        let current = self.current_function.ok_or_else(|| {
            affine_error(
                "kernel_affine_resource_call_scope",
                expression,
                "resource transfer is outside one exact task function",
            )
        })?;
        if function.declaration == current
            || self.resource_call_reaches(function.declaration, current)?
        {
            return Err(affine_error(
                "kernel_affine_resource_call_cycle",
                expression,
                "resource-bearing direct-call graph is cyclic",
            ));
        }
        Ok(EvaluatedValue::Unrestricted)
    }

    fn resource_call_reaches(
        &mut self,
        start: DeclarationId,
        target: DeclarationId,
    ) -> Result<bool, Diagnostic> {
        let mut pending = vec![start];
        let mut visited = BTreeSet::new();
        while let Some(declaration) = pending.pop() {
            if !visited.insert(declaration) {
                continue;
            }
            let function = self.local_function(declaration)?;
            for callee in self.resource_callees(function.body)? {
                if callee == target {
                    return Ok(true);
                }
                pending.push(callee);
            }
        }
        Ok(false)
    }

    fn resource_callees(&mut self, body: ExpressionId) -> Result<Vec<DeclarationId>, Diagnostic> {
        let mut callees = BTreeSet::new();
        let mut pending = vec![(body, 0_usize)];
        let mut visited = BTreeSet::new();
        while let Some((expression, depth)) = pending.pop() {
            if !visited.insert(expression) {
                continue;
            }
            self.step(expression, depth)?;
            let record = self.expression(expression)?;
            if let ExpressionOperation::Call { function, .. } = &record.operation
                && self.resource_function_parameter(*function)?.is_some()
            {
                if function.package != self.read.package_id() {
                    return Err(affine_error(
                        "kernel_affine_resource_call_package",
                        expression,
                        "resource transfer requires a private same-package task function",
                    ));
                }
                callees.insert(function.declaration);
            }
            pending.extend(
                record
                    .children()
                    .into_iter()
                    .map(|child| (child.expression, depth.saturating_add(1))),
            );
        }
        Ok(callees.into_iter().collect())
    }

    fn evaluate_capability_call(
        &mut self,
        expression: ExpressionId,
        requirement: RequirementReference,
        operation: super::OperationReference,
        arguments: &[ExpressionId],
        state: &mut FlowState,
        depth: usize,
    ) -> Result<EvaluatedValue, Diagnostic> {
        let requirement_record = self.requirement(requirement)?;
        let operation_record = self.operation(operation.package, operation.operation)?;
        if arguments.len() != operation_record.parameters.len() {
            return Err(affine_error(
                "kernel_affine_argument_count",
                expression,
                "capability argument count disagrees with its exact operation",
            ));
        }
        for (argument, parameter) in arguments.iter().zip(&operation_record.parameters) {
            let parameter = self.parameter(operation.package, *parameter)?;
            match parameter.use_mode {
                ParameterUse::Unrestricted => {
                    self.require_unrestricted(*argument, state, depth, "capability argument")?;
                }
                ParameterUse::Borrow | ParameterUse::Consume => {
                    let Some((ResourceShape::Direct, interface)) =
                        self.resource_type(parameter.ty)?
                    else {
                        return Err(affine_error(
                            "kernel_affine_parameter_shape",
                            expression,
                            "borrow or consume parameter is not a direct capability resource",
                        ));
                    };
                    let resource = self.take_local_resource(
                        *argument,
                        state,
                        parameter.use_mode,
                        Some(requirement),
                    )?;
                    if resource.provenance.interface != interface
                        || interface != requirement_record.interface
                    {
                        return Err(affine_error(
                            "kernel_affine_foreign_authority",
                            expression,
                            "resource argument does not belong to this exact requirement and interface",
                        ));
                    }
                }
            }
        }
        let Some((shape, interface)) = self.resource_type(operation_record.result)? else {
            return Ok(EvaluatedValue::Unrestricted);
        };
        if interface != requirement_record.interface
            || interface.package != operation.package
            || interface.declaration != operation_record.declaration
        {
            return Err(affine_error(
                "kernel_affine_acquisition_interface",
                expression,
                "capability operation resource result is not bound to its exact requirement interface",
            ));
        }
        Ok(EvaluatedValue::Resource(ResourceValue {
            shape,
            provenance: Provenance {
                requirement,
                interface,
            },
        }))
    }

    fn require_unrestricted(
        &mut self,
        expression: ExpressionId,
        state: &mut FlowState,
        depth: usize,
        role: &str,
    ) -> Result<(), Diagnostic> {
        if matches!(
            self.evaluate(expression, state, depth)?,
            EvaluatedValue::Resource(_)
        ) {
            return Err(affine_error(
                "kernel_affine_resource_escape",
                expression,
                format!("{role} cannot contain or copy a capability resource"),
            ));
        }
        Ok(())
    }

    fn take_local_resource(
        &mut self,
        expression: ExpressionId,
        state: &mut FlowState,
        use_mode: ParameterUse,
        requirement: Option<RequirementReference>,
    ) -> Result<ResourceValue, Diagnostic> {
        let record = self.expression(expression)?;
        let ExpressionOperation::Local { value } = record.operation else {
            return Err(affine_error(
                "kernel_affine_resource_argument",
                expression,
                "resource borrow or consume requires one exact lexical owner",
            ));
        };
        let owner = resource_owner(value).ok_or_else(|| {
            affine_error(
                "kernel_affine_resource_argument",
                expression,
                "resource argument does not name a function parameter, lexical, or match-payload owner",
            )
        })?;
        let label = resource_owner_label(owner);
        let slot = state.get_mut(&owner).ok_or_else(|| {
            affine_error(
                "kernel_affine_resource_fabricated",
                expression,
                format!("{label} owns no acquired capability resource"),
            )
        })?;
        if !slot.live {
            return Err(affine_error(
                "kernel_affine_use_after_consume",
                expression,
                format!("{label} was already consumed"),
            ));
        }
        if slot.value.shape != ResourceShape::Direct {
            return Err(affine_error(
                "kernel_affine_resource_shape",
                expression,
                format!("{label} is not a direct capability resource"),
            ));
        }
        if requirement.is_some_and(|expected| expected != slot.value.provenance.requirement) {
            return Err(affine_error(
                "kernel_affine_foreign_requirement",
                expression,
                format!("{label} was acquired from another exact requirement"),
            ));
        }
        if use_mode == ParameterUse::Consume {
            slot.live = false;
        }
        Ok(slot.value)
    }

    fn evaluate_match_value(
        &mut self,
        expression: ExpressionId,
        state: &mut FlowState,
        depth: usize,
    ) -> Result<Option<ResourceValue>, Diagnostic> {
        let record = self.expression(expression)?;
        if let ExpressionOperation::Local { value } = record.operation
            && let Some(owner) = resource_owner(value)
            && let Some(slot) = state.get_mut(&owner)
        {
            let label = resource_owner_label(owner);
            if !slot.live {
                return Err(affine_error(
                    "kernel_affine_use_after_consume",
                    expression,
                    format!("{label} was already consumed"),
                ));
            }
            if slot.value.shape != ResourceShape::Variant {
                return Err(affine_error(
                    "kernel_affine_match_shape",
                    expression,
                    "only a resource-bearing nominal variant transfers an affine match payload",
                ));
            }
            slot.live = false;
            return Ok(Some(slot.value));
        }
        match self.evaluate(expression, state, depth)? {
            EvaluatedValue::Unrestricted => Ok(None),
            EvaluatedValue::Resource(value) if value.shape == ResourceShape::Variant => {
                Ok(Some(value))
            }
            EvaluatedValue::Resource(_) => Err(affine_error(
                "kernel_affine_match_shape",
                expression,
                "direct capability resource cannot be matched as a nominal variant",
            )),
        }
    }

    fn merge_branches(
        &self,
        expression: ExpressionId,
        before: &FlowState,
        left: &FlowState,
        right: &FlowState,
        output: &mut FlowState,
    ) -> Result<(), Diagnostic> {
        self.merge_many_branches(expression, before, &[left.clone(), right.clone()], output)
    }

    fn merge_many_branches(
        &self,
        expression: ExpressionId,
        before: &FlowState,
        branches: &[FlowState],
        output: &mut FlowState,
    ) -> Result<(), Diagnostic> {
        *output = before.clone();
        for (owner, prior) in before {
            let Some(first) = branches.first().and_then(|branch| branch.get(owner)) else {
                continue;
            };
            if first.value != prior.value
                || branches
                    .iter()
                    .any(|branch| branch.get(owner).map(|slot| slot.live) != Some(first.live))
            {
                return Err(affine_error(
                    "kernel_affine_branch_join",
                    expression,
                    format!(
                        "{} is not preserved with identical provenance on every reachable branch",
                        resource_owner_label(*owner)
                    ),
                ));
            }
            if let Some(slot) = output.get_mut(owner) {
                slot.live = first.live;
            }
        }
        Ok(())
    }

    fn resource_type(
        &mut self,
        digest: TypeObjectDigest,
    ) -> Result<Option<(ResourceShape, DeclarationReference)>, Diagnostic> {
        self.resource_type_inner(digest, &mut BTreeSet::new(), &mut BTreeSet::new())
    }

    fn type_contains_resource(&mut self, digest: TypeObjectDigest) -> Result<bool, Diagnostic> {
        self.type_contains_resource_inner(digest, &mut BTreeSet::new(), &mut BTreeSet::new())
    }

    fn type_contains_resource_inner(
        &mut self,
        digest: TypeObjectDigest,
        active_types: &mut BTreeSet<TypeObjectDigest>,
        active_declarations: &mut BTreeSet<(
            PackageId,
            crate::platform::semantic_id::DeclarationId,
        )>,
    ) -> Result<bool, Diagnostic> {
        if !active_types.insert(digest) {
            return Ok(false);
        }
        let object = self.read.type_object(digest)?.ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Corrupt,
                "kernel_affine_type_missing",
                format!("affine validation cannot read exact type object {digest}"),
            )
        })?;
        let result = match object.form {
            TypeForm::CapabilityResource { .. } => true,
            TypeForm::Named { declaration } => {
                let key = (declaration.package, declaration.declaration);
                if !active_declarations.insert(key) {
                    false
                } else {
                    let mut contains = false;
                    for member in self.named_member_types(declaration)? {
                        if self.type_contains_resource_inner(
                            member,
                            active_types,
                            active_declarations,
                        )? {
                            contains = true;
                            break;
                        }
                    }
                    active_declarations.remove(&key);
                    contains
                }
            }
            TypeForm::StructuralRecord { fields } => {
                let mut contains = false;
                for field in fields {
                    if self.type_contains_resource_inner(
                        field.ty,
                        active_types,
                        active_declarations,
                    )? {
                        contains = true;
                        break;
                    }
                }
                contains
            }
            TypeForm::List { item } | TypeForm::Option { item } | TypeForm::Stream { item } => {
                self.type_contains_resource_inner(item, active_types, active_declarations)?
            }
            TypeForm::Map { key, value }
            | TypeForm::Result {
                ok: key,
                error: value,
            } => {
                self.type_contains_resource_inner(key, active_types, active_declarations)?
                    || self.type_contains_resource_inner(
                        value,
                        active_types,
                        active_declarations,
                    )?
            }
            TypeForm::Function { parameters, result } => {
                let mut contains =
                    self.type_contains_resource_inner(result, active_types, active_declarations)?;
                for parameter in parameters {
                    contains |= self.type_contains_resource_inner(
                        parameter,
                        active_types,
                        active_declarations,
                    )?;
                }
                contains
            }
            _ => false,
        };
        active_types.remove(&digest);
        Ok(result)
    }

    fn resource_type_inner(
        &mut self,
        digest: TypeObjectDigest,
        active_types: &mut BTreeSet<TypeObjectDigest>,
        active_declarations: &mut BTreeSet<(
            PackageId,
            crate::platform::semantic_id::DeclarationId,
        )>,
    ) -> Result<Option<(ResourceShape, DeclarationReference)>, Diagnostic> {
        if !active_types.insert(digest) {
            return Ok(None);
        }
        let object = self.read.type_object(digest)?.ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Corrupt,
                "kernel_affine_type_missing",
                format!("affine validation cannot read exact type object {digest}"),
            )
        })?;
        let result = match object.form {
            TypeForm::CapabilityResource { interface } => Some((ResourceShape::Direct, interface)),
            TypeForm::Named { declaration } => {
                let key = (declaration.package, declaration.declaration);
                if !active_declarations.insert(key) {
                    None
                } else {
                    let mut found = None;
                    for payload in self.variant_payloads(declaration)? {
                        if let Some((ResourceShape::Direct, interface)) =
                            self.resource_type_inner(payload, active_types, active_declarations)?
                        {
                            if found.is_some_and(|previous| previous != interface) {
                                return Err(Diagnostic::new(
                                    DiagnosticClass::Semantic,
                                    "kernel_affine_variant_interfaces",
                                    "resource-bearing variant contains multiple exact interfaces",
                                ));
                            }
                            found = Some(interface);
                        }
                    }
                    active_declarations.remove(&key);
                    found.map(|interface| (ResourceShape::Variant, interface))
                }
            }
            _ => None,
        };
        active_types.remove(&digest);
        Ok(result)
    }

    fn variant_payloads(
        &mut self,
        declaration: DeclarationReference,
    ) -> Result<Vec<TypeObjectDigest>, Diagnostic> {
        if declaration.package == self.read.package_id() {
            let Some(OwnerRecord::Declaration(record)) = self
                .read
                .owner(OwnerKey::Declaration(declaration.declaration))?
            else {
                return Ok(Vec::new());
            };
            let DeclarationPayload::Variant { cases } = record.payload else {
                return Ok(Vec::new());
            };
            let mut payloads = Vec::new();
            for case in cases {
                if let Some(OwnerRecord::Case(record)) = self.read.owner(OwnerKey::Case(case))?
                    && let Some(payload) = record.payload
                {
                    payloads.push(payload);
                }
            }
            return Ok(payloads);
        }
        let Some(PackageInterfaceRecord::Declaration(record)) = self.read.package_interface_owner(
            declaration.package,
            OwnerKey::Declaration(declaration.declaration),
        )?
        else {
            return Ok(Vec::new());
        };
        let PackageInterfaceDeclarationPayload::Variant { cases } = record.payload else {
            return Ok(Vec::new());
        };
        let mut payloads = Vec::new();
        for case in cases {
            if let Some(PackageInterfaceRecord::Case(record)) = self
                .read
                .package_interface_owner(declaration.package, OwnerKey::Case(case))?
                && let Some(payload) = record.payload
            {
                payloads.push(payload);
            }
        }
        Ok(payloads)
    }

    fn named_member_types(
        &mut self,
        declaration: DeclarationReference,
    ) -> Result<Vec<TypeObjectDigest>, Diagnostic> {
        if declaration.package == self.read.package_id() {
            let Some(OwnerRecord::Declaration(record)) = self
                .read
                .owner(OwnerKey::Declaration(declaration.declaration))?
            else {
                return Ok(Vec::new());
            };
            let members = match record.payload {
                DeclarationPayload::Record { fields } => fields
                    .into_iter()
                    .filter_map(|field| match self.read.owner(OwnerKey::Field(field)) {
                        Ok(Some(OwnerRecord::Field(record))) => Some(Ok(record.ty)),
                        Ok(_) => None,
                        Err(diagnostic) => Some(Err(diagnostic)),
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                DeclarationPayload::Variant { cases } => cases
                    .into_iter()
                    .filter_map(|case| match self.read.owner(OwnerKey::Case(case)) {
                        Ok(Some(OwnerRecord::Case(record))) => record.payload.map(Ok),
                        Ok(_) => None,
                        Err(diagnostic) => Some(Err(diagnostic)),
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                _ => Vec::new(),
            };
            return Ok(members);
        }
        let Some(PackageInterfaceRecord::Declaration(record)) = self.read.package_interface_owner(
            declaration.package,
            OwnerKey::Declaration(declaration.declaration),
        )?
        else {
            return Ok(Vec::new());
        };
        match record.payload {
            PackageInterfaceDeclarationPayload::Record { fields } => fields
                .into_iter()
                .filter_map(|field| {
                    match self
                        .read
                        .package_interface_owner(declaration.package, OwnerKey::Field(field))
                    {
                        Ok(Some(PackageInterfaceRecord::Field(record))) => Some(Ok(record.ty)),
                        Ok(_) => None,
                        Err(diagnostic) => Some(Err(diagnostic)),
                    }
                })
                .collect(),
            PackageInterfaceDeclarationPayload::Variant { cases } => cases
                .into_iter()
                .filter_map(|case| {
                    match self
                        .read
                        .package_interface_owner(declaration.package, OwnerKey::Case(case))
                    {
                        Ok(Some(PackageInterfaceRecord::Case(record))) => record.payload.map(Ok),
                        Ok(_) => None,
                        Err(diagnostic) => Some(Err(diagnostic)),
                    }
                })
                .collect(),
            _ => Ok(Vec::new()),
        }
    }

    fn expression(
        &mut self,
        expression: ExpressionId,
    ) -> Result<super::ExpressionRecord, Diagnostic> {
        match self.read.owner(OwnerKey::Expression(expression))? {
            Some(OwnerRecord::Expression(record)) => Ok(record),
            _ => Err(Diagnostic::new(
                DiagnosticClass::Corrupt,
                "kernel_affine_expression_missing",
                format!("affine validation cannot read expression {expression}"),
            )),
        }
    }

    fn binding(&mut self, binding: BindingId) -> Result<BindingRecord, Diagnostic> {
        match self.read.owner(OwnerKey::Binding(binding))? {
            Some(OwnerRecord::Binding(record)) if record.kind == BindingKind::Let => Ok(record),
            _ => Err(Diagnostic::new(
                DiagnosticClass::Corrupt,
                "kernel_affine_binding_missing",
                format!("affine validation cannot read let binding {binding}"),
            )),
        }
    }

    fn operation(
        &mut self,
        package: PackageId,
        operation: crate::platform::semantic_id::OperationId,
    ) -> Result<OperationRecord, Diagnostic> {
        match self.exact_owner(package, OwnerKey::Operation(operation))? {
            Some(ExactRecord::Local(OwnerRecord::Operation(record)))
            | Some(ExactRecord::Foreign(PackageInterfaceRecord::Operation(record))) => Ok(record),
            _ => Err(Diagnostic::new(
                DiagnosticClass::Corrupt,
                "kernel_affine_operation_missing",
                "affine validation cannot read an exact capability operation",
            )),
        }
    }

    fn parameter(
        &mut self,
        package: PackageId,
        parameter: crate::platform::semantic_id::ParameterId,
    ) -> Result<ParameterRecord, Diagnostic> {
        match self.exact_owner(package, OwnerKey::Parameter(parameter))? {
            Some(ExactRecord::Local(OwnerRecord::Parameter(record)))
            | Some(ExactRecord::Foreign(PackageInterfaceRecord::Parameter(record))) => Ok(record),
            _ => Err(Diagnostic::new(
                DiagnosticClass::Corrupt,
                "kernel_affine_parameter_missing",
                "affine validation cannot read an exact operation parameter",
            )),
        }
    }

    fn requirement(
        &mut self,
        requirement: RequirementReference,
    ) -> Result<RequirementRecord, Diagnostic> {
        match self.exact_owner(
            requirement.package,
            OwnerKey::Requirement(requirement.requirement),
        )? {
            Some(ExactRecord::Local(OwnerRecord::Requirement(record)))
            | Some(ExactRecord::Foreign(PackageInterfaceRecord::Requirement(record))) => Ok(record),
            _ => Err(Diagnostic::new(
                DiagnosticClass::Corrupt,
                "kernel_affine_requirement_missing",
                "affine validation cannot read an exact requirement",
            )),
        }
    }

    fn local_function(
        &mut self,
        declaration: DeclarationId,
    ) -> Result<FunctionDeclaration, Diagnostic> {
        match self.read.owner(OwnerKey::Declaration(declaration))? {
            Some(OwnerRecord::Declaration(record)) => match record.payload {
                DeclarationPayload::Function(function) => Ok(function),
                _ => Err(Diagnostic::new(
                    DiagnosticClass::Corrupt,
                    "kernel_affine_function_kind",
                    "resource call graph names a non-function declaration",
                )),
            },
            _ => Err(Diagnostic::new(
                DiagnosticClass::Corrupt,
                "kernel_affine_function_missing",
                "resource call graph names a missing function declaration",
            )),
        }
    }

    fn resource_function_parameter(
        &mut self,
        reference: DeclarationReference,
    ) -> Result<Option<ResourceFunctionParameter>, Diagnostic> {
        let owner = OwnerKey::Declaration(reference.declaration);
        if reference.package != self.read.package_id() {
            let Some(PackageInterfaceRecord::Declaration(declaration)) = self
                .read
                .package_interface_owner(reference.package, owner)?
            else {
                return Ok(None);
            };
            let parameters = match declaration.payload {
                PackageInterfaceDeclarationPayload::Function(signature) => signature.parameters,
                PackageInterfaceDeclarationPayload::External(signature) => signature.parameters,
                _ => return Ok(None),
            };
            for parameter in parameters {
                let record = self.parameter(reference.package, parameter)?;
                if self.type_contains_resource(record.ty)?
                    || record.use_mode != ParameterUse::Unrestricted
                    || record.resource_requirement.is_some()
                {
                    return Err(owner_affine_error(
                        "kernel_affine_resource_call_package",
                        owner,
                        "dependency function signatures cannot transfer capability resources",
                    ));
                }
            }
            return Ok(None);
        }

        let Some(OwnerRecord::Declaration(declaration)) = self.read.owner(owner)? else {
            return Err(owner_affine_error(
                "kernel_affine_function_missing",
                owner,
                "resource signature names a missing declaration",
            ));
        };
        let function = match declaration.payload {
            DeclarationPayload::Function(function) => function,
            DeclarationPayload::External(signature) => {
                for parameter in signature.parameters {
                    let record = self.parameter(reference.package, parameter)?;
                    if self.type_contains_resource(record.ty)?
                        || record.use_mode != ParameterUse::Unrestricted
                        || record.resource_requirement.is_some()
                    {
                        return Err(owner_affine_error(
                            "kernel_affine_external_parameter",
                            owner,
                            "external function signatures cannot transfer capability resources",
                        ));
                    }
                }
                return Ok(None);
            }
            _ => return Ok(None),
        };

        let mut resource = None;
        for (index, parameter) in function.parameters.iter().copied().enumerate() {
            let record = self.parameter(reference.package, parameter)?;
            if record.parent != super::ParameterParent::Function(reference.declaration) {
                return Err(owner_affine_error(
                    "kernel_affine_function_parameter_parent",
                    OwnerKey::Parameter(parameter),
                    "function parameter belongs to another semantic parent",
                ));
            }
            match self.resource_type(record.ty)? {
                Some((ResourceShape::Direct, interface)) => {
                    if resource.is_some() {
                        return Err(owner_affine_error(
                            "kernel_affine_function_resource_count",
                            owner,
                            "function has more than one direct resource parameter",
                        ));
                    }
                    if index.saturating_add(1) != function.parameters.len() {
                        return Err(owner_affine_error(
                            "kernel_affine_function_resource_order",
                            OwnerKey::Parameter(parameter),
                            "resource parameter must be final in its function signature",
                        ));
                    }
                    if record.use_mode != ParameterUse::Consume {
                        return Err(owner_affine_error(
                            "kernel_affine_function_resource_use",
                            OwnerKey::Parameter(parameter),
                            "direct resource function parameter must consume",
                        ));
                    }
                    let requirement = record.resource_requirement.ok_or_else(|| {
                        owner_affine_error(
                            "kernel_affine_function_resource_requirement",
                            OwnerKey::Parameter(parameter),
                            "direct resource function parameter requires one exact requirement binding",
                        )
                    })?;
                    resource = Some(ResourceFunctionParameter {
                        parameter,
                        requirement,
                        interface,
                        parameter_count: function.parameters.len(),
                    });
                }
                Some((ResourceShape::Variant, _)) => {
                    return Err(owner_affine_error(
                        "kernel_affine_function_parameter_container",
                        OwnerKey::Parameter(parameter),
                        "function resource parameter must use the direct capability-resource type",
                    ));
                }
                None if self.type_contains_resource(record.ty)? => {
                    return Err(owner_affine_error(
                        "kernel_affine_function_parameter_container",
                        OwnerKey::Parameter(parameter),
                        "function parameter contains a forbidden resource container",
                    ));
                }
                None => {
                    if record.use_mode != ParameterUse::Unrestricted {
                        return Err(owner_affine_error(
                            "kernel_affine_function_parameter_use",
                            OwnerKey::Parameter(parameter),
                            "nonresource function parameter must be unrestricted",
                        ));
                    }
                    if record.resource_requirement.is_some() {
                        return Err(owner_affine_error(
                            "kernel_affine_parameter_requirement_extra",
                            OwnerKey::Parameter(parameter),
                            "nonresource function parameter cannot bind a resource requirement",
                        ));
                    }
                }
            }
        }

        let Some(resource) = resource else {
            return Ok(None);
        };
        if declaration.visibility != DeclarationVisibility::Private {
            return Err(owner_affine_error(
                "kernel_affine_function_resource_visibility",
                owner,
                "resource-bearing task function must be private",
            ));
        }
        if !function.type_parameters.is_empty() {
            return Err(owner_affine_error(
                "kernel_affine_function_resource_generic",
                owner,
                "resource-bearing task function cannot be generic",
            ));
        }
        if self.type_contains_resource(function.result)? {
            return Err(owner_affine_error(
                "kernel_affine_function_result",
                owner,
                "resource-bearing task function cannot return a capability resource",
            ));
        }
        let FunctionEffect::Task { requirements } = function.effect else {
            return Err(owner_affine_error(
                "kernel_affine_function_resource_effect",
                owner,
                "resource-bearing function must be a task",
            ));
        };
        if resource.requirement.package != self.read.package_id() {
            return Err(owner_affine_error(
                "kernel_affine_function_resource_package",
                OwnerKey::Parameter(resource.parameter),
                "resource parameter must bind a same-package requirement",
            ));
        }
        if !requirements.contains(&resource.requirement) {
            return Err(owner_affine_error(
                "kernel_affine_function_resource_effect",
                OwnerKey::Parameter(resource.parameter),
                "resource parameter binding is absent from its function effect",
            ));
        }
        let requirement = self.requirement(resource.requirement)?;
        if requirement.interface != resource.interface {
            return Err(owner_affine_error(
                "kernel_affine_function_resource_interface",
                OwnerKey::Parameter(resource.parameter),
                "resource parameter type disagrees with its exact requirement interface",
            ));
        }
        Ok(Some(resource))
    }

    fn case(
        &mut self,
        package: PackageId,
        case: crate::platform::semantic_id::CaseId,
    ) -> Result<CaseRecord, Diagnostic> {
        match self.exact_owner(package, OwnerKey::Case(case))? {
            Some(ExactRecord::Local(OwnerRecord::Case(record)))
            | Some(ExactRecord::Foreign(PackageInterfaceRecord::Case(record))) => Ok(record),
            _ => Err(Diagnostic::new(
                DiagnosticClass::Corrupt,
                "kernel_affine_case_missing",
                "affine validation cannot read an exact variant case",
            )),
        }
    }

    fn exact_owner(
        &mut self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<ExactRecord>, Diagnostic> {
        if package == self.read.package_id() {
            self.read
                .owner(owner)
                .map(|value| value.map(ExactRecord::Local))
        } else {
            self.read
                .package_interface_owner(package, owner)
                .map(|value| value.map(ExactRecord::Foreign))
        }
    }

    fn step(&mut self, expression: ExpressionId, depth: usize) -> Result<(), Diagnostic> {
        if depth > MAXIMUM_EXPRESSION_DEPTH {
            return Err(affine_error(
                "kernel_affine_depth",
                expression,
                "affine validation exceeded the expression-depth bound",
            ));
        }
        *self.work = self.work.saturating_add(1);
        if *self.work > self.maximum_steps {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_affine_work",
                "affine validation exhausted its explicit work budget",
            ));
        }
        Ok(())
    }
}

enum ExactRecord {
    Local(OwnerRecord),
    Foreign(PackageInterfaceRecord),
}

fn resource_owner(reference: LocalValueReference) -> Option<LocalValueReference> {
    match reference {
        LocalValueReference::FunctionParameter(_)
        | LocalValueReference::LexicalBinding(_)
        | LocalValueReference::MatchPayload(_) => Some(reference),
        LocalValueReference::OperationParameter(_) | LocalValueReference::TransactionBinding(_) => {
            None
        }
    }
}

fn resource_owner_label(reference: LocalValueReference) -> String {
    match reference {
        LocalValueReference::FunctionParameter(parameter) => {
            format!("function parameter {parameter}")
        }
        LocalValueReference::LexicalBinding(binding) => format!("binding {binding}"),
        LocalValueReference::MatchPayload(binding) => {
            format!("match payload binding {binding}")
        }
        LocalValueReference::OperationParameter(parameter) => {
            format!("operation parameter {parameter}")
        }
        LocalValueReference::TransactionBinding(binding) => {
            format!("transaction binding {binding}")
        }
    }
}

fn merge_values(
    expression: ExpressionId,
    left: EvaluatedValue,
    right: EvaluatedValue,
) -> Result<EvaluatedValue, Diagnostic> {
    if left == right {
        Ok(left)
    } else {
        Err(affine_error(
            "kernel_affine_branch_value",
            expression,
            "branches do not return the same resource shape and provenance",
        ))
    }
}

fn merge_many_values(
    expression: ExpressionId,
    values: &[EvaluatedValue],
) -> Result<EvaluatedValue, Diagnostic> {
    let Some(first) = values.first().copied() else {
        return Ok(EvaluatedValue::Unrestricted);
    };
    if values.iter().all(|value| *value == first) {
        Ok(first)
    } else {
        Err(affine_error(
            "kernel_affine_branch_value",
            expression,
            "match arms do not return the same resource shape and provenance",
        ))
    }
}

fn affine_error(
    code: &'static str,
    expression: ExpressionId,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Semantic,
        code,
        format!("expression {expression}: {}", message.into()),
    )
}

fn owner_affine_error(
    code: &'static str,
    owner: OwnerKey,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Semantic,
        code,
        format!("owner {owner:?}: {}", message.into()),
    )
}
