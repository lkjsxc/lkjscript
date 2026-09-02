//! Typed Graph 7 declaration, type-object, and expression authoring builders.

mod declarations;
mod mutation;

pub use declarations::{
    AuthoredAnnotationValue, AuthoredCase, AuthoredField, AuthoredOperation, AuthoredPort,
    AuthoredPortImplementation, AuthoredPortReference, AuthoredRequirement, AuthoredResourceLimit,
};
pub(super) use declarations::{
    collect_annotation_symbols, collect_component_symbols, collect_constant_symbols,
    collect_documentation_symbols, collect_external_symbols, collect_interface_symbols,
    collect_record_symbols, collect_target_symbols, collect_variant_symbols, lower_annotation,
    lower_component, lower_constant, lower_documentation, lower_external, lower_interface,
    lower_record, lower_target, lower_variant,
};
pub(super) use mutation::{collect_mutation_symbols, lower_mutation};

use super::{
    AuthoredLowerer, DeclarationSelector, ModuleSelector, OwnerSelector, ParameterParentSelector,
    SymbolDefinitions, SymbolKind, define_symbol, request_error,
};
use crate::platform::change::{CanonicalBaseRead, WitnessBaseRead};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    CaseReference, ComparisonPolicy, DeclarationPayload, DeclarationRecord, DeclarationReference,
    DeclarationVisibility, ExpressionOperation, ExpressionRecord, FieldReference, FieldSelector,
    FunctionDeclaration, FunctionEffect, LocalValueReference, MapExpressionEntry,
    MatchExpressionArm, Name, OperationReference, OwnerHeader, OwnerKey, OwnerKind, OwnerRecord,
    ParameterParent, ParameterRecord, ParameterUse, RecordExpressionField, RequirementReference,
    StructuralTypeField, TextValue, TypeForm, TypeObjectDigest, TypeParameterRecord,
};
use crate::platform::semantic_id::{
    BindingId, CaseId, DeclarationId, FieldId, OperationId, ParameterId, RequirementId,
    TypeParameterId,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredTypeParameter {
    pub symbol: String,
    pub name: Name,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredParameter {
    pub symbol: String,
    pub name: Name,
    pub ty: AuthoredType,
    pub use_mode: ParameterUse,
    pub resource_requirement: Option<AuthoredRequirementReference>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredFunctionEffect {
    Pure {},
    Task {
        requirements: Vec<AuthoredRequirementReference>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredType {
    Unit {},
    Bool {},
    I64 {},
    Bytes {},
    Text {},
    StaticText {},
    Secret {},
    TypeParameter {
        parameter: AuthoredTypeParameterReference,
    },
    Named {
        declaration: AuthoredDeclarationReference,
    },
    CapabilityResource {
        interface: AuthoredDeclarationReference,
    },
    StructuralRecord {
        fields: Vec<AuthoredStructuralTypeField>,
    },
    List {
        item: Box<AuthoredType>,
    },
    Map {
        key: Box<AuthoredType>,
        value: Box<AuthoredType>,
    },
    Option {
        item: Box<AuthoredType>,
    },
    Result {
        ok: Box<AuthoredType>,
        error: Box<AuthoredType>,
    },
    Stream {
        item: Box<AuthoredType>,
    },
    Function {
        parameters: Vec<AuthoredType>,
        result: Box<AuthoredType>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredStructuralTypeField {
    pub name: Name,
    pub ty: AuthoredType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredTypeParameterReference {
    Id { parameter: TypeParameterId },
    Symbol { symbol: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredDeclarationReference {
    Local {
        declaration: DeclarationSelector,
    },
    Exact {
        package: crate::platform::kernel::PackageId,
        declaration: DeclarationId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredFieldReference {
    Exact {
        package: crate::platform::kernel::PackageId,
        field: FieldId,
    },
    Symbol {
        symbol: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredCaseReference {
    Exact {
        package: crate::platform::kernel::PackageId,
        case: CaseId,
    },
    Symbol {
        symbol: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredOperationReference {
    Exact {
        package: crate::platform::kernel::PackageId,
        operation: OperationId,
    },
    Symbol {
        symbol: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredRequirementReference {
    Exact {
        package: crate::platform::kernel::PackageId,
        requirement: RequirementId,
    },
    Symbol {
        symbol: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredLocalReference {
    FunctionParameter { parameter: ParameterId },
    OperationParameter { parameter: ParameterId },
    LexicalBinding { binding: BindingId },
    MatchPayload { binding: BindingId },
    TransactionBinding { binding: BindingId },
    Symbol { symbol: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredExpression {
    pub symbol: Option<String>,
    pub operation: AuthoredExpressionOperation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredExpressionOperation {
    Unit {},
    Bool {
        value: bool,
    },
    I64 {
        value: i64,
    },
    Text {
        value: String,
    },
    StaticText {
        value: String,
    },
    Local {
        value: AuthoredLocalReference,
    },
    Constant {
        declaration: AuthoredDeclarationReference,
    },
    If {
        condition: Box<AuthoredExpression>,
        when_true: Box<AuthoredExpression>,
        when_false: Box<AuthoredExpression>,
    },
    Let {
        bindings: Vec<AuthoredLetBinding>,
        body: Box<AuthoredExpression>,
    },
    Sequence {
        items: Vec<AuthoredExpression>,
    },
    Call {
        function: AuthoredDeclarationReference,
        type_arguments: Vec<AuthoredType>,
        arguments: Vec<AuthoredExpression>,
    },
    FunctionValue {
        function: AuthoredDeclarationReference,
        type_arguments: Vec<AuthoredType>,
    },
    Invoke {
        callee: Box<AuthoredExpression>,
        arguments: Vec<AuthoredExpression>,
    },
    Record {
        nominal_type: Option<AuthoredDeclarationReference>,
        fields: Vec<AuthoredRecordExpressionField>,
    },
    Variant {
        case: AuthoredCaseReference,
        payload: Option<Box<AuthoredExpression>>,
    },
    Field {
        value: Box<AuthoredExpression>,
        selector: AuthoredFieldSelector,
    },
    List {
        item_type: AuthoredType,
        items: Vec<AuthoredExpression>,
    },
    Map {
        key_type: AuthoredType,
        value_type: AuthoredType,
        entries: Vec<AuthoredMapExpressionEntry>,
    },
    Match {
        value: Box<AuthoredExpression>,
        arms: Vec<AuthoredMatchExpressionArm>,
    },
    CapabilityCall {
        requirement: AuthoredRequirementReference,
        operation: AuthoredOperationReference,
        arguments: Vec<AuthoredExpression>,
    },
    Transaction {
        requirement: AuthoredRequirementReference,
        binding: AuthoredBindingDefinition,
        body: Box<AuthoredExpression>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredLetBinding {
    pub symbol: String,
    pub name: Name,
    pub value: AuthoredExpression,
    pub declared_type: Option<AuthoredType>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredBindingDefinition {
    pub symbol: String,
    pub name: Name,
    pub declared_type: Option<AuthoredType>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredFieldSelector {
    Nominal { field: AuthoredFieldReference },
    Structural { name: Name },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredRecordExpressionField {
    pub selector: AuthoredFieldSelector,
    pub value: AuthoredExpression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredMapExpressionEntry {
    pub key: AuthoredExpression,
    pub value: AuthoredExpression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredMatchExpressionArm {
    pub case: AuthoredCaseReference,
    pub payload_binding: Option<AuthoredBindingDefinition>,
    pub body: AuthoredExpression,
}

#[allow(
    clippy::too_many_arguments,
    reason = "one function declaration owns these closed fields"
)]
pub(super) fn collect_function_symbols(
    symbol: &str,
    type_parameters: &[AuthoredTypeParameter],
    parameters: &[AuthoredParameter],
    body: &AuthoredExpression,
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    define_symbol(definitions, symbol, SymbolKind::Declaration)?;
    for parameter in type_parameters {
        define_symbol(definitions, &parameter.symbol, SymbolKind::TypeParameter)?;
    }
    for parameter in parameters {
        define_symbol(
            definitions,
            &parameter.symbol,
            SymbolKind::FunctionParameter,
        )?;
    }
    collect_expression_symbols(body, definitions)
}

pub(super) fn collect_test_symbols(
    symbol: &str,
    actual: &AuthoredExpression,
    expected: &AuthoredExpression,
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    define_symbol(definitions, symbol, SymbolKind::Declaration)?;
    collect_expression_symbols(actual, definitions)?;
    collect_expression_symbols(expected, definitions)
}

pub(super) fn collect_expression_symbols(
    expression: &AuthoredExpression,
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    if let Some(symbol) = &expression.symbol {
        define_symbol(definitions, symbol, SymbolKind::Expression)?;
    } else {
        definitions.define_anonymous_identity()?;
    }
    match &expression.operation {
        AuthoredExpressionOperation::If {
            condition,
            when_true,
            when_false,
        } => {
            collect_expression_symbols(condition, definitions)?;
            collect_expression_symbols(when_true, definitions)?;
            collect_expression_symbols(when_false, definitions)
        }
        AuthoredExpressionOperation::Let { bindings, body } => {
            for binding in bindings {
                define_symbol(definitions, &binding.symbol, SymbolKind::LexicalBinding)?;
                collect_expression_symbols(&binding.value, definitions)?;
            }
            collect_expression_symbols(body, definitions)
        }
        AuthoredExpressionOperation::Sequence { items }
        | AuthoredExpressionOperation::List { items, .. } => {
            collect_many_expression_symbols(items, definitions)
        }
        AuthoredExpressionOperation::Call { arguments, .. }
        | AuthoredExpressionOperation::CapabilityCall { arguments, .. } => {
            collect_many_expression_symbols(arguments, definitions)
        }
        AuthoredExpressionOperation::Invoke { callee, arguments } => {
            collect_expression_symbols(callee, definitions)?;
            collect_many_expression_symbols(arguments, definitions)
        }
        AuthoredExpressionOperation::Record { fields, .. } => {
            for field in fields {
                collect_expression_symbols(&field.value, definitions)?;
            }
            Ok(())
        }
        AuthoredExpressionOperation::Variant { payload, .. } => {
            if let Some(payload) = payload {
                collect_expression_symbols(payload, definitions)?;
            }
            Ok(())
        }
        AuthoredExpressionOperation::Field { value, .. } => {
            collect_expression_symbols(value, definitions)
        }
        AuthoredExpressionOperation::Map { entries, .. } => {
            for entry in entries {
                collect_expression_symbols(&entry.key, definitions)?;
                collect_expression_symbols(&entry.value, definitions)?;
            }
            Ok(())
        }
        AuthoredExpressionOperation::Match { value, arms } => {
            collect_expression_symbols(value, definitions)?;
            for arm in arms {
                if let Some(binding) = &arm.payload_binding {
                    define_symbol(
                        definitions,
                        &binding.symbol,
                        SymbolKind::MatchPayloadBinding,
                    )?;
                }
                collect_expression_symbols(&arm.body, definitions)?;
            }
            Ok(())
        }
        AuthoredExpressionOperation::Transaction { binding, body, .. } => {
            define_symbol(definitions, &binding.symbol, SymbolKind::TransactionBinding)?;
            collect_expression_symbols(body, definitions)
        }
        AuthoredExpressionOperation::Unit {}
        | AuthoredExpressionOperation::Bool { .. }
        | AuthoredExpressionOperation::I64 { .. }
        | AuthoredExpressionOperation::Text { .. }
        | AuthoredExpressionOperation::StaticText { .. }
        | AuthoredExpressionOperation::Local { .. }
        | AuthoredExpressionOperation::Constant { .. }
        | AuthoredExpressionOperation::FunctionValue { .. } => Ok(()),
    }
}

fn collect_many_expression_symbols(
    expressions: &[AuthoredExpression],
    definitions: &mut SymbolDefinitions,
) -> Result<(), Diagnostic> {
    for expression in expressions {
        collect_expression_symbols(expression, definitions)?;
    }
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "one function declaration owns these closed fields"
)]
pub(super) fn lower_function<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    symbol: &str,
    module: &ModuleSelector,
    name: &Name,
    visibility: DeclarationVisibility,
    type_parameters: &[AuthoredTypeParameter],
    parameters: &[AuthoredParameter],
    result: &AuthoredType,
    effect: &AuthoredFunctionEffect,
    body: &AuthoredExpression,
) -> Result<(), Diagnostic> {
    let declaration = lowerer.declaration_symbol(symbol)?;
    let module = lowerer.resolve_module(module)?;
    let mut type_parameter_ids = Vec::with_capacity(type_parameters.len());
    for parameter in type_parameters {
        let id = lowerer.type_parameter_symbol(&parameter.symbol)?;
        type_parameter_ids.push(id);
        lowerer.insert_created(OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(id), OwnerKind::TypeParameter),
            declaration,
            name: parameter.name.clone(),
        }))?;
    }
    let mut parameter_ids = Vec::with_capacity(parameters.len());
    for parameter in parameters {
        let id = lowerer.function_parameter_symbol(&parameter.symbol)?;
        let ty = lowerer.lower_type(&parameter.ty)?;
        let resource_requirement = parameter
            .resource_requirement
            .as_ref()
            .map(|requirement| lowerer.lower_requirement_reference(requirement))
            .transpose()?;
        parameter_ids.push(id);
        lowerer.insert_created(OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
            parent: ParameterParent::Function(declaration),
            name: parameter.name.clone(),
            ty,
            use_mode: parameter.use_mode,
            resource_requirement,
        }))?;
    }
    let result = lowerer.lower_type(result)?;
    let effect = lowerer.lower_effect(effect)?;
    let body = lowerer.lower_expression(body)?;
    let kind = match effect {
        FunctionEffect::Pure => OwnerKind::PureFunction,
        FunctionEffect::Task { .. } => OwnerKind::TaskFunction,
    };
    lowerer.insert_created(OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(OwnerKey::Declaration(declaration), kind),
        module,
        name: name.clone(),
        visibility,
        payload: DeclarationPayload::Function(FunctionDeclaration {
            type_parameters: type_parameter_ids,
            parameters: parameter_ids,
            result,
            effect,
            body,
        }),
    }))
}

#[allow(
    clippy::too_many_arguments,
    reason = "one test declaration owns these closed fields"
)]
pub(super) fn lower_test<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    symbol: &str,
    module: &ModuleSelector,
    name: &Name,
    visibility: DeclarationVisibility,
    actual: &AuthoredExpression,
    expected: &AuthoredExpression,
) -> Result<(), Diagnostic> {
    let declaration = lowerer.declaration_symbol(symbol)?;
    let module = lowerer.resolve_module(module)?;
    let actual = lowerer.lower_expression(actual)?;
    let expected = lowerer.lower_expression(expected)?;
    lowerer.insert_created(OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::Test),
        module,
        name: name.clone(),
        visibility,
        payload: DeclarationPayload::Test {
            actual,
            expected,
            comparison: ComparisonPolicy::Exact,
        },
    }))
}

impl<'a, B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized> AuthoredLowerer<'a, B, W> {
    fn lower_type(&mut self, authored: &AuthoredType) -> Result<TypeObjectDigest, Diagnostic> {
        let form = match authored {
            AuthoredType::Unit {} => TypeForm::Unit,
            AuthoredType::Bool {} => TypeForm::Bool,
            AuthoredType::I64 {} => TypeForm::I64,
            AuthoredType::Bytes {} => TypeForm::Bytes,
            AuthoredType::Text {} => TypeForm::Text,
            AuthoredType::StaticText {} => TypeForm::StaticText,
            AuthoredType::Secret {} => TypeForm::Secret,
            AuthoredType::TypeParameter { parameter } => TypeForm::TypeParameter {
                parameter: self.lower_type_parameter_reference(parameter)?,
            },
            AuthoredType::Named { declaration } => TypeForm::Named {
                declaration: self.lower_declaration_reference(declaration)?,
            },
            AuthoredType::CapabilityResource { interface } => TypeForm::CapabilityResource {
                interface: self.lower_declaration_reference(interface)?,
            },
            AuthoredType::StructuralRecord { fields } => {
                let mut lowered = Vec::with_capacity(fields.len());
                for field in fields {
                    lowered.push(StructuralTypeField {
                        name: field.name.clone(),
                        ty: self.lower_type(&field.ty)?,
                    });
                }
                lowered.sort_by(|left, right| left.name.cmp(&right.name));
                TypeForm::StructuralRecord { fields: lowered }
            }
            AuthoredType::List { item } => TypeForm::List {
                item: self.lower_type(item)?,
            },
            AuthoredType::Map { key, value } => TypeForm::Map {
                key: self.lower_type(key)?,
                value: self.lower_type(value)?,
            },
            AuthoredType::Option { item } => TypeForm::Option {
                item: self.lower_type(item)?,
            },
            AuthoredType::Result { ok, error } => TypeForm::Result {
                ok: self.lower_type(ok)?,
                error: self.lower_type(error)?,
            },
            AuthoredType::Stream { item } => TypeForm::Stream {
                item: self.lower_type(item)?,
            },
            AuthoredType::Function { parameters, result } => {
                let mut lowered = Vec::with_capacity(parameters.len());
                for parameter in parameters {
                    lowered.push(self.lower_type(parameter)?);
                }
                TypeForm::Function {
                    parameters: lowered,
                    result: self.lower_type(result)?,
                }
            }
        };
        let digest = self.types.intern(form).map_err(|diagnostic| {
            if diagnostic.code == "kernel_type_interner_exhausted" {
                request_error(
                    DiagnosticClass::Resource,
                    "change_budget_authored_type_nodes",
                    diagnostic.message,
                )
            } else {
                diagnostic
            }
        })?;
        self.work.type_nodes_interned = u64::try_from(self.types.len()).unwrap_or(u64::MAX);
        self.classify_interned_type(digest)?;
        Ok(digest)
    }

    fn classify_interned_type(&mut self, digest: TypeObjectDigest) -> Result<(), Diagnostic> {
        if !self.base_types.contains_key(&digest) {
            let read = self.base.read_type_object(digest)?;
            self.work.canonical.add(read.work);
            self.base_types.insert(digest, read.value);
            self.check_budget("authored type base read")?;
        }
        let object = self.types.get(digest).ok_or_else(|| {
            request_error(
                DiagnosticClass::Corrupt,
                "change_authored_type_interner",
                "request-local type interner lost an exact type object",
            )
        })?;
        if let Some(base) = self.base_types.get(&digest).and_then(Option::as_ref) {
            if base != object {
                return Err(request_error(
                    DiagnosticClass::Corrupt,
                    "change_authored_type_collision",
                    "accepted authority binds one type digest to different canonical meaning",
                ));
            }
            return Ok(());
        }
        if self.type_additions.contains(&digest) {
            return Ok(());
        }
        self.budget.check_canonical_edit_counts(
            u64::try_from(self.owner_edits.len()).unwrap_or(u64::MAX),
            u64::try_from(self.type_additions.len().saturating_add(1)).unwrap_or(u64::MAX),
            u64::try_from(self.dependency_edits.len()).unwrap_or(u64::MAX),
            u64::try_from(self.retirement_edits.len()).unwrap_or(u64::MAX),
            "authored canonical type edit admission",
        )?;
        self.type_additions.insert(digest);
        Ok(())
    }

    fn lower_effect(
        &mut self,
        authored: &AuthoredFunctionEffect,
    ) -> Result<FunctionEffect, Diagnostic> {
        match authored {
            AuthoredFunctionEffect::Pure {} => Ok(FunctionEffect::Pure),
            AuthoredFunctionEffect::Task { requirements } => {
                let mut lowered = requirements
                    .iter()
                    .map(|requirement| self.lower_requirement_reference(requirement))
                    .collect::<Result<Vec<_>, _>>()?;
                lowered.sort_unstable();
                if lowered.windows(2).any(|pair| pair[0] == pair[1]) {
                    return Err(request_error(
                        DiagnosticClass::Semantic,
                        "change_authored_requirement_duplicate",
                        "task effect contains a duplicate exact requirement",
                    ));
                }
                Ok(FunctionEffect::Task {
                    requirements: lowered,
                })
            }
        }
    }

    pub(super) fn lower_expression(
        &mut self,
        authored: &AuthoredExpression,
    ) -> Result<crate::platform::semantic_id::ExpressionId, Diagnostic> {
        let id = self.expression_identity(authored.symbol.as_deref())?;
        let operation = match &authored.operation {
            AuthoredExpressionOperation::Unit {} => ExpressionOperation::Unit {},
            AuthoredExpressionOperation::Bool { value } => {
                ExpressionOperation::Bool { value: *value }
            }
            AuthoredExpressionOperation::I64 { value } => {
                ExpressionOperation::I64 { value: *value }
            }
            AuthoredExpressionOperation::Text { value } => ExpressionOperation::Text {
                value: TextValue::Inline {
                    text: value.clone(),
                },
            },
            AuthoredExpressionOperation::StaticText { value } => ExpressionOperation::StaticText {
                value: TextValue::Inline {
                    text: value.clone(),
                },
            },
            AuthoredExpressionOperation::Local { value } => ExpressionOperation::Local {
                value: self.lower_local_reference(value)?,
            },
            AuthoredExpressionOperation::Constant { declaration } => {
                ExpressionOperation::Constant {
                    declaration: self.lower_declaration_reference(declaration)?,
                }
            }
            AuthoredExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => ExpressionOperation::If {
                condition: self.lower_expression(condition)?,
                when_true: self.lower_expression(when_true)?,
                when_false: self.lower_expression(when_false)?,
            },
            AuthoredExpressionOperation::Let { bindings, body } => {
                let mut lowered = Vec::with_capacity(bindings.len());
                for binding in bindings {
                    let id = self.lexical_binding_symbol(&binding.symbol)?;
                    let value = self.lower_expression(&binding.value)?;
                    let declared_type = binding
                        .declared_type
                        .as_ref()
                        .map(|value| self.lower_type(value))
                        .transpose()?;
                    self.insert_created(OwnerRecord::Binding(
                        crate::platform::kernel::BindingRecord {
                            header: OwnerHeader::new(OwnerKey::Binding(id), OwnerKind::Binding),
                            name: binding.name.clone(),
                            kind: crate::platform::kernel::BindingKind::Let,
                            value: Some(value),
                            declared_type,
                        },
                    ))?;
                    lowered.push(id);
                }
                ExpressionOperation::Let {
                    bindings: lowered,
                    body: self.lower_expression(body)?,
                }
            }
            AuthoredExpressionOperation::Sequence { items } => ExpressionOperation::Sequence {
                items: self.lower_expressions(items)?,
            },
            AuthoredExpressionOperation::Call {
                function,
                type_arguments,
                arguments,
            } => ExpressionOperation::Call {
                function: self.lower_declaration_reference(function)?,
                type_arguments: self.lower_types(type_arguments)?,
                arguments: self.lower_expressions(arguments)?,
            },
            AuthoredExpressionOperation::FunctionValue {
                function,
                type_arguments,
            } => ExpressionOperation::FunctionValue {
                function: self.lower_declaration_reference(function)?,
                type_arguments: self.lower_types(type_arguments)?,
            },
            AuthoredExpressionOperation::Invoke { callee, arguments } => {
                ExpressionOperation::Invoke {
                    callee: self.lower_expression(callee)?,
                    arguments: self.lower_expressions(arguments)?,
                }
            }
            AuthoredExpressionOperation::Record {
                nominal_type,
                fields,
            } => {
                let mut lowered = Vec::with_capacity(fields.len());
                for field in fields {
                    lowered.push(RecordExpressionField {
                        selector: self.lower_field_selector(&field.selector)?,
                        value: self.lower_expression(&field.value)?,
                    });
                }
                lowered.sort_by(|left, right| left.selector.cmp(&right.selector));
                ExpressionOperation::Record {
                    nominal_type: nominal_type
                        .as_ref()
                        .map(|value| self.lower_declaration_reference(value))
                        .transpose()?,
                    fields: lowered,
                }
            }
            AuthoredExpressionOperation::Variant { case, payload } => {
                ExpressionOperation::Variant {
                    case: self.lower_case_reference(case)?,
                    payload: payload
                        .as_ref()
                        .map(|value| self.lower_expression(value))
                        .transpose()?,
                }
            }
            AuthoredExpressionOperation::Field { value, selector } => ExpressionOperation::Field {
                value: self.lower_expression(value)?,
                selector: self.lower_field_selector(selector)?,
            },
            AuthoredExpressionOperation::List { item_type, items } => ExpressionOperation::List {
                item_type: self.lower_type(item_type)?,
                items: self.lower_expressions(items)?,
            },
            AuthoredExpressionOperation::Map {
                key_type,
                value_type,
                entries,
            } => {
                let mut lowered = Vec::with_capacity(entries.len());
                for entry in entries {
                    lowered.push(MapExpressionEntry {
                        key: self.lower_expression(&entry.key)?,
                        value: self.lower_expression(&entry.value)?,
                    });
                }
                ExpressionOperation::Map {
                    key_type: self.lower_type(key_type)?,
                    value_type: self.lower_type(value_type)?,
                    entries: lowered,
                }
            }
            AuthoredExpressionOperation::Match { value, arms } => {
                let value = self.lower_expression(value)?;
                let mut lowered = Vec::with_capacity(arms.len());
                for arm in arms {
                    let payload_binding = arm
                        .payload_binding
                        .as_ref()
                        .map(|binding| {
                            self.insert_scoped_binding(binding, SymbolKind::MatchPayloadBinding)
                        })
                        .transpose()?;
                    lowered.push(MatchExpressionArm {
                        case: self.lower_case_reference(&arm.case)?,
                        payload_binding,
                        body: self.lower_expression(&arm.body)?,
                    });
                }
                lowered.sort_by_key(|arm| arm.case);
                ExpressionOperation::Match {
                    value,
                    arms: lowered,
                }
            }
            AuthoredExpressionOperation::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => ExpressionOperation::CapabilityCall {
                requirement: self.lower_requirement_reference(requirement)?,
                operation: self.lower_operation_reference(operation)?,
                arguments: self.lower_expressions(arguments)?,
            },
            AuthoredExpressionOperation::Transaction {
                requirement,
                binding,
                body,
            } => ExpressionOperation::Transaction {
                requirement: self.lower_requirement_reference(requirement)?,
                binding: self.insert_scoped_binding(binding, SymbolKind::TransactionBinding)?,
                body: self.lower_expression(body)?,
            },
        };
        self.insert_created(OwnerRecord::Expression(ExpressionRecord::new(
            id, operation,
        )?))?;
        Ok(id)
    }

    fn lower_types(
        &mut self,
        values: &[AuthoredType],
    ) -> Result<Vec<TypeObjectDigest>, Diagnostic> {
        values.iter().map(|value| self.lower_type(value)).collect()
    }

    fn lower_expressions(
        &mut self,
        values: &[AuthoredExpression],
    ) -> Result<Vec<crate::platform::semantic_id::ExpressionId>, Diagnostic> {
        values
            .iter()
            .map(|value| self.lower_expression(value))
            .collect()
    }

    fn lower_declaration_reference(
        &mut self,
        selector: &AuthoredDeclarationReference,
    ) -> Result<DeclarationReference, Diagnostic> {
        match selector {
            AuthoredDeclarationReference::Local { declaration } => Ok(DeclarationReference {
                package: self.base.package_id(),
                declaration: self.resolve_creation_declaration(declaration)?,
            }),
            AuthoredDeclarationReference::Exact {
                package,
                declaration,
            } => {
                if *package == self.base.package_id() {
                    self.require_owner(OwnerKey::Declaration(*declaration))?;
                }
                Ok(DeclarationReference {
                    package: *package,
                    declaration: *declaration,
                })
            }
        }
    }

    fn lower_type_parameter_reference(
        &mut self,
        selector: &AuthoredTypeParameterReference,
    ) -> Result<TypeParameterId, Diagnostic> {
        match selector {
            AuthoredTypeParameterReference::Id { parameter } => {
                self.require_owner(OwnerKey::TypeParameter(*parameter))?;
                Ok(*parameter)
            }
            AuthoredTypeParameterReference::Symbol { symbol } => self.type_parameter_symbol(symbol),
        }
    }

    fn lower_local_reference(
        &mut self,
        selector: &AuthoredLocalReference,
    ) -> Result<LocalValueReference, Diagnostic> {
        let reference = match selector {
            AuthoredLocalReference::FunctionParameter { parameter } => {
                self.require_owner(OwnerKey::Parameter(*parameter))?;
                LocalValueReference::FunctionParameter(*parameter)
            }
            AuthoredLocalReference::OperationParameter { parameter } => {
                self.require_owner(OwnerKey::Parameter(*parameter))?;
                LocalValueReference::OperationParameter(*parameter)
            }
            AuthoredLocalReference::LexicalBinding { binding } => {
                self.require_owner(OwnerKey::Binding(*binding))?;
                LocalValueReference::LexicalBinding(*binding)
            }
            AuthoredLocalReference::MatchPayload { binding } => {
                self.require_owner(OwnerKey::Binding(*binding))?;
                LocalValueReference::MatchPayload(*binding)
            }
            AuthoredLocalReference::TransactionBinding { binding } => {
                self.require_owner(OwnerKey::Binding(*binding))?;
                LocalValueReference::TransactionBinding(*binding)
            }
            AuthoredLocalReference::Symbol { symbol } => match self.symbol_kind(symbol)? {
                SymbolKind::FunctionParameter => {
                    LocalValueReference::FunctionParameter(self.function_parameter_symbol(symbol)?)
                }
                SymbolKind::OperationParameter => LocalValueReference::OperationParameter(
                    self.operation_parameter_symbol(symbol)?,
                ),
                SymbolKind::LexicalBinding => {
                    LocalValueReference::LexicalBinding(self.lexical_binding_symbol(symbol)?)
                }
                SymbolKind::MatchPayloadBinding => {
                    LocalValueReference::MatchPayload(self.match_payload_symbol(symbol)?)
                }
                SymbolKind::TransactionBinding => LocalValueReference::TransactionBinding(
                    self.transaction_binding_symbol(symbol)?,
                ),
                _ => {
                    return Err(request_error(
                        DiagnosticClass::Semantic,
                        "change_authored_local_symbol_kind",
                        format!("request-local symbol {symbol} is not a local value"),
                    ));
                }
            },
        };
        Ok(reference)
    }

    fn lower_field_selector(
        &mut self,
        selector: &AuthoredFieldSelector,
    ) -> Result<FieldSelector, Diagnostic> {
        match selector {
            AuthoredFieldSelector::Nominal { field } => {
                Ok(FieldSelector::Nominal(self.lower_field_reference(field)?))
            }
            AuthoredFieldSelector::Structural { name } => {
                Ok(FieldSelector::Structural(name.clone()))
            }
        }
    }

    fn lower_field_reference(
        &mut self,
        selector: &AuthoredFieldReference,
    ) -> Result<FieldReference, Diagnostic> {
        match selector {
            AuthoredFieldReference::Exact { package, field } => {
                if *package == self.base.package_id() {
                    self.require_owner(OwnerKey::Field(*field))?;
                }
                Ok(FieldReference {
                    package: *package,
                    field: *field,
                })
            }
            AuthoredFieldReference::Symbol { symbol } => Ok(FieldReference {
                package: self.base.package_id(),
                field: self.field_symbol(symbol)?,
            }),
        }
    }

    fn lower_case_reference(
        &mut self,
        selector: &AuthoredCaseReference,
    ) -> Result<CaseReference, Diagnostic> {
        match selector {
            AuthoredCaseReference::Exact { package, case } => {
                if *package == self.base.package_id() {
                    self.require_owner(OwnerKey::Case(*case))?;
                }
                Ok(CaseReference {
                    package: *package,
                    case: *case,
                })
            }
            AuthoredCaseReference::Symbol { symbol } => Ok(CaseReference {
                package: self.base.package_id(),
                case: self.case_symbol(symbol)?,
            }),
        }
    }

    fn lower_operation_reference(
        &mut self,
        selector: &AuthoredOperationReference,
    ) -> Result<OperationReference, Diagnostic> {
        match selector {
            AuthoredOperationReference::Exact { package, operation } => {
                if *package == self.base.package_id() {
                    self.require_owner(OwnerKey::Operation(*operation))?;
                }
                Ok(OperationReference {
                    package: *package,
                    operation: *operation,
                })
            }
            AuthoredOperationReference::Symbol { symbol } => Ok(OperationReference {
                package: self.base.package_id(),
                operation: self.operation_symbol(symbol)?,
            }),
        }
    }

    fn lower_requirement_reference(
        &mut self,
        selector: &AuthoredRequirementReference,
    ) -> Result<RequirementReference, Diagnostic> {
        match selector {
            AuthoredRequirementReference::Exact {
                package,
                requirement,
            } => {
                if *package == self.base.package_id() {
                    self.require_owner(OwnerKey::Requirement(*requirement))?;
                }
                Ok(RequirementReference {
                    package: *package,
                    requirement: *requirement,
                })
            }
            AuthoredRequirementReference::Symbol { symbol } => Ok(RequirementReference {
                package: self.base.package_id(),
                requirement: self.requirement_symbol(symbol)?,
            }),
        }
    }

    fn insert_scoped_binding(
        &mut self,
        definition: &AuthoredBindingDefinition,
        kind: SymbolKind,
    ) -> Result<BindingId, Diagnostic> {
        let (id, binding_kind, declared_type) = match kind {
            SymbolKind::MatchPayloadBinding => (
                self.match_payload_symbol(&definition.symbol)?,
                crate::platform::kernel::BindingKind::MatchPayload,
                Some(
                    self.lower_type(definition.declared_type.as_ref().ok_or_else(|| {
                        request_error(
                            DiagnosticClass::Semantic,
                            "change_match_binding_type",
                            "match payload binding requires its exact case payload type",
                        )
                    })?)?,
                ),
            ),
            SymbolKind::TransactionBinding => (
                self.transaction_binding_symbol(&definition.symbol)?,
                crate::platform::kernel::BindingKind::Transaction,
                Some(self.lower_type(&AuthoredType::Unit {})?),
            ),
            _ => {
                return Err(request_error(
                    DiagnosticClass::Corrupt,
                    "change_authored_binding_definition_kind",
                    "scoped binding creation received a foreign symbol kind",
                ));
            }
        };
        self.insert_created(OwnerRecord::Binding(
            crate::platform::kernel::BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(id), OwnerKind::Binding),
                name: definition.name.clone(),
                kind: binding_kind,
                value: None,
                declared_type,
            },
        ))?;
        Ok(id)
    }
}
