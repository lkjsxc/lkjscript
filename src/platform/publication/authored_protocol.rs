//! Strict private Graph 5 authored-change JSON contract pending the direct public cutover.
//!
//! The current public CLI remains Change Contract 3. This module derives a candidate Change
//! Contract 5 schema from the actual Graph 5 request decoder and compact response projection so
//! the eventual cutover cannot depend on a handwritten parallel catalog.

use super::{
    ChangeCounts, PreparedAuthoredPublication, PublicationOptions, PublicationOutcome,
    SemanticDiffDigest, TransactionDigest, ValidationEvidence, WorkObservation,
};
use crate::platform::change::{AuthoredChangeSet, ChangeBudgetWork};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::json::{JsonIntegerPolicy, JsonLimits, decode_strict_with_integer_policy};
use crate::platform::kernel::OwnerKey;
use crate::platform::semantic_id::RevisionId;
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

pub const AUTHORED_CHANGE_CONTRACT_IDENTITY: &str = "lkjscript-change-5";
pub const AUTHORED_CHANGE_CONTRACT_VERSION: u16 = 5;
pub const AUTHORED_PROTOCOL_SCHEMA_ID: &str =
    "https://lkjscript.org/schema/private/graph5-authored-change-5.json";
pub const AUTHORED_PROTOCOL_SCHEMA_DIGEST_DOMAIN: &str =
    "lkjscript.graph5-authored-protocol-schema.v5";
pub const MAXIMUM_AUTHORED_RESPONSE_BYTES: usize = 4 * 1_048_576;
pub const MAXIMUM_AUTHORED_JSON_DEPTH: usize = 128;
pub const MAXIMUM_AUTHORED_JSON_ITEMS: usize =
    crate::platform::change::MAXIMUM_AUTHORED_CHANGE_BYTES / 2;

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5ChangeContractV1")]
pub enum AuthoredChangeContract {
    #[serde(rename = "lkjscript-change-5")]
    Current,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5AuthoredChangeRequestV1")]
#[serde(deny_unknown_fields)]
pub struct AuthoredChangeRequest {
    pub contract: AuthoredChangeContract,
    #[serde(default)]
    #[schemars(regex(pattern = "^[A-Za-z0-9_.-]{1,128}$"))]
    pub idempotency_key: Option<String>,
    #[serde(flatten)]
    pub semantic: AuthoredChangeSet,
    #[serde(default)]
    #[schemars(length(max = super::contract::MAXIMUM_INTENT_BYTES))]
    pub intent: Option<String>,
}

impl AuthoredChangeRequest {
    pub fn decode_json(bytes: &[u8]) -> Result<Self, Diagnostic> {
        if bytes.is_empty() || bytes.len() > crate::platform::change::MAXIMUM_AUTHORED_CHANGE_BYTES
        {
            return Err(protocol_error(
                DiagnosticClass::Resource,
                "change_protocol_request_bytes",
                format!(
                    "Graph 5 authored request must contain 1 through {} bytes",
                    crate::platform::change::MAXIMUM_AUTHORED_CHANGE_BYTES
                ),
            ));
        }
        let value = decode_strict_with_integer_policy(
            bytes,
            JsonLimits {
                maximum_bytes: crate::platform::change::MAXIMUM_AUTHORED_CHANGE_BYTES,
                maximum_depth: MAXIMUM_AUTHORED_JSON_DEPTH,
                maximum_items: MAXIMUM_AUTHORED_JSON_ITEMS,
                maximum_string_bytes: crate::platform::change::MAXIMUM_AUTHORED_CHANGE_BYTES,
            },
            JsonIntegerPolicy::SignedOrUnsigned64,
        )
        .map_err(|diagnostic| {
            let (code, message) = if diagnostic.code == "json_trailing" {
                (
                    "change_protocol_request_trailing",
                    "Graph 5 authored request contains trailing input",
                )
            } else {
                (
                    "change_protocol_request_json",
                    "Graph 5 authored request violates its bounded strict JSON contract",
                )
            };
            protocol_error(DiagnosticClass::Source, code, message)
        })?;
        let request: Self = serde_json::from_value(value).map_err(|_| {
            protocol_error(
                DiagnosticClass::Source,
                "change_protocol_request_json",
                "Graph 5 authored request is not one strict current JSON object",
            )
        })?;
        request.validate()?;
        Ok(request)
    }

    pub fn encode_json(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|_| {
            protocol_error(
                DiagnosticClass::Infrastructure,
                "change_protocol_request_encode",
                "Graph 5 authored request could not be encoded",
            )
        })?;
        if bytes.len() > crate::platform::change::MAXIMUM_AUTHORED_CHANGE_BYTES {
            return Err(protocol_error(
                DiagnosticClass::Resource,
                "change_protocol_request_bytes",
                "encoded Graph 5 authored request exceeds its byte bound",
            ));
        }
        Ok(bytes)
    }

    pub(crate) fn publication_options(&self) -> PublicationOptions {
        PublicationOptions {
            idempotency_key: self.idempotency_key.clone(),
            intent: self.intent.clone(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), Diagnostic> {
        self.semantic.budget.validate_request_counts(
            self.semantic.changes.len(),
            self.semantic.preconditions.len(),
        )?;
        if let Some(key) = &self.idempotency_key
            && (key.is_empty()
                || key.len() > super::contract::MAXIMUM_IDEMPOTENCY_KEY_BYTES
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')))
        {
            return Err(protocol_error(
                DiagnosticClass::Source,
                "change_protocol_idempotency",
                "idempotency key must contain 1 through 128 portable identifier bytes",
            ));
        }
        if self
            .intent
            .as_ref()
            .is_some_and(|intent| intent.len() > super::contract::MAXIMUM_INTENT_BYTES)
        {
            return Err(protocol_error(
                DiagnosticClass::Resource,
                "change_protocol_intent",
                "bounded nonsemantic intent exceeds its current byte limit",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5AuthoredChangeStatusV1")]
#[serde(rename_all = "snake_case")]
pub enum AuthoredChangeResponseStatus {
    Prepared,
    Accepted,
    AlreadyAccepted,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(rename = "lkjscript.Graph5AuthoredChangeResponseV1")]
#[serde(deny_unknown_fields)]
pub struct AuthoredChangeResponse {
    pub contract: AuthoredChangeContract,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub schema_digest: String,
    pub status: AuthoredChangeResponseStatus,
    pub base: RevisionId,
    pub result: RevisionId,
    #[schemars(with = "String", regex(pattern = "^transaction_[0-9a-f]{64}$"))]
    pub transaction: TransactionDigest,
    #[schemars(with = "String", regex(pattern = "^semantic_diff_[0-9a-f]{64}$"))]
    pub semantic_diff: SemanticDiffDigest,
    #[schemars(length(max = crate::platform::change::MAXIMUM_AUTHORED_CHANGES))]
    pub allocated: BTreeMap<String, OwnerKey>,
    pub counts: ChangeCounts,
    pub validation: ValidationEvidence,
    pub work: WorkObservation,
    pub budget_work: ChangeBudgetWork,
}

impl AuthoredChangeResponse {
    pub fn prepared(prepared: &PreparedAuthoredPublication) -> Result<Self, Diagnostic> {
        let bases = &prepared.publication.receipt.bases;
        let [base] = bases.as_slice() else {
            return Err(protocol_error(
                DiagnosticClass::Corrupt,
                "change_protocol_prepared_base",
                "prepared authored change does not bind one exact accepted base",
            ));
        };
        Ok(Self {
            contract: AuthoredChangeContract::Current,
            schema_digest: authored_protocol_schema_digest()?,
            status: AuthoredChangeResponseStatus::Prepared,
            base: *base,
            result: prepared.publication.receipt.result,
            transaction: prepared.publication.receipt.transaction,
            semantic_diff: prepared.publication.receipt.semantic_diff,
            allocated: prepared.allocated.clone(),
            counts: prepared.publication.receipt.counts,
            validation: prepared.publication.receipt.validation,
            work: prepared.publication.receipt.work,
            budget_work: prepared.publication.budget_work,
        })
    }

    pub fn encode_json(&self) -> Result<Vec<u8>, Diagnostic> {
        let bytes = serde_json::to_vec(self).map_err(|_| {
            protocol_error(
                DiagnosticClass::Infrastructure,
                "change_protocol_response_encode",
                "Graph 5 authored response could not be encoded",
            )
        })?;
        if bytes.len() > MAXIMUM_AUTHORED_RESPONSE_BYTES {
            return Err(protocol_error(
                DiagnosticClass::Resource,
                "change_protocol_response_bytes",
                "Graph 5 authored response exceeds its current byte bound",
            ));
        }
        Ok(bytes)
    }

    pub fn accepted(
        prepared: &PreparedAuthoredPublication,
        outcome: &PublicationOutcome,
    ) -> Result<Self, Diagnostic> {
        let (status, revision) = match outcome {
            PublicationOutcome::Accepted { current, .. } => (
                AuthoredChangeResponseStatus::Accepted,
                current.head.revision,
            ),
            PublicationOutcome::AlreadyAccepted { current } => (
                AuthoredChangeResponseStatus::AlreadyAccepted,
                current.head.revision,
            ),
            PublicationOutcome::Stale { .. } => {
                return Err(protocol_error(
                    DiagnosticClass::Semantic,
                    "change_protocol_stale_result",
                    "prepared Graph 5 authored change was stale at publication",
                ));
            }
        };
        let mut response = Self::prepared(prepared)?;
        if revision != response.result {
            return Err(protocol_error(
                DiagnosticClass::Corrupt,
                "change_protocol_result_revision",
                "accepted revision disagrees with the prepared Graph 5 response",
            ));
        }
        response.status = status;
        Ok(response)
    }
}

#[derive(JsonSchema)]
#[schemars(rename = "lkjscript.Graph5AuthoredProtocolDocumentV1")]
#[allow(dead_code)]
struct AuthoredProtocolDocument {
    request: AuthoredChangeRequest,
    response: AuthoredChangeResponse,
}

pub fn authored_protocol_schema() -> Result<Value, Diagnostic> {
    let schema = schema_for!(AuthoredProtocolDocument);
    let mut value = serde_json::to_value(schema).map_err(|_| {
        protocol_error(
            DiagnosticClass::Infrastructure,
            "change_protocol_schema_encode",
            "Graph 5 authored schema could not be encoded",
        )
    })?;
    let object = value.as_object_mut().ok_or_else(|| {
        protocol_error(
            DiagnosticClass::Corrupt,
            "change_protocol_schema_shape",
            "derived Graph 5 authored schema is not an object",
        )
    })?;
    object.insert(
        "$id".to_owned(),
        Value::String(AUTHORED_PROTOCOL_SCHEMA_ID.to_owned()),
    );
    object.insert(
        "title".to_owned(),
        Value::String("lkjscript Graph 5 authored change protocol".to_owned()),
    );
    object.insert(
        "x-lkjscript-contract".to_owned(),
        Value::String(AUTHORED_CHANGE_CONTRACT_IDENTITY.to_owned()),
    );
    object.insert(
        "x-lkjscript-availability".to_owned(),
        Value::String("private_cutover_candidate".to_owned()),
    );
    object
        .entry("$defs")
        .or_insert_with(|| Value::Object(Map::new()));
    Ok(value)
}

pub fn authored_protocol_schema_bytes() -> Result<Vec<u8>, Diagnostic> {
    let mut bytes = serde_json::to_vec_pretty(&authored_protocol_schema()?).map_err(|_| {
        protocol_error(
            DiagnosticClass::Infrastructure,
            "change_protocol_schema_bytes",
            "Graph 5 authored schema bytes could not be encoded",
        )
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn authored_protocol_schema_digest() -> Result<String, Diagnostic> {
    let bytes = serde_json::to_vec(&authored_protocol_schema()?).map_err(|_| {
        protocol_error(
            DiagnosticClass::Infrastructure,
            "change_protocol_schema_digest_encode",
            "Graph 5 authored schema could not be canonically encoded",
        )
    })?;
    let mut hasher = blake3::Hasher::new_derive_key(AUTHORED_PROTOCOL_SCHEMA_DIGEST_DOMAIN);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(&bytes);
    Ok(hasher.finalize().to_hex().to_string())
}

fn protocol_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::change::{
        AuthoredChange, AuthoredDeclarationReference, AuthoredExpression,
        AuthoredExpressionOperation, AuthoredFunctionEffect, AuthoredRequirement,
        AuthoredResourceLimit, AuthoredType, ChangeBudget, ModuleSelector,
    };
    use crate::platform::kernel::{
        DeclarationVisibility, ExpressionOperation, Name, PackageId, ResourceUnit,
    };
    use crate::platform::semantic_id::{DeclarationId, ExpressionId};

    fn request() -> AuthoredChangeRequest {
        AuthoredChangeRequest {
            contract: AuthoredChangeContract::Current,
            idempotency_key: Some("schema-roundtrip".to_owned()),
            semantic: AuthoredChangeSet {
                base: RevisionId::from_digest([1; 32]),
                preconditions: Vec::new(),
                changes: vec![AuthoredChange::CreateModule {
                    symbol: "$module".to_owned(),
                    name: Name::new("module").expect("fixture name"),
                }],
                budget: ChangeBudget::default(),
            },
            intent: Some("schema fixture".to_owned()),
        }
    }

    #[test]
    fn authored_request_decoder_is_exact_bounded_and_round_trips() {
        let request = request();
        let bytes = request.encode_json().expect("request JSON");
        assert_eq!(AuthoredChangeRequest::decode_json(&bytes).unwrap(), request);

        let text = String::from_utf8(bytes.clone()).expect("UTF-8 request");
        let duplicate = text.replacen('{', "{\"contract\":\"lkjscript-change-5\",", 1);
        assert_eq!(
            AuthoredChangeRequest::decode_json(duplicate.as_bytes())
                .expect_err("duplicate field must reject")
                .code,
            "change_protocol_request_json"
        );

        let mut unknown: Value = serde_json::from_slice(&bytes).expect("request value");
        unknown
            .as_object_mut()
            .expect("request object")
            .insert("unknown".to_owned(), Value::Bool(true));
        assert_eq!(
            AuthoredChangeRequest::decode_json(&serde_json::to_vec(&unknown).unwrap())
                .expect_err("unknown field must reject")
                .code,
            "change_protocol_request_json"
        );

        let predecessor = text.replace("lkjscript-change-5", "lkjscript-change-3");
        assert_eq!(
            AuthoredChangeRequest::decode_json(predecessor.as_bytes())
                .expect_err("predecessor contract must reject")
                .code,
            "change_protocol_request_json"
        );

        let mut trailing = bytes;
        trailing.extend_from_slice(b" true");
        assert_eq!(
            AuthoredChangeRequest::decode_json(&trailing)
                .expect_err("trailing input must reject")
                .code,
            "change_protocol_request_trailing"
        );
    }

    #[test]
    fn authored_request_decoder_rejects_unknown_fields_in_tagged_empty_forms() {
        let function_request = AuthoredChangeRequest {
            contract: AuthoredChangeContract::Current,
            idempotency_key: None,
            semantic: AuthoredChangeSet {
                base: RevisionId::from_digest([2; 32]),
                preconditions: Vec::new(),
                changes: vec![AuthoredChange::CreateFunction {
                    symbol: "$function".to_owned(),
                    module: ModuleSelector::Symbol {
                        symbol: "$module".to_owned(),
                    },
                    name: Name::new("function").expect("fixture name"),
                    visibility: DeclarationVisibility::Private,
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    result: AuthoredType::Unit {},
                    effect: AuthoredFunctionEffect::Pure {},
                    body: AuthoredExpression {
                        symbol: None,
                        operation: AuthoredExpressionOperation::Unit {},
                    },
                }],
                budget: ChangeBudget::default(),
            },
            intent: None,
        };
        let function_value = serde_json::to_value(function_request).expect("request value");
        for path in [
            &["changes", "0", "result"][..],
            &["changes", "0", "effect"][..],
            &["changes", "0", "body", "operation"][..],
        ] {
            let mut candidate = function_value.clone();
            object_at_path(&mut candidate, path).insert("unexpected".to_owned(), Value::Bool(true));
            assert_eq!(
                AuthoredChangeRequest::decode_json(&serde_json::to_vec(&candidate).unwrap())
                    .expect_err("unknown nested field must reject")
                    .code,
                "change_protocol_request_json"
            );
        }

        let replacement_request = AuthoredChangeRequest {
            contract: AuthoredChangeContract::Current,
            idempotency_key: None,
            semantic: AuthoredChangeSet {
                base: RevisionId::from_digest([3; 32]),
                preconditions: Vec::new(),
                changes: vec![AuthoredChange::ReplaceExpression {
                    expression: ExpressionId::allocate(b"strict-expression", 1),
                    operation: ExpressionOperation::Unit {},
                }],
                budget: ChangeBudget::default(),
            },
            intent: None,
        };
        let mut replacement_value =
            serde_json::to_value(replacement_request).expect("request value");
        object_at_path(&mut replacement_value, &["changes", "0", "operation"])
            .insert("unexpected".to_owned(), Value::Bool(true));
        assert_eq!(
            AuthoredChangeRequest::decode_json(&serde_json::to_vec(&replacement_value).unwrap())
                .expect_err("unknown exact-expression field must reject")
                .code,
            "change_protocol_request_json"
        );
    }

    #[test]
    fn authored_request_decoder_has_explicit_depth_and_unsigned_integer_policy() {
        let unsigned_request = AuthoredChangeRequest {
            contract: AuthoredChangeContract::Current,
            idempotency_key: None,
            semantic: AuthoredChangeSet {
                base: RevisionId::from_digest([4; 32]),
                preconditions: Vec::new(),
                changes: vec![AuthoredChange::CreateComponent {
                    symbol: "$component".to_owned(),
                    module: ModuleSelector::Symbol {
                        symbol: "$module".to_owned(),
                    },
                    name: Name::new("component").expect("fixture name"),
                    visibility: DeclarationVisibility::Private,
                    requirements: vec![AuthoredRequirement {
                        symbol: "$requirement".to_owned(),
                        name: Name::new("storage").expect("fixture name"),
                        interface: AuthoredDeclarationReference::Exact {
                            package: PackageId::migrate(b"protocol-package", 1),
                            declaration: DeclarationId::allocate(b"protocol-interface", 1),
                        },
                        operations: Vec::new(),
                        limits: vec![AuthoredResourceLimit {
                            name: Name::new("bytes").expect("fixture name"),
                            maximum: u64::MAX,
                            unit: ResourceUnit::Bytes,
                        }],
                    }],
                    ports: Vec::new(),
                }],
                budget: ChangeBudget::default(),
            },
            intent: None,
        };
        let unsigned_bytes = unsigned_request.encode_json().expect("request JSON");
        assert_eq!(
            AuthoredChangeRequest::decode_json(&unsigned_bytes).unwrap(),
            unsigned_request
        );

        let mut nested_type = r#"{"kind":"unit"}"#.to_owned();
        for _ in 0..=MAXIMUM_AUTHORED_JSON_DEPTH {
            nested_type = format!(r#"{{"kind":"list","item":{nested_type}}}"#);
        }
        let deep_request = format!(
            r#"{{"contract":"lkjscript-change-5","base":"{}","changes":[{{"op":"create_function","as":"$function","module":{{"by":"symbol","symbol":"$module"}},"name":"function","visibility":"private","result":{nested_type},"effect":{{"kind":"pure"}},"body":{{"operation":{{"kind":"unit"}}}}}}]}}"#,
            RevisionId::from_digest([5; 32])
        );
        assert_eq!(
            AuthoredChangeRequest::decode_json(deep_request.as_bytes())
                .expect_err("excessive request nesting must reject")
                .code,
            "change_protocol_request_json"
        );
    }

    #[test]
    fn authored_schema_is_deterministic_strict_and_derived_from_current_forms() {
        let first = authored_protocol_schema_bytes().expect("schema bytes");
        let second = authored_protocol_schema_bytes().expect("schema bytes");
        assert_eq!(first, second);
        assert!(first.len() < 256 * 1024);
        let schema: Value = serde_json::from_slice(&first).expect("schema JSON");
        assert_eq!(schema["$id"], AUTHORED_PROTOCOL_SCHEMA_ID);
        assert_eq!(
            schema["x-lkjscript-availability"],
            "private_cutover_candidate"
        );
        for definition in schema["$defs"]
            .as_object()
            .expect("schema definitions")
            .keys()
        {
            assert!(
                definition.starts_with("lkjscript."),
                "schema definition leaks an unstable Rust name: {definition}"
            );
        }
        let encoded = String::from_utf8(first).expect("schema UTF-8");
        for operation in [
            "create_module",
            "create_interface",
            "create_external",
            "create_component",
            "replace_expression",
            "add_dependency",
            "replace_dependency",
            "delete_dependency",
        ] {
            assert!(encoded.contains(&format!("\"const\": \"{operation}\"")));
        }
        assert!(encoded.contains("\"additionalProperties\": false"));
        assert_eq!(
            authored_protocol_schema_digest().unwrap(),
            "6846ee62409b56f1c69a54fca33ae2f60923af0eb26d45ea9aed6c385d84563a"
        );
    }

    fn object_at_path<'a>(value: &'a mut Value, path: &[&str]) -> &'a mut Map<String, Value> {
        let mut current = value;
        for segment in path {
            current = if let Ok(index) = segment.parse::<usize>() {
                &mut current.as_array_mut().expect("path array")[index]
            } else {
                current
                    .as_object_mut()
                    .expect("path object")
                    .get_mut(*segment)
                    .expect("path field")
            };
        }
        current.as_object_mut().expect("path object")
    }
}
