//! Concise public changes with typed request-local identity allocation.

use super::contract::registry::CHANGE_ALLOCATION_SEED_DOMAIN;
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::graph::DependencyBinding;
use super::language::{
    Component, Declaration, DeclarationReference, Effect, Expression, Field, Function, Parameter,
    Port, RecordField, RecordType, TestCase, Type, TypeField, TypeParameter, VariantCase,
    VariantType,
};
use super::meaning::{
    DeclarationIdentity, MeaningModule, MemberIdentity, RequestIdentityAllocator,
};
use super::package::{PackageId, RunnerKind};
use super::repository::SemanticRepository;
use super::semantic_digest::ArtifactDigest;
#[cfg(test)]
use super::semantic_id::TargetId;
use super::semantic_id::{DeclarationId, ModuleId, PortId, RevisionId};
use super::semantic_query::SemanticQueryIndex;
use super::semantic_transaction::{
    SemanticOperation, SemanticPrecondition, TransactionBudget, TransactionMode,
    TransactionRequest, TransactionResult, execute_transaction,
};
use super::syntax::SourceSpan;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CHANGE_CONTRACT_VERSION: u16 = 3;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeRequest {
    pub contract_version: u16,
    #[serde(default)]
    pub base_revision: Option<RevisionId>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub preconditions: Vec<SemanticPrecondition>,
    pub changes: Vec<Change>,
    #[serde(default)]
    pub budget: TransactionBudget,
    #[serde(default)]
    pub intent: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "change", rename_all = "snake_case", deny_unknown_fields)]
pub enum Change {
    AddDependency {
        alias: String,
        package_id: PackageId,
        semantic_revision: RevisionId,
        artifact: ArtifactDigest,
    },
    ReplaceDependency {
        alias: String,
        package_id: PackageId,
        semantic_revision: RevisionId,
        artifact: ArtifactDigest,
    },
    RemoveDependency {
        alias: String,
    },
    CreateModule {
        r#as: String,
        name: String,
    },
    CreateRecord {
        r#as: String,
        module: String,
        name: String,
        #[serde(default)]
        fields: Vec<FieldForm>,
        #[serde(default)]
        exported: bool,
    },
    CreateVariant {
        r#as: String,
        module: String,
        name: String,
        cases: Vec<CaseForm>,
        #[serde(default)]
        exported: bool,
    },
    CreateFunction {
        r#as: String,
        module: String,
        name: String,
        #[serde(default)]
        type_parameters: Vec<TypeParameterForm>,
        #[serde(default)]
        parameters: Vec<ParameterForm>,
        result: TypeForm,
        body: ExpressionForm,
        #[serde(default)]
        exported: bool,
    },
    CreateComponent {
        r#as: String,
        module: String,
        name: String,
        ports: Vec<PortForm>,
        #[serde(default)]
        exported: bool,
    },
    CreateTest {
        r#as: String,
        module: String,
        name: String,
        actual: ExpressionForm,
        expected: ExpressionForm,
    },
    CreateTarget {
        r#as: String,
        name: String,
        component: String,
        port: String,
        runner: RunnerKind,
    },
    RenameModule {
        module: String,
        new_name: String,
    },
    RenameDeclaration {
        declaration: String,
        new_name: String,
    },
    ReplaceBody {
        function: String,
        body: ExpressionForm,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ChangeKind {
    AddDependency,
    ReplaceDependency,
    RemoveDependency,
    CreateModule,
    CreateRecord,
    CreateVariant,
    CreateFunction,
    CreateComponent,
    CreateTest,
    CreateTarget,
    RenameModule,
    RenameDeclaration,
    ReplaceBody,
}

impl ChangeKind {
    pub const ALL: [Self; 13] = [
        Self::AddDependency,
        Self::ReplaceDependency,
        Self::RemoveDependency,
        Self::CreateModule,
        Self::CreateRecord,
        Self::CreateVariant,
        Self::CreateFunction,
        Self::CreateComponent,
        Self::CreateTest,
        Self::CreateTarget,
        Self::RenameModule,
        Self::RenameDeclaration,
        Self::ReplaceBody,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::AddDependency => "add_dependency",
            Self::ReplaceDependency => "replace_dependency",
            Self::RemoveDependency => "remove_dependency",
            Self::CreateModule => "create_module",
            Self::CreateRecord => "create_record",
            Self::CreateVariant => "create_variant",
            Self::CreateFunction => "create_function",
            Self::CreateComponent => "create_component",
            Self::CreateTest => "create_test",
            Self::CreateTarget => "create_target",
            Self::RenameModule => "rename_module",
            Self::RenameDeclaration => "rename_declaration",
            Self::ReplaceBody => "replace_body",
        }
    }
}

impl Change {
    pub const fn kind(&self) -> ChangeKind {
        match self {
            Self::AddDependency { .. } => ChangeKind::AddDependency,
            Self::ReplaceDependency { .. } => ChangeKind::ReplaceDependency,
            Self::RemoveDependency { .. } => ChangeKind::RemoveDependency,
            Self::CreateModule { .. } => ChangeKind::CreateModule,
            Self::CreateRecord { .. } => ChangeKind::CreateRecord,
            Self::CreateVariant { .. } => ChangeKind::CreateVariant,
            Self::CreateFunction { .. } => ChangeKind::CreateFunction,
            Self::CreateComponent { .. } => ChangeKind::CreateComponent,
            Self::CreateTest { .. } => ChangeKind::CreateTest,
            Self::CreateTarget { .. } => ChangeKind::CreateTarget,
            Self::RenameModule { .. } => ChangeKind::RenameModule,
            Self::RenameDeclaration { .. } => ChangeKind::RenameDeclaration,
            Self::ReplaceBody { .. } => ChangeKind::ReplaceBody,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldForm {
    #[serde(default)]
    pub r#as: Option<String>,
    pub name: String,
    pub r#type: TypeForm,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CaseForm {
    #[serde(default)]
    pub r#as: Option<String>,
    pub name: String,
    #[serde(default)]
    pub payload: Option<TypeForm>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParameterForm {
    #[serde(default)]
    pub r#as: Option<String>,
    pub name: String,
    pub r#type: TypeForm,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypeParameterForm {
    pub r#as: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PortForm {
    pub r#as: String,
    pub name: String,
    #[serde(default)]
    pub parameters: Vec<TypeForm>,
    pub result: TypeForm,
    pub function: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum TypeForm {
    Unit {},
    Bool {},
    I64 {},
    Bytes {},
    Text {},
    StaticText {},
    Secret {},
    Parameter {
        parameter: String,
    },
    Named {
        name: String,
    },
    Record {
        fields: Vec<TypeFieldForm>,
    },
    List {
        item: Box<TypeForm>,
    },
    Map {
        key: Box<TypeForm>,
        value: Box<TypeForm>,
    },
    Option {
        item: Box<TypeForm>,
    },
    Result {
        ok: Box<TypeForm>,
        error: Box<TypeForm>,
    },
    Stream {
        item: Box<TypeForm>,
    },
    Function {
        parameters: Vec<TypeForm>,
        result: Box<TypeForm>,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TypeFormKind {
    Unit,
    Bool,
    I64,
    Bytes,
    Text,
    StaticText,
    Secret,
    Parameter,
    Named,
    Record,
    List,
    Map,
    Option,
    Result,
    Stream,
    Function,
}

impl TypeFormKind {
    pub const ALL: [Self; 16] = [
        Self::Unit,
        Self::Bool,
        Self::I64,
        Self::Bytes,
        Self::Text,
        Self::StaticText,
        Self::Secret,
        Self::Parameter,
        Self::Named,
        Self::Record,
        Self::List,
        Self::Map,
        Self::Option,
        Self::Result,
        Self::Stream,
        Self::Function,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Bool => "bool",
            Self::I64 => "i64",
            Self::Bytes => "bytes",
            Self::Text => "text",
            Self::StaticText => "static_text",
            Self::Secret => "secret",
            Self::Parameter => "parameter",
            Self::Named => "named",
            Self::Record => "record",
            Self::List => "list",
            Self::Map => "map",
            Self::Option => "option",
            Self::Result => "result",
            Self::Stream => "stream",
            Self::Function => "function",
        }
    }
}

impl TypeForm {
    pub const fn kind(&self) -> TypeFormKind {
        match self {
            Self::Unit { .. } => TypeFormKind::Unit,
            Self::Bool { .. } => TypeFormKind::Bool,
            Self::I64 { .. } => TypeFormKind::I64,
            Self::Bytes { .. } => TypeFormKind::Bytes,
            Self::Text { .. } => TypeFormKind::Text,
            Self::StaticText { .. } => TypeFormKind::StaticText,
            Self::Secret { .. } => TypeFormKind::Secret,
            Self::Parameter { .. } => TypeFormKind::Parameter,
            Self::Named { .. } => TypeFormKind::Named,
            Self::Record { .. } => TypeFormKind::Record,
            Self::List { .. } => TypeFormKind::List,
            Self::Map { .. } => TypeFormKind::Map,
            Self::Option { .. } => TypeFormKind::Option,
            Self::Result { .. } => TypeFormKind::Result,
            Self::Stream { .. } => TypeFormKind::Stream,
            Self::Function { .. } => TypeFormKind::Function,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypeFieldForm {
    pub name: String,
    pub r#type: TypeForm,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum ExpressionForm {
    Unit {
        unit: bool,
    },
    Bool {
        bool: bool,
    },
    I64 {
        i64: i64,
    },
    Text {
        text: String,
    },
    StaticText {
        static_text: String,
    },
    Variable {
        variable: String,
    },
    Constant {
        constant: String,
    },
    Call {
        call: String,
        #[serde(default)]
        type_arguments: Vec<TypeForm>,
        #[serde(default)]
        arguments: Vec<ExpressionForm>,
    },
    Function {
        function: String,
        #[serde(default)]
        type_arguments: Vec<TypeForm>,
    },
    Invoke {
        invoke: Box<ExpressionForm>,
        #[serde(default)]
        arguments: Vec<ExpressionForm>,
    },
    Record {
        record: Vec<ExpressionFieldForm>,
    },
    List {
        list: Vec<ExpressionForm>,
        item_type: TypeForm,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ExpressionFormKind {
    Unit,
    Bool,
    I64,
    Text,
    StaticText,
    Variable,
    Constant,
    Call,
    Function,
    Invoke,
    Record,
    List,
}

impl ExpressionFormKind {
    pub const ALL: [Self; 12] = [
        Self::Unit,
        Self::Bool,
        Self::I64,
        Self::Text,
        Self::StaticText,
        Self::Variable,
        Self::Constant,
        Self::Call,
        Self::Function,
        Self::Invoke,
        Self::Record,
        Self::List,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Bool => "bool",
            Self::I64 => "i64",
            Self::Text => "text",
            Self::StaticText => "static_text",
            Self::Variable => "variable",
            Self::Constant => "constant",
            Self::Call => "call",
            Self::Function => "function",
            Self::Invoke => "invoke",
            Self::Record => "record",
            Self::List => "list",
        }
    }
}

impl ExpressionForm {
    pub const fn kind(&self) -> ExpressionFormKind {
        match self {
            Self::Unit { .. } => ExpressionFormKind::Unit,
            Self::Bool { .. } => ExpressionFormKind::Bool,
            Self::I64 { .. } => ExpressionFormKind::I64,
            Self::Text { .. } => ExpressionFormKind::Text,
            Self::StaticText { .. } => ExpressionFormKind::StaticText,
            Self::Variable { .. } => ExpressionFormKind::Variable,
            Self::Constant { .. } => ExpressionFormKind::Constant,
            Self::Call { .. } => ExpressionFormKind::Call,
            Self::Function { .. } => ExpressionFormKind::Function,
            Self::Invoke { .. } => ExpressionFormKind::Invoke,
            Self::Record { .. } => ExpressionFormKind::Record,
            Self::List { .. } => ExpressionFormKind::List,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpressionFieldForm {
    pub name: String,
    pub value: ExpressionForm,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AllocatedIdentity {
    pub domain: &'static str,
    pub id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeResult {
    pub contract_version: u16,
    pub base_revision: RevisionId,
    pub allocated_identities: BTreeMap<String, AllocatedIdentity>,
    pub transaction: TransactionResult,
}

#[derive(Clone, Debug)]
enum SymbolValue {
    Module {
        id: ModuleId,
        name: String,
    },
    Declaration {
        id: DeclarationId,
        module: ModuleId,
    },
    Port {
        id: PortId,
        component: DeclarationId,
    },
    Member,
    Target,
}

#[derive(Clone, Copy)]
struct ReferenceScope<'a> {
    repository: Option<&'a SemanticRepository>,
    base_revision: RevisionId,
    package_id: &'a PackageId,
    symbols: &'a BTreeMap<String, SymbolValue>,
}

pub fn execute_change(
    repository: &SemanticRepository,
    request: &ChangeRequest,
    commit: bool,
) -> Result<ChangeResult, Diagnostic> {
    validate_request(request)?;
    let binding = repository.current_binding()?;
    let base_revision = request.base_revision.unwrap_or(binding.head.revision);
    if request.idempotency_key.is_some() && request.base_revision.is_none() {
        return Err(change_error(
            DiagnosticClass::Source,
            "change_idempotency_base",
            "an idempotent change must name an exact base_revision",
        ));
    }
    let complete_root = request
        .changes
        .iter()
        .any(change_requires_complete_root)
        .then(|| repository.current())
        .transpose()?;
    let (operations, allocated_identities) = lower_changes(
        Some(repository),
        binding.head.repository_id,
        &binding.stored_root.package_id,
        base_revision,
        request,
        complete_root.as_ref().map(|current| &current.root),
    )?;
    let transaction_request = TransactionRequest {
        contract_version: super::semantic_transaction::TRANSACTION_CONTRACT_VERSION,
        graph_contract: super::meaning::GRAPH_CONTRACT_IDENTITY.to_owned(),
        repository_id: binding.head.repository_id,
        base_revision,
        draft: None,
        idempotency_key: request.idempotency_key.clone(),
        preconditions: request.preconditions.clone(),
        operations,
        budget: request.budget,
        intent: request.intent.clone(),
    };
    let transaction = execute_transaction(
        repository,
        &transaction_request,
        if commit {
            TransactionMode::Apply
        } else {
            TransactionMode::Validate
        },
    )?;
    Ok(ChangeResult {
        contract_version: CHANGE_CONTRACT_VERSION,
        base_revision,
        allocated_identities,
        transaction,
    })
}

fn validate_request(request: &ChangeRequest) -> Result<(), Diagnostic> {
    if request.contract_version != CHANGE_CONTRACT_VERSION {
        return Err(change_error(
            DiagnosticClass::Source,
            "change_contract",
            "change request uses an unknown contract",
        ));
    }
    if request.changes.is_empty() {
        return Err(change_error(
            DiagnosticClass::Source,
            "change_empty",
            "change request must contain at least one change",
        ));
    }
    Ok(())
}

fn lower_changes(
    repository: Option<&SemanticRepository>,
    repository_id: super::semantic_id::RepositoryId,
    package_id: &PackageId,
    base_revision: RevisionId,
    request: &ChangeRequest,
    root: Option<&super::graph::GraphRoot>,
) -> Result<(Vec<SemanticOperation>, BTreeMap<String, AllocatedIdentity>), Diagnostic> {
    let request_bytes = serde_json::to_vec(request).map_err(|error| {
        change_error(
            DiagnosticClass::Infrastructure,
            "change_encode",
            format!("normalized change could not be encoded: {error}"),
        )
    })?;
    let mut seed_hasher = blake3::Hasher::new_derive_key(CHANGE_ALLOCATION_SEED_DOMAIN);
    seed_hasher.update(&repository_id.bytes());
    seed_hasher.update(&base_revision.bytes());
    seed_hasher.update(&(request_bytes.len() as u64).to_be_bytes());
    seed_hasher.update(&request_bytes);
    let mut allocator = RequestIdentityAllocator::new(seed_hasher.finalize().as_bytes().to_vec());
    let mut symbols = BTreeMap::<String, SymbolValue>::new();
    let mut public = BTreeMap::<String, AllocatedIdentity>::new();
    let mut operations = Vec::with_capacity(request.changes.len());

    for change in &request.changes {
        match change {
            Change::AddDependency {
                alias,
                package_id,
                semantic_revision,
                artifact,
            } => operations.push(SemanticOperation::AddDependency {
                binding: DependencyBinding {
                    alias: alias.clone(),
                    package_id: package_id.clone(),
                    semantic_revision: *semantic_revision,
                    artifact: *artifact,
                },
            }),
            Change::ReplaceDependency {
                alias,
                package_id,
                semantic_revision,
                artifact,
            } => operations.push(SemanticOperation::ReplaceDependency {
                binding: DependencyBinding {
                    alias: alias.clone(),
                    package_id: package_id.clone(),
                    semantic_revision: *semantic_revision,
                    artifact: *artifact,
                },
            }),
            Change::RemoveDependency { alias } => {
                operations.push(SemanticOperation::RemoveDependency {
                    alias: alias.clone(),
                });
            }
            Change::CreateModule { r#as, name } => {
                validate_new_symbol(r#as, &symbols)?;
                let id = allocator.allocate_module()?;
                insert_symbol(
                    r#as,
                    SymbolValue::Module {
                        id,
                        name: name.clone(),
                    },
                    "module",
                    id.to_string(),
                    &mut symbols,
                    &mut public,
                );
                operations.push(SemanticOperation::CreateModule {
                    id,
                    name: name.clone(),
                });
            }
            Change::CreateRecord {
                r#as,
                module,
                name,
                fields,
                exported,
            } => {
                let (module, module_name) = resolve_module(module, &symbols, root)?;
                let declaration = Declaration::Record(RecordType {
                    name: name.clone(),
                    fields: fields
                        .iter()
                        .map(|field| {
                            Ok(Field {
                                name: field.name.clone(),
                                ty: lower_type(
                                    &field.r#type,
                                    ReferenceScope {
                                        repository,
                                        base_revision,
                                        package_id,
                                        symbols: &symbols,
                                    },
                                )?,
                                span: canonical_span(),
                            })
                        })
                        .collect::<Result<_, Diagnostic>>()?,
                    span: canonical_span(),
                });
                let identity =
                    MeaningModule::create_declaration_identity(&declaration, &mut allocator)?;
                insert_declaration_symbols(
                    r#as,
                    module,
                    &module_name,
                    name,
                    &identity,
                    fields.iter().map(|field| field.r#as.as_ref()),
                    &mut symbols,
                    &mut public,
                )?;
                operations.push(SemanticOperation::CreateDeclaration {
                    module,
                    identity,
                    declaration,
                    exported: *exported,
                });
            }
            Change::CreateVariant {
                r#as,
                module,
                name,
                cases,
                exported,
            } => {
                let (module, module_name) = resolve_module(module, &symbols, root)?;
                let declaration = Declaration::Variant(VariantType {
                    name: name.clone(),
                    cases: cases
                        .iter()
                        .map(|case| {
                            Ok(VariantCase {
                                name: case.name.clone(),
                                payload: case
                                    .payload
                                    .as_ref()
                                    .map(|value| {
                                        lower_type(
                                            value,
                                            ReferenceScope {
                                                repository,
                                                base_revision,
                                                package_id,
                                                symbols: &symbols,
                                            },
                                        )
                                    })
                                    .transpose()?,
                                span: canonical_span(),
                            })
                        })
                        .collect::<Result<_, Diagnostic>>()?,
                    span: canonical_span(),
                });
                let identity =
                    MeaningModule::create_declaration_identity(&declaration, &mut allocator)?;
                insert_declaration_symbols(
                    r#as,
                    module,
                    &module_name,
                    name,
                    &identity,
                    cases.iter().map(|case| case.r#as.as_ref()),
                    &mut symbols,
                    &mut public,
                )?;
                operations.push(SemanticOperation::CreateDeclaration {
                    module,
                    identity,
                    declaration,
                    exported: *exported,
                });
            }
            Change::CreateFunction {
                r#as,
                module,
                name,
                type_parameters,
                parameters,
                result,
                body,
                exported,
            } => {
                let (module, module_name) = resolve_module(module, &symbols, root)?;
                validate_new_symbol(r#as, &symbols)?;
                let mut scoped_symbols = symbols.clone();
                let mut type_parameter_scope = BTreeMap::new();
                for parameter in type_parameters {
                    validate_new_symbol(&parameter.r#as, &scoped_symbols)?;
                    scoped_symbols.insert(parameter.r#as.clone(), SymbolValue::Member);
                    type_parameter_scope.insert(parameter.r#as.clone(), parameter.name.clone());
                }
                let declaration = Declaration::Function(Function {
                    name: name.clone(),
                    type_parameters: type_parameters
                        .iter()
                        .map(|parameter| TypeParameter {
                            name: parameter.name.clone(),
                            span: canonical_span(),
                        })
                        .collect(),
                    parameters: parameters
                        .iter()
                        .map(|parameter| {
                            Ok(Parameter {
                                name: parameter.name.clone(),
                                ty: lower_type_in_scope(
                                    &parameter.r#type,
                                    ReferenceScope {
                                        repository,
                                        base_revision,
                                        package_id,
                                        symbols: &symbols,
                                    },
                                    &type_parameter_scope,
                                )?,
                                span: canonical_span(),
                            })
                        })
                        .collect::<Result<_, Diagnostic>>()?,
                    result: lower_type_in_scope(
                        result,
                        ReferenceScope {
                            repository,
                            base_revision,
                            package_id,
                            symbols: &symbols,
                        },
                        &type_parameter_scope,
                    )?,
                    effect: Effect::Pure,
                    body: lower_expression_in_scope(
                        body,
                        ReferenceScope {
                            repository,
                            base_revision,
                            package_id,
                            symbols: &symbols,
                        },
                        &type_parameter_scope,
                    )?,
                    span: canonical_span(),
                });
                let identity =
                    MeaningModule::create_declaration_identity(&declaration, &mut allocator)?;
                insert_declaration_symbols(
                    r#as,
                    module,
                    &module_name,
                    name,
                    &identity,
                    type_parameters
                        .iter()
                        .map(|parameter| Some(&parameter.r#as))
                        .chain(parameters.iter().map(|parameter| parameter.r#as.as_ref())),
                    &mut symbols,
                    &mut public,
                )?;
                operations.push(SemanticOperation::CreateDeclaration {
                    module,
                    identity,
                    declaration,
                    exported: *exported,
                });
            }
            Change::CreateComponent {
                r#as,
                module,
                name,
                ports,
                exported,
            } => {
                let (module, module_name) = resolve_module(module, &symbols, root)?;
                let declaration = Declaration::Component(Component {
                    name: name.clone(),
                    requirements: Vec::new(),
                    ports: ports
                        .iter()
                        .map(|port| {
                            Ok(Port {
                                name: port.name.clone(),
                                ty: Type::Function(
                                    port.parameters
                                        .iter()
                                        .map(|value| {
                                            lower_type(
                                                value,
                                                ReferenceScope {
                                                    repository,
                                                    base_revision,
                                                    package_id,
                                                    symbols: &symbols,
                                                },
                                            )
                                        })
                                        .collect::<Result<_, _>>()?,
                                    Box::new(lower_type(
                                        &port.result,
                                        ReferenceScope {
                                            repository,
                                            base_revision,
                                            package_id,
                                            symbols: &symbols,
                                        },
                                    )?),
                                ),
                                value: Expression::FunctionRef {
                                    function: resolve_declaration_reference(
                                        &port.function,
                                        ReferenceScope {
                                            repository,
                                            base_revision,
                                            package_id,
                                            symbols: &symbols,
                                        },
                                    )?,
                                    type_arguments: Vec::new(),
                                    span: canonical_span(),
                                },
                                span: canonical_span(),
                            })
                        })
                        .collect::<Result<_, Diagnostic>>()?,
                    span: canonical_span(),
                });
                let identity =
                    MeaningModule::create_declaration_identity(&declaration, &mut allocator)?;
                insert_component_symbols(
                    r#as,
                    module,
                    &module_name,
                    name,
                    &identity,
                    ports,
                    &mut symbols,
                    &mut public,
                )?;
                operations.push(SemanticOperation::CreateDeclaration {
                    module,
                    identity,
                    declaration,
                    exported: *exported,
                });
            }
            Change::CreateTest {
                r#as,
                module,
                name,
                actual,
                expected,
            } => {
                let (module, module_name) = resolve_module(module, &symbols, root)?;
                let declaration = Declaration::Test(TestCase {
                    name: name.clone(),
                    actual: lower_expression(
                        actual,
                        ReferenceScope {
                            repository,
                            base_revision,
                            package_id,
                            symbols: &symbols,
                        },
                    )?,
                    expected: lower_expression(
                        expected,
                        ReferenceScope {
                            repository,
                            base_revision,
                            package_id,
                            symbols: &symbols,
                        },
                    )?,
                    span: canonical_span(),
                });
                let identity =
                    MeaningModule::create_declaration_identity(&declaration, &mut allocator)?;
                insert_declaration_symbols(
                    r#as,
                    module,
                    &module_name,
                    name,
                    &identity,
                    std::iter::empty::<Option<&String>>(),
                    &mut symbols,
                    &mut public,
                )?;
                operations.push(SemanticOperation::CreateDeclaration {
                    module,
                    identity,
                    declaration,
                    exported: false,
                });
            }
            Change::CreateTarget {
                r#as,
                name,
                component,
                port,
                runner,
            } => {
                validate_new_symbol(r#as, &symbols)?;
                let (component_id, module) = resolve_component(component, &symbols)?;
                let port_id = resolve_port(port, component_id, &symbols)?;
                let id = allocator.allocate_target()?;
                operations.push(SemanticOperation::CreateTarget {
                    target: super::graph::TargetBinding {
                        id,
                        name: name.clone(),
                        component_module: module,
                        component: component_id,
                        port: port_id,
                        runner: *runner,
                    },
                });
                insert_symbol(
                    r#as,
                    SymbolValue::Target,
                    "target",
                    id.to_string(),
                    &mut symbols,
                    &mut public,
                );
            }
            Change::RenameModule { module, new_name } => {
                operations.push(SemanticOperation::RenameModule {
                    module: resolve_module(module, &symbols, root)?.0,
                    new_name: new_name.clone(),
                });
            }
            Change::RenameDeclaration {
                declaration,
                new_name,
            } => operations.push(SemanticOperation::RenameDeclaration {
                declaration: resolve_declaration_id(declaration, &symbols)?,
                new_name: new_name.clone(),
            }),
            Change::ReplaceBody { function, body } => {
                let declaration = resolve_declaration_id(function, &symbols)?;
                let expression = lower_expression(
                    body,
                    ReferenceScope {
                        repository,
                        base_revision,
                        package_id,
                        symbols: &symbols,
                    },
                )?;
                let temporary = Declaration::Function(Function {
                    name: "temporary".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    result: Type::Unit,
                    effect: Effect::Pure,
                    body: expression.clone(),
                    span: canonical_span(),
                });
                let mut identity =
                    MeaningModule::create_declaration_identity(&temporary, &mut allocator)?;
                if let Some(repository) = repository {
                    reuse_body_identities(repository, base_revision, declaration, &mut identity)?;
                }
                operations.push(SemanticOperation::ReplaceBody {
                    declaration,
                    body: expression,
                    bindings: identity.bindings,
                    expressions: identity.expressions,
                });
            }
        }
    }
    Ok((operations, public))
}

#[allow(clippy::too_many_arguments)]
fn insert_declaration_symbols<'a>(
    symbol: &str,
    module: ModuleId,
    _module_name: &str,
    _name: &str,
    identity: &DeclarationIdentity,
    member_symbols: impl Iterator<Item = Option<&'a String>>,
    symbols: &mut BTreeMap<String, SymbolValue>,
    public: &mut BTreeMap<String, AllocatedIdentity>,
) -> Result<(), Diagnostic> {
    validate_new_symbol(symbol, symbols)?;
    let member_symbols = member_symbols.collect::<Vec<_>>();
    let named_symbols = member_symbols
        .iter()
        .filter(|symbol| symbol.is_some())
        .count();
    if member_symbols.len() != identity.members.len() && named_symbols != 0 {
        return Err(change_error(
            DiagnosticClass::Source,
            "change_member_symbol_shape",
            "member symbols do not match allocated semantic members",
        ));
    }
    insert_symbol(
        symbol,
        SymbolValue::Declaration {
            id: identity.id,
            module,
        },
        "declaration",
        identity.id.to_string(),
        symbols,
        public,
    );
    for (member, symbol) in identity.members.iter().zip(member_symbols) {
        if let Some(symbol) = symbol {
            validate_new_symbol(symbol, symbols)?;
            insert_symbol(
                symbol,
                SymbolValue::Member,
                member_domain(member),
                member_id(member),
                symbols,
                public,
            );
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_component_symbols(
    symbol: &str,
    module: ModuleId,
    _module_name: &str,
    _name: &str,
    identity: &DeclarationIdentity,
    ports: &[PortForm],
    symbols: &mut BTreeMap<String, SymbolValue>,
    public: &mut BTreeMap<String, AllocatedIdentity>,
) -> Result<(), Diagnostic> {
    validate_new_symbol(symbol, symbols)?;
    let port_members = identity
        .members
        .iter()
        .filter_map(|member| match member {
            MemberIdentity::Port { id, name } => Some((*id, name)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if port_members.len() != ports.len() {
        return Err(change_error(
            DiagnosticClass::Corrupt,
            "change_port_identity_shape",
            "component port identities do not match the public change",
        ));
    }
    insert_symbol(
        symbol,
        SymbolValue::Declaration {
            id: identity.id,
            module,
        },
        "declaration",
        identity.id.to_string(),
        symbols,
        public,
    );
    for (port, (id, allocated_name)) in ports.iter().zip(port_members) {
        if port.name != *allocated_name {
            return Err(change_error(
                DiagnosticClass::Corrupt,
                "change_port_identity_name",
                "component port identity does not match the public change",
            ));
        }
        validate_new_symbol(&port.r#as, symbols)?;
        insert_symbol(
            &port.r#as,
            SymbolValue::Port {
                id,
                component: identity.id,
            },
            "port",
            id.to_string(),
            symbols,
            public,
        );
    }
    Ok(())
}

fn insert_symbol(
    symbol: &str,
    value: SymbolValue,
    domain: &'static str,
    id: String,
    symbols: &mut BTreeMap<String, SymbolValue>,
    public: &mut BTreeMap<String, AllocatedIdentity>,
) {
    symbols.insert(symbol.to_owned(), value);
    public.insert(symbol.to_owned(), AllocatedIdentity { domain, id });
}

fn validate_new_symbol(
    symbol: &str,
    symbols: &BTreeMap<String, SymbolValue>,
) -> Result<(), Diagnostic> {
    let value = symbol.strip_prefix('$').ok_or_else(|| {
        change_error(
            DiagnosticClass::Source,
            "change_symbol_prefix",
            "request-local symbols must start with '$'",
        )
    })?;
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(change_error(
            DiagnosticClass::Source,
            "change_symbol_syntax",
            format!("request-local symbol '{symbol}' is not canonical"),
        ));
    }
    if symbols.contains_key(symbol) {
        return Err(change_error(
            DiagnosticClass::Source,
            "change_symbol_duplicate",
            format!("request-local symbol '{symbol}' is defined more than once"),
        ));
    }
    Ok(())
}

fn resolve_module(
    value: &str,
    symbols: &BTreeMap<String, SymbolValue>,
    root: Option<&super::graph::GraphRoot>,
) -> Result<(ModuleId, String), Diagnostic> {
    if value.starts_with('$') {
        return match symbols.get(value) {
            Some(SymbolValue::Module { id, name }) => Ok((*id, name.clone())),
            Some(_) => Err(foreign_symbol(value, "module")),
            None => Err(undefined_symbol(value)),
        };
    }
    let id = value.parse::<ModuleId>()?;
    let root = root.ok_or_else(|| {
        change_error(
            DiagnosticClass::Infrastructure,
            "change_root_not_loaded",
            "this change form requires the exact complete root during lowering",
        )
    })?;
    let name = root
        .modules
        .iter()
        .find(|module| module.id == id)
        .map(|module| module.name.clone())
        .ok_or_else(|| {
            change_error(
                DiagnosticClass::Semantic,
                "change_module_missing",
                format!("module '{id}' is absent from the selected base"),
            )
        })?;
    Ok((id, name))
}

fn change_requires_complete_root(change: &Change) -> bool {
    !matches!(
        change,
        Change::AddDependency { .. }
            | Change::ReplaceDependency { .. }
            | Change::RemoveDependency { .. }
            | Change::CreateModule { .. }
            | Change::RenameDeclaration { .. }
            | Change::ReplaceBody { .. }
    )
}

fn reuse_body_identities(
    repository: &SemanticRepository,
    base_revision: RevisionId,
    declaration: DeclarationId,
    replacement: &mut DeclarationIdentity,
) -> Result<(), Diagnostic> {
    let summary = match SemanticQueryIndex::owner_summary_revision(
        repository,
        base_revision,
        &declaration.to_string(),
    ) {
        Ok(summary) => summary,
        Err(error) if error.class == DiagnosticClass::Source => return Ok(()),
        Err(error) => return Err(error),
    };
    let Some(module_id) = summary.module_id else {
        return Ok(());
    };
    let module = repository.module_by_id(base_revision, module_id)?;
    let Some((existing, _)) = module.declaration(declaration) else {
        return Ok(());
    };
    let expression_ids = existing
        .expressions
        .iter()
        .map(|identity| (identity.path.clone(), identity.id))
        .collect::<BTreeMap<_, _>>();
    for identity in &mut replacement.expressions {
        if let Some(id) = expression_ids.get(&identity.path) {
            identity.id = *id;
        }
    }
    let binding_ids = existing
        .bindings
        .iter()
        .map(|identity| {
            (
                (identity.expression_path.clone(), identity.slot),
                identity.id,
            )
        })
        .collect::<BTreeMap<_, _>>();
    for identity in &mut replacement.bindings {
        if let Some(id) = binding_ids.get(&(identity.expression_path.clone(), identity.slot)) {
            identity.id = *id;
        }
    }
    Ok(())
}

fn resolve_declaration_id(
    value: &str,
    symbols: &BTreeMap<String, SymbolValue>,
) -> Result<DeclarationId, Diagnostic> {
    if value.starts_with('$') {
        return match symbols.get(value) {
            Some(SymbolValue::Declaration { id, .. }) => Ok(*id),
            Some(_) => Err(foreign_symbol(value, "declaration")),
            None => Err(undefined_symbol(value)),
        };
    }
    value.parse::<DeclarationId>()
}

fn resolve_declaration_reference(
    value: &str,
    scope: ReferenceScope<'_>,
) -> Result<DeclarationReference, Diagnostic> {
    if value.starts_with('$') {
        return match scope.symbols.get(value) {
            Some(SymbolValue::Declaration { id, module, .. }) => Ok(DeclarationReference {
                package: scope.package_id.clone(),
                module: *module,
                declaration: *id,
            }),
            Some(_) => Err(foreign_symbol(value, "declaration")),
            None => Err(undefined_symbol(value)),
        };
    }
    if let Some(exact) = value.strip_prefix("exact:") {
        let mut fields = exact.split('/');
        let package = fields.next().unwrap_or_default();
        let module = fields.next().unwrap_or_default();
        let declaration = fields.next().unwrap_or_default();
        if package.is_empty()
            || module.is_empty()
            || declaration.is_empty()
            || fields.next().is_some()
        {
            return Err(change_error(
                DiagnosticClass::Source,
                "change_reference_selector",
                "an exact declaration selector must be 'exact:PACKAGE/MODULE/DECLARATION'",
            ));
        }
        return Ok(DeclarationReference {
            package: PackageId::parse(package)?,
            module: module.parse()?,
            declaration: declaration.parse()?,
        });
    }
    let declaration = value.parse::<DeclarationId>()?;
    let repository = scope.repository.ok_or_else(|| {
        change_error(
            DiagnosticClass::Infrastructure,
            "change_reference_repository",
            "resolving an existing declaration requires the selected repository",
        )
    })?;
    let summary = SemanticQueryIndex::owner_summary_revision(
        repository,
        scope.base_revision,
        &declaration.to_string(),
    )?;
    let module = summary.module_id.ok_or_else(|| {
        change_error(
            DiagnosticClass::Source,
            "change_reference_domain",
            "selected identity is not a declaration",
        )
    })?;
    Ok(DeclarationReference {
        package: summary.package_id,
        module,
        declaration,
    })
}

fn resolve_component(
    value: &str,
    symbols: &BTreeMap<String, SymbolValue>,
) -> Result<(DeclarationId, ModuleId), Diagnostic> {
    match symbols.get(value) {
        Some(SymbolValue::Declaration { id, module, .. }) => Ok((*id, *module)),
        Some(_) => Err(foreign_symbol(value, "component declaration")),
        None => Err(undefined_symbol(value)),
    }
}

fn resolve_port(
    value: &str,
    component: DeclarationId,
    symbols: &BTreeMap<String, SymbolValue>,
) -> Result<PortId, Diagnostic> {
    match symbols.get(value) {
        Some(SymbolValue::Port {
            id,
            component: owner,
            ..
        }) if *owner == component => Ok(*id),
        Some(SymbolValue::Port { .. }) => Err(change_error(
            DiagnosticClass::Source,
            "change_port_owner",
            "target port belongs to a different component",
        )),
        Some(_) => Err(foreign_symbol(value, "port")),
        None => Err(undefined_symbol(value)),
    }
}

fn lower_type(value: &TypeForm, scope: ReferenceScope<'_>) -> Result<Type, Diagnostic> {
    lower_type_in_scope(value, scope, &BTreeMap::new())
}

fn lower_type_in_scope(
    value: &TypeForm,
    scope: ReferenceScope<'_>,
    type_parameters: &BTreeMap<String, String>,
) -> Result<Type, Diagnostic> {
    Ok(match value {
        TypeForm::Unit {} => Type::Unit,
        TypeForm::Bool {} => Type::Bool,
        TypeForm::I64 {} => Type::I64,
        TypeForm::Bytes {} => Type::Bytes,
        TypeForm::Text {} => Type::Text,
        TypeForm::StaticText {} => Type::StaticText,
        TypeForm::Secret {} => Type::Secret,
        TypeForm::Parameter { parameter } => {
            let name = type_parameters.get(parameter).ok_or_else(|| {
                if parameter.starts_with('$') {
                    undefined_symbol(parameter)
                } else {
                    change_error(
                        DiagnosticClass::Source,
                        "change_type_parameter_symbol",
                        "type parameter references must use a declaration-local '$' symbol",
                    )
                }
            })?;
            Type::Parameter(name.clone())
        }
        TypeForm::Named { name } => Type::Named(resolve_declaration_reference(name, scope)?),
        TypeForm::Record { fields } => Type::Record(
            fields
                .iter()
                .map(|field| {
                    Ok(TypeField {
                        name: field.name.clone(),
                        ty: lower_type_in_scope(&field.r#type, scope, type_parameters)?,
                    })
                })
                .collect::<Result<_, Diagnostic>>()?,
        ),
        TypeForm::List { item } => {
            Type::List(Box::new(lower_type_in_scope(item, scope, type_parameters)?))
        }
        TypeForm::Map { key, value } => Type::Map(
            Box::new(lower_type_in_scope(key, scope, type_parameters)?),
            Box::new(lower_type_in_scope(value, scope, type_parameters)?),
        ),
        TypeForm::Option { item } => {
            Type::Option(Box::new(lower_type_in_scope(item, scope, type_parameters)?))
        }
        TypeForm::Result { ok, error } => Type::Result(
            Box::new(lower_type_in_scope(ok, scope, type_parameters)?),
            Box::new(lower_type_in_scope(error, scope, type_parameters)?),
        ),
        TypeForm::Stream { item } => {
            Type::Stream(Box::new(lower_type_in_scope(item, scope, type_parameters)?))
        }
        TypeForm::Function { parameters, result } => Type::Function(
            parameters
                .iter()
                .map(|value| lower_type_in_scope(value, scope, type_parameters))
                .collect::<Result<_, _>>()?,
            Box::new(lower_type_in_scope(result, scope, type_parameters)?),
        ),
    })
}

fn lower_expression(
    value: &ExpressionForm,
    scope: ReferenceScope<'_>,
) -> Result<Expression, Diagnostic> {
    lower_expression_in_scope(value, scope, &BTreeMap::new())
}

fn lower_expression_in_scope(
    value: &ExpressionForm,
    scope: ReferenceScope<'_>,
    type_parameters: &BTreeMap<String, String>,
) -> Result<Expression, Diagnostic> {
    Ok(match value {
        ExpressionForm::Unit { unit } => {
            if !unit {
                return Err(change_error(
                    DiagnosticClass::Source,
                    "change_unit_value",
                    "unit expression marker must be true",
                ));
            }
            Expression::Unit(canonical_span())
        }
        ExpressionForm::Bool { bool: value } => Expression::Bool(*value, canonical_span()),
        ExpressionForm::I64 { i64: value } => Expression::I64(*value, canonical_span()),
        ExpressionForm::Text { text } => Expression::Text(text.clone(), canonical_span()),
        ExpressionForm::StaticText { static_text } => {
            Expression::StaticText(static_text.clone(), canonical_span())
        }
        ExpressionForm::Variable { variable } => {
            Expression::Variable(variable.clone(), canonical_span())
        }
        ExpressionForm::Constant { constant } => Expression::Constant(
            resolve_declaration_reference(constant, scope)?,
            canonical_span(),
        ),
        ExpressionForm::Call {
            call,
            type_arguments,
            arguments,
        } => Expression::Call {
            function: resolve_declaration_reference(call, scope)?,
            type_arguments: type_arguments
                .iter()
                .map(|argument| lower_type_in_scope(argument, scope, type_parameters))
                .collect::<Result<_, _>>()?,
            arguments: arguments
                .iter()
                .map(|argument| lower_expression_in_scope(argument, scope, type_parameters))
                .collect::<Result<_, _>>()?,
            span: canonical_span(),
        },
        ExpressionForm::Function {
            function,
            type_arguments,
        } => Expression::FunctionRef {
            function: resolve_declaration_reference(function, scope)?,
            type_arguments: type_arguments
                .iter()
                .map(|argument| lower_type_in_scope(argument, scope, type_parameters))
                .collect::<Result<_, _>>()?,
            span: canonical_span(),
        },
        ExpressionForm::Invoke { invoke, arguments } => Expression::Invoke {
            callee: Box::new(lower_expression_in_scope(invoke, scope, type_parameters)?),
            arguments: arguments
                .iter()
                .map(|argument| lower_expression_in_scope(argument, scope, type_parameters))
                .collect::<Result<_, _>>()?,
            span: canonical_span(),
        },
        ExpressionForm::Record { record } => Expression::Record {
            ty: None,
            fields: record
                .iter()
                .map(|field| {
                    Ok(RecordField {
                        name: field.name.clone(),
                        value: lower_expression_in_scope(&field.value, scope, type_parameters)?,
                        span: canonical_span(),
                    })
                })
                .collect::<Result<_, Diagnostic>>()?,
            span: canonical_span(),
        },
        ExpressionForm::List { list, item_type } => Expression::List {
            item_type: lower_type_in_scope(item_type, scope, type_parameters)?,
            items: list
                .iter()
                .map(|item| lower_expression_in_scope(item, scope, type_parameters))
                .collect::<Result<_, _>>()?,
            span: canonical_span(),
        },
    })
}

fn member_domain(member: &MemberIdentity) -> &'static str {
    match member {
        MemberIdentity::TypeParameter { .. } => "type_parameter",
        MemberIdentity::Field { .. } => "field",
        MemberIdentity::Case { .. } => "case",
        MemberIdentity::Operation { .. } => "operation",
        MemberIdentity::Parameter { .. } => "parameter",
        MemberIdentity::TaskRequirement { .. } | MemberIdentity::ComponentRequirement { .. } => {
            "requirement"
        }
        MemberIdentity::Port { .. } => "port",
    }
}

fn member_id(member: &MemberIdentity) -> String {
    match member {
        MemberIdentity::TypeParameter { id, .. } => id.to_string(),
        MemberIdentity::Field { id, .. } => id.to_string(),
        MemberIdentity::Case { id, .. } => id.to_string(),
        MemberIdentity::Operation { id, .. } => id.to_string(),
        MemberIdentity::Parameter { id, .. } => id.to_string(),
        MemberIdentity::TaskRequirement { id, .. }
        | MemberIdentity::ComponentRequirement { id, .. } => id.to_string(),
        MemberIdentity::Port { id, .. } => id.to_string(),
    }
}

fn undefined_symbol(symbol: &str) -> Diagnostic {
    change_error(
        DiagnosticClass::Source,
        "change_symbol_undefined",
        format!("request-local symbol '{symbol}' is not defined before use"),
    )
}

fn foreign_symbol(symbol: &str, expected: &str) -> Diagnostic {
    change_error(
        DiagnosticClass::Source,
        "change_symbol_domain",
        format!("request-local symbol '{symbol}' is not a {expected}"),
    )
}

fn canonical_span() -> SourceSpan {
    SourceSpan {
        byte_start: 0,
        byte_end: 0,
        line: 1,
        column: 1,
    }
}

fn change_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_and_expression_forms_are_strict_and_concise() {
        let request: ChangeRequest = serde_json::from_value(serde_json::json!({
            "contract_version": 3,
            "changes": [
                {"change": "create_module", "as": "$app", "name": "app"},
                {
                    "change": "create_function",
                    "as": "$main",
                    "module": "$app",
                    "name": "main",
                    "result": {"type": "text"},
                    "body": {"text": "hello"},
                    "exported": true
                }
            ]
        }))
        .expect("concise request");
        assert_eq!(request.changes.len(), 2);
    }

    #[test]
    fn one_change_allocates_and_lowers_explicit_generics_and_invocation() {
        let request: ChangeRequest = serde_json::from_value(serde_json::json!({
            "contract_version": 3,
            "changes": [
                {"change": "create_module", "as": "$app", "name": "app"},
                {
                    "change": "create_function",
                    "as": "$identity",
                    "module": "$app",
                    "name": "identity",
                    "type_parameters": [{"as": "$IdentityT", "name": "T"}],
                    "parameters": [{
                        "as": "$identity-value",
                        "name": "value",
                        "type": {"type": "parameter", "parameter": "$IdentityT"}
                    }],
                    "result": {"type": "parameter", "parameter": "$IdentityT"},
                    "body": {"variable": "value"},
                    "exported": true
                },
                {
                    "change": "create_function",
                    "as": "$apply",
                    "module": "$app",
                    "name": "apply",
                    "type_parameters": [{"as": "$ApplyT", "name": "T"}],
                    "parameters": [
                        {
                            "name": "mapper",
                            "type": {
                                "type": "function",
                                "parameters": [{"type": "parameter", "parameter": "$ApplyT"}],
                                "result": {"type": "parameter", "parameter": "$ApplyT"}
                            }
                        },
                        {
                            "name": "value",
                            "type": {"type": "parameter", "parameter": "$ApplyT"}
                        }
                    ],
                    "result": {"type": "parameter", "parameter": "$ApplyT"},
                    "body": {
                        "invoke": {"variable": "mapper"},
                        "arguments": [{"variable": "value"}]
                    }
                },
                {
                    "change": "create_test",
                    "as": "$identity-test",
                    "module": "$app",
                    "name": "identity-text",
                    "actual": {
                        "call": "$identity",
                        "type_arguments": [{"type": "text"}],
                        "arguments": [{"text": "value"}]
                    },
                    "expected": {"text": "value"}
                }
            ]
        }))
        .expect("generic change request");
        let root = super::super::graph::GraphRoot {
            graph_contract_version: super::super::meaning::GRAPH_CONTRACT_VERSION,
            repository_id: super::super::semantic_id::RepositoryId::migrate(b"generic-change", 1),
            package_id: super::super::package::PackageId::parse("1234567890abcdef1234567890abcdef")
                .expect("package"),
            package_name: "generic-change".to_owned(),
            modules: Vec::new(),
            dependencies: Vec::new(),
            targets: Vec::new(),
            tombstones: Vec::new(),
        };
        let base = RevisionId::from_digest([1; 32]);
        let (operations, allocated) = lower_changes(
            None,
            root.repository_id,
            &root.package_id,
            base,
            &request,
            Some(&root),
        )
        .expect("generic request lowers");
        assert_eq!(allocated["$IdentityT"].domain, "type_parameter");
        assert_eq!(allocated["$ApplyT"].domain, "type_parameter");
        let apply = operations.iter().find_map(|operation| match operation {
            SemanticOperation::CreateDeclaration {
                identity,
                declaration: Declaration::Function(function),
                ..
            } if function.name == "apply" => Some((identity, function)),
            _ => None,
        });
        let (identity, function) = apply.expect("lowered apply function");
        assert!(matches!(
            identity.members.first(),
            Some(MemberIdentity::TypeParameter { name, .. }) if name == "T"
        ));
        assert!(matches!(function.body, Expression::Invoke { .. }));
    }

    #[test]
    fn exact_declaration_selectors_are_closed_and_domain_separated() {
        let package =
            PackageId::parse("1234567890abcdef1234567890abcdef").expect("package identity");
        let module = ModuleId::migrate(b"exact-selector", 1);
        let declaration = DeclarationId::migrate(b"exact-selector", 2);
        let symbols = BTreeMap::new();
        let scope = ReferenceScope {
            repository: None,
            base_revision: RevisionId::from_digest([1; 32]),
            package_id: &package,
            symbols: &symbols,
        };
        let selector = format!("exact:{}/{module}/{declaration}", package.as_str());
        assert_eq!(
            resolve_declaration_reference(&selector, scope).expect("exact selector"),
            DeclarationReference {
                package: package.clone(),
                module,
                declaration,
            }
        );

        let malformed = format!("{selector}/trailing");
        let error = resolve_declaration_reference(&malformed, scope)
            .expect_err("trailing selector field must reject");
        assert_eq!(error.code, "change_reference_selector");

        let foreign = TargetId::migrate(b"exact-selector", 3);
        let foreign_selector = format!("exact:{}/{module}/{foreign}", package.as_str());
        let error = resolve_declaration_reference(&foreign_selector, scope)
            .expect_err("foreign identity domain must reject");
        assert_eq!(error.code, "semantic_identity_domain");
    }
}
