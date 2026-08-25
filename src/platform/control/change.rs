//! Closed compact-record adapter for normalized semantic changes.

use super::{CompactField, CompactRecord, parse_records};
use crate::platform::change::{
    AuthoredCase, AuthoredChange, AuthoredChangeSet, AuthoredDeclarationReference,
    AuthoredDeletePolicy, AuthoredExpression, AuthoredExpressionOperation, AuthoredField,
    AuthoredFunctionEffect, AuthoredLocalReference, AuthoredParameter, AuthoredType,
    AuthoredTypeParameterReference, DeclarationSelector, ModuleSelector, OwnerSelector,
    ParameterParentSelector,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass, SourceLocation};
use crate::platform::kernel::{DeclarationVisibility, Name, OwnerKey};
use crate::platform::publication::{PublicationOptions, idempotency_key_is_valid};
use crate::platform::semantic_id::{BindingId, DeclarationId, ModuleId, ParameterId, RevisionId};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

pub const COMPACT_CHANGE_CONTRACT_IDENTITY: &str = "lkjscript-change-records-2";
pub const CHANGE_PLAN_DIGEST_DOMAIN: &str = "lkjscript.change-plan.v3";
pub const COMPACT_CHANGE_OPERATIONS: &[&str] = &[
    "create.module",
    "create.record",
    "create.variant",
    "create.function",
    "create.constant",
    "create.test",
    "add.field",
    "add.case",
    "add.parameter",
    "delete.owner",
    "rename.owner",
    "move.declaration",
    "replace.body",
];
pub const COMPACT_DELETE_POLICIES: &[&str] = &["reject"];
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CompactChangeOperationField {
    pub(crate) operation: &'static str,
    pub(crate) name: &'static str,
    pub(crate) required: bool,
    pub(crate) form: &'static str,
}

pub(crate) const COMPACT_CHANGE_OPERATION_FIELDS: &[CompactChangeOperationField] = &[
    CompactChangeOperationField {
        operation: "delete.owner",
        name: "owner",
        required: true,
        form: "exact_owner",
    },
    CompactChangeOperationField {
        operation: "delete.owner",
        name: "policy",
        required: true,
        form: "delete_policy",
    },
];
pub const COMPACT_TYPE_FORMS: &[&str] = &[
    "unit",
    "bool",
    "i64",
    "bytes",
    "text",
    "static-text",
    "secret",
    "parameter",
    "named",
    "list",
    "map",
    "option",
    "result",
    "stream",
    "function",
];
pub const COMPACT_EXPRESSION_FORMS: &[&str] = &[
    "unit",
    "bool",
    "i64",
    "text",
    "static-text",
    "local",
    "constant",
    "if",
    "sequence",
    "call",
];

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ChangePlanDigest([u8; 32]);

impl fmt::Display for ChangePlanDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("plan_")?;
        formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
    }
}

impl FromStr for ChangePlanDigest {
    type Err = Diagnostic;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let encoded = value.strip_prefix("plan_").ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Source,
                "change_plan_domain",
                "reviewed plan digest must start with 'plan_'",
            )
        })?;
        if encoded.len() != 64 {
            return Err(Diagnostic::new(
                DiagnosticClass::Source,
                "change_plan_length",
                "reviewed plan digest must contain 64 lowercase hexadecimal characters",
            ));
        }
        let mut bytes = [0_u8; 32];
        for (index, pair) in encoded.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            let high = lower_hex(pair[0]).ok_or_else(|| invalid_plan_hex(encoded))?;
            let low = lower_hex(pair[1]).ok_or_else(|| invalid_plan_hex(encoded))?;
            bytes[index] = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

/// One transport-decoded request. The semantic operations remain owned by the change engine;
/// publication options and the reviewed-plan digest remain operational control data.
#[derive(Clone, Debug)]
pub(crate) struct CompactChangeRequest {
    pub semantic: AuthoredChangeSet,
    pub options: PublicationOptions,
    pub plan: ChangePlanDigest,
}

pub(crate) fn decode_compact_change(
    path: &str,
    input: &[u8],
) -> Result<CompactChangeRequest, Vec<Diagnostic>> {
    let records = parse_records(path, input)?;
    Decoder::new(records)
        .decode()
        .map_err(|diagnostic| vec![diagnostic])
}

#[derive(Clone, Debug)]
struct IndexedValue {
    index: usize,
    value: String,
    location: SourceLocation,
}

struct Decoder {
    records: Vec<CompactRecord>,
    types: BTreeMap<String, CompactRecord>,
    expressions: BTreeMap<String, CompactRecord>,
    arguments: BTreeMap<String, Vec<IndexedValue>>,
    type_parameters: BTreeMap<String, Vec<IndexedValue>>,
    changes: Vec<CompactRecord>,
    type_cache: BTreeMap<String, AuthoredType>,
    type_stack: BTreeSet<String>,
    expression_stack: BTreeSet<String>,
    expression_uses: BTreeMap<String, usize>,
}

impl Decoder {
    fn new(records: Vec<CompactRecord>) -> Self {
        Self {
            records,
            types: BTreeMap::new(),
            expressions: BTreeMap::new(),
            arguments: BTreeMap::new(),
            type_parameters: BTreeMap::new(),
            changes: Vec::new(),
            type_cache: BTreeMap::new(),
            type_stack: BTreeSet::new(),
            expression_stack: BTreeSet::new(),
            expression_uses: BTreeMap::new(),
        }
    }

    fn decode(mut self) -> Result<CompactChangeRequest, Diagnostic> {
        let mut request = None;
        for record in std::mem::take(&mut self.records) {
            match record.operation.as_str() {
                "request" => {
                    if request.is_some() {
                        return Err(record_error(
                            &record,
                            "change_request_duplicate",
                            "compact change contains more than one request record",
                        ));
                    }
                    request = Some(record);
                }
                "expression.argument" => self.insert_indexed_edge(&record, "expression", false)?,
                "type.argument" => self.insert_indexed_edge(&record, "type", true)?,
                operation if operation.starts_with("type.") => {
                    let label = required(&record, "as")?.to_owned();
                    validate_local_label(&record, "as", &label, '@')?;
                    if self.types.insert(label.clone(), record.clone()).is_some() {
                        return Err(field_error(
                            &record,
                            "as",
                            "change_type_duplicate",
                            format!("type label '{label}' is defined more than once"),
                        ));
                    }
                }
                operation if operation.starts_with("expression.") => {
                    let symbol = required(&record, "as")?.to_owned();
                    validate_local_label(&record, "as", &symbol, '$')?;
                    if self
                        .expressions
                        .insert(symbol.clone(), record.clone())
                        .is_some()
                    {
                        return Err(field_error(
                            &record,
                            "as",
                            "change_expression_duplicate",
                            format!("expression symbol '{symbol}' is defined more than once"),
                        ));
                    }
                    self.expression_uses.insert(symbol, 0);
                }
                operation if is_change_operation(operation) => self.changes.push(record),
                _ => {
                    return Err(record_error(
                        &record,
                        "change_operation_unknown",
                        format!(
                            "unknown compact change record '{}'; use 'capabilities change'",
                            record.operation
                        ),
                    ));
                }
            }
        }
        let request = request.ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Source,
                "change_request_missing",
                "compact change requires one request record with an exact base revision",
            )
        })?;
        check_fields(&request, &["base", "idempotency", "intent"])?;
        let base = parse_field::<RevisionId>(&request, "base")?;
        let idempotency_key = optional(&request, "idempotency").map(str::to_owned);
        if let Some(key) = idempotency_key.as_deref()
            && !idempotency_key_is_valid(key)
        {
            return Err(field_error(
                &request,
                "idempotency",
                "change_idempotency",
                "idempotency must contain 1 through 128 portable identifier bytes",
            ));
        }
        let intent = optional(&request, "intent").map(str::to_owned);
        if intent.as_ref().is_some_and(|value| {
            value.len() > crate::platform::publication::contract::MAXIMUM_INTENT_BYTES
        }) {
            return Err(field_error(
                &request,
                "intent",
                "change_intent_bytes",
                "intent exceeds its 4096-byte operational bound",
            ));
        }
        if self.changes.is_empty() {
            return Err(record_error(
                &request,
                "change_operations_missing",
                "compact change requires at least one semantic operation",
            ));
        }

        let mut changes = Vec::with_capacity(self.changes.len());
        for record in std::mem::take(&mut self.changes) {
            changes.push(self.decode_change(&record)?);
        }
        for (symbol, uses) in &self.expression_uses {
            if *uses == 0 {
                let record = self.expressions.get(symbol).ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticClass::Infrastructure,
                        "change_expression_inventory",
                        "expression use inventory lost a definition",
                    )
                })?;
                return Err(field_error(
                    record,
                    "as",
                    "change_expression_unused",
                    format!("expression '{symbol}' is not reachable from a semantic operation"),
                ));
            }
            if *uses > 1 {
                let record = self.expressions.get(symbol).ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticClass::Infrastructure,
                        "change_expression_inventory",
                        "expression use inventory lost a definition",
                    )
                })?;
                return Err(field_error(
                    record,
                    "as",
                    "change_expression_shared",
                    format!(
                        "expression '{symbol}' is referenced {uses} times; expression definitions form one owned tree"
                    ),
                ));
            }
        }
        for (parent, edges) in self.arguments.iter().chain(self.type_parameters.iter()) {
            if !self.expressions.contains_key(parent) && !self.types.contains_key(parent) {
                return Err(Diagnostic::source(
                    "change_edge_parent",
                    format!("edge parent '{parent}' has no matching compact definition"),
                    edges
                        .first()
                        .map(|edge| edge.location.clone())
                        .unwrap_or_else(|| request.location.clone()),
                ));
            }
        }

        let semantic = AuthoredChangeSet {
            base,
            preconditions: Vec::new(),
            changes,
            budget: Default::default(),
        };
        let options = PublicationOptions {
            idempotency_key,
            intent,
        };
        let plan = compact_change_plan_digest(&semantic, &options)?;
        Ok(CompactChangeRequest {
            semantic,
            options,
            plan,
        })
    }

    fn insert_indexed_edge(
        &mut self,
        record: &CompactRecord,
        value_field: &str,
        type_edge: bool,
    ) -> Result<(), Diagnostic> {
        check_fields(record, &["parent", "index", value_field])?;
        let parent = required(record, "parent")?.to_owned();
        let index = parse_field::<usize>(record, "index")?;
        let value = required(record, value_field)?.to_owned();
        let edge = IndexedValue {
            index,
            value,
            location: record.location.clone(),
        };
        let edges = if type_edge {
            self.type_parameters.entry(parent).or_default()
        } else {
            self.arguments.entry(parent).or_default()
        };
        if edges.iter().any(|candidate| candidate.index == index) {
            return Err(field_error(
                record,
                "index",
                "change_edge_index_duplicate",
                format!("parent repeats child index {index}"),
            ));
        }
        edges.push(edge);
        Ok(())
    }

    fn decode_change(&mut self, record: &CompactRecord) -> Result<AuthoredChange, Diagnostic> {
        match record.operation.as_str() {
            "create.module" => {
                check_fields(record, &["as", "name"])?;
                Ok(AuthoredChange::CreateModule {
                    symbol: symbol(record, "as")?,
                    name: parse_name(record, "name")?,
                })
            }
            "create.record" => {
                check_fields(record, &["as", "module", "name", "visibility"])?;
                Ok(AuthoredChange::CreateRecord {
                    symbol: symbol(record, "as")?,
                    module: parse_module_selector(record, "module")?,
                    name: parse_name(record, "name")?,
                    visibility: parse_visibility(record, "visibility")?,
                    fields: Vec::new(),
                })
            }
            "create.variant" => {
                check_fields(record, &["as", "module", "name", "visibility"])?;
                Ok(AuthoredChange::CreateVariant {
                    symbol: symbol(record, "as")?,
                    module: parse_module_selector(record, "module")?,
                    name: parse_name(record, "name")?,
                    visibility: parse_visibility(record, "visibility")?,
                    cases: Vec::new(),
                })
            }
            "create.function" => {
                check_fields(
                    record,
                    &[
                        "as",
                        "module",
                        "name",
                        "visibility",
                        "result",
                        "effect",
                        "body",
                    ],
                )?;
                let effect = required(record, "effect")?;
                if effect != "pure" {
                    return Err(field_error(
                        record,
                        "effect",
                        "change_effect_unsupported",
                        "the current compact create.function record supports effect=pure",
                    ));
                }
                let body = required(record, "body")?.to_owned();
                Ok(AuthoredChange::CreateFunction {
                    symbol: symbol(record, "as")?,
                    module: parse_module_selector(record, "module")?,
                    name: parse_name(record, "name")?,
                    visibility: parse_visibility(record, "visibility")?,
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    result: self.decode_type(required(record, "result")?)?,
                    effect: AuthoredFunctionEffect::Pure {},
                    body: self.decode_expression(&body)?,
                })
            }
            "create.constant" => {
                check_fields(
                    record,
                    &["as", "module", "name", "visibility", "type", "value"],
                )?;
                let value = required(record, "value")?.to_owned();
                Ok(AuthoredChange::CreateConstant {
                    symbol: symbol(record, "as")?,
                    module: parse_module_selector(record, "module")?,
                    name: parse_name(record, "name")?,
                    visibility: parse_visibility(record, "visibility")?,
                    ty: self.decode_type(required(record, "type")?)?,
                    value: self.decode_expression(&value)?,
                })
            }
            "create.test" => {
                check_fields(
                    record,
                    &["as", "module", "name", "visibility", "actual", "expected"],
                )?;
                let actual = required(record, "actual")?.to_owned();
                let expected = required(record, "expected")?.to_owned();
                Ok(AuthoredChange::CreateTest {
                    symbol: symbol(record, "as")?,
                    module: parse_module_selector(record, "module")?,
                    name: parse_name(record, "name")?,
                    visibility: parse_visibility(record, "visibility")?,
                    actual: self.decode_expression(&actual)?,
                    expected: self.decode_expression(&expected)?,
                })
            }
            "add.field" => {
                check_fields(record, &["as", "record", "name", "type"])?;
                Ok(AuthoredChange::AddField {
                    record: parse_declaration_selector(record, "record")?,
                    field: AuthoredField {
                        symbol: symbol(record, "as")?,
                        name: parse_name(record, "name")?,
                        ty: self.decode_type(required(record, "type")?)?,
                    },
                })
            }
            "add.case" => {
                check_fields(record, &["as", "variant", "name", "payload"])?;
                Ok(AuthoredChange::AddCase {
                    variant: parse_declaration_selector(record, "variant")?,
                    case: AuthoredCase {
                        symbol: symbol(record, "as")?,
                        name: parse_name(record, "name")?,
                        payload: optional(record, "payload")
                            .map(|value| self.decode_type(value))
                            .transpose()?,
                    },
                })
            }
            "add.parameter" => {
                check_fields(record, &["as", "function", "name", "type"])?;
                Ok(AuthoredChange::AddParameter {
                    parent: ParameterParentSelector::Declaration {
                        declaration: parse_declaration_selector(record, "function")?,
                    },
                    parameter: AuthoredParameter {
                        symbol: symbol(record, "as")?,
                        name: parse_name(record, "name")?,
                        ty: self.decode_type(required(record, "type")?)?,
                    },
                })
            }
            "delete.owner" => {
                check_described_operation_fields(record, "delete.owner")?;
                let policy = required(record, "policy")?;
                if policy != "reject" {
                    return Err(field_error(
                        record,
                        "policy",
                        "change_delete_policy",
                        format!("deletion policy must be reject; observed '{policy}'"),
                    ));
                }
                Ok(AuthoredChange::DeleteOwner {
                    owner: OwnerSelector::Exact {
                        owner: parse_field::<OwnerKey>(record, "owner")?,
                    },
                    policy: AuthoredDeletePolicy::Reject,
                })
            }
            "rename.owner" => {
                check_fields(record, &["owner", "name"])?;
                Ok(AuthoredChange::RenameOwner {
                    owner: parse_owner_selector(record, "owner")?,
                    name: parse_name(record, "name")?,
                })
            }
            "move.declaration" => {
                check_fields(record, &["declaration", "module"])?;
                Ok(AuthoredChange::MoveDeclaration {
                    declaration: parse_declaration_selector(record, "declaration")?,
                    module: parse_module_selector(record, "module")?,
                })
            }
            "replace.body" => {
                check_fields(record, &["function", "body"])?;
                let body = required(record, "body")?.to_owned();
                Ok(AuthoredChange::ReplaceFunctionBody {
                    function: parse_declaration_selector(record, "function")?,
                    body: self.decode_expression(&body)?,
                })
            }
            _ => Err(record_error(
                record,
                "change_operation_unknown",
                "compact change operation is not registered",
            )),
        }
    }

    fn decode_type(&mut self, reference: &str) -> Result<AuthoredType, Diagnostic> {
        let primitive = match reference {
            "unit" => Some(AuthoredType::Unit {}),
            "bool" => Some(AuthoredType::Bool {}),
            "i64" => Some(AuthoredType::I64 {}),
            "bytes" => Some(AuthoredType::Bytes {}),
            "text" => Some(AuthoredType::Text {}),
            "static-text" => Some(AuthoredType::StaticText {}),
            "secret" => Some(AuthoredType::Secret {}),
            _ => None,
        };
        if let Some(primitive) = primitive {
            return Ok(primitive);
        }
        if !reference.starts_with('@') {
            return Err(Diagnostic::new(
                DiagnosticClass::Source,
                "change_type_reference",
                format!("unknown type reference '{reference}'"),
            ));
        }
        if let Some(cached) = self.type_cache.get(reference) {
            return Ok(cached.clone());
        }
        if !self.type_stack.insert(reference.to_owned()) {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "change_type_cycle",
                format!("type definition cycle reaches '{reference}'"),
            ));
        }
        let record = self.types.get(reference).cloned().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Source,
                "change_type_undefined",
                format!("type label '{reference}' is not defined"),
            )
        })?;
        let ty = match record.operation.as_str() {
            "type.unit" | "type.bool" | "type.i64" | "type.bytes" | "type.text"
            | "type.static-text" | "type.secret" => {
                check_fields(&record, &["as"])?;
                match record.operation.as_str() {
                    "type.unit" => AuthoredType::Unit {},
                    "type.bool" => AuthoredType::Bool {},
                    "type.i64" => AuthoredType::I64 {},
                    "type.bytes" => AuthoredType::Bytes {},
                    "type.text" => AuthoredType::Text {},
                    "type.static-text" => AuthoredType::StaticText {},
                    _ => AuthoredType::Secret {},
                }
            }
            "type.list" | "type.option" | "type.stream" => {
                check_fields(&record, &["as", "item"])?;
                let item = Box::new(self.decode_type(required(&record, "item")?)?);
                match record.operation.as_str() {
                    "type.list" => AuthoredType::List { item },
                    "type.option" => AuthoredType::Option { item },
                    _ => AuthoredType::Stream { item },
                }
            }
            "type.map" => {
                check_fields(&record, &["as", "key", "value"])?;
                AuthoredType::Map {
                    key: Box::new(self.decode_type(required(&record, "key")?)?),
                    value: Box::new(self.decode_type(required(&record, "value")?)?),
                }
            }
            "type.result" => {
                check_fields(&record, &["as", "ok", "error"])?;
                AuthoredType::Result {
                    ok: Box::new(self.decode_type(required(&record, "ok")?)?),
                    error: Box::new(self.decode_type(required(&record, "error")?)?),
                }
            }
            "type.named" => {
                check_fields(&record, &["as", "declaration"])?;
                AuthoredType::Named {
                    declaration: parse_declaration_reference(&record, "declaration")?,
                }
            }
            "type.parameter" => {
                check_fields(&record, &["as", "parameter"])?;
                AuthoredType::TypeParameter {
                    parameter: parse_type_parameter_reference(&record, "parameter")?,
                }
            }
            "type.function" => {
                check_fields(&record, &["as", "result"])?;
                let parameters = self
                    .ordered_edges(reference, true)?
                    .into_iter()
                    .map(|edge| self.decode_type(&edge.value))
                    .collect::<Result<Vec<_>, _>>()?;
                AuthoredType::Function {
                    parameters,
                    result: Box::new(self.decode_type(required(&record, "result")?)?),
                }
            }
            operation => {
                return Err(record_error(
                    &record,
                    "change_type_form_unknown",
                    format!("unknown compact type form '{operation}'"),
                ));
            }
        };
        self.type_stack.remove(reference);
        self.type_cache.insert(reference.to_owned(), ty.clone());
        Ok(ty)
    }

    fn decode_expression(&mut self, symbol: &str) -> Result<AuthoredExpression, Diagnostic> {
        if !symbol.starts_with('$') {
            return Err(Diagnostic::new(
                DiagnosticClass::Source,
                "change_expression_reference",
                format!("expression reference '{symbol}' must be a $ symbol"),
            ));
        }
        if !self.expression_stack.insert(symbol.to_owned()) {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "change_expression_cycle",
                format!("expression definition cycle reaches '{symbol}'"),
            ));
        }
        let record = self.expressions.get(symbol).cloned().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Source,
                "change_expression_undefined",
                format!("expression symbol '{symbol}' is not defined"),
            )
        })?;
        let uses = self.expression_uses.get_mut(symbol).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "change_expression_inventory",
                "expression use inventory lost a definition",
            )
        })?;
        *uses = uses.saturating_add(1);
        let operation = match record.operation.as_str() {
            "expression.unit" => {
                check_fields(&record, &["as"])?;
                AuthoredExpressionOperation::Unit {}
            }
            "expression.bool" => {
                check_fields(&record, &["as", "value"])?;
                AuthoredExpressionOperation::Bool {
                    value: parse_bool(&record, "value")?,
                }
            }
            "expression.i64" => {
                check_fields(&record, &["as", "value"])?;
                AuthoredExpressionOperation::I64 {
                    value: parse_field(&record, "value")?,
                }
            }
            "expression.text" | "expression.static-text" => {
                check_fields(&record, &["as", "value"])?;
                let value = required(&record, "value")?.to_owned();
                if record.operation == "expression.text" {
                    AuthoredExpressionOperation::Text { value }
                } else {
                    AuthoredExpressionOperation::StaticText { value }
                }
            }
            "expression.local" => {
                check_fields(&record, &["as", "value"])?;
                AuthoredExpressionOperation::Local {
                    value: parse_local_reference(&record, "value")?,
                }
            }
            "expression.constant" => {
                check_fields(&record, &["as", "declaration"])?;
                AuthoredExpressionOperation::Constant {
                    declaration: parse_declaration_reference(&record, "declaration")?,
                }
            }
            "expression.if" => {
                check_fields(&record, &["as", "condition", "when-true", "when-false"])?;
                let condition = required(&record, "condition")?.to_owned();
                let when_true = required(&record, "when-true")?.to_owned();
                let when_false = required(&record, "when-false")?.to_owned();
                AuthoredExpressionOperation::If {
                    condition: Box::new(self.decode_expression(&condition)?),
                    when_true: Box::new(self.decode_expression(&when_true)?),
                    when_false: Box::new(self.decode_expression(&when_false)?),
                }
            }
            "expression.sequence" => {
                check_fields(&record, &["as"])?;
                AuthoredExpressionOperation::Sequence {
                    items: self.decode_expression_edges(symbol)?,
                }
            }
            "expression.call" => {
                check_fields(&record, &["as", "function"])?;
                AuthoredExpressionOperation::Call {
                    function: parse_declaration_reference(&record, "function")?,
                    type_arguments: self
                        .ordered_edges(symbol, true)?
                        .into_iter()
                        .map(|edge| self.decode_type(&edge.value))
                        .collect::<Result<Vec<_>, _>>()?,
                    arguments: self.decode_expression_edges(symbol)?,
                }
            }
            operation => {
                return Err(record_error(
                    &record,
                    "change_expression_form_unknown",
                    format!("unknown compact expression form '{operation}'"),
                ));
            }
        };
        self.expression_stack.remove(symbol);
        Ok(AuthoredExpression {
            symbol: Some(symbol.to_owned()),
            operation,
        })
    }

    fn decode_expression_edges(
        &mut self,
        parent: &str,
    ) -> Result<Vec<AuthoredExpression>, Diagnostic> {
        self.ordered_edges(parent, false)?
            .into_iter()
            .map(|edge| self.decode_expression(&edge.value))
            .collect()
    }

    fn ordered_edges(
        &self,
        parent: &str,
        type_edges: bool,
    ) -> Result<Vec<IndexedValue>, Diagnostic> {
        let mut edges = if type_edges {
            self.type_parameters.get(parent)
        } else {
            self.arguments.get(parent)
        }
        .cloned()
        .unwrap_or_default();
        edges.sort_by_key(|edge| edge.index);
        for (expected, edge) in edges.iter().enumerate() {
            if edge.index != expected {
                return Err(Diagnostic::source(
                    "change_edge_index_order",
                    format!(
                        "parent '{parent}' child indexes must be contiguous from zero; expected {expected}, observed {}",
                        edge.index
                    ),
                    edge.location.clone(),
                ));
            }
        }
        Ok(edges)
    }
}

fn is_change_operation(operation: &str) -> bool {
    COMPACT_CHANGE_OPERATIONS.contains(&operation)
}

fn compact_change_plan_digest(
    request: &AuthoredChangeSet,
    options: &PublicationOptions,
) -> Result<ChangePlanDigest, Diagnostic> {
    let intent = crate::platform::change::canonical_authored_intent_bytes(request)?;
    let budget = crate::platform::change::canonical_authored_budget_bytes(request.budget)?;
    let mut hasher = blake3::Hasher::new_derive_key(CHANGE_PLAN_DIGEST_DOMAIN);
    hash_digest_field(&mut hasher, COMPACT_CHANGE_CONTRACT_IDENTITY.as_bytes())?;
    hash_digest_field(&mut hasher, &intent)?;
    hash_digest_field(&mut hasher, &budget)?;
    hash_optional_digest_field(&mut hasher, options.idempotency_key.as_deref())?;
    hash_optional_digest_field(&mut hasher, options.intent.as_deref())?;
    Ok(ChangePlanDigest(*hasher.finalize().as_bytes()))
}

const fn lower_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn invalid_plan_hex(value: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "change_plan_hex",
        format!("reviewed plan digest '{value}' is not canonical lowercase hexadecimal"),
    )
}

fn hash_optional_digest_field(
    hasher: &mut blake3::Hasher,
    value: Option<&str>,
) -> Result<(), Diagnostic> {
    match value {
        Some(value) => {
            hasher.update(&[1]);
            hash_digest_field(hasher, value.as_bytes())
        }
        None => {
            hasher.update(&[0]);
            Ok(())
        }
    }
}

fn hash_digest_field(hasher: &mut blake3::Hasher, value: &[u8]) -> Result<(), Diagnostic> {
    let length = u64::try_from(value.len()).map_err(|_| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "change_plan_field_length",
            "normalized plan field exceeds its digest length domain",
        )
    })?;
    hasher.update(&length.to_be_bytes());
    hasher.update(value);
    Ok(())
}

fn check_fields(record: &CompactRecord, allowed: &[&str]) -> Result<(), Diagnostic> {
    for field in &record.fields {
        if !allowed.contains(&field.name.as_str()) {
            return Err(Diagnostic::source(
                "change_field_unknown",
                format!(
                    "record '{}' does not define field '{}'; use 'capabilities change'",
                    record.operation, field.name
                ),
                field.location.clone(),
            ));
        }
    }
    Ok(())
}

fn check_described_operation_fields(
    record: &CompactRecord,
    operation: &str,
) -> Result<(), Diagnostic> {
    let descriptors = COMPACT_CHANGE_OPERATION_FIELDS
        .iter()
        .filter(|descriptor| descriptor.operation == operation)
        .collect::<Vec<_>>();
    for field in &record.fields {
        if !descriptors
            .iter()
            .any(|descriptor| descriptor.name == field.name)
        {
            return Err(field_error(
                record,
                &field.name,
                "change_field_unknown",
                format!(
                    "operation '{}' does not accept field '{}'",
                    record.operation, field.name
                ),
            ));
        }
    }
    for descriptor in descriptors {
        if descriptor.required {
            required(record, descriptor.name)?;
        }
    }
    Ok(())
}

fn required<'a>(record: &'a CompactRecord, name: &str) -> Result<&'a str, Diagnostic> {
    optional(record, name).ok_or_else(|| {
        record_error(
            record,
            "change_field_missing",
            format!("record '{}' requires field '{name}'", record.operation),
        )
    })
}

fn optional<'a>(record: &'a CompactRecord, name: &str) -> Option<&'a str> {
    record
        .fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.as_str())
}

fn field<'a>(record: &'a CompactRecord, name: &str) -> Option<&'a CompactField> {
    record.fields.iter().find(|field| field.name == name)
}

fn parse_field<T>(record: &CompactRecord, name: &str) -> Result<T, Diagnostic>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    let value = required(record, name)?;
    value.parse().map_err(|error| {
        field_error(
            record,
            name,
            "change_field_value",
            format!("field '{name}' has invalid value '{value}': {error}"),
        )
    })
}

fn parse_bool(record: &CompactRecord, name: &str) -> Result<bool, Diagnostic> {
    match required(record, name)? {
        "true" => Ok(true),
        "false" => Ok(false),
        value => Err(field_error(
            record,
            name,
            "change_boolean",
            format!("field '{name}' requires true or false, observed '{value}'"),
        )),
    }
}

fn parse_name(record: &CompactRecord, field_name: &str) -> Result<Name, Diagnostic> {
    let value = required(record, field_name)?;
    Name::new(value).map_err(|error| field_error(record, field_name, error.code, error.message))
}

fn symbol(record: &CompactRecord, name: &str) -> Result<String, Diagnostic> {
    let value = required(record, name)?.to_owned();
    validate_local_label(record, name, &value, '$')?;
    Ok(value)
}

fn validate_local_label(
    record: &CompactRecord,
    field_name: &str,
    value: &str,
    prefix: char,
) -> Result<(), Diagnostic> {
    let mut characters = value.chars();
    if characters.next() != Some(prefix)
        || !characters
            .clone()
            .next()
            .is_some_and(|value| value.is_ascii_alphabetic() || value == '_')
        || !characters.all(|value| value.is_ascii_alphanumeric() || value == '_' || value == '-')
        || value.len() > 128
    {
        return Err(field_error(
            record,
            field_name,
            "change_local_label",
            format!(
                "field '{field_name}' requires {prefix} followed by 1 through 127 portable identifier bytes"
            ),
        ));
    }
    Ok(())
}

fn parse_visibility(
    record: &CompactRecord,
    field_name: &str,
) -> Result<DeclarationVisibility, Diagnostic> {
    match required(record, field_name)? {
        "private" => Ok(DeclarationVisibility::Private),
        "package" => Ok(DeclarationVisibility::Package),
        "public" => Ok(DeclarationVisibility::Public),
        value => Err(field_error(
            record,
            field_name,
            "change_visibility",
            format!("visibility must be private, package, or public; observed '{value}'"),
        )),
    }
}

fn parse_module_selector(
    record: &CompactRecord,
    field_name: &str,
) -> Result<ModuleSelector, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        Ok(ModuleSelector::Symbol {
            symbol: value.to_owned(),
        })
    } else if value.starts_with(ModuleId::PREFIX) {
        Ok(ModuleSelector::Id {
            module: parse_field(record, field_name)?,
        })
    } else {
        Ok(ModuleSelector::Name {
            name: parse_name(record, field_name)?,
        })
    }
}

fn parse_declaration_selector(
    record: &CompactRecord,
    field_name: &str,
) -> Result<DeclarationSelector, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        Ok(DeclarationSelector::Symbol {
            symbol: value.to_owned(),
        })
    } else if value.starts_with(DeclarationId::PREFIX) {
        Ok(DeclarationSelector::Id {
            declaration: parse_field(record, field_name)?,
        })
    } else if let Some((module, name)) = value.split_once('/') {
        let module = if module.starts_with(ModuleId::PREFIX) {
            ModuleSelector::Id {
                module: module.parse().map_err(|error: Diagnostic| {
                    field_error(record, field_name, error.code, error.message)
                })?,
            }
        } else {
            ModuleSelector::Name {
                name: Name::new(module)
                    .map_err(|error| field_error(record, field_name, error.code, error.message))?,
            }
        };
        Ok(DeclarationSelector::Qualified {
            module,
            name: Name::new(name)
                .map_err(|error| field_error(record, field_name, error.code, error.message))?,
        })
    } else {
        Err(field_error(
            record,
            field_name,
            "change_declaration_selector",
            "declaration selector requires $symbol, decl_ID, or MODULE/NAME",
        ))
    }
}

fn parse_owner_selector(
    record: &CompactRecord,
    field_name: &str,
) -> Result<OwnerSelector, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        Ok(OwnerSelector::Symbol {
            symbol: value.to_owned(),
        })
    } else {
        Ok(OwnerSelector::Exact {
            owner: parse_field(record, field_name)?,
        })
    }
}

fn parse_declaration_reference(
    record: &CompactRecord,
    field_name: &str,
) -> Result<AuthoredDeclarationReference, Diagnostic> {
    let value = required(record, field_name)?;
    if let Some((package, declaration)) = value.split_once('/') {
        return Ok(AuthoredDeclarationReference::Exact {
            package: package.parse().map_err(|error: Diagnostic| {
                field_error(record, field_name, error.code, error.message)
            })?,
            declaration: declaration.parse().map_err(|error: Diagnostic| {
                field_error(record, field_name, error.code, error.message)
            })?,
        });
    }
    Ok(AuthoredDeclarationReference::Local {
        declaration: parse_declaration_selector(record, field_name)?,
    })
}

fn parse_type_parameter_reference(
    record: &CompactRecord,
    field_name: &str,
) -> Result<AuthoredTypeParameterReference, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        Ok(AuthoredTypeParameterReference::Symbol {
            symbol: value.to_owned(),
        })
    } else {
        Ok(AuthoredTypeParameterReference::Id {
            parameter: parse_field(record, field_name)?,
        })
    }
}

fn parse_local_reference(
    record: &CompactRecord,
    field_name: &str,
) -> Result<AuthoredLocalReference, Diagnostic> {
    let value = required(record, field_name)?;
    if value.starts_with('$') {
        validate_local_label(record, field_name, value, '$')?;
        return Ok(AuthoredLocalReference::Symbol {
            symbol: value.to_owned(),
        });
    }
    if value.starts_with(ParameterId::PREFIX) {
        return Ok(AuthoredLocalReference::FunctionParameter {
            parameter: parse_field(record, field_name)?,
        });
    }
    if value.starts_with(BindingId::PREFIX) {
        return Ok(AuthoredLocalReference::LexicalBinding {
            binding: parse_field(record, field_name)?,
        });
    }
    Err(field_error(
        record,
        field_name,
        "change_local_reference",
        "local reference requires $symbol, param_ID, or bind_ID",
    ))
}

fn record_error(
    record: &CompactRecord,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::source(code, message, record.location.clone())
}

fn field_error(
    record: &CompactRecord,
    field_name: &str,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    let location = field(record, field_name)
        .map(|field| field.location.clone())
        .unwrap_or_else(|| record.location.clone());
    Diagnostic::source(code, message, location)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn revision() -> RevisionId {
        RevisionId::from_digest([7; 32])
    }

    #[test]
    fn connected_creation_decodes_to_one_typed_request_and_stable_plan() {
        let input = format!(
            "request base={} idempotency=connected-1\n\
             create.module as=$notes name=notes\n\
             create.record as=$note module=$notes name=Note visibility=public\n\
             add.field as=$text record=$note name=text type=text\n\
             expression.local as=$read value=$value\n\
             create.function as=$make module=$notes name=make visibility=public result=text effect=pure body=$read\n\
             add.parameter as=$value function=$make name=value type=text\n",
            revision()
        );
        let decoded = decode_compact_change("change.lk", input.as_bytes()).unwrap();
        assert_eq!(decoded.semantic.base, revision());
        assert_eq!(decoded.semantic.changes.len(), 5);
        assert_eq!(
            decoded.options.idempotency_key.as_deref(),
            Some("connected-1")
        );
        let repeated = decode_compact_change("other.lk", input.as_bytes()).unwrap();
        assert_eq!(decoded.plan, repeated.plan);
    }

    #[test]
    fn flat_expression_edges_are_ordered_and_nested_without_shared_authority() {
        let input = format!(
            "request base={}\n\
             expression.text as=$second value=second\n\
             expression.text as=$first value=first\n\
             expression.sequence as=$body\n\
             expression.argument parent=$body index=1 expression=$second\n\
             expression.argument parent=$body index=0 expression=$first\n\
             create.module as=$m name=m\n\
             create.function as=$f module=$m name=f visibility=private result=text effect=pure body=$body\n",
            revision()
        );
        let decoded = decode_compact_change("change.lk", input.as_bytes()).unwrap();
        let AuthoredChange::CreateFunction { body, .. } = &decoded.semantic.changes[1] else {
            panic!("function operation")
        };
        let AuthoredExpressionOperation::Sequence { items } = &body.operation else {
            panic!("sequence body")
        };
        assert_eq!(items.len(), 2);
        assert!(matches!(
            &items[0].operation,
            AuthoredExpressionOperation::Text { value } if value == "first"
        ));
    }

    #[test]
    fn malformed_records_report_exact_independent_locations() {
        let input = format!(
            "request base={}\ncreate.module as=$m name=m extra=no\n",
            revision()
        );
        let error = decode_compact_change("change.lk", input.as_bytes()).unwrap_err();
        assert_eq!(error[0].code, "change_field_unknown");
        assert_eq!(error[0].location.as_ref().unwrap().line, 2);
    }

    #[test]
    fn deletion_is_exact_and_reject_only() {
        let owner = OwnerKey::Module(ModuleId::migrate(b"compact-delete", 1));
        let input = format!(
            "request base={}\ndelete.owner owner={owner} policy=reject\n",
            revision()
        );
        let decoded = decode_compact_change("delete.lk", input.as_bytes()).unwrap();
        assert!(matches!(
            &decoded.semantic.changes[..],
            [AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner: observed },
                policy: AuthoredDeletePolicy::Reject,
            }] if *observed == owner
        ));

        let unsupported = format!(
            "request base={}\ndelete.owner owner={owner} policy=owned-closure\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("delete.lk", unsupported.as_bytes()).unwrap_err()[0].code,
            "change_delete_policy"
        );
        let predecessor = format!(
            "request base={}\ndelete.owner owner={owner} cascade=true policy=reject\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("delete.lk", predecessor.as_bytes()).unwrap_err()[0].code,
            "change_field_unknown"
        );
    }

    #[test]
    fn cycles_duplicate_edges_and_unused_expressions_fail_closed() {
        let cycle = format!(
            "request base={}\nexpression.if as=$body condition=$body when-true=$body when-false=$body\ncreate.module as=$m name=m\ncreate.function as=$f module=$m name=f visibility=private result=unit effect=pure body=$body\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("cycle.lk", cycle.as_bytes()).unwrap_err()[0].code,
            "change_expression_cycle"
        );

        let unused = format!(
            "request base={}\nexpression.unit as=$unused\ncreate.module as=$m name=m\n",
            revision()
        );
        assert_eq!(
            decode_compact_change("unused.lk", unused.as_bytes()).unwrap_err()[0].code,
            "change_expression_unused"
        );
    }

    #[test]
    fn json_and_unknown_operations_are_not_alternate_inputs() {
        let json = br#"{"base":"rev_dead"}"#;
        assert_eq!(
            decode_compact_change("change.lk", json).unwrap_err()[0].code,
            "control_operation"
        );
        let unknown = format!("request base={}\ncreate.unknown as=$x\n", revision());
        assert_eq!(
            decode_compact_change("change.lk", unknown.as_bytes()).unwrap_err()[0].code,
            "change_operation_unknown"
        );
    }

    #[test]
    fn reviewed_plan_binds_budget_and_operational_options() {
        let input = format!(
            "request base={}\ncreate.module as=$module name=module\n",
            revision()
        );
        let decoded = decode_compact_change("change.lk", input.as_bytes()).unwrap();

        let mut budget_changed = decoded.semantic.clone();
        budget_changed.budget.canonical_reads.maximum_bytes -= 1;
        assert_ne!(
            decoded.plan,
            compact_change_plan_digest(&budget_changed, &decoded.options).unwrap()
        );

        let options_changed = PublicationOptions {
            idempotency_key: Some("reviewed-plan-option".to_owned()),
            intent: None,
        };
        assert_ne!(
            decoded.plan,
            compact_change_plan_digest(&decoded.semantic, &options_changed).unwrap()
        );
    }
}
