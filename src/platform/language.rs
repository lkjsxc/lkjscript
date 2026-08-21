//! Authored module AST and exact source-to-AST validation.

use super::diagnostic::{Diagnostic, SourceLocation};
use super::syntax::{Form, FormKind, SourceDocument, SourceSpan};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Module {
    pub name: String,
    pub imports: Vec<Import>,
    pub exports: Vec<String>,
    pub declarations: Vec<Declaration>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Import {
    pub alias: String,
    pub module: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Declaration {
    Record(RecordType),
    Variant(VariantType),
    Interface(Interface),
    External(ExternalFunction),
    Function(Function),
    Constant(Constant),
    Component(Component),
    Test(TestCase),
}

impl Declaration {
    pub fn name(&self) -> &str {
        match self {
            Self::Record(value) => &value.name,
            Self::Variant(value) => &value.name,
            Self::Interface(value) => &value.name,
            Self::External(value) => &value.name,
            Self::Function(value) => &value.name,
            Self::Constant(value) => &value.name,
            Self::Component(value) => &value.name,
            Self::Test(value) => &value.name,
        }
    }

    pub fn span(&self) -> &SourceSpan {
        match self {
            Self::Record(value) => &value.span,
            Self::Variant(value) => &value.span,
            Self::Interface(value) => &value.span,
            Self::External(value) => &value.span,
            Self::Function(value) => &value.span,
            Self::Constant(value) => &value.span,
            Self::Component(value) => &value.span,
            Self::Test(value) => &value.span,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordType {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VariantType {
    pub name: String,
    pub cases: Vec<VariantCase>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VariantCase {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Type>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Interface {
    pub name: String,
    pub operations: Vec<InterfaceOperation>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InterfaceOperation {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub result: Type,
    pub idempotency: Idempotency,
    pub visibility: Visibility,
    pub span: SourceSpan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Idempotency {
    Idempotent,
    IdempotentWithKey,
    NonIdempotent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Visibility {
    None,
    Possible,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalFunction {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub result: Type,
    pub implementation: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub result: Type,
    pub effect: Effect,
    pub body: Expression,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Effect {
    Pure,
    Task { capabilities: Vec<TaskCapability> },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskCapability {
    pub alias: String,
    pub interface: String,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Constant {
    pub name: String,
    pub ty: Type,
    pub value: Expression,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub name: String,
    pub requirements: Vec<Requirement>,
    pub ports: Vec<Port>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Requirement {
    pub alias: String,
    pub interface: String,
    pub operations: Vec<String>,
    pub limits: Vec<NamedLimit>,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NamedLimit {
    pub name: String,
    pub value: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Port {
    pub name: String,
    pub ty: Type,
    pub value: Expression,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TestCase {
    pub name: String,
    pub actual: Expression,
    pub expected: Expression,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Type {
    Unit,
    Bool,
    I64,
    Bytes,
    Text,
    StaticText,
    Secret,
    Named(String),
    Record(Vec<TypeField>),
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Stream(Box<Type>),
    Function(Vec<Type>, Box<Type>),
}

impl Type {
    pub fn is_durable(&self) -> bool {
        match self {
            Self::Secret | Self::Stream(_) | Self::Function(_, _) => false,
            Self::List(item) | Self::Option(item) => item.is_durable(),
            Self::Record(fields) => fields.iter().all(|field| field.ty.is_durable()),
            Self::Map(key, value) | Self::Result(key, value) => {
                key.is_durable() && value.is_durable()
            }
            Self::Unit
            | Self::Bool
            | Self::I64
            | Self::Bytes
            | Self::Text
            | Self::StaticText
            | Self::Named(_) => true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeField {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Expression {
    Unit(SourceSpan),
    Bool(bool, SourceSpan),
    I64(i64, SourceSpan),
    Text(String, SourceSpan),
    StaticText(String, SourceSpan),
    Variable(String, SourceSpan),
    If {
        condition: Box<Expression>,
        when_true: Box<Expression>,
        when_false: Box<Expression>,
        span: SourceSpan,
    },
    Let {
        bindings: Vec<Binding>,
        body: Box<Expression>,
        span: SourceSpan,
    },
    Do {
        expressions: Vec<Expression>,
        span: SourceSpan,
    },
    Call {
        function: String,
        arguments: Vec<Expression>,
        span: SourceSpan,
    },
    Record {
        ty: Option<String>,
        fields: Vec<RecordField>,
        span: SourceSpan,
    },
    Variant {
        ty: String,
        case: String,
        payload: Option<Box<Expression>>,
        span: SourceSpan,
    },
    Field {
        value: Box<Expression>,
        field: String,
        span: SourceSpan,
    },
    List {
        item_type: Type,
        items: Vec<Expression>,
        span: SourceSpan,
    },
    Map {
        key_type: Type,
        value_type: Type,
        entries: Vec<MapEntry>,
        span: SourceSpan,
    },
    Match {
        value: Box<Expression>,
        arms: Vec<MatchArm>,
        span: SourceSpan,
    },
    FunctionRef {
        function: String,
        span: SourceSpan,
    },
    Perform {
        capability: String,
        operation: String,
        arguments: Vec<Expression>,
        span: SourceSpan,
    },
    Transaction {
        capability: String,
        binding: String,
        body: Box<Expression>,
        span: SourceSpan,
    },
}

impl Expression {
    pub fn span(&self) -> &SourceSpan {
        match self {
            Self::Unit(span)
            | Self::Bool(_, span)
            | Self::I64(_, span)
            | Self::Text(_, span)
            | Self::StaticText(_, span)
            | Self::Variable(_, span) => span,
            Self::If { span, .. }
            | Self::Let { span, .. }
            | Self::Do { span, .. }
            | Self::Call { span, .. }
            | Self::Record { span, .. }
            | Self::Variant { span, .. }
            | Self::Field { span, .. }
            | Self::List { span, .. }
            | Self::Map { span, .. }
            | Self::Match { span, .. }
            | Self::FunctionRef { span, .. }
            | Self::Perform { span, .. }
            | Self::Transaction { span, .. } => span,
        }
    }

    pub fn performed_capabilities(&self, output: &mut BTreeSet<String>) {
        match self {
            Self::Perform {
                capability,
                arguments,
                ..
            } => {
                output.insert(capability.clone());
                for argument in arguments {
                    argument.performed_capabilities(output);
                }
            }
            Self::Transaction {
                capability, body, ..
            } => {
                output.insert(capability.clone());
                body.performed_capabilities(output);
            }
            Self::If {
                condition,
                when_true,
                when_false,
                ..
            } => {
                condition.performed_capabilities(output);
                when_true.performed_capabilities(output);
                when_false.performed_capabilities(output);
            }
            Self::Let { bindings, body, .. } => {
                for binding in bindings {
                    binding.value.performed_capabilities(output);
                }
                body.performed_capabilities(output);
            }
            Self::Do { expressions, .. }
            | Self::List {
                items: expressions, ..
            } => {
                for expression in expressions {
                    expression.performed_capabilities(output);
                }
            }
            Self::Call { arguments, .. } => {
                for argument in arguments {
                    argument.performed_capabilities(output);
                }
            }
            Self::Record { fields, .. } => {
                for field in fields {
                    field.value.performed_capabilities(output);
                }
            }
            Self::Variant { payload, .. } => {
                if let Some(payload) = payload {
                    payload.performed_capabilities(output);
                }
            }
            Self::Field { value, .. } | Self::Match { value, .. } => {
                value.performed_capabilities(output);
                if let Self::Match { arms, .. } = self {
                    for arm in arms {
                        arm.body.performed_capabilities(output);
                    }
                }
            }
            Self::Map { entries, .. } => {
                for entry in entries {
                    entry.key.performed_capabilities(output);
                    entry.value.performed_capabilities(output);
                }
            }
            Self::Unit(_)
            | Self::Bool(_, _)
            | Self::I64(_, _)
            | Self::Text(_, _)
            | Self::StaticText(_, _)
            | Self::Variable(_, _)
            | Self::FunctionRef { .. } => {}
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub name: String,
    pub value: Expression,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecordField {
    pub name: String,
    pub value: Expression,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MapEntry {
    pub key: Expression,
    pub value: Expression,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MatchArm {
    pub case: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,
    pub body: Expression,
    pub span: SourceSpan,
}

pub fn parse_module(document: &SourceDocument) -> Result<Module, Diagnostic> {
    if document.forms().len() != 1 {
        return Err(at_document(
            document,
            document.forms()[1].span.clone(),
            "module_root_count",
            "a source file must contain exactly one module form",
        ));
    }
    let root = list(
        document,
        &document.forms()[0],
        "module_root",
        "expected a module list",
    )?;
    if root.len() < 2 || atom(document, &root[0], "module_keyword")? != "module" {
        return Err(at(
            document,
            &root[0],
            "module_keyword",
            "the root form must start with 'module'",
        ));
    }
    let name = identifier(document, &root[1], IdentifierKind::Module)?;
    let mut imports = Vec::new();
    let mut exports = Vec::new();
    let mut declarations = Vec::new();
    for form in &root[2..] {
        let items = list(document, form, "module_item", "module item must be a list")?;
        if items.is_empty() {
            return Err(at(
                document,
                form,
                "module_item_empty",
                "module item is empty",
            ));
        }
        match atom(document, &items[0], "module_item_kind")? {
            "import" => imports.push(parse_import(document, form, items)?),
            "export" => parse_exports(document, form, items, &mut exports)?,
            "record" => {
                declarations.push(Declaration::Record(parse_record(document, form, items)?))
            }
            "variant" => {
                declarations.push(Declaration::Variant(parse_variant(document, form, items)?))
            }
            "interface" => declarations.push(Declaration::Interface(parse_interface(
                document, form, items,
            )?)),
            "extern" => declarations.push(Declaration::External(parse_external(
                document, form, items,
            )?)),
            "fn" | "task" => declarations.push(Declaration::Function(parse_function(
                document, form, items,
            )?)),
            "const" => declarations.push(Declaration::Constant(parse_constant(
                document, form, items,
            )?)),
            "component" => declarations.push(Declaration::Component(parse_component(
                document, form, items,
            )?)),
            "test" => declarations.push(Declaration::Test(parse_test(document, form, items)?)),
            other => {
                return Err(at(
                    document,
                    &items[0],
                    "module_item_unknown",
                    format!("unknown module item '{other}'"),
                ));
            }
        }
    }
    reject_duplicates(
        document,
        imports.iter().map(|value| (&value.alias, &value.span)),
        "import_alias_duplicate",
        "import alias",
    )?;
    reject_duplicates(
        document,
        declarations
            .iter()
            .map(|value| (value.name(), value.span())),
        "declaration_duplicate",
        "declaration",
    )?;
    reject_duplicates(
        document,
        exports.iter().map(|value| (value, &root[0].span)),
        "export_duplicate",
        "export",
    )?;
    Ok(Module {
        name,
        imports,
        exports,
        declarations,
    })
}

fn parse_import(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<Import, Diagnostic> {
    exact_arity(document, form, items, 3, "import_arity")?;
    Ok(Import {
        alias: identifier(document, &items[1], IdentifierKind::Value)?,
        module: identifier(document, &items[2], IdentifierKind::Module)?,
        span: form.span.clone(),
    })
}

fn parse_exports(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
    output: &mut Vec<String>,
) -> Result<(), Diagnostic> {
    if items.len() < 2 {
        return Err(at(
            document,
            form,
            "export_arity",
            "export requires at least one declaration name",
        ));
    }
    for item in &items[1..] {
        output.push(identifier(document, item, IdentifierKind::Declaration)?);
    }
    Ok(())
}

fn parse_record(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<RecordType, Diagnostic> {
    if items.len() < 2 {
        return Err(at(
            document,
            form,
            "record_arity",
            "record requires a type name",
        ));
    }
    let name = identifier(document, &items[1], IdentifierKind::Type)?;
    let mut fields = Vec::new();
    for item in &items[2..] {
        let pair = list(
            document,
            item,
            "record_field",
            "record field must be '(name Type)'",
        )?;
        exact_arity(document, item, pair, 2, "record_field_arity")?;
        fields.push(Field {
            name: identifier(document, &pair[0], IdentifierKind::Value)?,
            ty: parse_type(document, &pair[1])?,
            span: item.span.clone(),
        });
    }
    reject_duplicates(
        document,
        fields.iter().map(|value| (&value.name, &value.span)),
        "record_field_duplicate",
        "record field",
    )?;
    Ok(RecordType {
        name,
        fields,
        span: form.span.clone(),
    })
}

fn parse_variant(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<VariantType, Diagnostic> {
    if items.len() < 3 {
        return Err(at(
            document,
            form,
            "variant_arity",
            "variant requires a type name and at least one case",
        ));
    }
    let name = identifier(document, &items[1], IdentifierKind::Type)?;
    let mut cases = Vec::new();
    for item in &items[2..] {
        let pair = list(
            document,
            item,
            "variant_case",
            "variant case must be a list",
        )?;
        if !(1..=2).contains(&pair.len()) {
            return Err(at(
                document,
                item,
                "variant_case_arity",
                "variant case must be '(Case)' or '(Case Type)'",
            ));
        }
        cases.push(VariantCase {
            name: identifier(document, &pair[0], IdentifierKind::Type)?,
            payload: pair
                .get(1)
                .map(|value| parse_type(document, value))
                .transpose()?,
            span: item.span.clone(),
        });
    }
    reject_duplicates(
        document,
        cases.iter().map(|value| (&value.name, &value.span)),
        "variant_case_duplicate",
        "variant case",
    )?;
    Ok(VariantType {
        name,
        cases,
        span: form.span.clone(),
    })
}

fn parse_interface(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<Interface, Diagnostic> {
    if items.len() < 3 {
        return Err(at(
            document,
            form,
            "interface_arity",
            "interface requires a name and at least one operation",
        ));
    }
    let name = identifier(document, &items[1], IdentifierKind::Type)?;
    let mut operations = Vec::new();
    for operation in &items[2..] {
        let values = list(
            document,
            operation,
            "interface_operation",
            "interface operation must be a list",
        )?;
        exact_arity(document, operation, values, 6, "interface_operation_arity")?;
        if atom(document, &values[0], "interface_operation_keyword")? != "operation" {
            return Err(at(
                document,
                &values[0],
                "interface_operation_keyword",
                "interface item must start with 'operation'",
            ));
        }
        let idempotency = match atom(document, &values[4], "operation_idempotency")? {
            "idempotent" => Idempotency::Idempotent,
            "idempotent-with-key" => Idempotency::IdempotentWithKey,
            "non-idempotent" => Idempotency::NonIdempotent,
            other => {
                return Err(at(
                    document,
                    &values[4],
                    "operation_idempotency",
                    format!("unknown operation idempotency '{other}'"),
                ));
            }
        };
        let visibility = match atom(document, &values[5], "operation_visibility")? {
            "no-visibility" => Visibility::None,
            "possible-visibility" => Visibility::Possible,
            other => {
                return Err(at(
                    document,
                    &values[5],
                    "operation_visibility",
                    format!("unknown operation visibility '{other}'"),
                ));
            }
        };
        operations.push(InterfaceOperation {
            name: identifier(document, &values[1], IdentifierKind::Value)?,
            parameters: parse_parameters(document, &values[2])?,
            result: parse_type(document, &values[3])?,
            idempotency,
            visibility,
            span: operation.span.clone(),
        });
    }
    reject_duplicates(
        document,
        operations
            .iter()
            .map(|operation| (&operation.name, &operation.span)),
        "interface_operation_duplicate",
        "interface operation",
    )?;
    Ok(Interface {
        name,
        operations,
        span: form.span.clone(),
    })
}

fn parse_external(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<ExternalFunction, Diagnostic> {
    exact_arity(document, form, items, 5, "external_arity")?;
    Ok(ExternalFunction {
        name: identifier(document, &items[1], IdentifierKind::Value)?,
        parameters: parse_parameters(document, &items[2])?,
        result: parse_type(document, &items[3])?,
        implementation: identifier(document, &items[4], IdentifierKind::Qualified)?,
        span: form.span.clone(),
    })
}

fn parse_function(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<Function, Diagnostic> {
    let is_task = atom(document, &items[0], "function_kind")? == "task";
    let expected = if is_task { 6 } else { 5 };
    exact_arity(document, form, items, expected, "function_arity")?;
    let name = identifier(document, &items[1], IdentifierKind::Value)?;
    let parameters = parse_parameters(document, &items[2])?;
    let result = parse_type(document, &items[3])?;
    let (effect, body_index) = if is_task {
        let requirement_forms = list(
            document,
            &items[4],
            "task_requires",
            "task capability declaration must be '(requires (alias interface) ...)'",
        )?;
        if requirement_forms.is_empty()
            || atom(document, &requirement_forms[0], "task_requires_keyword")? != "requires"
        {
            return Err(at(
                document,
                &items[4],
                "task_requires_keyword",
                "task capability declaration must start with 'requires'",
            ));
        }
        let mut capabilities = Vec::new();
        for item in &requirement_forms[1..] {
            let pair = list(
                document,
                item,
                "task_capability",
                "task capability must be '(alias interface)'",
            )?;
            exact_arity(document, item, pair, 2, "task_capability_arity")?;
            capabilities.push(TaskCapability {
                alias: identifier(document, &pair[0], IdentifierKind::Value)?,
                interface: identifier(document, &pair[1], IdentifierKind::Qualified)?,
                span: item.span.clone(),
            });
        }
        reject_duplicates(
            document,
            capabilities.iter().map(|value| (&value.alias, &value.span)),
            "task_capability_duplicate",
            "task capability",
        )?;
        (Effect::Task { capabilities }, 5)
    } else {
        (Effect::Pure, 4)
    };
    Ok(Function {
        name,
        parameters,
        result,
        effect,
        body: parse_expression(document, &items[body_index])?,
        span: form.span.clone(),
    })
}

fn parse_parameters(document: &SourceDocument, form: &Form) -> Result<Vec<Parameter>, Diagnostic> {
    let parameter_forms = list(
        document,
        form,
        "function_parameters",
        "parameters must be a list",
    )?;
    let mut parameters = Vec::new();
    for item in parameter_forms {
        let pair = list(
            document,
            item,
            "function_parameter",
            "parameter must be '(name Type)'",
        )?;
        exact_arity(document, item, pair, 2, "function_parameter_arity")?;
        parameters.push(Parameter {
            name: identifier(document, &pair[0], IdentifierKind::Value)?,
            ty: parse_type(document, &pair[1])?,
            span: item.span.clone(),
        });
    }
    reject_duplicates(
        document,
        parameters.iter().map(|value| (&value.name, &value.span)),
        "function_parameter_duplicate",
        "function parameter",
    )?;
    Ok(parameters)
}

fn parse_constant(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<Constant, Diagnostic> {
    exact_arity(document, form, items, 4, "constant_arity")?;
    Ok(Constant {
        name: identifier(document, &items[1], IdentifierKind::Value)?,
        ty: parse_type(document, &items[2])?,
        value: parse_expression(document, &items[3])?,
        span: form.span.clone(),
    })
}

fn parse_component(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<Component, Diagnostic> {
    if items.len() < 3 {
        return Err(at(
            document,
            form,
            "component_arity",
            "component requires a name and at least one port",
        ));
    }
    let name = identifier(document, &items[1], IdentifierKind::Type)?;
    let mut requirements = Vec::new();
    let mut ports = Vec::new();
    for item in &items[2..] {
        let child = list(
            document,
            item,
            "component_item",
            "component item must be a list",
        )?;
        if child.is_empty() {
            return Err(at(
                document,
                item,
                "component_item_empty",
                "component item is empty",
            ));
        }
        match atom(document, &child[0], "component_item_kind")? {
            "require" => requirements.push(parse_requirement(document, item, child)?),
            "port" => {
                exact_arity(document, item, child, 4, "port_arity")?;
                ports.push(Port {
                    name: identifier(document, &child[1], IdentifierKind::Value)?,
                    ty: parse_type(document, &child[2])?,
                    value: parse_expression(document, &child[3])?,
                    span: item.span.clone(),
                });
            }
            other => {
                return Err(at(
                    document,
                    &child[0],
                    "component_item_unknown",
                    format!("unknown component item '{other}'"),
                ));
            }
        }
    }
    if ports.is_empty() {
        return Err(at(
            document,
            form,
            "component_without_port",
            "component must expose at least one typed port",
        ));
    }
    reject_duplicates(
        document,
        requirements.iter().map(|value| (&value.alias, &value.span)),
        "component_requirement_duplicate",
        "component requirement",
    )?;
    reject_duplicates(
        document,
        ports.iter().map(|value| (&value.name, &value.span)),
        "component_port_duplicate",
        "component port",
    )?;
    Ok(Component {
        name,
        requirements,
        ports,
        span: form.span.clone(),
    })
}

fn parse_requirement(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<Requirement, Diagnostic> {
    if items.len() < 4 {
        return Err(at(
            document,
            form,
            "requirement_arity",
            "requirement needs an alias, interface, and operations",
        ));
    }
    let alias = identifier(document, &items[1], IdentifierKind::Value)?;
    let interface = identifier(document, &items[2], IdentifierKind::Qualified)?;
    let operations_form = list(
        document,
        &items[3],
        "requirement_operations",
        "requirement operations must be a list",
    )?;
    if operations_form.len() < 2
        || atom(
            document,
            &operations_form[0],
            "requirement_operations_keyword",
        )? != "operations"
    {
        return Err(at(
            document,
            &items[3],
            "requirement_operations_keyword",
            "requirement operations must start with 'operations'",
        ));
    }
    let mut operations = Vec::new();
    for operation in &operations_form[1..] {
        operations.push(identifier(document, operation, IdentifierKind::Value)?);
    }
    let mut limits = Vec::new();
    for limit in &items[4..] {
        let pair = list(
            document,
            limit,
            "requirement_limit",
            "limit must be '(limit name value)'",
        )?;
        exact_arity(document, limit, pair, 3, "requirement_limit_arity")?;
        if atom(document, &pair[0], "requirement_limit_keyword")? != "limit" {
            return Err(at(
                document,
                &pair[0],
                "requirement_limit_keyword",
                "requirement bound must start with 'limit'",
            ));
        }
        let value = integer(document, &pair[2], "requirement_limit_value")?;
        let value = u64::try_from(value).map_err(|_| {
            at(
                document,
                &pair[2],
                "requirement_limit_range",
                "requirement limit must be non-negative",
            )
        })?;
        limits.push(NamedLimit {
            name: identifier(document, &pair[1], IdentifierKind::Value)?,
            value,
        });
    }
    reject_duplicates(
        document,
        operations.iter().map(|value| (value, &items[3].span)),
        "requirement_operation_duplicate",
        "requirement operation",
    )?;
    reject_duplicates(
        document,
        limits.iter().map(|value| (&value.name, &form.span)),
        "requirement_limit_duplicate",
        "requirement limit",
    )?;
    Ok(Requirement {
        alias,
        interface,
        operations,
        limits,
        span: form.span.clone(),
    })
}

fn parse_test(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<TestCase, Diagnostic> {
    exact_arity(document, form, items, 4, "test_arity")?;
    Ok(TestCase {
        name: identifier(document, &items[1], IdentifierKind::Value)?,
        actual: parse_expression(document, &items[2])?,
        expected: parse_expression(document, &items[3])?,
        span: form.span.clone(),
    })
}

pub fn parse_type(document: &SourceDocument, form: &Form) -> Result<Type, Diagnostic> {
    if let Some(name) = form.atom() {
        return Ok(match name {
            "Unit" => Type::Unit,
            "Bool" => Type::Bool,
            "I64" => Type::I64,
            "Bytes" => Type::Bytes,
            "Text" => Type::Text,
            "StaticText" => Type::StaticText,
            "Secret" => Type::Secret,
            _ => {
                validate_identifier(name, IdentifierKind::Qualified)
                    .map_err(|message| at(document, form, "type_name", message))?;
                Type::Named(name.to_owned())
            }
        });
    }
    let items = list(
        document,
        form,
        "type_form",
        "type must be a name or type application",
    )?;
    if items.is_empty() {
        return Err(at(
            document,
            form,
            "type_empty",
            "type application is empty",
        ));
    }
    let constructor = atom(document, &items[0], "type_constructor")?;
    let unary = |kind: fn(Box<Type>) -> Type| -> Result<Type, Diagnostic> {
        exact_arity(document, form, items, 2, "type_arity")?;
        Ok(kind(Box::new(parse_type(document, &items[1])?)))
    };
    match constructor {
        "List" => unary(Type::List),
        "Option" => unary(Type::Option),
        "Stream" => unary(Type::Stream),
        "Map" | "Result" => {
            exact_arity(document, form, items, 3, "type_arity")?;
            let first = Box::new(parse_type(document, &items[1])?);
            let second = Box::new(parse_type(document, &items[2])?);
            Ok(if constructor == "Map" {
                Type::Map(first, second)
            } else {
                Type::Result(first, second)
            })
        }
        "Record" => {
            let mut fields = Vec::new();
            for item in &items[1..] {
                let pair = list(
                    document,
                    item,
                    "record_type_field",
                    "structural record field must be '(name Type)'",
                )?;
                exact_arity(document, item, pair, 2, "record_type_field_arity")?;
                fields.push(TypeField {
                    name: identifier(document, &pair[0], IdentifierKind::Value)?,
                    ty: parse_type(document, &pair[1])?,
                });
            }
            let mut names = BTreeSet::new();
            for field in &fields {
                if !names.insert(&field.name) {
                    return Err(at(
                        document,
                        form,
                        "record_type_field_duplicate",
                        format!("duplicate structural record field '{}'", field.name),
                    ));
                }
            }
            Ok(Type::Record(fields))
        }
        "Function" => {
            exact_arity(document, form, items, 3, "type_arity")?;
            let parameters = list(
                document,
                &items[1],
                "function_type_parameters",
                "function type parameters must be a list",
            )?;
            Ok(Type::Function(
                parameters
                    .iter()
                    .map(|value| parse_type(document, value))
                    .collect::<Result<Vec<_>, _>>()?,
                Box::new(parse_type(document, &items[2])?),
            ))
        }
        _ => Err(at(
            document,
            &items[0],
            "type_constructor_unknown",
            format!("unknown type constructor '{constructor}'"),
        )),
    }
}

pub fn parse_expression(document: &SourceDocument, form: &Form) -> Result<Expression, Diagnostic> {
    match &form.value {
        FormKind::Integer(value) => return Ok(Expression::I64(*value, form.span.clone())),
        FormKind::String(value) => {
            return Ok(Expression::Text(value.clone(), form.span.clone()));
        }
        FormKind::Atom(value) => {
            return Ok(match value.as_str() {
                "unit" => Expression::Unit(form.span.clone()),
                "true" => Expression::Bool(true, form.span.clone()),
                "false" => Expression::Bool(false, form.span.clone()),
                name => {
                    validate_identifier(name, IdentifierKind::Qualified)
                        .map_err(|message| at(document, form, "expression_name", message))?;
                    Expression::Variable(name.to_owned(), form.span.clone())
                }
            });
        }
        FormKind::List(_) => {}
    }
    let items = form.list().ok_or_else(|| {
        at(
            document,
            form,
            "expression_form",
            "expression must be a value or list",
        )
    })?;
    if items.is_empty() {
        return Err(at(
            document,
            form,
            "expression_empty",
            "expression list is empty",
        ));
    }
    let keyword = atom(document, &items[0], "expression_keyword")?;
    match keyword {
        "static-text" => {
            exact_arity(document, form, items, 2, "static_text_arity")?;
            let FormKind::String(value) = &items[1].value else {
                return Err(at(
                    document,
                    &items[1],
                    "static_text_literal",
                    "static-text requires one authored string literal",
                ));
            };
            Ok(Expression::StaticText(value.clone(), form.span.clone()))
        }
        "if" => {
            exact_arity(document, form, items, 4, "if_arity")?;
            Ok(Expression::If {
                condition: Box::new(parse_expression(document, &items[1])?),
                when_true: Box::new(parse_expression(document, &items[2])?),
                when_false: Box::new(parse_expression(document, &items[3])?),
                span: form.span.clone(),
            })
        }
        "let" => parse_let(document, form, items),
        "do" => {
            if items.len() < 2 {
                return Err(at(
                    document,
                    form,
                    "do_arity",
                    "do requires at least one expression",
                ));
            }
            Ok(Expression::Do {
                expressions: items[1..]
                    .iter()
                    .map(|value| parse_expression(document, value))
                    .collect::<Result<Vec<_>, _>>()?,
                span: form.span.clone(),
            })
        }
        "call" => {
            if items.len() < 2 {
                return Err(at(
                    document,
                    form,
                    "call_arity",
                    "call requires a function name",
                ));
            }
            Ok(Expression::Call {
                function: identifier(document, &items[1], IdentifierKind::Qualified)?,
                arguments: items[2..]
                    .iter()
                    .map(|value| parse_expression(document, value))
                    .collect::<Result<Vec<_>, _>>()?,
                span: form.span.clone(),
            })
        }
        "record" => parse_record_expression(document, form, items),
        "variant" => {
            if !(3..=4).contains(&items.len()) {
                return Err(at(
                    document,
                    form,
                    "variant_expression_arity",
                    "variant expression must be '(variant Type Case [payload])'",
                ));
            }
            Ok(Expression::Variant {
                ty: identifier(document, &items[1], IdentifierKind::Qualified)?,
                case: identifier(document, &items[2], IdentifierKind::Type)?,
                payload: items
                    .get(3)
                    .map(|value| parse_expression(document, value).map(Box::new))
                    .transpose()?,
                span: form.span.clone(),
            })
        }
        "field" => {
            exact_arity(document, form, items, 3, "field_arity")?;
            Ok(Expression::Field {
                value: Box::new(parse_expression(document, &items[1])?),
                field: identifier(document, &items[2], IdentifierKind::Value)?,
                span: form.span.clone(),
            })
        }
        "list" => {
            if items.len() < 2 {
                return Err(at(
                    document,
                    form,
                    "list_arity",
                    "list requires an explicit item type",
                ));
            }
            Ok(Expression::List {
                item_type: parse_type(document, &items[1])?,
                items: items[2..]
                    .iter()
                    .map(|value| parse_expression(document, value))
                    .collect::<Result<Vec<_>, _>>()?,
                span: form.span.clone(),
            })
        }
        "map" => parse_map(document, form, items),
        "match" => parse_match(document, form, items),
        "function" => {
            exact_arity(document, form, items, 2, "function_reference_arity")?;
            Ok(Expression::FunctionRef {
                function: identifier(document, &items[1], IdentifierKind::Qualified)?,
                span: form.span.clone(),
            })
        }
        "perform" => {
            if items.len() < 3 {
                return Err(at(
                    document,
                    form,
                    "perform_arity",
                    "perform requires a capability alias and operation",
                ));
            }
            Ok(Expression::Perform {
                capability: identifier(document, &items[1], IdentifierKind::Value)?,
                operation: identifier(document, &items[2], IdentifierKind::Value)?,
                arguments: items[3..]
                    .iter()
                    .map(|value| parse_expression(document, value))
                    .collect::<Result<Vec<_>, _>>()?,
                span: form.span.clone(),
            })
        }
        "transaction" => {
            exact_arity(document, form, items, 4, "transaction_arity")?;
            Ok(Expression::Transaction {
                capability: identifier(document, &items[1], IdentifierKind::Value)?,
                binding: identifier(document, &items[2], IdentifierKind::Value)?,
                body: Box::new(parse_expression(document, &items[3])?),
                span: form.span.clone(),
            })
        }
        other => Err(at(
            document,
            &items[0],
            "expression_unknown",
            format!("unknown expression '{other}'; calls must start with 'call'"),
        )),
    }
}

fn parse_let(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<Expression, Diagnostic> {
    exact_arity(document, form, items, 3, "let_arity")?;
    let bindings_form = list(
        document,
        &items[1],
        "let_bindings",
        "let bindings must be a list",
    )?;
    let mut bindings = Vec::new();
    for binding in bindings_form {
        let pair = list(
            document,
            binding,
            "let_binding",
            "let binding must be '(name expression)'",
        )?;
        exact_arity(document, binding, pair, 2, "let_binding_arity")?;
        bindings.push(Binding {
            name: identifier(document, &pair[0], IdentifierKind::Value)?,
            value: parse_expression(document, &pair[1])?,
            span: binding.span.clone(),
        });
    }
    reject_duplicates(
        document,
        bindings.iter().map(|value| (&value.name, &value.span)),
        "let_binding_duplicate",
        "let binding",
    )?;
    Ok(Expression::Let {
        bindings,
        body: Box::new(parse_expression(document, &items[2])?),
        span: form.span.clone(),
    })
}

fn parse_record_expression(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<Expression, Diagnostic> {
    if items.len() < 2 {
        return Err(at(
            document,
            form,
            "record_expression_arity",
            "record expression requires a type name or '_'",
        ));
    }
    let type_name = atom(document, &items[1], "record_expression_type")?;
    let ty = if type_name == "_" {
        None
    } else {
        validate_identifier(type_name, IdentifierKind::Qualified)
            .map_err(|message| at(document, &items[1], "record_expression_type", message))?;
        Some(type_name.to_owned())
    };
    let mut fields = Vec::new();
    for item in &items[2..] {
        let pair = list(
            document,
            item,
            "record_expression_field",
            "record field value must be '(name expression)'",
        )?;
        exact_arity(document, item, pair, 2, "record_expression_field_arity")?;
        fields.push(RecordField {
            name: identifier(document, &pair[0], IdentifierKind::Value)?,
            value: parse_expression(document, &pair[1])?,
            span: item.span.clone(),
        });
    }
    reject_duplicates(
        document,
        fields.iter().map(|value| (&value.name, &value.span)),
        "record_expression_field_duplicate",
        "record expression field",
    )?;
    Ok(Expression::Record {
        ty,
        fields,
        span: form.span.clone(),
    })
}

fn parse_map(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<Expression, Diagnostic> {
    if items.len() < 3 {
        return Err(at(
            document,
            form,
            "map_arity",
            "map requires explicit key and value types",
        ));
    }
    let mut entries = Vec::new();
    for item in &items[3..] {
        let entry = list(
            document,
            item,
            "map_entry",
            "map entry must be '(entry key value)'",
        )?;
        exact_arity(document, item, entry, 3, "map_entry_arity")?;
        if atom(document, &entry[0], "map_entry_keyword")? != "entry" {
            return Err(at(
                document,
                &entry[0],
                "map_entry_keyword",
                "map entry must start with 'entry'",
            ));
        }
        entries.push(MapEntry {
            key: parse_expression(document, &entry[1])?,
            value: parse_expression(document, &entry[2])?,
            span: item.span.clone(),
        });
    }
    Ok(Expression::Map {
        key_type: parse_type(document, &items[1])?,
        value_type: parse_type(document, &items[2])?,
        entries,
        span: form.span.clone(),
    })
}

fn parse_match(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
) -> Result<Expression, Diagnostic> {
    if items.len() < 3 {
        return Err(at(
            document,
            form,
            "match_arity",
            "match requires a value and at least one case",
        ));
    }
    let mut arms = Vec::new();
    for item in &items[2..] {
        let arm = list(document, item, "match_arm", "match arm must be a list")?;
        if !(3..=4).contains(&arm.len()) || atom(document, &arm[0], "match_arm_keyword")? != "case"
        {
            return Err(at(
                document,
                item,
                "match_arm_shape",
                "match arm must be '(case Case body)' or '(case Case binding body)'",
            ));
        }
        let (binding, body) = if arm.len() == 4 {
            (
                Some(identifier(document, &arm[2], IdentifierKind::Value)?),
                &arm[3],
            )
        } else {
            (None, &arm[2])
        };
        arms.push(MatchArm {
            case: identifier(document, &arm[1], IdentifierKind::Type)?,
            binding,
            body: parse_expression(document, body)?,
            span: item.span.clone(),
        });
    }
    reject_duplicates(
        document,
        arms.iter().map(|value| (&value.case, &value.span)),
        "match_arm_duplicate",
        "match arm",
    )?;
    Ok(Expression::Match {
        value: Box::new(parse_expression(document, &items[1])?),
        arms,
        span: form.span.clone(),
    })
}

#[derive(Clone, Copy)]
enum IdentifierKind {
    Module,
    Type,
    Value,
    Declaration,
    Qualified,
}

fn identifier(
    document: &SourceDocument,
    form: &Form,
    kind: IdentifierKind,
) -> Result<String, Diagnostic> {
    let value = atom(document, form, "identifier")?;
    validate_identifier(value, kind)
        .map_err(|message| at(document, form, "identifier", message))?;
    Ok(value.to_owned())
}

fn validate_identifier(value: &str, kind: IdentifierKind) -> Result<(), String> {
    if value.len() > 128 {
        return Err("identifier exceeds 128 bytes".to_owned());
    }
    let segments: Vec<_> = value.split('.').collect();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Err("identifier contains an empty segment".to_owned());
    }
    if !matches!(kind, IdentifierKind::Module | IdentifierKind::Qualified) && segments.len() != 1 {
        return Err("local identifier may not contain '.'".to_owned());
    }
    for segment in &segments {
        let mut bytes = segment.bytes();
        let Some(first) = bytes.next() else {
            return Err("identifier is empty".to_owned());
        };
        if !first.is_ascii_alphabetic() && first != b'_' {
            return Err("identifier must start with an ASCII letter or '_'".to_owned());
        }
        if !bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')) {
            return Err("identifier contains a character outside [A-Za-z0-9_-]".to_owned());
        }
    }
    match kind {
        IdentifierKind::Type if !value.as_bytes()[0].is_ascii_uppercase() => {
            Err("type and variant names must start with an uppercase letter".to_owned())
        }
        IdentifierKind::Value if !value.as_bytes()[0].is_ascii_lowercase() && value != "_" => {
            Err("value names must start with a lowercase letter".to_owned())
        }
        IdentifierKind::Module
            if segments
                .iter()
                .any(|segment| !segment.as_bytes()[0].is_ascii_lowercase()) =>
        {
            Err("module segments must start with a lowercase letter".to_owned())
        }
        _ => Ok(()),
    }
}

fn reject_duplicates<'a, Name>(
    document: &SourceDocument,
    values: impl Iterator<Item = (Name, &'a SourceSpan)>,
    code: &str,
    label: &str,
) -> Result<(), Diagnostic>
where
    Name: AsRef<str>,
{
    let mut seen = BTreeSet::new();
    for (value, span) in values {
        let value = value.as_ref();
        if !seen.insert(value.to_owned()) {
            return Err(at_document(
                document,
                span.clone(),
                code,
                format!("duplicate {label} '{value}'"),
            ));
        }
    }
    Ok(())
}

fn exact_arity(
    document: &SourceDocument,
    form: &Form,
    items: &[Form],
    expected: usize,
    code: &str,
) -> Result<(), Diagnostic> {
    if items.len() != expected {
        return Err(at(
            document,
            form,
            code,
            format!(
                "form has {} items; exactly {expected} are required",
                items.len()
            ),
        ));
    }
    Ok(())
}

fn atom<'a>(document: &SourceDocument, form: &'a Form, code: &str) -> Result<&'a str, Diagnostic> {
    form.atom()
        .ok_or_else(|| at(document, form, code, "expected an atom"))
}

fn integer(document: &SourceDocument, form: &Form, code: &str) -> Result<i64, Diagnostic> {
    match form.value {
        FormKind::Integer(value) => Ok(value),
        _ => Err(at(document, form, code, "expected an integer")),
    }
}

fn list<'a>(
    document: &SourceDocument,
    form: &'a Form,
    code: &str,
    message: &str,
) -> Result<&'a [Form], Diagnostic> {
    form.list().ok_or_else(|| at(document, form, code, message))
}

fn at(
    document: &SourceDocument,
    form: &Form,
    code: &str,
    message: impl Into<String>,
) -> Diagnostic {
    at_document(document, form.span.clone(), code, message)
}

fn at_document(
    document: &SourceDocument,
    span: SourceSpan,
    code: &str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::source(
        code,
        message,
        SourceLocation {
            path: document.path().to_owned(),
            byte_offset: span.byte_start,
            line: span.line,
            column: span.column,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::syntax::{SourceLimits, parse_source};

    const SERVICE: &str = r#"(module resources
  (import json std.json)
  (import http std.http)
  (import relational std.relational)
  (import clock std.clock)
  (import random std.secure-random)
  (export Resource Web create-resource)
  (record Resource (id Text) (owner Text) (title Text) (revision I64))
  (variant Lookup (Missing) (Found Resource))
  (fn owns ((actor Text) (resource Resource)) Bool
    (call std.eq actor (field resource owner)))
  (task create-resource ((request http.Request)) http.Response
    (requires (db relational.Database) (clock clock.Clock) (random random.SecureRandom))
    (let ((now (perform clock utc-now))
          (id (perform random bytes 16)))
      (transaction db tx
        (perform tx execute insert-resource
          (record _ (id id) (created-at now))))))
  (component Web
    (require db relational.Database (operations query execute transaction)
      (limit maximum-rows 1000))
    (require clock clock.Clock (operations utc-now))
    (require random random.SecureRandom (operations bytes)
      (limit maximum-bytes 64))
    (port service http.Service (call http.service (function create-resource))))
  (test owner-check (call owns "actor-1"
    (record Resource (id "r1") (owner "actor-1") (title "Title") (revision 1))) true))"#;

    #[test]
    fn service_shape_parses_with_discoverable_effects() {
        let document = parse_source(
            "src/resources.lkj",
            SERVICE.as_bytes(),
            SourceLimits::default(),
        )
        .expect("source parser");
        let module = parse_module(&document).expect("module parser");
        assert_eq!(module.name, "resources");
        assert_eq!(module.imports.len(), 5);
        assert_eq!(module.declarations.len(), 6);
        let function = module
            .declarations
            .iter()
            .find_map(|declaration| match declaration {
                Declaration::Function(function) if function.name == "create-resource" => {
                    Some(function)
                }
                _ => None,
            })
            .expect("task declaration");
        let Effect::Task { capabilities } = &function.effect else {
            panic!("create-resource is not a task");
        };
        assert_eq!(
            capabilities
                .iter()
                .map(|capability| (capability.alias.as_str(), capability.interface.as_str()))
                .collect::<Vec<_>>(),
            vec![
                ("db", "relational.Database"),
                ("clock", "clock.Clock"),
                ("random", "random.SecureRandom")
            ]
        );
        let mut performed = BTreeSet::new();
        function.body.performed_capabilities(&mut performed);
        assert_eq!(
            performed,
            BTreeSet::from([
                "clock".to_owned(),
                "db".to_owned(),
                "random".to_owned(),
                "tx".to_owned()
            ])
        );
    }

    #[test]
    fn duplicate_owners_and_unknown_forms_reject() {
        let source = b"(module bad (record Item) (record Item) (mystery x))";
        let document = parse_source("bad", source, SourceLimits::default()).expect("syntax");
        let error = parse_module(&document).expect_err("unknown form rejects first");
        assert_eq!(error.code, "module_item_unknown");

        let source = b"(module bad (record Item) (record Item))";
        let document = parse_source("bad", source, SourceLimits::default()).expect("syntax");
        let error = parse_module(&document).expect_err("duplicate declaration rejects");
        assert_eq!(error.code, "declaration_duplicate");
    }

    #[test]
    fn live_types_are_not_durable() {
        assert!(!Type::Stream(Box::new(Type::Bytes)).is_durable());
        assert!(!Type::Secret.is_durable());
        assert!(Type::List(Box::new(Type::Text)).is_durable());
    }
}
