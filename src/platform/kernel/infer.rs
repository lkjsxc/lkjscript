//! Independent exact-ID expression type and effect oracle for Graph 5.

use super::contract::{MAXIMUM_EXPRESSION_DEPTH, MAXIMUM_TYPE_DEPTH, MAXIMUM_VALIDATION_WORK};
use super::digest::TypeObjectDigest;
use super::expression::{ExpressionOperation, FieldSelector, LocalValueReference};
use super::id::{OwnerKey, OwnerKind, PackageId};
use super::owner::{
    BindingKind, DeclarationPayload, FunctionEffect, OwnerRecord, ParameterParent,
    PortImplementation,
};
use super::reference::DeclarationReference;
use super::type_object::{StructuralTypeField, TypeForm, TypeObject};
use super::validate::KernelSnapshot;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{DeclarationId, ExpressionId, RequirementId, TypeParameterId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
struct ExecutionContext {
    declaration: Option<DeclarationId>,
    pure: bool,
    requirements: BTreeSet<RequirementId>,
    allow_task_function_value: bool,
}

#[derive(Clone, Debug)]
struct FunctionSignature {
    parameters: Vec<TypeObjectDigest>,
    result: TypeObjectDigest,
    requirements: BTreeSet<RequirementId>,
    task: bool,
}

pub(super) fn validate_expression_meaning(
    snapshot: &KernelSnapshot,
    diagnostics: &mut Vec<Diagnostic>,
    work: &mut usize,
) {
    let mut validator = ExpressionValidator {
        snapshot,
        diagnostics,
        work,
        ephemeral_types: BTreeMap::new(),
    };
    validator.validate_roots();
}

struct ExpressionValidator<'a, 'b> {
    snapshot: &'a KernelSnapshot,
    diagnostics: &'b mut Vec<Diagnostic>,
    work: &'b mut usize,
    ephemeral_types: BTreeMap<TypeObjectDigest, TypeObject>,
}

impl ExpressionValidator<'_, '_> {
    fn validate_roots(&mut self) {
        let owners = self.snapshot.owners.values().cloned().collect::<Vec<_>>();
        for owner in owners {
            if self.exhausted() {
                return;
            }
            match owner {
                OwnerRecord::Declaration(declaration) => match declaration.payload {
                    DeclarationPayload::Function(function) => {
                        let (pure, requirements) = match function.effect {
                            FunctionEffect::Pure => (true, BTreeSet::new()),
                            FunctionEffect::Task { requirements } => {
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
                                self.diagnostics.push(diagnostic);
                            }
                            (Err(actual), Err(expected)) => {
                                self.diagnostics.push(actual);
                                self.diagnostics.push(expected);
                            }
                            (Ok(_), Ok(_)) => {}
                        }
                    }
                    _ => {}
                },
                OwnerRecord::Port(port) => {
                    let requirements = self.component_requirements(port.declaration);
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
                            match self.function_signature(function, &[]) {
                                Ok(signature) => match self.function_type(&signature) {
                                    Ok(actual) if actual == port.function_type => {}
                                    Ok(_) => self.error(
                                        "kernel_type_port_function",
                                        "port function type disagrees with its exact declaration",
                                    ),
                                    Err(diagnostic) => self.diagnostics.push(diagnostic),
                                },
                                Err(diagnostic) => self.diagnostics.push(diagnostic),
                            }
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
            Err(diagnostic) => self.diagnostics.push(diagnostic),
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
        let record = match self.snapshot.owners.get(&OwnerKey::Expression(expression)) {
            Some(OwnerRecord::Expression(record)) => record.clone(),
            _ => {
                return Err(type_error(
                    "kernel_type_expression_missing",
                    format!("expression {expression} is missing"),
                ));
            }
        };
        let next = depth.saturating_add(1);
        match record.operation {
            ExpressionOperation::Unit => self.canonical_type(TypeForm::Unit),
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
                    let binding_record = match self.snapshot.owners.get(&OwnerKey::Binding(binding))
                    {
                        Some(OwnerRecord::Binding(record)) => record.clone(),
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
                let signature = self.function_signature(function, &type_arguments)?;
                self.validate_call_effect(&signature, context)?;
                self.validate_arguments(&arguments, &signature.parameters, context, next)?;
                Ok(signature.result)
            }
            ExpressionOperation::FunctionValue {
                function,
                type_arguments,
            } => {
                let signature = self.function_signature(function, &type_arguments)?;
                if signature.task && !context.allow_task_function_value {
                    return Err(type_error(
                        "kernel_type_task_function_value",
                        "task function value is unavailable in this expression context",
                    ));
                }
                self.function_type(&signature)
            }
            ExpressionOperation::Invoke { callee, arguments } => {
                let callee_type = self.infer(callee, context, next)?;
                let object = self.type_object(callee_type)?;
                let TypeForm::Function { parameters, result } = object.form else {
                    return Err(type_error(
                        "kernel_type_invoke",
                        "invoke callee is not a function value",
                    ));
                };
                self.validate_arguments(&arguments, &parameters, context, next)?;
                Ok(result)
            }
            ExpressionOperation::Record {
                nominal_type,
                fields,
            } => self.infer_record(nominal_type, &fields, context, next),
            ExpressionOperation::Variant { case, payload } => {
                if case.package != self.snapshot.root.package_id {
                    return Err(self.foreign_interface("variant case", case.package));
                }
                let case_record = match self.snapshot.owners.get(&OwnerKey::Case(case.case)) {
                    Some(OwnerRecord::Case(record)) => record.clone(),
                    _ => {
                        return Err(type_error(
                            "kernel_type_case_missing",
                            "variant case is missing",
                        ));
                    }
                };
                self.validate_optional_payload(
                    payload,
                    case_record.payload,
                    context,
                    next,
                    "variant",
                )?;
                self.named_type(DeclarationReference {
                    package: case.package,
                    declaration: case_record.declaration,
                })
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
                if requirement.package != self.snapshot.root.package_id
                    || operation.package != self.snapshot.root.package_id
                {
                    return Err(self.foreign_interface("capability", requirement.package));
                }
                if !context.requirements.contains(&requirement.requirement) {
                    return Err(type_error(
                        "kernel_type_capability_missing",
                        "capability requirement is unavailable in this task context",
                    ));
                }
                let operation_record = match self
                    .snapshot
                    .owners
                    .get(&OwnerKey::Operation(operation.operation))
                {
                    Some(OwnerRecord::Operation(record)) => record.clone(),
                    _ => {
                        return Err(type_error(
                            "kernel_type_operation_missing",
                            "capability operation is missing",
                        ));
                    }
                };
                let parameters = self.parameter_types(&operation_record.parameters)?;
                self.validate_arguments(&arguments, &parameters, context, next)?;
                Ok(operation_record.result)
            }
            ExpressionOperation::Transaction {
                requirement, body, ..
            } => {
                if context.pure {
                    return Err(type_error(
                        "kernel_type_pure_transaction",
                        "pure expression opens a live transaction",
                    ));
                }
                if requirement.package != self.snapshot.root.package_id
                    || !context.requirements.contains(&requirement.requirement)
                {
                    return Err(type_error(
                        "kernel_type_transaction_requirement",
                        "transaction requirement is unavailable in this task context",
                    ));
                }
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
                let record = match self.snapshot.owners.get(&OwnerKey::Parameter(parameter)) {
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
                let record = match self.snapshot.owners.get(&OwnerKey::Parameter(parameter)) {
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
            | LocalValueReference::MatchPayload(binding)
            | LocalValueReference::TransactionBinding(binding) => {
                let record = match self.snapshot.owners.get(&OwnerKey::Binding(binding)) {
                    Some(OwnerRecord::Binding(record)) => record.clone(),
                    _ => {
                        return Err(type_error(
                            "kernel_type_binding_missing",
                            "binding is missing",
                        ));
                    }
                };
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
        }
    }

    fn infer_record(
        &mut self,
        nominal_type: Option<DeclarationReference>,
        fields: &[super::expression::RecordExpressionField],
        context: &ExecutionContext,
        depth: usize,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        if let Some(declaration) = nominal_type {
            if declaration.package != self.snapshot.root.package_id {
                return Err(self.foreign_interface("record", declaration.package));
            }
            let expected = match self
                .snapshot
                .owners
                .get(&OwnerKey::Declaration(declaration.declaration))
            {
                Some(OwnerRecord::Declaration(record)) => match &record.payload {
                    DeclarationPayload::Record { fields } => fields.clone(),
                    _ => {
                        return Err(type_error(
                            "kernel_type_record_kind",
                            "nominal record expression names a non-record declaration",
                        ));
                    }
                },
                _ => {
                    return Err(type_error(
                        "kernel_type_record_missing",
                        "nominal record declaration is missing",
                    ));
                }
            };
            if expected.len() != fields.len() {
                return Err(type_error(
                    "kernel_type_record_field_count",
                    "nominal record expression has the wrong field set",
                ));
            }
            for expected_field in expected {
                let field_record = match self.snapshot.owners.get(&OwnerKey::Field(expected_field))
                {
                    Some(OwnerRecord::Field(record)) => record.clone(),
                    _ => {
                        return Err(type_error(
                            "kernel_type_field_missing",
                            "record field is missing",
                        ));
                    }
                };
                let value = fields
                    .iter()
                    .find_map(|field| match field.selector {
                        FieldSelector::Nominal(reference) if reference.field == expected_field => {
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
                    field_record.ty,
                    actual,
                    "kernel_type_record_field",
                    "record field value",
                )?;
            }
            self.named_type(declaration)
        } else {
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
                if reference.package != self.snapshot.root.package_id {
                    return Err(self.foreign_interface("field", reference.package));
                }
                let field = match self.snapshot.owners.get(&OwnerKey::Field(reference.field)) {
                    Some(OwnerRecord::Field(record)) => record,
                    _ => {
                        return Err(type_error(
                            "kernel_type_field_missing",
                            "nominal field is missing",
                        ));
                    }
                };
                let object = self.type_object(value_type)?;
                if !matches!(
                    object.form,
                    TypeForm::Named { declaration }
                        if declaration.package == reference.package
                            && declaration.declaration == field.declaration
                ) {
                    return Err(type_error(
                        "kernel_type_field_owner",
                        "nominal field does not belong to the selected value type",
                    ));
                }
                Ok(field.ty)
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
        let TypeForm::Named { declaration } = value_object.form else {
            return Err(type_error(
                "kernel_type_match_value",
                "match value is not a nominal variant",
            ));
        };
        if declaration.package != self.snapshot.root.package_id {
            return Err(self.foreign_interface("match", declaration.package));
        }
        let expected_cases = match self
            .snapshot
            .owners
            .get(&OwnerKey::Declaration(declaration.declaration))
        {
            Some(OwnerRecord::Declaration(record)) => match &record.payload {
                DeclarationPayload::Variant { cases } => cases.clone(),
                _ => {
                    return Err(type_error(
                        "kernel_type_match_kind",
                        "match value names a non-variant declaration",
                    ));
                }
            },
            _ => {
                return Err(type_error(
                    "kernel_type_match_variant_missing",
                    "match variant declaration is missing",
                ));
            }
        };
        let actual_cases = arms.iter().map(|arm| arm.case.case).collect::<Vec<_>>();
        if expected_cases != actual_cases {
            return Err(type_error(
                "kernel_type_match_exhaustive",
                "match arms do not exactly cover the variant cases",
            ));
        }
        let mut result = None;
        for arm in arms {
            let case = match self.snapshot.owners.get(&OwnerKey::Case(arm.case.case)) {
                Some(OwnerRecord::Case(record)) => record,
                _ => {
                    return Err(type_error(
                        "kernel_type_case_missing",
                        "match case is missing",
                    ));
                }
            };
            if case.declaration != declaration.declaration {
                return Err(type_error(
                    "kernel_type_match_case_owner",
                    "match arm case belongs to another variant",
                ));
            }
            match (case.payload, arm.payload_binding) {
                (Some(expected), Some(binding)) => {
                    let declared = match self.snapshot.owners.get(&OwnerKey::Binding(binding)) {
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

    fn constant_type(
        &self,
        reference: DeclarationReference,
    ) -> Result<TypeObjectDigest, Diagnostic> {
        if reference.package != self.snapshot.root.package_id {
            return Err(self.foreign_interface("constant", reference.package));
        }
        match self
            .snapshot
            .owners
            .get(&OwnerKey::Declaration(reference.declaration))
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
    ) -> Result<FunctionSignature, Diagnostic> {
        if reference.package != self.snapshot.root.package_id {
            return Err(self.foreign_interface("function", reference.package));
        }
        let record = match self
            .snapshot
            .owners
            .get(&OwnerKey::Declaration(reference.declaration))
        {
            Some(OwnerRecord::Declaration(record)) => record.clone(),
            _ => {
                return Err(type_error(
                    "kernel_type_function_missing",
                    "function declaration is missing",
                ));
            }
        };
        let (type_parameters, parameters, result, requirements, task) = match record.payload {
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
        };
        if type_parameters.len() != type_arguments.len() {
            return Err(type_error(
                "kernel_type_argument_count",
                "function type argument count disagrees with its declaration",
            ));
        }
        let substitutions = type_parameters
            .into_iter()
            .zip(type_arguments.iter().copied())
            .collect::<BTreeMap<_, _>>();
        let mut parameter_types = self.parameter_types(&parameters)?;
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
        parameters: &[crate::platform::semantic_id::ParameterId],
    ) -> Result<Vec<TypeObjectDigest>, Diagnostic> {
        parameters
            .iter()
            .map(
                |parameter| match self.snapshot.owners.get(&OwnerKey::Parameter(*parameter)) {
                    Some(OwnerRecord::Parameter(record)) => Ok(record.ty),
                    _ => Err(type_error(
                        "kernel_type_parameter_missing",
                        "signature parameter record is missing",
                    )),
                },
            )
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
        if !signature.requirements.is_subset(&context.requirements) {
            return Err(type_error(
                "kernel_type_task_requirement",
                "task call requires an unavailable exact capability requirement",
            ));
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
        let (digest, _) = super::codec::encode_type_object(&object)?;
        self.ephemeral_types.entry(digest).or_insert(object);
        Ok(digest)
    }

    fn type_object(&self, digest: TypeObjectDigest) -> Result<TypeObject, Diagnostic> {
        self.snapshot
            .types
            .get(&digest)
            .or_else(|| self.ephemeral_types.get(&digest))
            .cloned()
            .ok_or_else(|| {
                type_error(
                    "kernel_type_object_missing",
                    format!("type object {digest} is unavailable for semantic inference"),
                )
            })
    }

    fn component_requirements(&self, declaration: DeclarationId) -> BTreeSet<RequirementId> {
        match self
            .snapshot
            .owners
            .get(&OwnerKey::Declaration(declaration))
        {
            Some(OwnerRecord::Declaration(record)) => match &record.payload {
                DeclarationPayload::Component { requirements, .. } => {
                    requirements.iter().copied().collect()
                }
                _ => BTreeSet::new(),
            },
            _ => BTreeSet::new(),
        }
    }

    fn foreign_interface(&self, label: &str, package: PackageId) -> Diagnostic {
        if self.snapshot.dependencies.contains_key(&package) {
            type_error(
                "kernel_type_dependency_interface",
                format!(
                    "{label} requires the exact dependency semantic interface for package {package}"
                ),
            )
        } else {
            type_error(
                "kernel_type_dependency_missing",
                format!("{label} names unbound package {package}"),
            )
        }
    }

    fn consume_work(&mut self) -> Result<(), Diagnostic> {
        *self.work = self.work.saturating_add(1);
        if *self.work > MAXIMUM_VALIDATION_WORK {
            return Err(type_error(
                "kernel_type_work",
                "expression type validation exhausted its explicit work budget",
            ));
        }
        Ok(())
    }

    fn exhausted(&self) -> bool {
        *self.work > MAXIMUM_VALIDATION_WORK
    }

    fn error(&mut self, code: &str, message: impl Into<String>) {
        self.diagnostics.push(type_error(code, message));
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
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}
