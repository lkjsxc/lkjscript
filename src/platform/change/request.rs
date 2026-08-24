//! High-level Graph 5 authored intent lowered to exact primitive owner edits.

mod creation;
mod precondition;

pub use creation::{
    AuthoredAnnotationValue, AuthoredBindingDefinition, AuthoredCase, AuthoredCaseReference,
    AuthoredDeclarationReference, AuthoredExpression, AuthoredExpressionOperation, AuthoredField,
    AuthoredFieldReference, AuthoredFieldSelector, AuthoredFunctionEffect, AuthoredLetBinding,
    AuthoredLocalReference, AuthoredMapExpressionEntry, AuthoredMatchExpressionArm,
    AuthoredOperation, AuthoredOperationReference, AuthoredParameter, AuthoredPort,
    AuthoredPortImplementation, AuthoredPortReference, AuthoredRecordExpressionField,
    AuthoredRequirement, AuthoredRequirementReference, AuthoredResourceLimit,
    AuthoredStructuralTypeField, AuthoredType, AuthoredTypeParameter,
    AuthoredTypeParameterReference,
};
pub use precondition::AuthoredPrecondition;

use super::{
    CanonicalBaseRead, CanonicalReadWork, ChangeBudget, ChangeBudgetWork, PrimitiveEdit,
    WitnessBaseRead, WitnessReadWork,
};
use crate::platform::contract::registry::CHANGE_ALLOCATION_SEED_DOMAIN;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    ExpressionOperation, ExpressionRecord, ModuleRecord, Name, NamespaceClass, OwnerHeader,
    OwnerKey, OwnerKind, OwnerRecord, TypeObjectInterner, encode_owner,
};
use crate::platform::semantic_id::{
    AnnotationId, BindingId, CaseId, DeclarationId, DocumentationId, ExpressionId, FieldId,
    ModuleId, OperationId, ParameterId, PortId, RequirementId, RevisionId, TargetId,
    TypeParameterId,
};
use crate::platform::witness::NamespaceKey;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAXIMUM_AUTHORED_CHANGES: usize = 10_000;
pub const MAXIMUM_AUTHORED_CHANGE_BYTES: usize = 4 * 1_048_576;
const MAXIMUM_REQUEST_SYMBOL_BYTES: usize = 128;

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoredChangeSet {
    pub base: RevisionId,
    #[serde(default)]
    pub preconditions: Vec<AuthoredPrecondition>,
    pub changes: Vec<AuthoredChange>,
    #[serde(default)]
    pub budget: ChangeBudget,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthoredChange {
    CreateModule {
        #[serde(rename = "as")]
        symbol: String,
        name: Name,
    },
    CreateFunction {
        #[serde(rename = "as")]
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        #[serde(default)]
        type_parameters: Vec<AuthoredTypeParameter>,
        #[serde(default)]
        parameters: Vec<AuthoredParameter>,
        result: AuthoredType,
        effect: AuthoredFunctionEffect,
        body: AuthoredExpression,
    },
    CreateRecord {
        #[serde(rename = "as")]
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        fields: Vec<AuthoredField>,
    },
    CreateVariant {
        #[serde(rename = "as")]
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        cases: Vec<AuthoredCase>,
    },
    CreateInterface {
        #[serde(rename = "as")]
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        operations: Vec<AuthoredOperation>,
    },
    CreateExternal {
        #[serde(rename = "as")]
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        #[serde(default)]
        type_parameters: Vec<AuthoredTypeParameter>,
        #[serde(default)]
        parameters: Vec<AuthoredParameter>,
        result: AuthoredType,
        implementation: Name,
    },
    CreateConstant {
        #[serde(rename = "as")]
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        ty: AuthoredType,
        value: AuthoredExpression,
    },
    CreateComponent {
        #[serde(rename = "as")]
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        #[serde(default)]
        requirements: Vec<AuthoredRequirement>,
        ports: Vec<AuthoredPort>,
    },
    CreateTest {
        #[serde(rename = "as")]
        symbol: String,
        module: ModuleSelector,
        name: Name,
        visibility: crate::platform::kernel::DeclarationVisibility,
        actual: AuthoredExpression,
        expected: AuthoredExpression,
    },
    CreateTarget {
        #[serde(rename = "as")]
        symbol: String,
        name: Name,
        component: AuthoredDeclarationReference,
        port: AuthoredPortReference,
        runner: crate::platform::package::RunnerKind,
    },
    CreateDocumentation {
        #[serde(rename = "as")]
        symbol: String,
        owner: OwnerSelector,
        class: crate::platform::kernel::DocumentationClass,
        text: String,
    },
    CreateAnnotation {
        #[serde(rename = "as")]
        symbol: String,
        owner: OwnerSelector,
        class: crate::platform::kernel::AnnotationClass,
        key: Name,
        value: AuthoredAnnotationValue,
    },
    RenameOwner {
        owner: OwnerSelector,
        name: Name,
    },
    MoveDeclaration {
        declaration: DeclarationSelector,
        module: ModuleSelector,
    },
    ReplaceExpression {
        expression: ExpressionId,
        operation: ExpressionOperation,
    },
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "by", rename_all = "snake_case", deny_unknown_fields)]
pub enum OwnerSelector {
    Exact { owner: OwnerKey },
    ModuleName { name: Name },
    DeclarationName { module: ModuleSelector, name: Name },
    Symbol { symbol: String },
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "by", rename_all = "snake_case", deny_unknown_fields)]
pub enum ModuleSelector {
    Id { module: ModuleId },
    Name { name: Name },
    Symbol { symbol: String },
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "by", rename_all = "snake_case", deny_unknown_fields)]
pub enum DeclarationSelector {
    Id {
        declaration: crate::platform::semantic_id::DeclarationId,
    },
    Qualified {
        module: ModuleSelector,
        name: Name,
    },
    Symbol {
        symbol: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum SymbolKind {
    Module,
    Declaration,
    TypeParameter,
    Field,
    Case,
    Operation,
    FunctionParameter,
    OperationParameter,
    LexicalBinding,
    MatchPayloadBinding,
    TransactionBinding,
    Expression,
    Requirement,
    Port,
    Target,
    Documentation,
    Annotation,
}

impl SymbolKind {
    const fn allocation_domain(self) -> u8 {
        match self {
            Self::Module => 1,
            Self::Declaration => 2,
            Self::TypeParameter => 3,
            Self::Field => 4,
            Self::Case => 5,
            Self::Operation => 6,
            Self::FunctionParameter | Self::OperationParameter => 7,
            Self::LexicalBinding | Self::MatchPayloadBinding | Self::TransactionBinding => 8,
            Self::Expression => 9,
            Self::Requirement => 10,
            Self::Port => 11,
            Self::Target => 12,
            Self::Documentation => 13,
            Self::Annotation => 14,
        }
    }

    fn allocate(self, seed: &[u8], ordinal: u64) -> OwnerKey {
        match self {
            Self::Module => OwnerKey::Module(ModuleId::allocate(seed, ordinal)),
            Self::Declaration => OwnerKey::Declaration(DeclarationId::allocate(seed, ordinal)),
            Self::TypeParameter => {
                OwnerKey::TypeParameter(TypeParameterId::allocate(seed, ordinal))
            }
            Self::Field => OwnerKey::Field(FieldId::allocate(seed, ordinal)),
            Self::Case => OwnerKey::Case(CaseId::allocate(seed, ordinal)),
            Self::Operation => OwnerKey::Operation(OperationId::allocate(seed, ordinal)),
            Self::FunctionParameter | Self::OperationParameter => {
                OwnerKey::Parameter(ParameterId::allocate(seed, ordinal))
            }
            Self::LexicalBinding | Self::MatchPayloadBinding | Self::TransactionBinding => {
                OwnerKey::Binding(BindingId::allocate(seed, ordinal))
            }
            Self::Expression => OwnerKey::Expression(ExpressionId::allocate(seed, ordinal)),
            Self::Requirement => OwnerKey::Requirement(RequirementId::allocate(seed, ordinal)),
            Self::Port => OwnerKey::Port(PortId::allocate(seed, ordinal)),
            Self::Target => OwnerKey::Target(TargetId::allocate(seed, ordinal)),
            Self::Documentation => {
                OwnerKey::Documentation(DocumentationId::allocate(seed, ordinal))
            }
            Self::Annotation => OwnerKey::Annotation(AnnotationId::allocate(seed, ordinal)),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AuthoredLoweringWork {
    pub operations_lowered: u64,
    pub preconditions_checked: u64,
    pub canonical: CanonicalReadWork,
    pub witness: WitnessReadWork,
}

impl AuthoredLoweringWork {
    pub(crate) fn budget_work(self) -> ChangeBudgetWork {
        ChangeBudgetWork::authored(
            usize::try_from(self.operations_lowered).unwrap_or(usize::MAX),
            usize::try_from(self.preconditions_checked).unwrap_or(usize::MAX),
            self.canonical,
            self.witness,
        )
    }
}

#[derive(Clone, Debug)]
pub struct AuthoredLowering {
    pub edits: Vec<PrimitiveEdit>,
    pub allocated: BTreeMap<String, OwnerKey>,
    pub work: AuthoredLoweringWork,
}

pub fn lower_authored_changes<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    base: &B,
    witness: &W,
    request: &AuthoredChangeSet,
    idempotency_key: Option<&str>,
) -> Result<AuthoredLowering, Diagnostic> {
    if base.exact_revision() != Some(request.base) {
        return Err(request_error(
            DiagnosticClass::Semantic,
            "change_authored_stale_base",
            "authored change base is not the exact pinned repository revision",
        ));
    }
    if !witness.witness_contract_is_current()
        || witness.witness_repository_id() != base.repository_id()
        || witness.witness_package_id() != base.package_id()
        || witness.witness_manifest().semantic_root
            != crate::platform::kernel::encode_root(base.semantic_root())?.0
    {
        return Err(request_error(
            DiagnosticClass::Corrupt,
            "change_authored_witness_base",
            "authored change inputs do not share one exact canonical and witness base",
        ));
    }
    if request.changes.is_empty() || request.changes.len() > MAXIMUM_AUTHORED_CHANGES {
        return Err(request_error(
            DiagnosticClass::Resource,
            "change_authored_count",
            format!("authored change requires 1 through {MAXIMUM_AUTHORED_CHANGES} operations"),
        ));
    }
    let budget = request
        .budget
        .validate_request_counts(request.changes.len(), request.preconditions.len())?;

    let seed = allocation_seed(base, request, idempotency_key)?;
    let definitions = collect_symbol_definitions(request)?;
    let allocated = allocate_symbols(&seed, &definitions)?;
    let mut lowerer = AuthoredLowerer::new(base, witness, seed, allocated, definitions, budget)?;
    lowerer.work.operations_lowered = u64::try_from(request.changes.len()).unwrap_or(u64::MAX);
    precondition::evaluate(&mut lowerer, &request.preconditions)?;
    lowerer.check_budget("authored preconditions")?;
    for change in &request.changes {
        if let AuthoredChange::CreateModule { symbol, name } = change {
            let module = lowerer.module_symbol(symbol)?;
            lowerer.insert_created(OwnerRecord::Module(ModuleRecord {
                header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
                name: name.clone(),
            }))?;
        }
        lowerer.check_budget("module creation lowering")?;
    }
    for change in &request.changes {
        match change {
            AuthoredChange::CreateRecord {
                symbol,
                module,
                name,
                visibility,
                fields,
            } => creation::lower_record(&mut lowerer, symbol, module, name, *visibility, fields)?,
            AuthoredChange::CreateVariant {
                symbol,
                module,
                name,
                visibility,
                cases,
            } => creation::lower_variant(&mut lowerer, symbol, module, name, *visibility, cases)?,
            AuthoredChange::CreateInterface {
                symbol,
                module,
                name,
                visibility,
                operations,
            } => creation::lower_interface(
                &mut lowerer,
                symbol,
                module,
                name,
                *visibility,
                operations,
            )?,
            AuthoredChange::CreateExternal {
                symbol,
                module,
                name,
                visibility,
                type_parameters,
                parameters,
                result,
                implementation,
            } => creation::lower_external(
                &mut lowerer,
                symbol,
                module,
                name,
                *visibility,
                type_parameters,
                parameters,
                result,
                implementation,
            )?,
            AuthoredChange::CreateFunction {
                symbol,
                module,
                name,
                visibility,
                type_parameters,
                parameters,
                result,
                effect,
                body,
            } => creation::lower_function(
                &mut lowerer,
                symbol,
                module,
                name,
                *visibility,
                type_parameters,
                parameters,
                result,
                effect,
                body,
            )?,
            AuthoredChange::CreateConstant {
                symbol,
                module,
                name,
                visibility,
                ty,
                value,
            } => creation::lower_constant(
                &mut lowerer,
                symbol,
                module,
                name,
                *visibility,
                ty,
                value,
            )?,
            AuthoredChange::CreateComponent {
                symbol,
                module,
                name,
                visibility,
                requirements,
                ports,
            } => creation::lower_component(
                &mut lowerer,
                symbol,
                module,
                name,
                *visibility,
                requirements,
                ports,
            )?,
            AuthoredChange::CreateTest {
                symbol,
                module,
                name,
                visibility,
                actual,
                expected,
            } => creation::lower_test(
                &mut lowerer,
                symbol,
                module,
                name,
                *visibility,
                actual,
                expected,
            )?,
            AuthoredChange::CreateTarget {
                symbol,
                name,
                component,
                port,
                runner,
            } => creation::lower_target(&mut lowerer, symbol, name, component, port, *runner)?,
            AuthoredChange::CreateDocumentation {
                symbol,
                owner,
                class,
                text,
            } => creation::lower_documentation(&mut lowerer, symbol, owner, *class, text)?,
            AuthoredChange::CreateAnnotation {
                symbol,
                owner,
                class,
                key,
                value,
            } => creation::lower_annotation(&mut lowerer, symbol, owner, *class, key, value)?,
            _ => {}
        }
        lowerer.check_budget("owner creation lowering")?;
    }
    for change in &request.changes {
        match change {
            AuthoredChange::CreateModule { .. }
            | AuthoredChange::CreateRecord { .. }
            | AuthoredChange::CreateVariant { .. }
            | AuthoredChange::CreateInterface { .. }
            | AuthoredChange::CreateExternal { .. }
            | AuthoredChange::CreateFunction { .. }
            | AuthoredChange::CreateConstant { .. }
            | AuthoredChange::CreateComponent { .. }
            | AuthoredChange::CreateTest { .. }
            | AuthoredChange::CreateTarget { .. }
            | AuthoredChange::CreateDocumentation { .. }
            | AuthoredChange::CreateAnnotation { .. } => {}
            AuthoredChange::RenameOwner { owner, name } => {
                let owner = lowerer.resolve_owner(owner)?;
                rename_owner(lowerer.candidate_mut(owner)?, name.clone())?;
            }
            AuthoredChange::MoveDeclaration {
                declaration,
                module,
            } => {
                let declaration = lowerer.resolve_declaration(declaration)?;
                let module = lowerer.resolve_module(module)?;
                let OwnerRecord::Declaration(record) =
                    lowerer.candidate_mut(OwnerKey::Declaration(declaration))?
                else {
                    return Err(request_error(
                        DiagnosticClass::Corrupt,
                        "change_authored_declaration_record",
                        "resolved declaration identity is bound to a foreign owner record",
                    ));
                };
                record.module = module;
            }
            AuthoredChange::ReplaceExpression {
                expression,
                operation,
            } => {
                let owner = OwnerKey::Expression(*expression);
                let candidate = lowerer.candidate_mut(owner)?;
                let OwnerRecord::Expression(_) = candidate else {
                    return Err(request_error(
                        DiagnosticClass::Semantic,
                        "change_authored_expression_kind",
                        "expression selector does not name an expression owner",
                    ));
                };
                *candidate =
                    OwnerRecord::Expression(ExpressionRecord::new(*expression, operation.clone())?);
            }
        }
        lowerer.check_budget("authored mutation lowering")?;
    }
    lowerer.finish()
}

fn collect_symbol_definitions(
    request: &AuthoredChangeSet,
) -> Result<BTreeMap<String, SymbolKind>, Diagnostic> {
    let mut definitions = BTreeMap::new();
    for change in &request.changes {
        match change {
            AuthoredChange::CreateModule { symbol, .. } => {
                define_symbol(&mut definitions, symbol, SymbolKind::Module)?;
            }
            AuthoredChange::CreateRecord { symbol, fields, .. } => {
                creation::collect_record_symbols(symbol, fields, &mut definitions)?
            }
            AuthoredChange::CreateVariant { symbol, cases, .. } => {
                creation::collect_variant_symbols(symbol, cases, &mut definitions)?
            }
            AuthoredChange::CreateInterface {
                symbol, operations, ..
            } => creation::collect_interface_symbols(symbol, operations, &mut definitions)?,
            AuthoredChange::CreateExternal {
                symbol,
                type_parameters,
                parameters,
                ..
            } => creation::collect_external_symbols(
                symbol,
                type_parameters,
                parameters,
                &mut definitions,
            )?,
            AuthoredChange::CreateFunction {
                symbol,
                type_parameters,
                parameters,
                body,
                ..
            } => creation::collect_function_symbols(
                symbol,
                type_parameters,
                parameters,
                body,
                &mut definitions,
            )?,
            AuthoredChange::CreateConstant { symbol, value, .. } => {
                creation::collect_constant_symbols(symbol, value, &mut definitions)?
            }
            AuthoredChange::CreateComponent {
                symbol,
                requirements,
                ports,
                ..
            } => {
                creation::collect_component_symbols(symbol, requirements, ports, &mut definitions)?
            }
            AuthoredChange::CreateTest {
                symbol,
                actual,
                expected,
                ..
            } => creation::collect_test_symbols(symbol, actual, expected, &mut definitions)?,
            AuthoredChange::CreateTarget { symbol, .. } => {
                creation::collect_target_symbols(symbol, &mut definitions)?
            }
            AuthoredChange::CreateDocumentation { symbol, .. } => {
                creation::collect_documentation_symbols(symbol, &mut definitions)?
            }
            AuthoredChange::CreateAnnotation { symbol, .. } => {
                creation::collect_annotation_symbols(symbol, &mut definitions)?
            }
            AuthoredChange::RenameOwner { .. }
            | AuthoredChange::MoveDeclaration { .. }
            | AuthoredChange::ReplaceExpression { .. } => {}
        }
    }
    Ok(definitions)
}

pub(super) fn define_symbol(
    definitions: &mut BTreeMap<String, SymbolKind>,
    symbol: &str,
    kind: SymbolKind,
) -> Result<(), Diagnostic> {
    validate_symbol(symbol)?;
    if definitions.insert(symbol.to_owned(), kind).is_some() {
        return Err(request_error(
            DiagnosticClass::Source,
            "change_authored_symbol_duplicate",
            format!("request-local symbol {symbol} is defined more than once"),
        ));
    }
    Ok(())
}

fn allocate_symbols(
    seed: &[u8],
    definitions: &BTreeMap<String, SymbolKind>,
) -> Result<BTreeMap<String, OwnerKey>, Diagnostic> {
    let mut ordinals = BTreeMap::<u8, u64>::new();
    let mut allocated = BTreeMap::new();
    for (symbol, kind) in definitions {
        let ordinal = ordinals.entry(kind.allocation_domain()).or_default();
        *ordinal = ordinal.checked_add(1).ok_or_else(|| {
            request_error(
                DiagnosticClass::Resource,
                "change_authored_allocation_ordinal",
                "request-local allocation ordinal was exhausted",
            )
        })?;
        allocated.insert(symbol.clone(), kind.allocate(seed, *ordinal));
    }
    Ok(allocated)
}

fn allocation_seed<B: CanonicalBaseRead + ?Sized>(
    base: &B,
    request: &AuthoredChangeSet,
    idempotency_key: Option<&str>,
) -> Result<[u8; 32], Diagnostic> {
    let configuration = bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding();
    let mut counter =
        bincode::enc::EncoderImpl::new(bincode::enc::write::SizeWriter::default(), configuration);
    Encode::encode(request, &mut counter).map_err(|error| {
        request_error(
            DiagnosticClass::Resource,
            "change_authored_encode",
            format!("authored change cannot be canonically sized: {error}"),
        )
    })?;
    let encoded_bytes = counter.into_writer().bytes_written;
    if encoded_bytes > MAXIMUM_AUTHORED_CHANGE_BYTES {
        return Err(request_error(
            DiagnosticClass::Resource,
            "change_authored_bytes",
            format!(
                "authored change requires {encoded_bytes} canonical bytes, exceeding the {MAXIMUM_AUTHORED_CHANGE_BYTES}-byte budget"
            ),
        ));
    }
    let mut bytes = vec![0_u8; encoded_bytes];
    let written =
        bincode::encode_into_slice(request, &mut bytes, configuration).map_err(|error| {
            request_error(
                DiagnosticClass::Resource,
                "change_authored_encode",
                format!("authored change cannot be canonically encoded: {error}"),
            )
        })?;
    debug_assert_eq!(written, encoded_bytes);
    let idempotency = idempotency_key.unwrap_or_default().as_bytes();
    let request_length = u64::try_from(bytes.len()).map_err(|_| {
        request_error(
            DiagnosticClass::Resource,
            "change_authored_length",
            "authored change byte length exceeds its canonical allocation domain",
        )
    })?;
    let idempotency_length = u64::try_from(idempotency.len()).map_err(|_| {
        request_error(
            DiagnosticClass::Resource,
            "change_authored_idempotency_length",
            "idempotency identity byte length exceeds its canonical allocation domain",
        )
    })?;
    let mut hasher = blake3::Hasher::new_derive_key(CHANGE_ALLOCATION_SEED_DOMAIN);
    hasher.update(&base.repository_id().bytes());
    hasher.update(&request.base.bytes());
    hasher.update(&request_length.to_be_bytes());
    hasher.update(&bytes);
    hasher.update(&idempotency_length.to_be_bytes());
    hasher.update(idempotency);
    Ok(*hasher.finalize().as_bytes())
}

struct WorkingOwner {
    before: Option<crate::platform::kernel::OwnerObjectDigest>,
    record: OwnerRecord,
}

struct AuthoredLowerer<'a, B: ?Sized, W: ?Sized> {
    base: &'a B,
    witness: &'a W,
    allocation_seed: [u8; 32],
    allocated: BTreeMap<String, OwnerKey>,
    definitions: BTreeMap<String, SymbolKind>,
    owners: BTreeMap<OwnerKey, WorkingOwner>,
    namespace: BTreeMap<NamespaceKey, Option<OwnerKey>>,
    types: TypeObjectInterner,
    next_anonymous_expression_ordinal: u64,
    work: AuthoredLoweringWork,
    budget: ChangeBudget,
}

impl<'a, B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized> AuthoredLowerer<'a, B, W> {
    fn new(
        base: &'a B,
        witness: &'a W,
        allocation_seed: [u8; 32],
        allocated: BTreeMap<String, OwnerKey>,
        definitions: BTreeMap<String, SymbolKind>,
        budget: ChangeBudget,
    ) -> Result<Self, Diagnostic> {
        let expression_symbol_count = definitions
            .values()
            .filter(|kind| **kind == SymbolKind::Expression)
            .count();
        let next_anonymous_expression_ordinal = u64::try_from(expression_symbol_count)
            .ok()
            .and_then(|count| count.checked_add(1))
            .ok_or_else(|| {
                request_error(
                    DiagnosticClass::Resource,
                    "change_authored_expression_ordinal",
                    "expression allocation ordinal was exhausted",
                )
            })?;
        Ok(Self {
            base,
            witness,
            allocation_seed,
            allocated,
            definitions,
            owners: BTreeMap::new(),
            namespace: BTreeMap::new(),
            types: TypeObjectInterner::default(),
            next_anonymous_expression_ordinal,
            work: AuthoredLoweringWork::default(),
            budget,
        })
    }

    fn check_budget(&self, phase: &str) -> Result<(), Diagnostic> {
        self.budget.check_observed(self.work.budget_work(), phase)
    }

    fn insert_created(&mut self, record: OwnerRecord) -> Result<(), Diagnostic> {
        let owner = record.owner();
        if self.owners.contains_key(&owner) {
            return Err(request_error(
                DiagnosticClass::Corrupt,
                "change_authored_allocation_collision",
                "one request-local identity was allocated more than once",
            ));
        }
        self.owners.insert(
            owner,
            WorkingOwner {
                before: None,
                record,
            },
        );
        Ok(())
    }

    fn resolve_owner(&mut self, selector: &OwnerSelector) -> Result<OwnerKey, Diagnostic> {
        let owner = match selector {
            OwnerSelector::Exact { owner } => *owner,
            OwnerSelector::ModuleName { name } => {
                OwnerKey::Module(self.resolve_module(&ModuleSelector::Name { name: name.clone() })?)
            }
            OwnerSelector::DeclarationName { module, name } => OwnerKey::Declaration(
                self.resolve_declaration(&DeclarationSelector::Qualified {
                    module: module.clone(),
                    name: name.clone(),
                })?,
            ),
            OwnerSelector::Symbol { symbol } => self.resolve_symbol(symbol)?,
        };
        self.require_owner(owner)?;
        Ok(owner)
    }

    fn resolve_module(&mut self, selector: &ModuleSelector) -> Result<ModuleId, Diagnostic> {
        let owner = match selector {
            ModuleSelector::Id { module } => OwnerKey::Module(*module),
            ModuleSelector::Name { name } => self.namespace_owner(NamespaceKey {
                parent: None,
                class: NamespaceClass::Module,
                name: name.clone(),
            })?,
            ModuleSelector::Symbol { symbol } => self.resolve_symbol(symbol)?,
        };
        self.require_owner(owner)?;
        match owner {
            OwnerKey::Module(module) => Ok(module),
            _ => Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_module_kind",
                "module selector resolved to a foreign owner domain",
            )),
        }
    }

    fn resolve_declaration(
        &mut self,
        selector: &DeclarationSelector,
    ) -> Result<crate::platform::semantic_id::DeclarationId, Diagnostic> {
        let owner = match selector {
            DeclarationSelector::Id { declaration } => OwnerKey::Declaration(*declaration),
            DeclarationSelector::Qualified { module, name } => {
                let module = self.resolve_module(module)?;
                self.namespace_owner(NamespaceKey {
                    parent: Some(OwnerKey::Module(module)),
                    class: NamespaceClass::Declaration,
                    name: name.clone(),
                })?
            }
            DeclarationSelector::Symbol { symbol } => {
                self.symbol_owner(symbol, SymbolKind::Declaration)?
            }
        };
        self.require_owner(owner)?;
        match owner {
            OwnerKey::Declaration(declaration) => Ok(declaration),
            _ => Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_declaration_kind",
                "declaration selector resolved to a foreign owner domain",
            )),
        }
    }

    fn resolve_symbol(&self, symbol: &str) -> Result<OwnerKey, Diagnostic> {
        validate_symbol(symbol)?;
        self.allocated.get(symbol).copied().ok_or_else(|| {
            request_error(
                DiagnosticClass::Source,
                "change_authored_symbol_missing",
                format!("request-local symbol {symbol} has no unique definition"),
            )
        })
    }

    fn symbol_kind(&self, symbol: &str) -> Result<SymbolKind, Diagnostic> {
        validate_symbol(symbol)?;
        self.definitions.get(symbol).copied().ok_or_else(|| {
            request_error(
                DiagnosticClass::Source,
                "change_authored_symbol_missing",
                format!("request-local symbol {symbol} has no unique definition"),
            )
        })
    }

    fn symbol_owner(&self, symbol: &str, expected: SymbolKind) -> Result<OwnerKey, Diagnostic> {
        let actual = self.symbol_kind(symbol)?;
        if actual != expected {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_authored_symbol_kind",
                format!("request-local symbol {symbol} has kind {actual:?}, expected {expected:?}"),
            ));
        }
        self.resolve_symbol(symbol)
    }

    fn module_symbol(&self, symbol: &str) -> Result<ModuleId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Module)? {
            OwnerKey::Module(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn declaration_symbol(&self, symbol: &str) -> Result<DeclarationId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Declaration)? {
            OwnerKey::Declaration(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn type_parameter_symbol(&self, symbol: &str) -> Result<TypeParameterId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::TypeParameter)? {
            OwnerKey::TypeParameter(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn field_symbol(&self, symbol: &str) -> Result<FieldId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Field)? {
            OwnerKey::Field(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn case_symbol(&self, symbol: &str) -> Result<CaseId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Case)? {
            OwnerKey::Case(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn operation_symbol(&self, symbol: &str) -> Result<OperationId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Operation)? {
            OwnerKey::Operation(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn function_parameter_symbol(&self, symbol: &str) -> Result<ParameterId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::FunctionParameter)? {
            OwnerKey::Parameter(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn operation_parameter_symbol(&self, symbol: &str) -> Result<ParameterId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::OperationParameter)? {
            OwnerKey::Parameter(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn lexical_binding_symbol(&self, symbol: &str) -> Result<BindingId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::LexicalBinding)? {
            OwnerKey::Binding(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn match_payload_symbol(&self, symbol: &str) -> Result<BindingId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::MatchPayloadBinding)? {
            OwnerKey::Binding(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn transaction_binding_symbol(&self, symbol: &str) -> Result<BindingId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::TransactionBinding)? {
            OwnerKey::Binding(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn requirement_symbol(&self, symbol: &str) -> Result<RequirementId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Requirement)? {
            OwnerKey::Requirement(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn port_symbol(&self, symbol: &str) -> Result<PortId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Port)? {
            OwnerKey::Port(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn target_symbol(&self, symbol: &str) -> Result<TargetId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Target)? {
            OwnerKey::Target(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn documentation_symbol(&self, symbol: &str) -> Result<DocumentationId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Documentation)? {
            OwnerKey::Documentation(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn annotation_symbol(&self, symbol: &str) -> Result<AnnotationId, Diagnostic> {
        match self.symbol_owner(symbol, SymbolKind::Annotation)? {
            OwnerKey::Annotation(value) => Ok(value),
            _ => Err(symbol_domain_corrupt(symbol)),
        }
    }

    fn resolve_creation_owner(&mut self, selector: &OwnerSelector) -> Result<OwnerKey, Diagnostic> {
        match selector {
            OwnerSelector::Symbol { symbol } => self.resolve_symbol(symbol),
            _ => self.resolve_owner(selector),
        }
    }

    fn resolve_creation_declaration(
        &mut self,
        selector: &DeclarationSelector,
    ) -> Result<DeclarationId, Diagnostic> {
        match selector {
            DeclarationSelector::Symbol { symbol } => self.declaration_symbol(symbol),
            _ => self.resolve_declaration(selector),
        }
    }

    fn lower_port_reference(
        &mut self,
        selector: &AuthoredPortReference,
    ) -> Result<crate::platform::kernel::PortReference, Diagnostic> {
        match selector {
            AuthoredPortReference::Exact { package, port } => {
                if *package == self.base.package_id() {
                    self.require_owner(OwnerKey::Port(*port))?;
                }
                Ok(crate::platform::kernel::PortReference {
                    package: *package,
                    port: *port,
                })
            }
            AuthoredPortReference::Symbol { symbol } => {
                Ok(crate::platform::kernel::PortReference {
                    package: self.base.package_id(),
                    port: self.port_symbol(symbol)?,
                })
            }
        }
    }

    fn expression_identity(&mut self, symbol: Option<&str>) -> Result<ExpressionId, Diagnostic> {
        if let Some(symbol) = symbol {
            return match self.symbol_owner(symbol, SymbolKind::Expression)? {
                OwnerKey::Expression(value) => Ok(value),
                _ => Err(symbol_domain_corrupt(symbol)),
            };
        }
        let ordinal = self.next_anonymous_expression_ordinal;
        self.next_anonymous_expression_ordinal = ordinal.checked_add(1).ok_or_else(|| {
            request_error(
                DiagnosticClass::Resource,
                "change_authored_expression_ordinal",
                "anonymous expression allocation ordinal was exhausted",
            )
        })?;
        Ok(ExpressionId::allocate(&self.allocation_seed, ordinal))
    }

    fn namespace_owner(&mut self, key: NamespaceKey) -> Result<OwnerKey, Diagnostic> {
        if !self.namespace.contains_key(&key) {
            let read = self.witness.read_namespace(&key)?;
            self.work.witness.add(read.work);
            self.namespace.insert(key.clone(), read.value);
        }
        self.namespace.get(&key).copied().flatten().ok_or_else(|| {
            request_error(
                DiagnosticClass::Semantic,
                "change_authored_selector_missing",
                format!("qualified selector has no owner at namespace key {key:?}"),
            )
        })
    }

    fn require_owner(&mut self, owner: OwnerKey) -> Result<(), Diagnostic> {
        if self.owners.contains_key(&owner) {
            return Ok(());
        }
        let read = self.base.read_owner(owner)?;
        self.work.canonical.add(read.work);
        let record = read.value.ok_or_else(|| {
            request_error(
                DiagnosticClass::Semantic,
                "change_authored_owner_missing",
                format!("selector names missing owner {owner:?}"),
            )
        })?;
        let (before, _) = encode_owner(&record)?;
        self.owners.insert(
            owner,
            WorkingOwner {
                before: Some(before),
                record,
            },
        );
        Ok(())
    }

    fn candidate_mut(&mut self, owner: OwnerKey) -> Result<&mut OwnerRecord, Diagnostic> {
        self.require_owner(owner)?;
        self.owners
            .get_mut(&owner)
            .map(|working| &mut working.record)
            .ok_or_else(|| {
                request_error(
                    DiagnosticClass::Corrupt,
                    "change_authored_owner_cache",
                    "resolved owner was not retained in the authored candidate overlay",
                )
            })
    }

    fn finish(self) -> Result<AuthoredLowering, Diagnostic> {
        let mut edits = Vec::new();
        for (digest, object) in self.types.into_objects() {
            edits.push(PrimitiveEdit::AddTypeObject { digest, object });
        }
        for (_, working) in self.owners {
            let (after, _) = encode_owner(&working.record)?;
            match working.before {
                None => edits.push(PrimitiveEdit::InsertOwner {
                    record: working.record,
                }),
                Some(before) if before != after => edits.push(PrimitiveEdit::ReplaceOwner {
                    expected: before,
                    record: working.record,
                }),
                Some(_) => {}
            }
        }
        Ok(AuthoredLowering {
            edits,
            allocated: self.allocated,
            work: self.work,
        })
    }
}

fn rename_owner(record: &mut OwnerRecord, name: Name) -> Result<(), Diagnostic> {
    let target = record.name_mut().ok_or_else(|| {
        request_error(
            DiagnosticClass::Semantic,
            "change_authored_rename_kind",
            "selected owner kind has no renameable semantic name",
        )
    })?;
    *target = name;
    Ok(())
}

fn validate_symbol(symbol: &str) -> Result<(), Diagnostic> {
    if !symbol.starts_with('$')
        || symbol.len() < 2
        || symbol.len() > MAXIMUM_REQUEST_SYMBOL_BYTES
        || !symbol.as_bytes()[1..]
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'-'))
    {
        return Err(request_error(
            DiagnosticClass::Source,
            "change_authored_symbol",
            format!(
                "request-local symbol must start with '$' and contain 1 through {} ASCII name bytes",
                MAXIMUM_REQUEST_SYMBOL_BYTES - 1
            ),
        ));
    }
    Ok(())
}

fn request_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

fn symbol_domain_corrupt(symbol: &str) -> Diagnostic {
    request_error(
        DiagnosticClass::Corrupt,
        "change_authored_symbol_domain",
        format!("request-local symbol {symbol} disagrees with its allocated identity domain"),
    )
}
