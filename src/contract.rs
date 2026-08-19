//! Executable owner for the closed machine contract and its dependency-closed projections.

use crate::machine::{
    BoundaryErrorKind, JSON_ENVELOPE_VERSION, MAX_BOUNDARY_ERROR_MESSAGE_BYTES,
    MAX_JSON_INPUT_BYTES, MAX_JSON_OUTPUT_BYTES,
};
use crate::machine_contract::*;
use crate::protocol::{PROTOCOL_VERSION, RequestCode, ResponseCode};
use crate::query::{
    MAX_BATCH_ITEMS, MAX_BATCH_QUERIES, MAX_CONTEXT_ITEMS, MAX_PAGE_ITEMS, QueryCode,
};
use crate::schema::{NodeKind, OperationCode, SemanticType};
use crate::transaction::{MAX_RETURNED_BINDINGS, TransactionOpCode};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const MACHINE_SCHEMA_IDENTITY: &str = "lkjscript-machine-schema-v12";
const MACHINE_SCHEMA_DIGEST_DOMAIN: &str = "lkjscript.machine-schema.digest.v2";

fn scalar_types() -> Vec<MachineScalarDescription> {
    let boolean = |name: &str| MachineScalarDescription {
        name: name.into(),
        json_kind: JsonScalarKind::Boolean,
        domain: MachineScalarDomain::Boolean,
    };
    let string = |name: &str| MachineScalarDescription {
        name: name.into(),
        json_kind: JsonScalarKind::String,
        domain: MachineScalarDomain::Utf8String,
    };
    let signed = |name: &str, minimum, maximum| MachineScalarDescription {
        name: name.into(),
        json_kind: JsonScalarKind::Number,
        domain: MachineScalarDomain::SignedInteger { minimum, maximum },
    };
    let unsigned = |name: &str, minimum, maximum| MachineScalarDescription {
        name: name.into(),
        json_kind: JsonScalarKind::Number,
        domain: MachineScalarDomain::UnsignedInteger { minimum, maximum },
    };
    let hex = |name: &str, encoded_bytes| MachineScalarDescription {
        name: name.into(),
        json_kind: JsonScalarKind::String,
        domain: MachineScalarDomain::LowercaseHex { encoded_bytes },
    };
    vec![
        boolean("bool"),
        string("string"),
        signed("i64", i64::MIN, i64::MAX),
        MachineScalarDescription {
            name: "bytes_string".into(),
            json_kind: JsonScalarKind::String,
            domain: MachineScalarDomain::CanonicalUrlSafeBase64 {
                padding: false,
                whitespace: false,
                canonical_trailing_bits: true,
                maximum_decoded_bytes: crate::schema::MAXIMUM_BYTE_STRING_BYTES as u64,
                maximum_encoded_bytes: crate::schema::MAXIMUM_BYTE_STRING_ENCODED_BYTES as u64,
            },
        },
        unsigned("u8", u8::MIN.into(), u8::MAX.into()),
        unsigned("u16", u16::MIN.into(), u16::MAX.into()),
        unsigned("u32", u32::MIN.into(), u32::MAX.into()),
        unsigned("u64", u64::MIN, u64::MAX),
        hex("workspace_id", crate::WorkspaceId::BYTE_LEN as u8),
        hex("idempotency_key", 16),
        MachineScalarDescription {
            name: "node_id".into(),
            json_kind: JsonScalarKind::String,
            domain: MachineScalarDomain::NodeId {
                workspace_bytes: crate::WorkspaceId::BYTE_LEN as u8,
                durable_minimum_serial: 1,
                durable_maximum_serial: crate::ids::MAX_DURABLE_NODE_SERIAL,
                function_local_grammar: "WORKSPACE:lFUNCTION.ORDINAL".into(),
                maximum_function_serial: crate::ids::MAX_LOCAL_FUNCTION_SERIAL,
                maximum_local_ordinal: crate::ids::MAX_FUNCTION_LOCAL_ORDINAL,
            },
        },
        hex("snapshot_hash", crate::SnapshotHash::BYTE_LEN as u8),
        hex("change_digest", crate::ChangeDigest::BYTE_LEN as u8),
        hex("machine_schema_digest", MachineSchemaDigest::BYTE_LEN as u8),
        unsigned("revision", 0, u64::MAX),
        unsigned("request_id", 1, u64::MAX),
        unsigned("query_id", 0, u64::MAX),
        MachineScalarDescription {
            name: "draft_symbol".into(),
            json_kind: JsonScalarKind::String,
            domain: MachineScalarDomain::CanonicalIdentifier {
                grammar: "[a-z][a-z0-9_]*".into(),
                minimum_utf8_bytes: 1,
                maximum_utf8_bytes: crate::ids::MAX_DRAFT_SYMBOL_BYTES as u64,
            },
        },
    ]
}

fn machine_field(name: &str, type_expression: &str, required: bool) -> MachineFieldDescription {
    MachineFieldDescription {
        name: name.into(),
        type_expression: if required {
            type_expression.into()
        } else {
            format!("optional<{type_expression}>")
        },
        required,
    }
}
fn unit_payload() -> PayloadShapeDescription {
    PayloadShapeDescription {
        shape: PayloadShapeKind::Unit,
        newtype: None,
        fields: Vec::new(),
    }
}
pub(crate) fn newtype_payload(type_expression: &str) -> PayloadShapeDescription {
    PayloadShapeDescription {
        shape: PayloadShapeKind::Newtype,
        newtype: Some(type_expression.into()),
        fields: Vec::new(),
    }
}
pub(crate) fn record_payload(fields: &[(&str, &str, bool)]) -> PayloadShapeDescription {
    PayloadShapeDescription {
        shape: PayloadShapeKind::Record,
        newtype: None,
        fields: fields
            .iter()
            .map(|(name, ty, required)| machine_field(name, ty, *required))
            .collect(),
    }
}
fn variant_payload(name: &str, payload: PayloadShapeDescription) -> VariantPayloadDescription {
    VariantPayloadDescription {
        name: name.into(),
        payload,
    }
}

fn named_variant(name: &str, variants: Vec<VariantPayloadDescription>) -> NamedVariantDescription {
    NamedVariantDescription {
        name: name.into(),
        tagging: "adjacently_tagged".into(),
        tag_field: Some("kind".into()),
        content_field: Some("data".into()),
        variants,
    }
}

fn external_variant(
    name: &str,
    variants: Vec<VariantPayloadDescription>,
) -> NamedVariantDescription {
    NamedVariantDescription {
        name: name.into(),
        tagging: "externally_tagged".into(),
        tag_field: None,
        content_field: None,
        variants,
    }
}

fn unit_variants(
    name: &str,
    values: impl IntoIterator<Item = (&'static str, u8)>,
) -> NamedVariantDescription {
    NamedVariantDescription {
        name: name.into(),
        tagging: "string_enum".into(),
        tag_field: None,
        content_field: None,
        variants: values
            .into_iter()
            .map(|(variant, _)| variant_payload(variant, unit_payload()))
            .collect(),
    }
}

fn named_record(name: &str, fields: &[(&str, &str, bool)]) -> NamedPayloadDescription {
    NamedPayloadDescription {
        name: name.into(),
        payload: record_payload(fields),
    }
}

fn draft_field(
    name: &str,
    field_type: DraftFieldType,
    required: bool,
    declares_symbol: bool,
) -> DraftFieldDescription {
    DraftFieldDescription {
        name: name.to_owned(),
        field_type,
        required,
        nullable: !required,
        declares_symbol,
    }
}

fn structured_records() -> Vec<DraftRecordDescription> {
    use DraftFieldType as T;
    vec![
        DraftRecordDescription {
            name: "create_product_type".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, true, true),
                draft_field("module", T::NodeTarget, true, false),
                draft_field("name", T::String, true, false),
                draft_field("fields", T::ProductFieldList, true, false),
            ],
        },
        DraftRecordDescription {
            name: "product_field".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, true, true),
                draft_field("name", T::String, true, false),
                draft_field("ty", T::TypeDraft, true, false),
            ],
        },
        DraftRecordDescription {
            name: "create_sum_type".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, true, true),
                draft_field("module", T::NodeTarget, true, false),
                draft_field("name", T::String, true, false),
                draft_field("variants", T::SumVariantList, true, false),
            ],
        },
        DraftRecordDescription {
            name: "create_sequence_type".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, true, true),
                draft_field("module", T::NodeTarget, true, false),
                draft_field("name", T::String, true, false),
                draft_field("element", T::TypeDraft, true, false),
            ],
        },
        DraftRecordDescription {
            name: "sum_variant".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, true, true),
                draft_field("name", T::String, true, false),
                draft_field("payload", T::TypeDraft, false, false),
            ],
        },
        DraftRecordDescription {
            name: "create_function".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, true, true),
                draft_field("module", T::NodeTarget, true, false),
                draft_field("name", T::String, true, false),
                draft_field("parameters", T::ParameterList, true, false),
                draft_field("result", T::TypeDraft, true, false),
                draft_field("body", T::FunctionBody, false, false),
            ],
        },
        DraftRecordDescription {
            name: "function_parameter".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, true, true),
                draft_field("name", T::String, true, false),
                draft_field("ty", T::TypeDraft, true, false),
            ],
        },
        DraftRecordDescription {
            name: "function_body".into(),
            fields: vec![
                draft_field("operations", T::ExpressionList, true, false),
                draft_field("return_value", T::Value, true, false),
            ],
        },
        DraftRecordDescription {
            name: "yielding_body".into(),
            fields: vec![
                draft_field("operations", T::ExpressionList, true, false),
                draft_field("yield_value", T::Value, true, false),
            ],
        },
        DraftRecordDescription {
            name: "product_field_value".into(),
            fields: vec![
                draft_field("field", T::NodeTarget, true, false),
                draft_field("value", T::Value, true, false),
            ],
        },
        DraftRecordDescription {
            name: "operation_match_arm".into(),
            fields: vec![
                draft_field("variant", T::NodeTarget, true, false),
                draft_field("region", T::NodeTarget, true, false),
            ],
        },
        DraftRecordDescription {
            name: "match_arm".into(),
            fields: vec![
                draft_field("variant", T::NodeTarget, true, false),
                draft_field("payload_symbol", T::DraftSymbol, false, true),
                draft_field("body", T::YieldingBody, true, false),
            ],
        },
        DraftRecordDescription {
            name: "expression".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, false, true),
                draft_field("operation", T::ExpressionKind, true, false),
            ],
        },
        DraftRecordDescription {
            name: "define_function_body".into(),
            fields: vec![
                draft_field("function", T::NodeId, true, false),
                draft_field("body", T::FunctionBody, true, false),
            ],
        },
        DraftRecordDescription {
            name: "insert_expression".into(),
            fields: vec![
                draft_field("block", T::NodeId, true, false),
                draft_field("before", T::NodeId, false, false),
                draft_field("expression", T::Expression, true, false),
            ],
        },
    ]
}

fn expression_variant(code: crate::transaction::ExpressionDraftCode) -> DraftVariantDescription {
    use crate::transaction::ExpressionDraftCode as C;
    use DraftFieldType as T;
    let (shape, newtype, fields) = match code {
        C::ConstUnit => (PayloadShapeKind::Unit, None, vec![]),
        C::ConstBool => (PayloadShapeKind::Newtype, Some(T::Bool), vec![]),
        C::ConstI64 => (PayloadShapeKind::Newtype, Some(T::I64), vec![]),
        C::ConstBytes => (PayloadShapeKind::Newtype, Some(T::Bytes), vec![]),
        C::ConstText => (PayloadShapeKind::Newtype, Some(T::String), vec![]),
        C::AddI64 | C::LtI64 | C::EqualI64 | C::AndBool | C::OrBool => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("lhs", T::Value, true, false),
                draft_field("rhs", T::Value, true, false),
            ],
        ),
        C::BytesEqual | C::BytesConcat => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("lhs", T::Value, true, false),
                draft_field("rhs", T::Value, true, false),
            ],
        ),
        C::TextEqual | C::TextConcat => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("lhs", T::Value, true, false),
                draft_field("rhs", T::Value, true, false),
            ],
        ),
        C::NotBool | C::TextLen => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("value", T::Value, true, false)],
        ),
        C::SequenceEmpty => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("sequence", T::NodeTarget, true, false)],
        ),
        C::SequenceLen => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("sequence", T::NodeTarget, true, false),
                draft_field("value", T::Value, true, false),
            ],
        ),
        C::SequenceGet => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("sequence", T::NodeTarget, true, false),
                draft_field("value", T::Value, true, false),
                draft_field("index", T::Value, true, false),
            ],
        ),
        C::SequenceAppend => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("sequence", T::NodeTarget, true, false),
                draft_field("value", T::Value, true, false),
                draft_field("element", T::Value, true, false),
            ],
        ),
        C::SequenceReplace => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("sequence", T::NodeTarget, true, false),
                draft_field("value", T::Value, true, false),
                draft_field("index", T::Value, true, false),
                draft_field("element", T::Value, true, false),
            ],
        ),
        C::BytesLen => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("value", T::Value, true, false)],
        ),
        C::BytesAt => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("value", T::Value, true, false),
                draft_field("index", T::Value, true, false),
            ],
        ),
        C::BytesSlice => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("value", T::Value, true, false),
                draft_field("start", T::Value, true, false),
                draft_field("length", T::Value, true, false),
            ],
        ),
        C::Call => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("function", T::NodeTarget, true, false),
                draft_field("arguments", T::ValueList, true, false),
            ],
        ),
        C::Hole => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("expected", T::TypeDraft, true, false)],
        ),
        C::If => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("condition", T::Value, true, false),
                draft_field("result", T::TypeDraft, true, false),
                draft_field("then_body", T::YieldingBody, true, false),
                draft_field("else_body", T::YieldingBody, true, false),
            ],
        ),
        C::ForI64 => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("start", T::Value, true, false),
                draft_field("end_exclusive", T::Value, true, false),
                draft_field("step", T::I64, true, false),
                draft_field("initial", T::Value, true, false),
                draft_field("carried", T::TypeDraft, true, false),
                draft_field("index_symbol", T::DraftSymbol, true, true),
                draft_field("carried_symbol", T::DraftSymbol, true, true),
                draft_field("body", T::YieldingBody, true, false),
            ],
        ),
        C::ConstructProduct => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("product", T::NodeTarget, true, false),
                draft_field("fields", T::ProductFieldValueList, true, false),
            ],
        ),
        C::ProjectField => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("value", T::Value, true, false),
                draft_field("field", T::NodeTarget, true, false),
            ],
        ),
        C::ConstructVariant => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("variant", T::NodeTarget, true, false),
                draft_field("payload", T::Value, false, false),
            ],
        ),
        C::MatchSum => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("scrutinee", T::Value, true, false),
                draft_field("result", T::TypeDraft, true, false),
                draft_field("arms", T::MatchArmList, true, false),
            ],
        ),
    };
    DraftVariantDescription {
        name: code.machine_name().into(),
        shape,
        newtype,
        fields,
    }
}

fn operation_variant(code: OperationCode) -> DraftVariantDescription {
    use DraftFieldType as T;
    use OperationCode as C;
    let (shape, newtype, fields) = match code {
        C::ConstUnit => (PayloadShapeKind::Unit, None, vec![]),
        C::ConstI64 => (PayloadShapeKind::Newtype, Some(T::I64), vec![]),
        C::ConstBool => (PayloadShapeKind::Newtype, Some(T::Bool), vec![]),
        C::ConstBytes => (PayloadShapeKind::Newtype, Some(T::Bytes), vec![]),
        C::ConstText => (PayloadShapeKind::Newtype, Some(T::String), vec![]),
        C::AddI64 | C::LtI64 | C::EqualI64 | C::AndBool | C::OrBool => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("lhs", T::Value, true, false),
                draft_field("rhs", T::Value, true, false),
            ],
        ),
        C::BytesEqual | C::BytesConcat => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("lhs", T::Value, true, false),
                draft_field("rhs", T::Value, true, false),
            ],
        ),
        C::TextEqual | C::TextConcat => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("lhs", T::Value, true, false),
                draft_field("rhs", T::Value, true, false),
            ],
        ),
        C::NotBool | C::TextLen => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("value", T::Value, true, false)],
        ),
        C::SequenceEmpty => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("sequence", T::NodeTarget, true, false)],
        ),
        C::SequenceLen => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("sequence", T::NodeTarget, true, false),
                draft_field("value", T::Value, true, false),
            ],
        ),
        C::SequenceGet => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("sequence", T::NodeTarget, true, false),
                draft_field("value", T::Value, true, false),
                draft_field("index", T::Value, true, false),
            ],
        ),
        C::SequenceAppend => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("sequence", T::NodeTarget, true, false),
                draft_field("value", T::Value, true, false),
                draft_field("element", T::Value, true, false),
            ],
        ),
        C::SequenceReplace => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("sequence", T::NodeTarget, true, false),
                draft_field("value", T::Value, true, false),
                draft_field("index", T::Value, true, false),
                draft_field("element", T::Value, true, false),
            ],
        ),
        C::BytesLen => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("value", T::Value, true, false)],
        ),
        C::BytesAt => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("value", T::Value, true, false),
                draft_field("index", T::Value, true, false),
            ],
        ),
        C::BytesSlice => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("value", T::Value, true, false),
                draft_field("start", T::Value, true, false),
                draft_field("length", T::Value, true, false),
            ],
        ),
        C::Call => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("function", T::NodeTarget, true, false),
                draft_field("arguments", T::ValueList, true, false),
            ],
        ),
        C::Hole => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("expected", T::TypeDraft, true, false)],
        ),
        C::If => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("condition", T::Value, true, false),
                draft_field("result", T::TypeDraft, true, false),
                draft_field("then_region", T::NodeTarget, true, false),
                draft_field("else_region", T::NodeTarget, true, false),
            ],
        ),
        C::ForI64 => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("start", T::Value, true, false),
                draft_field("end_exclusive", T::Value, true, false),
                draft_field("step", T::I64, true, false),
                draft_field("initial", T::Value, true, false),
                draft_field("carried", T::TypeDraft, true, false),
                draft_field("body_region", T::NodeTarget, true, false),
            ],
        ),
        C::Return | C::Yield => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("value", T::Value, true, false)],
        ),
        C::ConstructProduct => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("product", T::NodeTarget, true, false),
                draft_field("fields", T::ProductFieldValueList, true, false),
            ],
        ),
        C::ProjectField => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("value", T::Value, true, false),
                draft_field("field", T::NodeTarget, true, false),
            ],
        ),
        C::ConstructVariant => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("variant", T::NodeTarget, true, false),
                draft_field("payload", T::Value, false, false),
            ],
        ),
        C::MatchSum => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("scrutinee", T::Value, true, false),
                draft_field("result", T::TypeDraft, true, false),
                draft_field("arms", T::OperationMatchArmList, true, false),
            ],
        ),
    };
    DraftVariantDescription {
        name: code.machine_name().into(),
        shape,
        newtype,
        fields,
    }
}

fn type_variants() -> Vec<DraftVariantDescription> {
    use DraftFieldType as T;
    vec![
        DraftVariantDescription {
            name: "unit".into(),
            shape: PayloadShapeKind::Unit,
            newtype: None,
            fields: Vec::new(),
        },
        DraftVariantDescription {
            name: "bool".into(),
            shape: PayloadShapeKind::Unit,
            newtype: None,
            fields: Vec::new(),
        },
        DraftVariantDescription {
            name: "i64".into(),
            shape: PayloadShapeKind::Unit,
            newtype: None,
            fields: Vec::new(),
        },
        DraftVariantDescription {
            name: "bytes".into(),
            shape: PayloadShapeKind::Unit,
            newtype: None,
            fields: Vec::new(),
        },
        DraftVariantDescription {
            name: "text".into(),
            shape: PayloadShapeKind::Unit,
            newtype: None,
            fields: Vec::new(),
        },
        DraftVariantDescription {
            name: "nominal".into(),
            shape: PayloadShapeKind::Newtype,
            newtype: Some(T::NodeTarget),
            fields: Vec::new(),
        },
    ]
}

fn value_variant(code: crate::transaction::ValueDraftCode) -> DraftVariantDescription {
    use crate::transaction::ValueDraftCode as C;
    use DraftFieldType as T;
    let (shape, newtype, fields) = match code {
        C::FunctionParameter | C::BlockArgument => {
            (PayloadShapeKind::Newtype, Some(T::NodeTarget), Vec::new())
        }
        C::OperationResult => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("operation", T::NodeTarget, true, false),
                draft_field("output", T::U8, true, false),
            ],
        ),
        C::InlineExpression => (
            PayloadShapeKind::Newtype,
            Some(T::ExpressionKind),
            Vec::new(),
        ),
    };
    DraftVariantDescription {
        name: code.machine_name().into(),
        shape,
        newtype,
        fields,
    }
}

fn semantic_records() -> Vec<NamedPayloadDescription> {
    vec![
        named_record(
            "canonical_product_field_value",
            &[("field", "node_id", true), ("value", "value_ref", true)],
        ),
        named_record(
            "canonical_match_arm",
            &[("variant", "node_id", true), ("region", "node_id", true)],
        ),
    ]
}

fn semantic_variants() -> Vec<NamedVariantDescription> {
    vec![
        external_variant(
            "semantic_type",
            vec![
                variant_payload("unit", unit_payload()),
                variant_payload("bool", unit_payload()),
                variant_payload("i64", unit_payload()),
                variant_payload("bytes", unit_payload()),
                variant_payload("text", unit_payload()),
                variant_payload("nominal", newtype_payload("node_id")),
            ],
        ),
        named_variant(
            "value_ref",
            vec![
                variant_payload("function_parameter", newtype_payload("node_id")),
                variant_payload("block_argument", newtype_payload("node_id")),
                variant_payload(
                    "operation_result",
                    record_payload(&[("operation", "node_id", true), ("output", "u8", true)]),
                ),
            ],
        ),
        external_variant(
            "region_role",
            vec![
                variant_payload("if_then", unit_payload()),
                variant_payload("if_else", unit_payload()),
                variant_payload("for_body", unit_payload()),
                variant_payload("match_arm", newtype_payload("node_id")),
            ],
        ),
        named_variant(
            "operation_kind",
            vec![
                variant_payload("const_unit", unit_payload()),
                variant_payload("const_i64", newtype_payload("i64")),
                variant_payload("const_bool", newtype_payload("bool")),
                variant_payload(
                    "add_i64",
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
                variant_payload(
                    "lt_i64",
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
                variant_payload(
                    "call",
                    record_payload(&[
                        ("function", "node_id", true),
                        ("arguments", "list<value_ref>", true),
                    ]),
                ),
                variant_payload(
                    "hole",
                    record_payload(&[("expected", "semantic_type", true)]),
                ),
                variant_payload(
                    "if",
                    record_payload(&[
                        ("condition", "value_ref", true),
                        ("result", "semantic_type", true),
                        ("then_region", "node_id", true),
                        ("else_region", "node_id", true),
                    ]),
                ),
                variant_payload(
                    "for_i64",
                    record_payload(&[
                        ("start", "value_ref", true),
                        ("end_exclusive", "value_ref", true),
                        ("step", "i64", true),
                        ("initial", "value_ref", true),
                        ("carried", "semantic_type", true),
                        ("body_region", "node_id", true),
                    ]),
                ),
                variant_payload("return", record_payload(&[("value", "value_ref", true)])),
                variant_payload("yield", record_payload(&[("value", "value_ref", true)])),
                variant_payload(
                    "construct_product",
                    record_payload(&[
                        ("product", "node_id", true),
                        ("fields", "list<canonical_product_field_value>", true),
                    ]),
                ),
                variant_payload(
                    "project_field",
                    record_payload(&[("value", "value_ref", true), ("field", "node_id", true)]),
                ),
                variant_payload(
                    "construct_variant",
                    record_payload(&[
                        ("variant", "node_id", true),
                        ("payload", "value_ref", false),
                    ]),
                ),
                variant_payload(
                    "match_sum",
                    record_payload(&[
                        ("scrutinee", "value_ref", true),
                        ("result", "semantic_type", true),
                        ("arms", "list<canonical_match_arm>", true),
                    ]),
                ),
                variant_payload("const_bytes", newtype_payload("bytes_string")),
                variant_payload("bytes_len", record_payload(&[("value", "value_ref", true)])),
                variant_payload(
                    "bytes_at",
                    record_payload(&[("value", "value_ref", true), ("index", "value_ref", true)]),
                ),
                variant_payload(
                    "bytes_slice",
                    record_payload(&[
                        ("value", "value_ref", true),
                        ("start", "value_ref", true),
                        ("length", "value_ref", true),
                    ]),
                ),
                variant_payload(
                    "bytes_equal",
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
                variant_payload(
                    "bytes_concat",
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
                variant_payload(
                    "equal_i64",
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
                variant_payload("not_bool", record_payload(&[("value", "value_ref", true)])),
                variant_payload(
                    "and_bool",
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
                variant_payload(
                    "or_bool",
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
                variant_payload("const_text", newtype_payload("string")),
                variant_payload("text_len", record_payload(&[("value", "value_ref", true)])),
                variant_payload(
                    "text_equal",
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
                variant_payload(
                    "text_concat",
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
                variant_payload(
                    "sequence_empty",
                    record_payload(&[("sequence", "node_id", true)]),
                ),
                variant_payload(
                    "sequence_len",
                    record_payload(&[("sequence", "node_id", true), ("value", "value_ref", true)]),
                ),
                variant_payload(
                    "sequence_get",
                    record_payload(&[
                        ("sequence", "node_id", true),
                        ("value", "value_ref", true),
                        ("index", "value_ref", true),
                    ]),
                ),
                variant_payload(
                    "sequence_append",
                    record_payload(&[
                        ("sequence", "node_id", true),
                        ("value", "value_ref", true),
                        ("element", "value_ref", true),
                    ]),
                ),
                variant_payload(
                    "sequence_replace",
                    record_payload(&[
                        ("sequence", "node_id", true),
                        ("value", "value_ref", true),
                        ("index", "value_ref", true),
                        ("element", "value_ref", true),
                    ]),
                ),
            ],
        ),
        named_variant(
            "node",
            vec![
                variant_payload(
                    "workspace_root",
                    record_payload(&[
                        ("packages", "list<node_id>", true),
                        ("targets", "list<node_id>", true),
                    ]),
                ),
                variant_payload(
                    "package",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("modules", "list<node_id>", true),
                        ("entry", "node_id", false),
                    ]),
                ),
                variant_payload(
                    "module",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("types", "list<node_id>", true),
                        ("functions", "list<node_id>", true),
                    ]),
                ),
                variant_payload(
                    "product_type",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("fields", "list<node_id>", true),
                    ]),
                ),
                variant_payload(
                    "product_field",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("ordinal", "u32", true),
                        ("name", "string", true),
                        ("ty", "semantic_type", true),
                    ]),
                ),
                variant_payload(
                    "sum_type",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("variants", "list<node_id>", true),
                    ]),
                ),
                variant_payload(
                    "sum_variant",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("ordinal", "u32", true),
                        ("name", "string", true),
                        ("payload", "semantic_type", false),
                    ]),
                ),
                variant_payload(
                    "sequence_type",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("element", "semantic_type", true),
                    ]),
                ),
                variant_payload(
                    "function",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("parameters", "list<node_id>", true),
                        ("result", "semantic_type", true),
                        ("body", "node_id", false),
                    ]),
                ),
                variant_payload(
                    "parameter",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("ordinal", "u32", true),
                        ("name", "string", true),
                        ("ty", "semantic_type", true),
                    ]),
                ),
                variant_payload(
                    "region",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("blocks", "list<node_id>", true),
                    ]),
                ),
                variant_payload(
                    "block",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("arguments", "list<node_id>", true),
                        ("operations", "list<node_id>", true),
                        ("terminator", "node_id", false),
                    ]),
                ),
                variant_payload(
                    "block_argument",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("ordinal", "u32", true),
                        ("ty", "semantic_type", true),
                    ]),
                ),
                variant_payload(
                    "operation",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("operation", "operation_kind", true),
                    ]),
                ),
                variant_payload(
                    "build_target",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("definition", "build_target_definition", true),
                    ]),
                ),
            ],
        ),
    ]
}

fn transaction_records() -> Vec<NamedPayloadDescription> {
    vec![
        named_record(
            "apply_transaction_request",
            &[
                ("transaction", "transaction", true),
                ("response", "transaction_response_spec", true),
            ],
        ),
        named_record(
            "transaction",
            &[
                ("workspace", "workspace_id", true),
                ("base_revision", "revision", true),
                ("idempotency_key", "idempotency_key", false),
                ("mode", "transaction_mode", true),
                ("operations", "list<transaction_operation>", true),
            ],
        ),
        named_record(
            "transaction_response_spec",
            &[("return_symbols", "list<draft_symbol>", true)],
        ),
        named_record(
            "transaction_receipt",
            &[
                ("workspace", "workspace_id", true),
                ("base_revision", "revision", true),
                ("revision", "revision", true),
                ("hash", "snapshot_hash", true),
                ("published", "bool", true),
                ("created_count", "u64", true),
                (
                    "returned_bindings",
                    "list<tuple<draft_symbol,node_id>>",
                    true,
                ),
                ("change_count", "u64", true),
                ("change_digest", "change_digest", true),
                ("complete_before", "bool", true),
                ("complete_after", "bool", true),
                ("blocker_count_before", "u64", true),
                ("blocker_count_after", "u64", true),
            ],
        ),
        named_record(
            "target_release_dependency",
            &[("slot", "string", true), ("target", "node_id", true)],
        ),
        named_record(
            "release_export_request",
            &[("name", "string", true), ("target", "node_id", true)],
        ),
        named_record(
            "release_import_request",
            &[
                ("local", "node_id", true),
                ("dependency_slot", "string", true),
                ("export", "string", true),
            ],
        ),
        named_record(
            "release_trap",
            &[
                ("code", "release_trap_code", true),
                ("target", "node_id", false),
            ],
        ),
        named_record(
            "release_test_case",
            &[
                ("name", "string", true),
                ("target", "node_id", true),
                ("arguments", "list<runtime_value>", true),
                ("expected", "release_test_expectation", true),
                ("policy", "run_policy", true),
            ],
        ),
        named_record(
            "release_target_definition",
            &[
                ("root", "node_id", true),
                ("coordinate", "string", true),
                ("user_version", "string", true),
                ("exports", "list<release_export_request>", true),
                ("dependencies", "list<target_release_dependency>", true),
                ("imports", "list<release_import_request>", true),
                ("tests", "list<release_test_case>", true),
            ],
        ),
        named_record(
            "target_item",
            &[
                ("release_target", "node_id", true),
                ("item", "node_id", true),
            ],
        ),
        named_record(
            "target_field_value",
            &[
                ("field", "target_item", true),
                ("value", "target_value", true),
            ],
        ),
        named_record(
            "target_host_request_route",
            &[
                ("variant", "target_item", true),
                ("operation", "host_operation", true),
            ],
        ),
        named_record(
            "target_host_outcome_route",
            &[
                ("operation", "host_operation", true),
                ("class", "host_outcome_class", true),
                ("variant", "target_item", true),
            ],
        ),
        named_record(
            "target_application_import",
            &[
                ("slot", "string", true),
                ("interface", "host_interface", true),
                ("request", "target_item", true),
                ("outcome", "target_item", true),
                ("command_variant", "target_item", true),
                ("outcome_variant", "target_item", true),
                ("requests", "list<target_host_request_route>", true),
                ("outcomes", "list<target_host_outcome_route>", true),
            ],
        ),
        named_record(
            "target_stateful_application_profile",
            &[
                ("resume", "target_item", true),
                ("query_entry", "target_item", true),
                ("state", "target_item", true),
                ("event", "target_item", true),
                ("response", "target_item", true),
                ("query", "target_item", true),
                ("query_result", "target_item", true),
                ("command", "target_item", true),
                ("outcome", "target_item", true),
                ("decision", "target_item", true),
                ("declined_variant", "target_item", true),
                ("declined_payload", "target_item", true),
                ("declined_response_field", "target_item", true),
                ("unchanged_variant", "target_item", true),
                ("unchanged_payload", "target_item", true),
                ("unchanged_response_field", "target_item", true),
                ("completed_variant", "target_item", true),
                ("completed_payload", "target_item", true),
                ("completed_state_field", "target_item", true),
                ("completed_response_field", "target_item", true),
                ("suspended_variant", "target_item", true),
                ("suspended_payload", "target_item", true),
                ("suspended_state_field", "target_item", true),
                ("suspended_response_field", "target_item", true),
                ("suspended_command_field", "target_item", true),
                ("imports", "list<target_application_import>", true),
            ],
        ),
        named_record(
            "target_trap",
            &[
                ("code", "application_trap_code", true),
                ("target", "target_item", false),
            ],
        ),
        named_record(
            "target_application_test_case",
            &[
                ("name", "string", true),
                ("target", "target_item", true),
                ("arguments", "list<target_value>", true),
                ("expected", "target_test_expectation", true),
                ("policy", "run_policy", true),
            ],
        ),
        named_record(
            "application_target_definition",
            &[
                ("root_release", "node_id", true),
                ("entry", "target_item", true),
                ("profile", "target_invocation_profile", true),
                ("policy", "run_policy", true),
                ("tests", "list<target_application_test_case>", true),
            ],
        ),
        named_record(
            "product_target_definition",
            &[("application", "node_id", true)],
        ),
    ]
}

fn transaction_variants() -> Vec<NamedVariantDescription> {
    vec![
        named_variant(
            "transaction_operation",
            TransactionOpCode::ALL
                .into_iter()
                .map(transaction_payload)
                .collect(),
        ),
        named_variant(
            "node_target",
            vec![
                variant_payload("existing", newtype_payload("node_id")),
                variant_payload("draft", newtype_payload("draft_symbol")),
            ],
        ),
        named_variant(
            "build_target_definition",
            vec![
                variant_payload("release", newtype_payload("release_target_definition")),
                variant_payload(
                    "application",
                    newtype_payload("application_target_definition"),
                ),
                variant_payload("product", newtype_payload("product_target_definition")),
            ],
        ),
        named_variant(
            "release_test_expectation",
            vec![
                variant_payload("value", newtype_payload("runtime_value")),
                variant_payload("trap", newtype_payload("release_trap")),
            ],
        ),
        unit_variants(
            "release_trap_code",
            [
                ("runtime_trap", 1),
                ("byte_index_out_of_bounds", 2),
                ("byte_slice_out_of_bounds", 3),
            ],
        ),
        named_variant(
            "target_value",
            vec![
                variant_payload("unit", unit_payload()),
                variant_payload("bool", newtype_payload("bool")),
                variant_payload("i64", newtype_payload("i64")),
                variant_payload("bytes", newtype_payload("bytes_string")),
                variant_payload("text", newtype_payload("string")),
                variant_payload(
                    "product",
                    record_payload(&[
                        ("ty", "target_item", true),
                        ("fields", "list<target_field_value>", true),
                    ]),
                ),
                variant_payload(
                    "sum",
                    record_payload(&[
                        ("ty", "target_item", true),
                        ("variant", "target_item", true),
                        ("payload", "target_value", false),
                    ]),
                ),
                variant_payload(
                    "sequence",
                    record_payload(&[
                        ("ty", "target_item", true),
                        ("elements", "list<target_value>", true),
                    ]),
                ),
            ],
        ),
        named_variant(
            "target_invocation_profile",
            vec![
                variant_payload("typed", unit_payload()),
                variant_payload("bytes_stream", unit_payload()),
                variant_payload(
                    "stateful",
                    newtype_payload("target_stateful_application_profile"),
                ),
            ],
        ),
        named_variant(
            "target_test_expectation",
            vec![
                variant_payload("value", newtype_payload("target_value")),
                variant_payload("trap", newtype_payload("target_trap")),
            ],
        ),
        unit_variants("host_interface", [("immutable_blob", 1)]),
        unit_variants("host_operation", [("put_blob", 1), ("inspect_blob", 2)]),
        unit_variants(
            "host_outcome_class",
            [
                ("succeeded", 1),
                ("already_present", 2),
                ("known_failure_before_visibility", 3),
                ("outcome_unknown", 4),
                ("reconciliation_present", 5),
                ("reconciliation_absent", 6),
                ("reconciliation_indeterminate", 7),
                ("cancelled_before_action", 8),
                ("timeout_before_action", 9),
                ("timeout_after_possible_visibility", 10),
                ("cleanup_failure", 11),
            ],
        ),
        unit_variants(
            "application_trap_code",
            [
                ("runtime_trap", 1),
                ("byte_index_out_of_bounds", 2),
                ("byte_slice_out_of_bounds", 3),
            ],
        ),
        unit_variants("transaction_mode", [("commit", 1), ("validate_only", 2)]),
    ]
}

fn run_records() -> Vec<NamedPayloadDescription> {
    vec![
        named_record(
            "run_policy",
            &[("fuel", "u64", true), ("maximum_frames", "u32", true)],
        ),
        named_record(
            "run_result",
            &[
                ("value", "runtime_value", true),
                ("compile_nanoseconds", "u64", true),
                ("execute_nanoseconds", "u64", true),
            ],
        ),
        named_record(
            "runtime_field_value",
            &[("field", "node_id", true), ("value", "runtime_value", true)],
        ),
        named_record(
            "runtime_product_data",
            &[
                ("ty", "node_id", true),
                ("fields", "list<runtime_field_value>", true),
            ],
        ),
        named_record(
            "runtime_sum_data",
            &[
                ("ty", "node_id", true),
                ("variant", "node_id", true),
                ("payload", "runtime_value", false),
            ],
        ),
        named_record(
            "runtime_sequence_data",
            &[
                ("ty", "node_id", true),
                ("elements", "list<runtime_value>", true),
            ],
        ),
    ]
}

fn run_variants() -> Vec<NamedVariantDescription> {
    vec![named_variant(
        "runtime_value",
        vec![
            variant_payload("unit", unit_payload()),
            variant_payload("bool", newtype_payload("bool")),
            variant_payload("i64", newtype_payload("i64")),
            variant_payload("bytes", newtype_payload("bytes_string")),
            variant_payload("text", newtype_payload("string")),
            variant_payload("product", newtype_payload("runtime_product_data")),
            variant_payload("sum", newtype_payload("runtime_sum_data")),
            variant_payload("sequence", newtype_payload("runtime_sequence_data")),
        ],
    )]
}

fn error_records() -> Vec<NamedPayloadDescription> {
    vec![named_record(
        "boundary_error",
        &[
            ("kind", "boundary_error_kind", true),
            ("message", "string", true),
        ],
    )]
}

fn error_variants() -> Vec<NamedVariantDescription> {
    vec![unit_variants(
        "boundary_error_kind",
        [
            ("invalid_json", 1),
            ("input_too_large", 2),
            ("transport", 3),
            ("output", 4),
            ("usage", 5),
        ],
    )]
}

fn identity_variants() -> Vec<NamedVariantDescription> {
    vec![
        named_variant(
            "request",
            RequestCode::ALL.into_iter().map(request_payload).collect(),
        ),
        named_variant(
            "response",
            ResponseCode::ALL
                .into_iter()
                .map(response_payload)
                .collect(),
        ),
    ]
}

fn request_payload(code: RequestCode) -> VariantPayloadDescription {
    let payload = match code {
        RequestCode::CreateWorkspace | RequestCode::Shutdown => unit_payload(),
        RequestCode::DescribeSchema => newtype_payload("describe_schema_request"),
        RequestCode::ApplyTransaction => newtype_payload("apply_transaction_request"),
        RequestCode::QueryBatch => newtype_payload("query_batch_request"),
        RequestCode::Run => record_payload(&[
            ("workspace", "workspace_id", true),
            ("revision", "revision", true),
            ("entry", "node_id", true),
            ("arguments", "list<runtime_value>", true),
            ("policy", "run_policy", true),
        ]),
    };
    variant_payload(code.machine_name(), payload)
}

fn response_payload(code: ResponseCode) -> VariantPayloadDescription {
    let payload = match code {
        ResponseCode::WorkspaceCreated => newtype_payload("workspace_summary"),
        ResponseCode::TransactionReceipt => newtype_payload("transaction_receipt"),
        ResponseCode::QueryBatchResult => newtype_payload("query_batch_result"),
        ResponseCode::Run => newtype_payload("run_result"),
        ResponseCode::Acknowledged => unit_payload(),
        ResponseCode::Error => newtype_payload("error"),
        ResponseCode::DescribeSchema => newtype_payload("describe_schema_result"),
    };
    variant_payload(code.machine_name(), payload)
}

fn transaction_payload(code: TransactionOpCode) -> VariantPayloadDescription {
    let payload = match code {
        TransactionOpCode::CreatePackage => {
            record_payload(&[("symbol", "draft_symbol", true), ("name", "string", true)])
        }
        TransactionOpCode::CreateBuildTarget => record_payload(&[
            ("symbol", "draft_symbol", true),
            ("name", "string", true),
            ("definition", "build_target_definition", true),
        ]),
        TransactionOpCode::ReplaceBuildTarget => record_payload(&[
            ("target", "node_id", true),
            ("definition", "build_target_definition", true),
        ]),
        TransactionOpCode::AddReleaseTargetExport => record_payload(&[
            ("target", "node_id", true),
            ("name", "string", true),
            ("item", "node_id", true),
        ]),
        TransactionOpCode::SetReleaseTargetExport => record_payload(&[
            ("target", "node_id", true),
            ("name", "string", true),
            ("item", "node_id", true),
        ]),
        TransactionOpCode::SetApplicationQueryBoundary => record_payload(&[
            ("target", "node_id", true),
            ("query_entry", "target_item", true),
            ("query", "target_item", true),
        ]),
        TransactionOpCode::AddApplicationTargetTest => record_payload(&[
            ("target", "node_id", true),
            ("case", "target_application_test_case", true),
        ]),
        TransactionOpCode::CreateModule => record_payload(&[
            ("symbol", "draft_symbol", true),
            ("package", "node_target", true),
            ("name", "string", true),
        ]),
        TransactionOpCode::CreateProductType => record_payload(&[
            ("symbol", "draft_symbol", true),
            ("module", "node_target", true),
            ("name", "string", true),
            ("fields", "list<product_field>", true),
        ]),
        TransactionOpCode::CreateSumType => record_payload(&[
            ("symbol", "draft_symbol", true),
            ("module", "node_target", true),
            ("name", "string", true),
            ("variants", "list<sum_variant>", true),
        ]),
        TransactionOpCode::CreateSequenceType => record_payload(&[
            ("symbol", "draft_symbol", true),
            ("module", "node_target", true),
            ("name", "string", true),
            ("element", "type_draft", true),
        ]),
        TransactionOpCode::CreateFunction => record_payload(&[
            ("symbol", "draft_symbol", true),
            ("module", "node_target", true),
            ("name", "string", true),
            ("parameters", "list<function_parameter>", true),
            ("result", "type_draft", true),
            ("body", "function_body", false),
        ]),
        TransactionOpCode::DefineFunctionBody => record_payload(&[
            ("function", "node_id", true),
            ("body", "function_body", true),
        ]),
        TransactionOpCode::ReplaceFunctionBody => record_payload(&[
            ("function", "node_id", true),
            ("body", "function_body", true),
        ]),
        TransactionOpCode::InsertExpression => record_payload(&[
            ("block", "node_id", true),
            ("before", "node_id", false),
            ("expression", "expression", true),
        ]),
        TransactionOpCode::SetEntryFunction => record_payload(&[
            ("package", "node_target", true),
            ("function", "node_target", true),
        ]),
        TransactionOpCode::RenameNode => {
            record_payload(&[("node", "node_target", true), ("name", "string", true)])
        }
        TransactionOpCode::ReplaceOperation => record_payload(&[
            ("operation", "node_target", true),
            ("replacement", "operation_draft", true),
        ]),
        TransactionOpCode::ReplaceOperand => record_payload(&[
            ("operation", "node_target", true),
            ("index", "u64", true),
            ("value", "value_draft", true),
        ]),
        TransactionOpCode::DeleteOwnedSubtree => record_payload(&[("root", "node_target", true)]),
        TransactionOpCode::RefineHole => record_payload(&[
            ("hole", "node_target", true),
            ("replacement", "operation_draft", true),
        ]),
    };
    variant_payload(code.machine_name(), payload)
}

fn query_payload(code: QueryCode) -> VariantPayloadDescription {
    let payload = match code {
        QueryCode::WorkspaceSummary => unit_payload(),
        QueryCode::Node => record_payload(&[("node", "node_id", true), ("expand", "bool", true)]),
        QueryCode::Blockers => record_payload(&[("page", "page_request", true)]),
        QueryCode::OwnerChain => {
            record_payload(&[("node", "node_id", true), ("page", "page_request", true)])
        }
        QueryCode::Body => {
            record_payload(&[("block", "node_id", true), ("page", "page_request", true)])
        }
        QueryCode::IncomingUses => {
            record_payload(&[("value", "value_ref", true), ("page", "page_request", true)])
        }
        QueryCode::DefinitionReferences => {
            record_payload(&[("target", "node_id", true), ("page", "page_request", true)])
        }
        QueryCode::Dependencies => {
            record_payload(&[("node", "node_id", true), ("page", "page_request", true)])
        }
        QueryCode::VisibleValues => record_payload(&[
            ("purpose", "visible_cursor_purpose", true),
            ("target", "repair_target", true),
            ("include_incompatible", "bool", true),
            ("page", "page_request", true),
        ]),
        QueryCode::LegalConstructors => record_payload(&[
            ("target", "repair_target", true),
            ("include_incompatible", "bool", true),
            ("constructors", "page_request", true),
            ("values", "page_request", true),
        ]),
        QueryCode::SemanticDiff => {
            record_payload(&[("from", "revision", true), ("page", "page_request", true)])
        }
        QueryCode::RepairContext => record_payload(&[
            ("target", "repair_target", true),
            ("budget", "context_budget", true),
        ]),
        QueryCode::NominalType => record_payload(&[
            ("declaration", "node_id", true),
            ("page", "page_request", true),
        ]),
    };
    variant_payload(code.machine_name(), payload)
}

fn query_result_payload(code: QueryCode) -> VariantPayloadDescription {
    let ty = match code {
        QueryCode::WorkspaceSummary => "workspace_summary",
        QueryCode::Node => "node_view",
        QueryCode::Blockers => "page<completeness_blocker>",
        QueryCode::OwnerChain => "page<owner_fact>",
        QueryCode::Body => "page<body_item>",
        QueryCode::IncomingUses => "page<use_site>",
        QueryCode::DefinitionReferences => "page<definition_reference_site>",
        QueryCode::Dependencies => "page<dependency_fact>",
        QueryCode::VisibleValues => "page<visible_value>",
        QueryCode::LegalConstructors => "legal_constructors_result",
        QueryCode::SemanticDiff => "semantic_diff_page",
        QueryCode::RepairContext => "repair_context",
        QueryCode::NominalType => "nominal_type_result",
    };
    variant_payload(code.machine_name(), newtype_payload(ty))
}

fn query_records() -> Vec<NamedPayloadDescription> {
    vec![
        named_record(
            "query_batch_request",
            &[
                ("workspace", "workspace_id", true),
                ("revision", "revision", true),
                ("queries", "list<query_item>", true),
            ],
        ),
        named_record(
            "query_item",
            &[("id", "query_id", true), ("query", "query", true)],
        ),
        named_record(
            "query_batch_result",
            &[
                ("workspace", "workspace_id", true),
                ("revision", "revision", true),
                ("results", "list<query_item_result>", true),
            ],
        ),
        named_record(
            "query_item_result",
            &[("id", "query_id", true), ("outcome", "query_outcome", true)],
        ),
        named_record(
            "workspace_summary",
            &[
                ("workspace", "workspace_id", true),
                ("revision", "revision", true),
                ("hash", "snapshot_hash", true),
                ("root", "node_id", true),
                ("node_count", "u64", true),
                ("durable_identity_count", "u64", true),
                ("function_local_reference_count", "u64", true),
                ("anchor_count", "u64", true),
                ("tombstone_count", "u64", true),
                ("complete", "bool", true),
                ("blocker_count", "u64", true),
                ("entry_count", "u64", true),
            ],
        ),
        named_record(
            "function_signature_summary",
            &[
                ("parameter_count", "u64", true),
                ("result", "semantic_type", true),
            ],
        ),
        named_record(
            "name_preview",
            &[("value", "string", true), ("truncated", "bool", true)],
        ),
        named_record(
            "node_summary",
            &[
                ("workspace", "workspace_id", true),
                ("revision", "revision", true),
                ("node", "node_id", true),
                ("identity_class", "node_identity_class", true),
                ("kind", "node_kind", true),
                ("owner", "node_id", false),
                ("display_name", "name_preview", false),
                ("signature", "function_signature_summary", false),
                ("value_type", "semantic_type", false),
                ("complete", "bool", true),
                ("blocker_count", "u64", true),
                ("child_count", "u64", true),
                ("outgoing_reference_count", "u64", true),
            ],
        ),
        named_record(
            "node_view",
            &[("summary", "node_summary", true), ("record", "node", false)],
        ),
        named_record(
            "completeness_blocker",
            &[
                ("owner", "node_id", true),
                ("target", "node_id", false),
                ("category", "expected_category", true),
                ("expected_type", "semantic_type", false),
            ],
        ),
        named_record(
            "owner_fact",
            &[
                ("node", "node_id", true),
                ("kind", "node_kind", true),
                ("name", "name_preview", false),
            ],
        ),
        named_record(
            "owned_region_summary",
            &[("region", "node_id", true), ("role", "region_role", true)],
        ),
        named_record(
            "body_item",
            &[
                ("operation", "node_id", true),
                ("ordinal", "u64", true),
                ("code", "operation_code", true),
                ("result_types", "list<semantic_type>", true),
                ("operands", "list<value_ref>", true),
                ("definitions", "list<definition_reference_site>", true),
                ("complete", "bool", true),
                ("terminator", "bool", true),
                ("literal", "literal_value", false),
                ("owned_regions", "list<owned_region_summary>", true),
            ],
        ),
        named_record(
            "use_site",
            &[
                ("source", "node_id", true),
                ("operand_index", "u64", true),
                ("target", "value_ref", true),
                ("owner_block", "node_id", true),
                ("owner_function", "node_id", true),
                ("expected_type", "semantic_type", true),
                ("use_mode", "operand_use", true),
            ],
        ),
        named_record(
            "definition_reference_site",
            &[
                ("source", "node_id", true),
                ("slot", "definition_slot", true),
                ("target", "node_id", true),
            ],
        ),
        named_record(
            "visible_value",
            &[
                ("value", "value_ref", true),
                ("ty", "semantic_type", true),
                ("compatible", "bool", true),
                ("producer", "node_id", true),
                ("producer_code", "operation_code", false),
                ("owner_function", "node_id", true),
                ("ordinal", "u64", false),
                ("name", "name_preview", false),
            ],
        ),
        named_record(
            "legal_constructors_result",
            &[
                ("target", "repair_target", true),
                ("expected_type", "semantic_type", true),
                ("constructors", "page<constructor_descriptor>", true),
                ("visible_values", "page<visible_value>", true),
            ],
        ),
        named_record(
            "context_budget",
            &[
                ("body_before", "u32", true),
                ("body_after", "u32", true),
                ("visible_values", "u32", true),
                ("incoming_uses", "u32", true),
                ("include_incompatible", "bool", true),
            ],
        ),
        named_record(
            "semantic_diff_page",
            &[
                ("from", "revision", true),
                ("to", "revision", true),
                ("change_count", "u64", true),
                ("change_digest", "change_digest", true),
                ("page", "page<change>", true),
            ],
        ),
        named_record(
            "block_argument_fact",
            &[
                ("argument", "node_id", true),
                ("block", "node_id", true),
                ("region", "node_id", true),
                ("ordinal", "u32", true),
                ("role", "block_argument_role", true),
                ("ty", "semantic_type", true),
            ],
        ),
        named_record(
            "enclosing_region_fact",
            &[
                ("region", "node_id", true),
                ("owner_operation", "node_id", true),
                ("role", "region_role", true),
            ],
        ),
        named_record(
            "repair_context",
            &[
                ("workspace", "workspace_id", true),
                ("revision", "revision", true),
                ("target", "repair_target", true),
                ("operation", "node_id", true),
                ("operation_code", "operation_code", true),
                ("operand_index", "u64", false),
                ("expected_type", "semantic_type", true),
                ("use_mode", "operand_use", false),
                ("current_value", "value_ref", false),
                ("current_actual_type", "semantic_type", false),
                ("owner_block", "node_id", true),
                ("owner_function", "node_id", true),
                ("ordinal", "u64", true),
                ("function_signature", "function_signature_summary", true),
                ("owner_chain", "list<owner_fact>", true),
                ("enclosing_regions", "list<enclosing_region_fact>", true),
                ("visible_block_arguments", "list<block_argument_fact>", true),
                ("body_window", "list<body_item>", true),
                ("visible_values", "page<visible_value>", true),
                ("incoming_uses", "page<use_site>", true),
                ("legal_constructor_count", "u64", true),
                ("legal_constructors", "list<constructor_descriptor>", true),
                ("nominal_type", "nominal_type_result", false),
                (
                    "nominal_type_continuation",
                    "nominal_type_continuation",
                    false,
                ),
                ("blocker", "completeness_blocker", false),
                ("refinement_operation", "transaction_operation_code", false),
            ],
        ),
        NamedPayloadDescription {
            name: "nominal_type_result".into(),
            payload: record_payload(&[
                ("declaration", "node_id", true),
                ("name", "string", true),
                ("kind", "node_kind", true),
                ("owner", "node_id", true),
                ("layout", "nominal_layout_summary", true),
                ("members", "page<nominal_member_fact>", true),
            ]),
        },
        NamedPayloadDescription {
            name: "nominal_layout_summary".into(),
            payload: record_payload(&[
                ("representable", "bool", true),
                ("failure", "layout_failure", false),
                ("size", "u64", false),
                ("align", "u64", false),
                ("cells", "u64", false),
                ("discriminant_bytes", "u8", false),
                ("payload_offset", "u64", false),
            ]),
        },
        NamedPayloadDescription {
            name: "constructor_descriptor".into(),
            payload: record_payload(&[
                ("code", "operation_code", true),
                ("result_type", "semantic_type", true),
                ("operand_count", "u64", true),
                ("operand_types", "list<semantic_type>", true),
                ("operand_uses", "list<operand_use>", true),
                ("literal_fields", "list<literal_field>", true),
                ("call_target", "node_id", false),
                ("declaration", "node_id", false),
                ("member_count", "u64", true),
                ("members", "list<node_id>", true),
                ("requirements_complete", "bool", true),
                (
                    "nominal_type_continuation",
                    "nominal_type_continuation",
                    false,
                ),
                ("direct_refinement", "bool", true),
                ("complete", "bool", true),
                ("terminator", "bool", true),
            ]),
        },
        NamedPayloadDescription {
            name: "nominal_type_continuation".into(),
            payload: record_payload(&[
                ("declaration", "node_id", true),
                ("page", "page_request", true),
            ]),
        },
        NamedPayloadDescription {
            name: "page_request".into(),
            payload: record_payload(&[("after", "page_cursor", false), ("limit", "u32", true)]),
        },
        named_record(
            "page",
            &[
                ("items", "list<type_parameter>", true),
                ("next", "page_cursor", false),
                ("total", "u64", false),
            ],
        ),
        named_record(
            "change",
            &[("node", "node_id", true), ("kind", "change_kind", true)],
        ),
    ]
}

fn query_variants() -> Vec<NamedVariantDescription> {
    vec![
        named_variant(
            "query",
            QueryCode::ALL.into_iter().map(query_payload).collect(),
        ),
        named_variant(
            "query_result",
            QueryCode::ALL
                .into_iter()
                .map(query_result_payload)
                .collect(),
        ),
        named_variant("nominal_member_fact", query_member_payloads()),
        named_variant("page_cursor", query_cursor_payloads()),
        named_variant(
            "query_outcome",
            vec![
                variant_payload("success", newtype_payload("query_result")),
                variant_payload("error", newtype_payload("error")),
            ],
        ),
        named_variant(
            "repair_target",
            vec![
                variant_payload("hole", newtype_payload("node_id")),
                variant_payload(
                    "operand",
                    record_payload(&[("operation", "node_id", true), ("index", "u64", true)]),
                ),
            ],
        ),
        unit_variants(
            "expected_category",
            [
                ("entry_function", 1),
                ("function_body", 2),
                ("expression", 3),
            ],
        ),
        unit_variants(
            "visible_cursor_purpose",
            [
                ("visible_values", 1),
                ("legal_constructors", 2),
                ("repair_context", 3),
            ],
        ),
        unit_variants(
            "layout_failure",
            [
                ("byte_size_overflow", 1),
                ("cell_count_overflow", 2),
                ("invalid_dependency", 3),
            ],
        ),
        unit_variants(
            "definition_slot",
            [
                ("package_entry", 1),
                ("call_target", 2),
                ("function_result_type", 3),
                ("parameter_type", 4),
                ("product_field_type", 5),
                ("sum_variant_payload_type", 6),
                ("sequence_element_type", 7),
                ("block_argument_type", 8),
                ("operation_type", 9),
                ("product_declaration", 10),
                ("product_field", 11),
                ("sum_variant", 12),
                ("match_variant", 13),
                ("sequence_declaration", 14),
            ],
        ),
        named_variant(
            "literal_value",
            vec![
                variant_payload("i64", newtype_payload("i64")),
                variant_payload("bool", newtype_payload("bool")),
                variant_payload("expected_type", newtype_payload("semantic_type")),
                variant_payload("bytes", newtype_payload("bytes_string")),
                variant_payload("text", newtype_payload("string")),
            ],
        ),
        named_variant(
            "dependency_fact",
            vec![
                variant_payload(
                    "value_operand",
                    record_payload(&[("index", "u64", true), ("value", "value_ref", true)]),
                ),
                variant_payload(
                    "definition",
                    record_payload(&[
                        ("slot", "definition_slot", true),
                        ("target", "node_id", true),
                    ]),
                ),
            ],
        ),
        named_variant(
            "scalar_value",
            vec![
                variant_payload("i64", newtype_payload("i64")),
                variant_payload("bool", newtype_payload("bool")),
                variant_payload("type", newtype_payload("semantic_type")),
                variant_payload("bytes", newtype_payload("bytes_string")),
                variant_payload("text", newtype_payload("string")),
            ],
        ),
        named_variant(
            "change_kind",
            vec![
                variant_payload("created", record_payload(&[("kind", "node_kind", true)])),
                variant_payload("deleted", record_payload(&[("kind", "node_kind", true)])),
                variant_payload(
                    "renamed",
                    record_payload(&[("before", "string", true), ("after", "string", true)]),
                ),
                variant_payload(
                    "scalar_attribute_changed",
                    record_payload(&[
                        ("before", "scalar_value", true),
                        ("after", "scalar_value", true),
                    ]),
                ),
                variant_payload(
                    "containment_changed",
                    record_payload(&[("before_count", "u64", true), ("after_count", "u64", true)]),
                ),
                variant_payload(
                    "operand_changed",
                    record_payload(&[
                        ("index", "u64", true),
                        ("before", "value_ref", false),
                        ("after", "value_ref", false),
                    ]),
                ),
                variant_payload(
                    "definition_changed",
                    record_payload(&[("before", "node_id", true), ("after", "node_id", true)]),
                ),
                variant_payload(
                    "entry_function_changed",
                    record_payload(&[("before", "node_id", false), ("after", "node_id", false)]),
                ),
                variant_payload(
                    "completeness_changed",
                    record_payload(&[("complete", "bool", true)]),
                ),
                variant_payload(
                    "operation_refined",
                    record_payload(&[
                        ("before", "operation_code", true),
                        ("after", "operation_code", true),
                        ("result_type", "semantic_type", true),
                        ("replacement", "operation_kind", true),
                    ]),
                ),
                variant_payload("allocated_and_tombstoned", unit_payload()),
                variant_payload(
                    "function_body_changed",
                    record_payload(&[
                        ("before_items", "u64", true),
                        ("after_items", "u64", true),
                        ("added_items", "u64", true),
                        ("removed_items", "u64", true),
                        ("modified_items", "u64", true),
                    ]),
                ),
            ],
        ),
    ]
}

fn query_member_payloads() -> Vec<VariantPayloadDescription> {
    vec![
        variant_payload(
            "product_field",
            record_payload(&[
                ("field", "node_id", true),
                ("name", "string", true),
                ("ordinal", "u32", true),
                ("ty", "semantic_type", true),
                ("offset", "u64", false),
                ("cells", "u64", false),
            ]),
        ),
        variant_payload(
            "sum_variant",
            record_payload(&[
                ("variant", "node_id", true),
                ("name", "string", true),
                ("ordinal", "u32", true),
                ("payload", "semantic_type", false),
                ("discriminant", "u64", false),
                ("payload_size", "u64", false),
                ("payload_align", "u64", false),
                ("payload_cells", "u64", false),
            ]),
        ),
    ]
}

fn query_cursor_payloads() -> Vec<VariantPayloadDescription> {
    let common = |extra: &[(&str, &str, bool)]| {
        let mut fields = vec![
            ("workspace", "workspace_id", true),
            ("revision", "revision", true),
        ];
        fields.extend_from_slice(extra);
        record_payload(&fields)
    };
    vec![
        variant_payload("blockers", common(&[("next", "u64", true)])),
        variant_payload(
            "owner_chain",
            common(&[("node", "node_id", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "body",
            common(&[("block", "node_id", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "incoming_uses",
            common(&[("value", "value_ref", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "definition_references",
            common(&[("target", "node_id", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "dependencies",
            common(&[("node", "node_id", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "visible_values",
            common(&[
                ("purpose", "visible_cursor_purpose", true),
                ("target", "repair_target", true),
                ("expected", "semantic_type", true),
                ("include_incompatible", "bool", true),
                ("next", "u64", true),
            ]),
        ),
        variant_payload(
            "legal_constructors",
            common(&[
                ("target", "repair_target", true),
                ("expected", "semantic_type", true),
                ("next", "u64", true),
            ]),
        ),
        variant_payload(
            "diff",
            record_payload(&[
                ("workspace", "workspace_id", true),
                ("from", "revision", true),
                ("to", "revision", true),
                ("next", "u64", true),
            ]),
        ),
        variant_payload(
            "nominal_type",
            common(&[("declaration", "node_id", true), ("next", "u64", true)]),
        ),
    ]
}

fn error_payload() -> PayloadShapeDescription {
    record_payload(&[
        ("code", "error_code", true),
        ("workspace", "workspace_id", false),
        ("revision", "revision", false),
        ("operation_index", "u32", false),
        ("draft_symbol", "draft_symbol", false),
        ("draft_path", "string", false),
        ("target", "node_id", false),
        ("expected_kind", "node_kind", false),
        ("actual_kind", "node_kind", false),
        ("expected_type", "semantic_type", false),
        ("actual_type", "semantic_type", false),
        ("related", "list<node_id>", true),
        ("retryable", "bool", true),
        ("message", "string", true),
    ])
}

fn envelope_payloads() -> Vec<NamedPayloadDescription> {
    vec![
        NamedPayloadDescription {
            name: "request_envelope".into(),
            payload: record_payload(&[
                ("version", "u16", true),
                ("request_id", "request_id", true),
                ("request", "request", true),
            ]),
        },
        NamedPayloadDescription {
            name: "response_envelope".into(),
            payload: record_payload(&[
                ("version", "u16", true),
                ("request_id", "request_id", true),
                ("response", "response", true),
            ]),
        },
        NamedPayloadDescription {
            name: "boundary_error_envelope".into(),
            payload: record_payload(&[
                ("version", "u16", true),
                ("request_id", "request_id", false),
                ("error", "boundary_error", true),
            ]),
        },
    ]
}

fn schema_discovery_records() -> Vec<NamedPayloadDescription> {
    let record = |name: &str, fields: &[(&str, &str, bool)]| NamedPayloadDescription {
        name: name.into(),
        payload: record_payload(fields),
    };
    vec![
        record(
            "machine_field_description",
            &[
                ("name", "string", true),
                ("type_expression", "string", true),
                ("required", "bool", true),
            ],
        ),
        record(
            "payload_shape_description",
            &[
                ("shape", "payload_shape_kind", true),
                ("newtype", "string", false),
                ("fields", "list<machine_field_description>", true),
            ],
        ),
        record(
            "variant_payload_description",
            &[
                ("name", "string", true),
                ("payload", "payload_shape_description", true),
            ],
        ),
        record(
            "named_payload_description",
            &[
                ("name", "string", true),
                ("payload", "payload_shape_description", true),
            ],
        ),
        record(
            "named_variant_description",
            &[
                ("name", "string", true),
                ("tagging", "string", true),
                ("tag_field", "string", false),
                ("content_field", "string", false),
                ("variants", "list<variant_payload_description>", true),
            ],
        ),
        record("code_description", &[("name", "string", true)]),
        record(
            "draft_field_type_description",
            &[
                ("name", "string", true),
                ("type_expression", "string", true),
            ],
        ),
        record(
            "draft_field_description",
            &[
                ("name", "string", true),
                ("field_type", "draft_field_type", true),
                ("required", "bool", true),
                ("nullable", "bool", true),
                ("declares_symbol", "bool", true),
            ],
        ),
        record(
            "draft_record_description",
            &[
                ("name", "string", true),
                ("fields", "list<draft_field_description>", true),
            ],
        ),
        record(
            "draft_variant_description",
            &[
                ("name", "string", true),
                ("shape", "payload_shape_kind", true),
                ("newtype", "draft_field_type", false),
                ("fields", "list<draft_field_description>", true),
            ],
        ),
        record(
            "structured_authoring_description",
            &[
                (
                    "draft_field_types",
                    "list<draft_field_type_description>",
                    true,
                ),
                ("records", "list<draft_record_description>", true),
                (
                    "expression_variants",
                    "list<draft_variant_description>",
                    true,
                ),
                (
                    "operation_variants",
                    "list<draft_variant_description>",
                    true,
                ),
                ("value_variants", "list<draft_variant_description>", true),
                ("type_variants", "list<draft_variant_description>", true),
                ("expression_tagging", "string", true),
                ("operation_tagging", "string", true),
                ("value_tagging", "string", true),
                ("type_tagging", "string", true),
                ("allocation_order", "string", true),
                ("inline_expression_variants", "list<string>", true),
                ("inline_holes_allowed", "bool", true),
                ("inline_region_operations_allowed", "bool", true),
                ("maintenance_accepts_inline_values", "bool", true),
                ("nesting_metric", "string", true),
                ("explicit_symbols_are_selectable", "bool", true),
                ("implicit_symbols_are_selectable", "bool", true),
                ("implicit_node_kinds", "list<node_kind>", true),
                ("maximum_request_depth", "u32", true),
                ("maximum_request_items", "u64", true),
                ("counted_item_categories", "list<string>", true),
            ],
        ),
        record(
            "operand_description",
            &[("ty", "type_rule", true), ("use_mode", "operand_use", true)],
        ),
        record(
            "block_argument_description",
            &[
                ("role", "block_argument_role", true),
                ("ty", "type_rule", true),
            ],
        ),
        record(
            "region_description",
            &[
                ("role", "region_role", true),
                ("block_arguments", "list<block_argument_description>", true),
                ("terminator", "operation_code", true),
                ("yield_type", "type_rule", true),
            ],
        ),
        record(
            "operation_description",
            &[
                ("name", "string", true),
                ("operand_arity", "operand_arity", true),
                ("operands", "list<operand_description>", true),
                ("results", "list<type_rule>", true),
                ("literal_fields", "list<literal_field>", true),
                ("region_arity", "region_arity", true),
                ("regions", "list<region_description>", true),
                ("complete", "bool", true),
                ("terminator", "bool", true),
            ],
        ),
        record(
            "run_field_description",
            &[
                ("name", "string", true),
                ("field_type", "run_field_type", true),
                ("required", "bool", true),
            ],
        ),
        record(
            "runtime_value_description",
            &[
                ("name", "string", true),
                ("payload", "runtime_value_payload", true),
                ("fields", "list<machine_field_description>", true),
                ("invariants", "list<string>", true),
            ],
        ),
        record(
            "run_description",
            &[
                ("fields", "list<run_field_description>", true),
                ("policy_fields", "list<run_field_description>", true),
                ("runtime_values", "list<runtime_value_description>", true),
                ("records", "list<named_payload_description>", true),
                ("variants", "list<named_variant_description>", true),
                ("limit_scope", "list<string>", true),
            ],
        ),
        record(
            "schema_discovery_description",
            &[
                ("digest_format", "string", true),
                ("digest_domain", "string", true),
                ("request", "payload_shape_description", true),
                ("records", "list<named_payload_description>", true),
                ("variants", "list<named_variant_description>", true),
                (
                    "projection_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                ("result_payloads", "list<variant_payload_description>", true),
                ("roots", "list<string>", true),
                ("type_constructors", "list<string>", true),
                ("maximum_roots_per_request", "u8", true),
                ("full_available", "bool", true),
                ("known_digest_match_follows_root_validation", "bool", true),
            ],
        ),
        record(
            "schema_description",
            &[
                ("machine_schema_identity", "string", true),
                ("protocol_version", "u16", true),
                ("json_envelope_version", "u16", true),
                ("artifact_format_version", "u16", true),
                ("artifact_magic_hex", "string", true),
                ("semantic_schema_identity", "string", true),
                ("schema_discovery", "schema_discovery_description", true),
                ("scalar_types", "list<machine_scalar_description>", true),
                ("semantic_types", "list<code_description>", true),
                ("node_kinds", "list<code_description>", true),
                ("name_contract", "name_contract_description", true),
                ("operations", "list<operation_description>", true),
                ("semantic_records", "list<named_payload_description>", true),
                ("semantic_variants", "list<named_variant_description>", true),
                ("transaction_operations", "list<code_description>", true),
                (
                    "transaction_operation_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                (
                    "transaction_records",
                    "list<named_payload_description>",
                    true,
                ),
                (
                    "transaction_variants",
                    "list<named_variant_description>",
                    true,
                ),
                (
                    "structured_authoring",
                    "structured_authoring_description",
                    true,
                ),
                ("run", "run_description", true),
                ("queries", "list<code_description>", true),
                ("query_payloads", "list<variant_payload_description>", true),
                (
                    "query_result_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                ("query_records", "list<named_payload_description>", true),
                ("query_variants", "list<named_variant_description>", true),
                (
                    "query_member_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                (
                    "query_cursor_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                ("errors", "list<code_description>", true),
                ("error_payload", "payload_shape_description", true),
                ("error_records", "list<named_payload_description>", true),
                ("error_variants", "list<named_variant_description>", true),
                ("requests", "list<code_description>", true),
                (
                    "request_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                ("responses", "list<code_description>", true),
                (
                    "response_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                ("identity_variants", "list<named_variant_description>", true),
                ("envelopes", "list<named_payload_description>", true),
                ("boundary_error_kinds", "list<string>", true),
                ("limits", "boundary_limits", true),
                ("id_formats", "id_formats_description", true),
                (
                    "nominal_declarations",
                    "nominal_declarations_description",
                    true,
                ),
            ],
        ),
        record(
            "machine_scalar_description",
            &[
                ("name", "string", true),
                ("json_kind", "json_scalar_kind", true),
                ("domain", "machine_scalar_domain", true),
            ],
        ),
        record(
            "name_contract_description",
            &[
                ("named_node_kinds", "list<node_kind>", true),
                ("minimum_utf8_bytes", "u64", true),
                ("maximum_utf8_bytes", "u64", true),
                (
                    "sibling_uniqueness_groups",
                    "list<name_uniqueness_group_description>",
                    true,
                ),
            ],
        ),
        record(
            "name_uniqueness_group_description",
            &[
                ("name", "string", true),
                ("owner_kind", "node_kind", true),
                ("member_kinds", "list<node_kind>", true),
            ],
        ),
        record(
            "boundary_limits",
            &[
                ("maximum_request_frame_bytes", "u64", true),
                ("maximum_response_frame_bytes", "u64", true),
                ("maximum_artifact_bytes", "u64", true),
                ("maximum_artifact_name_bytes", "u64", true),
                ("maximum_json_input_bytes", "u64", true),
                ("maximum_json_output_bytes", "u64", true),
                ("maximum_page_items", "u32", true),
                ("maximum_batch_queries", "u32", true),
                ("maximum_batch_items", "u32", true),
                ("maximum_context_items_per_category", "u32", true),
                ("maximum_returned_bindings", "u32", true),
                ("maximum_run_arguments", "u32", true),
                ("maximum_run_fuel", "u64", true),
                ("maximum_run_frames", "u32", true),
                ("maximum_run_live_cells", "u64", true),
                ("maximum_runtime_value_depth", "u32", true),
                ("maximum_runtime_value_items", "u64", true),
                ("maximum_runtime_value_bytes", "u64", true),
                ("maximum_byte_literal_bytes", "u64", true),
                ("maximum_transaction_byte_literal_bytes", "u64", true),
                ("maximum_runtime_byte_value_bytes", "u64", true),
                ("maximum_run_argument_byte_bytes", "u64", true),
                ("maximum_run_managed_visible_bytes", "u64", true),
                ("maximum_run_retained_backing_bytes", "u64", true),
                ("maximum_run_managed_objects", "u64", true),
                ("maximum_error_related_ids", "u32", true),
                ("maximum_boundary_error_message_bytes", "u64", true),
                ("maximum_persistence_head_bytes", "u64", true),
            ],
        ),
        record(
            "id_formats_description",
            &[
                ("workspace", "string", true),
                ("idempotency_key", "string", true),
                ("node", "string", true),
                ("snapshot_hash", "string", true),
                ("change_digest", "string", true),
                ("revision", "string", true),
                ("request_id", "string", true),
                ("query_id", "string", true),
                ("draft_symbol", "string", true),
                ("machine_schema_digest", "string", true),
            ],
        ),
        record(
            "nominal_declarations_description",
            &[
                ("declaration_kinds", "list<node_kind>", true),
                ("member_kinds", "list<node_kind>", true),
                ("shape_invariants", "list<string>", true),
                ("layout_invariants", "list<string>", true),
            ],
        ),
        record(
            "schema_manifest",
            &[
                ("schema_identity", "string", true),
                ("digest", "machine_schema_digest", true),
                ("protocol_version", "u16", true),
                ("json_envelope_version", "u16", true),
                ("artifact_format_version", "u16", true),
                ("artifact_magic_hex", "string", true),
                ("semantic_schema_identity", "string", true),
                ("roots", "list<string>", true),
                ("type_constructors", "list<string>", true),
                ("maximum_roots_per_request", "u8", true),
                ("full_available", "bool", true),
                ("maximum_request_frame_bytes", "u64", true),
                ("maximum_response_frame_bytes", "u64", true),
                ("maximum_json_output_bytes", "u64", true),
            ],
        ),
        record(
            "schema_definitions",
            &[
                ("digest", "machine_schema_digest", true),
                ("roots", "list<schema_root>", true),
                ("type_constructors", "list<string>", true),
                ("definitions", "list<schema_definition>", true),
            ],
        ),
        record(
            "schema_definition",
            &[
                ("name", "string", true),
                ("dependencies", "list<string>", true),
                ("body", "schema_definition_body", true),
            ],
        ),
        record(
            "draft_variant_family_description",
            &[
                ("name", "string", true),
                ("tagging", "string", true),
                ("variants", "list<draft_variant_description>", true),
            ],
        ),
        record(
            "endpoint_description",
            &[
                ("name", "string", true),
                ("family", "string", true),
                ("template", "string", true),
                (
                    "bindings",
                    "list<endpoint_variant_binding_description>",
                    true,
                ),
                ("protocol_version", "u16", true),
                ("json_envelope_version", "u16", true),
                ("boundary_error_envelope", "string", true),
                ("typed_error", "string", true),
                ("id_formats", "string", true),
                ("limits", "string", true),
            ],
        ),
        record(
            "endpoint_variant_binding_description",
            &[
                ("parameter", "string", true),
                ("variant", "variant_payload_description", true),
            ],
        ),
        record(
            "endpoint_protocol_template_description",
            &[
                ("name", "string", true),
                (
                    "parameters",
                    "list<endpoint_template_parameter_description>",
                    true,
                ),
                ("records", "list<named_payload_description>", true),
                ("variants", "list<named_variant_description>", true),
            ],
        ),
        record(
            "endpoint_template_parameter_description",
            &[
                ("name", "string", true),
                ("target_variant", "string", true),
                ("semantics", "string", true),
            ],
        ),
        record(
            "code_family_description",
            &[("name", "string", true), ("members", "list<string>", true)],
        ),
        record(
            "structured_authoring_policy_description",
            &[
                ("allocation_order", "string", true),
                ("inline_expression_variants", "list<string>", true),
                ("inline_holes_allowed", "bool", true),
                ("inline_region_operations_allowed", "bool", true),
                ("maintenance_accepts_inline_values", "bool", true),
                ("nesting_metric", "string", true),
                ("explicit_symbols_are_selectable", "bool", true),
                ("implicit_symbols_are_selectable", "bool", true),
                ("implicit_node_kinds", "list<node_kind>", true),
                ("maximum_request_depth", "u32", true),
                ("maximum_request_items", "u64", true),
                ("counted_item_categories", "list<string>", true),
            ],
        ),
    ]
}

fn schema_discovery_variants(
    projection_payloads: &[VariantPayloadDescription],
    result_payloads: &[VariantPayloadDescription],
) -> Vec<NamedVariantDescription> {
    vec![
        named_variant("schema_projection", projection_payloads.to_vec()),
        named_variant("describe_schema_result", result_payloads.to_vec()),
        unit_variants(
            "schema_root",
            SchemaRoot::ALL
                .into_iter()
                .map(|root| (root.machine_name(), 0)),
        ),
        named_variant(
            "schema_definition_body",
            vec![
                variant_payload("scalar", newtype_payload("machine_scalar_description")),
                variant_payload("record", newtype_payload("named_payload_description")),
                variant_payload("variant", newtype_payload("named_variant_description")),
                variant_payload("draft_record", newtype_payload("draft_record_description")),
                variant_payload(
                    "draft_variant",
                    newtype_payload("draft_variant_family_description"),
                ),
                variant_payload("endpoint", newtype_payload("endpoint_description")),
                variant_payload(
                    "endpoint_template",
                    newtype_payload("endpoint_protocol_template_description"),
                ),
                variant_payload("codes", newtype_payload("code_family_description")),
                variant_payload("operations", newtype_payload("list<operation_description>")),
                variant_payload(
                    "structured_authoring",
                    newtype_payload("structured_authoring_policy_description"),
                ),
                variant_payload(
                    "name_contract",
                    newtype_payload("name_contract_description"),
                ),
                variant_payload(
                    "nominal_declarations",
                    newtype_payload("nominal_declarations_description"),
                ),
                variant_payload("id_formats", newtype_payload("id_formats_description")),
                variant_payload("limits", newtype_payload("boundary_limits")),
            ],
        ),
        unit_variants(
            "payload_shape_kind",
            [("unit", 1), ("newtype", 2), ("record", 3)],
        ),
        unit_variants(
            "json_scalar_kind",
            [("boolean", 1), ("number", 2), ("string", 3)],
        ),
        named_variant(
            "machine_scalar_domain",
            vec![
                variant_payload("boolean", unit_payload()),
                variant_payload("utf8_string", unit_payload()),
                variant_payload(
                    "signed_integer",
                    record_payload(&[("minimum", "i64", true), ("maximum", "i64", true)]),
                ),
                variant_payload(
                    "unsigned_integer",
                    record_payload(&[("minimum", "u64", true), ("maximum", "u64", true)]),
                ),
                variant_payload(
                    "lowercase_hex",
                    record_payload(&[("encoded_bytes", "u8", true)]),
                ),
                variant_payload(
                    "canonical_url_safe_base64",
                    record_payload(&[
                        ("padding", "bool", true),
                        ("whitespace", "bool", true),
                        ("canonical_trailing_bits", "bool", true),
                        ("maximum_decoded_bytes", "u64", true),
                        ("maximum_encoded_bytes", "u64", true),
                    ]),
                ),
                variant_payload(
                    "node_id",
                    record_payload(&[
                        ("workspace_bytes", "u8", true),
                        ("durable_minimum_serial", "u64", true),
                        ("durable_maximum_serial", "u64", true),
                        ("function_local_grammar", "string", true),
                        ("maximum_function_serial", "u64", true),
                        ("maximum_local_ordinal", "u32", true),
                    ]),
                ),
                variant_payload(
                    "canonical_identifier",
                    record_payload(&[
                        ("grammar", "string", true),
                        ("minimum_utf8_bytes", "u64", true),
                        ("maximum_utf8_bytes", "u64", true),
                    ]),
                ),
            ],
        ),
        unit_variants(
            "run_field_type",
            [
                ("workspace", 1),
                ("revision", 2),
                ("node", 3),
                ("runtime_value_list", 4),
                ("run_policy", 5),
                ("u64", 6),
                ("u32", 7),
            ],
        ),
        unit_variants(
            "runtime_value_payload",
            [
                ("none", 1),
                ("bool", 2),
                ("i64", 3),
                ("bytes", 4),
                ("text", 5),
                ("product", 6),
                ("sum", 7),
                ("sequence", 8),
            ],
        ),
        unit_variants(
            "draft_field_type",
            DraftFieldType::ALL
                .into_iter()
                .map(|field_type| (field_type.machine_name(), 0)),
        ),
        named_variant(
            "operand_arity",
            vec![
                variant_payload("fixed", newtype_payload("u8")),
                variant_payload("call_target_parameters", unit_payload()),
                variant_payload("product_fields", unit_payload()),
                variant_payload("variant_payload", unit_payload()),
            ],
        ),
        named_variant(
            "region_arity",
            vec![
                variant_payload("fixed", newtype_payload("u8")),
                variant_payload(
                    "match_variants",
                    record_payload(&[
                        ("payload_type", "type_rule", true),
                        ("terminator", "operation_code", true),
                        ("yield_type", "type_rule", true),
                    ]),
                ),
            ],
        ),
        unit_variants("operand_use", [("read", 1)]),
        unit_variants(
            "literal_field",
            [
                ("i64_value", 1),
                ("bool_value", 2),
                ("expected_type", 3),
                ("result_type", 4),
                ("carried_type", 5),
                ("positive_step", 6),
                ("bytes_value", 7),
                ("text_value", 8),
            ],
        ),
        unit_variants(
            "block_argument_role",
            [("loop_index", 1), ("loop_carried", 2), ("match_payload", 3)],
        ),
        named_variant(
            "type_rule",
            vec![
                variant_payload("fixed", newtype_payload("semantic_type")),
                variant_payload("payload_expected", unit_payload()),
                variant_payload("owner_function_result", unit_payload()),
                variant_payload("payload_result", unit_payload()),
                variant_payload("payload_carried", unit_payload()),
                variant_payload("call_target_parameter", unit_payload()),
                variant_payload("call_target_result", unit_payload()),
                variant_payload("owning_region_yield", unit_payload()),
                variant_payload("product_field_type", unit_payload()),
                variant_payload("product_declaration_result", unit_payload()),
                variant_payload("projection_owner", unit_payload()),
                variant_payload("projected_field_result", unit_payload()),
                variant_payload("variant_payload", unit_payload()),
                variant_payload("variant_owner_result", unit_payload()),
                variant_payload("match_scrutinee", unit_payload()),
                variant_payload("match_result", unit_payload()),
                variant_payload("sequence_declaration_result", unit_payload()),
                variant_payload("sequence_owner", unit_payload()),
                variant_payload("sequence_element", unit_payload()),
            ],
        ),
    ]
}

pub(crate) fn schema_type_constructors() -> Vec<String> {
    ["list<T>", "optional<T>", "tuple<T,...>", "page<T>"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn schema_discovery_description() -> SchemaDiscoveryDescription {
    let projection_payloads = vec![
        variant_payload("manifest", unit_payload()),
        variant_payload(
            "roots",
            record_payload(&[("roots", "list<schema_root>", true)]),
        ),
        variant_payload("full", unit_payload()),
    ];
    let result_payloads = vec![
        variant_payload(
            "unchanged",
            record_payload(&[("digest", "machine_schema_digest", true)]),
        ),
        variant_payload("manifest", newtype_payload("schema_manifest")),
        variant_payload("roots", newtype_payload("schema_definitions")),
        variant_payload(
            "full",
            record_payload(&[
                ("digest", "machine_schema_digest", true),
                ("description", "schema_description", true),
            ]),
        ),
    ];
    SchemaDiscoveryDescription {
        digest_format: "64 lowercase hexadecimal characters encoding 32 bytes".into(),
        digest_domain: MACHINE_SCHEMA_DIGEST_DOMAIN.into(),
        request: record_payload(&[
            ("projection", "schema_projection", true),
            ("known_digest", "machine_schema_digest", false),
        ]),
        records: schema_discovery_records(),
        variants: schema_discovery_variants(&projection_payloads, &result_payloads),
        projection_payloads,
        result_payloads,
        roots: SchemaRoot::ALL
            .into_iter()
            .map(|root| root.machine_name().to_owned())
            .collect(),
        type_constructors: schema_type_constructors(),
        maximum_roots_per_request: MAX_SCHEMA_ROOTS as u8,
        full_available: true,
        known_digest_match_follows_root_validation: true,
    }
}

fn name_contract_description() -> NameContractDescription {
    let sibling_uniqueness_groups = crate::schema::NameUniquenessGroup::ALL
        .into_iter()
        .map(|group| NameUniquenessGroupDescription {
            name: group.machine_name().into(),
            owner_kind: group.owner_kind(),
            member_kinds: group.member_kinds().to_vec(),
        })
        .collect::<Vec<_>>();
    let mut named_node_kinds = Vec::new();
    for kind in sibling_uniqueness_groups
        .iter()
        .flat_map(|group| group.member_kinds.iter().copied())
    {
        if !named_node_kinds.contains(&kind) {
            named_node_kinds.push(kind);
        }
    }
    NameContractDescription {
        named_node_kinds,
        minimum_utf8_bytes: crate::schema::MINIMUM_NAME_UTF8_BYTES as u64,
        maximum_utf8_bytes: crate::artifact::MAXIMUM_ARTIFACT_NAME_BYTES as u64,
        sibling_uniqueness_groups,
    }
}

pub fn schema_description() -> SchemaDescription {
    SchemaDescription {
        machine_schema_identity: MACHINE_SCHEMA_IDENTITY.into(),
        protocol_version: PROTOCOL_VERSION,
        json_envelope_version: JSON_ENVELOPE_VERSION,
        artifact_format_version: crate::artifact::FORMAT_VERSION.0,
        artifact_magic_hex: crate::artifact::MAGIC
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        semantic_schema_identity: String::from_utf8_lossy(&crate::artifact::SCHEMA_ID.0).into_owned(),
        schema_discovery: schema_discovery_description(),
        scalar_types: scalar_types(),
        semantic_types: SemanticType::PRIMITIVES
            .into_iter()
            .map(|code| described(code.machine_name()))
            .chain(std::iter::once(described("nominal")))
            .collect(),
        node_kinds: NodeKind::ALL
            .into_iter()
            .map(|code| described(code.machine_name()))
            .collect(),
        name_contract: name_contract_description(),
        operations: OperationCode::ALL
            .into_iter()
            .map(|code| {
                let descriptor = code.descriptor();
                OperationDescription {
                    name: descriptor.machine_name.to_owned(),
                    operand_arity: descriptor.operand_arity,
                    operands: descriptor
                        .operands
                        .iter()
                        .map(|operand| OperandDescription {
                            ty: operand.ty,
                            use_mode: operand.use_mode,
                        })
                        .collect(),
                    results: descriptor.results.to_vec(),
                    literal_fields: descriptor.literal_fields.to_vec(),
                    region_arity: descriptor.region_arity,
                    regions: descriptor
                        .regions
                        .iter()
                        .map(|region| RegionDescription {
                            role: region.role,
                            block_arguments: region
                                .block_arguments
                                .iter()
                                .map(|argument| BlockArgumentDescription {
                                    role: argument.role,
                                    ty: argument.ty,
                                })
                                .collect(),
                            terminator: region.terminator,
                            yield_type: region.yield_type,
                        })
                        .collect(),
                    complete: descriptor.complete,
                    terminator: descriptor.terminator,
                }
            })
            .collect(),
        semantic_records: semantic_records(),
        semantic_variants: semantic_variants(),
        transaction_operations: TransactionOpCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name()))
            .collect(),
        transaction_operation_payloads: TransactionOpCode::ALL
            .into_iter()
            .map(transaction_payload)
            .collect(),
        transaction_records: transaction_records(),
        transaction_variants: transaction_variants(),
        structured_authoring: StructuredAuthoringDescription {
            draft_field_types: DraftFieldType::ALL
                .into_iter()
                .map(|field_type| DraftFieldTypeDescription {
                    name: field_type.machine_name().into(),
                    type_expression: field_type.type_expression().into(),
                })
                .collect(),
            records: structured_records(),
            expression_variants: crate::transaction::ExpressionDraftCode::ALL
                .into_iter()
                .map(expression_variant)
                .collect(),
            operation_variants: OperationCode::ALL
                .into_iter()
                .map(operation_variant)
                .collect(),
            value_variants: crate::transaction::ValueDraftCode::ALL
                .into_iter()
                .map(value_variant)
                .collect(),
            type_variants: type_variants(),
            expression_tagging: "adjacently_tagged(kind,data)".into(),
            operation_tagging: "adjacently_tagged(kind,data)".into(),
            value_tagging: "adjacently_tagged(kind,data)".into(),
            type_tagging: "externally_tagged; unit variants are strings and nominal is an object keyed by nominal".into(),
            allocation_order: "transaction_order; structured bodies preserve expression order; inline value children are normalized depth-first and left-to-right before their parent; product fields and match arms use declaration order".to_owned(),
            inline_expression_variants: crate::transaction::ExpressionDraftCode::ALL
                .into_iter()
                .filter(|code| code.is_inline_eligible())
                .map(|code| code.machine_name().to_owned())
                .collect(),
            inline_holes_allowed: false,
            inline_region_operations_allowed: false,
            maintenance_accepts_inline_values: false,
            nesting_metric: "maximum number of inline-expression or operation-owned-body edges on one structured proposal path; list wrappers, call arguments, product fields, match-arm labels, and variant payload wrappers do not add depth".to_owned(),
            explicit_symbols_are_selectable: true,
            implicit_symbols_are_selectable: false,
            implicit_node_kinds: vec![
                NodeKind::Region,
                NodeKind::Block,
                NodeKind::BlockArgument,
                NodeKind::Operation,
            ],
            maximum_request_depth: crate::transaction::MAX_STRUCTURED_DRAFT_DEPTH as u32,
            maximum_request_items: crate::transaction::MAX_STRUCTURED_DRAFT_ITEMS as u64,
            counted_item_categories: vec![
                "transaction_operation".into(),
                "function_parameter".into(),
                "product_field".into(),
                "sum_variant".into(),
                "function_body".into(),
                "yielding_body".into(),
                "explicit_or_inline_expression".into(),
                "call_argument".into(),
                "product_binding".into(),
                "match_arm".into(),
            ],
        },
        run: RunDescription {
            fields: [
                ("workspace", RunFieldType::Workspace),
                ("revision", RunFieldType::Revision),
                ("entry", RunFieldType::Node),
                ("arguments", RunFieldType::RuntimeValueList),
                ("policy", RunFieldType::RunPolicy),
            ]
            .into_iter()
            .map(|(name, field_type)| RunFieldDescription {
                name: name.into(),
                field_type,
                required: true,
            })
            .collect(),
            policy_fields: [
                ("fuel", RunFieldType::U64),
                ("maximum_frames", RunFieldType::U32),
            ]
            .into_iter()
            .map(|(name, field_type)| RunFieldDescription {
                name: name.into(),
                field_type,
                required: true,
            })
            .collect(),
            runtime_values: crate::interpret::RuntimeValueCode::ALL
                .into_iter()
                .map(|code| RuntimeValueDescription {
                    name: code.machine_name().into(),
                    payload: match code {
                        crate::interpret::RuntimeValueCode::Unit => RuntimeValuePayload::None,
                        crate::interpret::RuntimeValueCode::Bool => RuntimeValuePayload::Bool,
                        crate::interpret::RuntimeValueCode::I64 => RuntimeValuePayload::I64,
                        crate::interpret::RuntimeValueCode::Bytes => RuntimeValuePayload::Bytes,
                        crate::interpret::RuntimeValueCode::Text => RuntimeValuePayload::Text,
                        crate::interpret::RuntimeValueCode::Product => RuntimeValuePayload::Product,
                        crate::interpret::RuntimeValueCode::Sum => RuntimeValuePayload::Sum,
                        crate::interpret::RuntimeValueCode::Sequence => RuntimeValuePayload::Sequence,
                    },
                    fields: match code {
                        crate::interpret::RuntimeValueCode::Unit => vec![],
                        crate::interpret::RuntimeValueCode::Bool => vec![MachineFieldDescription { name: "data".into(), type_expression: "bool".into(), required: true }],
                        crate::interpret::RuntimeValueCode::I64 => vec![MachineFieldDescription { name: "data".into(), type_expression: "i64".into(), required: true }],
                        crate::interpret::RuntimeValueCode::Bytes => vec![MachineFieldDescription { name: "data".into(), type_expression: "bytes_string".into(), required: true }],
                        crate::interpret::RuntimeValueCode::Text => vec![MachineFieldDescription { name: "data".into(), type_expression: "string".into(), required: true }],
                        crate::interpret::RuntimeValueCode::Product => vec![MachineFieldDescription { name: "data".into(), type_expression: "runtime_product_data".into(), required: true }],
                        crate::interpret::RuntimeValueCode::Sum => vec![MachineFieldDescription { name: "data".into(), type_expression: "runtime_sum_data".into(), required: true }],
                        crate::interpret::RuntimeValueCode::Sequence => vec![MachineFieldDescription { name: "data".into(), type_expression: "runtime_sequence_data".into(), required: true }],
                    },
                    invariants: match code {
                        crate::interpret::RuntimeValueCode::Product => vec![
                            "ty is a semantic product declaration Node ID; fields name every exact owned field identity once".into(),
                            "input field order is arbitrary and normalized; output field order is canonical declaration order".into(),
                            "each field value has the field's exact semantic type; compiler indexes and layout offsets are forbidden".into(),
                        ],
                        crate::interpret::RuntimeValueCode::Sum => vec![
                            "ty is a semantic sum declaration Node ID and variant is one exact owned semantic variant Node ID".into(),
                            "payload is absent for nullary variants and present with the exact payload type otherwise".into(),
                            "compiler discriminants and dense type or variant indexes are forbidden".into(),
                        ],
                        crate::interpret::RuntimeValueCode::Bytes => vec![
                            "equality and behavior depend only on visible ordered octets; backing, view, sharing, and runtime handles are unobservable".into(),
                            "data is canonical unpadded URL-safe base64 with no whitespace and canonical trailing bits".into(),
                        ],
                        crate::interpret::RuntimeValueCode::Text => vec![
                            "data is valid UTF-8, compared by exact UTF-8 bytes, and is not normalized".into(),
                        ],
                        crate::interpret::RuntimeValueCode::Sequence => vec![
                            "ty is an exact semantic sequence declaration and elements have its exact element type".into(),
                            "element order is observable; allocation, capacity, and sharing are unobservable".into(),
                        ],
                        _ => vec!["value must have the exact primitive semantic type".into()],
                    },
                })
                .collect(),
            records: run_records(),
            variants: run_variants(),
            limit_scope: vec![
                "argument count applies to the complete Run arguments list".into(),
                "runtime value depth applies per nested value root; item and structural-byte limits aggregate across all Run arguments".into(),
                "live-cell policy applies to peak frame arrays plus argument, edge, return, and public flatten scratch before allocation or cell transfer".into(),
                "fuel charges before work: one base per instruction or transfer plus max(1, materialized cells) for every logical value transfer; variant construction charges its full canonical sum cells".into(),
                "bytes_slice additionally charges one logical view unit without charging per visible octet".into(),
                "bytes_equal additionally charges one fuel unit per compared octet and stops at the first mismatch; differing lengths compare no octets".into(),
                "bytes_concat additionally charges one fuel unit per octet in the complete logical result, independent of allocation or reuse".into(),
                "decoded byte values, cumulative invocation visible construction, live distinct backing bytes, and live managed object count have independent limits".into(),
            ],
        },
        queries: QueryCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name()))
            .collect(),
        query_payloads: QueryCode::ALL.into_iter().map(query_payload).collect(),
        query_result_payloads: QueryCode::ALL
            .into_iter()
            .map(query_result_payload)
            .collect(),
        query_records: query_records(),
        query_variants: query_variants(),
        query_member_payloads: query_member_payloads(),
        query_cursor_payloads: query_cursor_payloads(),
        errors: crate::ErrorCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name()))
            .collect(),
        error_payload: error_payload(),
        error_records: error_records(),
        error_variants: error_variants(),
        requests: RequestCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name()))
            .collect(),
        request_payloads: RequestCode::ALL.into_iter().map(request_payload).collect(),
        responses: ResponseCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name()))
            .collect(),
        response_payloads: ResponseCode::ALL
            .into_iter()
            .map(response_payload)
            .collect(),
        identity_variants: identity_variants(),
        envelopes: envelope_payloads(),
        boundary_error_kinds: [
            BoundaryErrorKind::InvalidJson,
            BoundaryErrorKind::InputTooLarge,
            BoundaryErrorKind::Transport,
            BoundaryErrorKind::Output,
            BoundaryErrorKind::Usage,
        ]
        .into_iter()
        .map(|kind| kind.machine_name().into())
        .collect(),
        limits: BoundaryLimits {
            maximum_request_frame_bytes: MAX_JSON_INPUT_BYTES as u64,
            maximum_response_frame_bytes: MAX_JSON_OUTPUT_BYTES as u64,
            maximum_artifact_bytes: crate::artifact::MAXIMUM_ARTIFACT_BYTES as u64,
            maximum_artifact_name_bytes: crate::artifact::MAXIMUM_ARTIFACT_NAME_BYTES as u64,
            maximum_json_input_bytes: MAX_JSON_INPUT_BYTES as u64,
            maximum_json_output_bytes: MAX_JSON_OUTPUT_BYTES as u64,
            maximum_page_items: MAX_PAGE_ITEMS,
            maximum_batch_queries: MAX_BATCH_QUERIES as u32,
            maximum_batch_items: MAX_BATCH_ITEMS,
            maximum_context_items_per_category: MAX_CONTEXT_ITEMS,
            maximum_returned_bindings: MAX_RETURNED_BINDINGS as u32,
            maximum_run_arguments: crate::interpret::MAX_RUN_ARGUMENTS as u32,
            maximum_run_fuel: crate::interpret::MAX_RUN_FUEL,
            maximum_run_frames: crate::interpret::MAX_RUN_FRAMES,
            maximum_run_live_cells: crate::interpret::MAX_RUN_LIVE_CELLS as u64,
            maximum_runtime_value_depth: crate::interpret::MAX_RUNTIME_VALUE_DEPTH as u32,
            maximum_runtime_value_items: crate::interpret::MAX_RUNTIME_VALUE_ITEMS as u64,
            maximum_runtime_value_bytes: crate::interpret::MAX_RUNTIME_VALUE_BYTES as u64,
            maximum_byte_literal_bytes: crate::schema::MAXIMUM_BYTE_LITERAL_BYTES as u64,
            maximum_transaction_byte_literal_bytes: crate::schema::MAXIMUM_TRANSACTION_BYTE_LITERAL_BYTES as u64,
            maximum_runtime_byte_value_bytes: crate::schema::MAXIMUM_BYTE_STRING_BYTES as u64,
            maximum_run_argument_byte_bytes: crate::interpret::MAX_RUN_ARGUMENT_BYTE_BYTES as u64,
            maximum_run_managed_visible_bytes: crate::interpret::MAX_RUN_MANAGED_VISIBLE_BYTES as u64,
            maximum_run_retained_backing_bytes: crate::interpret::MAX_RUN_RETAINED_BACKING_BYTES as u64,
            maximum_run_managed_objects: crate::interpret::MAX_RUN_MANAGED_OBJECTS as u64,
            maximum_error_related_ids: crate::error::MAX_ERROR_RELATED_IDS as u32,
            maximum_boundary_error_message_bytes: MAX_BOUNDARY_ERROR_MESSAGE_BYTES as u64,
            maximum_persistence_head_bytes: crate::persistence::MAXIMUM_HEAD_BYTES as u64,
        },
        id_formats: IdFormats {
            workspace: "32 lowercase hexadecimal characters".to_owned(),
            idempotency_key: "32 lowercase hexadecimal characters".to_owned(),
            node: "durable WORKSPACE:SERIAL or revision-bound WORKSPACE:lFUNCTION.ORDINAL"
                .to_owned(),
            snapshot_hash: "64 lowercase hexadecimal characters".to_owned(),
            change_digest: "64 lowercase hexadecimal characters".to_owned(),
            revision: "JSON unsigned 64-bit integer".to_owned(),
            request_id: "JSON nonzero unsigned 64-bit integer".to_owned(),
            query_id: "JSON unsigned 64-bit integer".to_owned(),
            draft_symbol: "1 to 64 ASCII bytes matching [a-z][a-z0-9_]*".to_owned(),
            machine_schema_digest: "64 lowercase hexadecimal characters".to_owned(),
        },
        nominal_declarations: NominalDeclarationsDescription {
            declaration_kinds: vec![
                NodeKind::ProductType,
                NodeKind::SumType,
                NodeKind::SequenceType,
            ],
            member_kinds: vec![NodeKind::ProductField, NodeKind::SumVariant],
            shape_invariants: vec![
                "nominal type identity is its declaration Node ID; member identity is its field or variant Node ID".into(),
                "one declaration identity has immutable owner, ordered member identities, ordinals, field types, and variant payload contracts".into(),
                "product construction names every exact owned field once; sum construction names one exact owned variant and its exact optional payload".into(),
                "closed-sum match has exactly one identity-keyed arm per variant and canonical storage follows declaration order".into(),
                "direct and indirect by-value nominal cycles reject atomically while sequence indirection permits finite recursive values".into(),
                "a sequence declaration owns one exact homogeneous element type and deterministic order".into(),
            ],
            layout_invariants: vec![
                "layout is deterministic derived state and is absent from semantic artifacts".into(),
                "product fields use declaration order with checked alignment; sum discriminants use variant ordinals".into(),
                "runtime aggregate accounting uses materialized cells rather than one scalar per aggregate".into(),
            ],
        },
    }
}

fn canonicalize_schema(mut schema: SchemaDescription) -> SchemaDescription {
    schema
        .scalar_types
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .semantic_types
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .node_kinds
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .name_contract
        .named_node_kinds
        .sort_by_key(|item| item.machine_name());
    schema
        .name_contract
        .sibling_uniqueness_groups
        .sort_by(|left, right| left.name.cmp(&right.name));
    for group in &mut schema.name_contract.sibling_uniqueness_groups {
        group.member_kinds.sort_by_key(|item| item.machine_name());
    }
    schema
        .operations
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .semantic_records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .semantic_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.semantic_variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .transaction_operations
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .transaction_operation_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .transaction_records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .transaction_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.transaction_variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .structured_authoring
        .draft_field_types
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .structured_authoring
        .records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .structured_authoring
        .expression_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .structured_authoring
        .operation_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .structured_authoring
        .value_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .structured_authoring
        .type_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .structured_authoring
        .implicit_node_kinds
        .sort_by_key(|item| item.machine_name());
    schema.structured_authoring.counted_item_categories.sort();
    schema
        .structured_authoring
        .inline_expression_variants
        .sort();
    schema
        .run
        .runtime_values
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .run
        .records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .run
        .variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.run.variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .queries
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .query_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .query_result_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .query_records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .query_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.query_variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .query_member_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .query_cursor_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .errors
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .error_records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .error_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.error_variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .requests
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .request_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .responses
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .response_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .identity_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.identity_variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .envelopes
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema.boundary_error_kinds.sort();
    schema
        .schema_discovery
        .records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .schema_discovery
        .variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.schema_discovery.variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .schema_discovery
        .projection_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .schema_discovery
        .result_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema.schema_discovery.roots.sort();
    schema.schema_discovery.type_constructors.sort();
    schema
        .nominal_declarations
        .declaration_kinds
        .sort_by_key(|item| item.machine_name());
    schema
        .nominal_declarations
        .member_kinds
        .sort_by_key(|item| item.machine_name());
    schema
}

pub fn machine_schema_digest(
    description: &SchemaDescription,
) -> crate::Result<MachineSchemaDigest> {
    let canonical = canonicalize_schema(description.clone());
    let catalogue = schema_definition_catalogue(&canonical).map_err(|error| {
        crate::LkError::new(
            crate::ErrorCode::ProtocolMalformed,
            format!("cannot derive machine schema digest catalogue: {error}"),
        )
    })?;
    let bytes = serde_json::to_vec(&(canonical, catalogue)).map_err(|error| {
        crate::LkError::new(
            crate::ErrorCode::ProtocolMalformed,
            format!("cannot encode machine schema digest input: {error}"),
        )
    })?;
    let mut hasher = blake3::Hasher::new_derive_key(MACHINE_SCHEMA_DIGEST_DOMAIN);
    hasher.update(&bytes);
    Ok(MachineSchemaDigest::from_bytes(
        *hasher.finalize().as_bytes(),
    ))
}

pub fn active_machine_schema_digest() -> crate::Result<MachineSchemaDigest> {
    machine_schema_digest(&schema_description())
}

pub fn describe_schema(request: &DescribeSchemaRequest) -> Result<DescribeSchemaResult, String> {
    request.validate().map_err(str::to_owned)?;
    let description = schema_description();
    let catalogue = schema_definition_catalogue(&description)?;
    let projected = match &request.projection {
        SchemaProjection::Roots { roots } => Some(project_schema_roots(&catalogue, roots)?),
        SchemaProjection::Manifest | SchemaProjection::Full => None,
    };
    let digest = machine_schema_digest(&description).map_err(|error| error.to_string())?;
    if request.known_digest == Some(digest) {
        return Ok(DescribeSchemaResult::Unchanged { digest });
    }
    match &request.projection {
        SchemaProjection::Manifest => Ok(DescribeSchemaResult::Manifest(schema_manifest(
            &description,
            digest,
        ))),
        SchemaProjection::Roots { .. } => {
            let Some((roots, definitions)) = projected else {
                return Err("schema root projection was not preflighted".to_owned());
            };
            Ok(DescribeSchemaResult::Roots(SchemaDefinitions {
                digest,
                roots,
                type_constructors: schema_type_constructors(),
                definitions,
            }))
        }
        SchemaProjection::Full => Ok(DescribeSchemaResult::Full {
            digest,
            description: Box::new(description),
        }),
    }
}

pub(crate) fn schema_manifest(
    description: &SchemaDescription,
    digest: MachineSchemaDigest,
) -> SchemaManifest {
    SchemaManifest {
        schema_identity: description.machine_schema_identity.clone(),
        digest,
        protocol_version: description.protocol_version,
        json_envelope_version: description.json_envelope_version,
        artifact_format_version: description.artifact_format_version,
        artifact_magic_hex: description.artifact_magic_hex.clone(),
        semantic_schema_identity: description.semantic_schema_identity.clone(),
        roots: SchemaRoot::ALL
            .into_iter()
            .map(|root| root.machine_name().to_owned())
            .collect(),
        type_constructors: schema_type_constructors(),
        maximum_roots_per_request: MAX_SCHEMA_ROOTS as u8,
        full_available: true,
        maximum_request_frame_bytes: description.limits.maximum_request_frame_bytes,
        maximum_response_frame_bytes: description.limits.maximum_response_frame_bytes,
        maximum_json_output_bytes: description.limits.maximum_json_output_bytes,
    }
}

pub(crate) fn project_schema_roots(
    catalogue: &BTreeMap<String, SchemaDefinition>,
    roots: &[SchemaRoot],
) -> Result<(Vec<SchemaRoot>, Vec<SchemaDefinition>), String> {
    let mut canonical_roots = roots.to_vec();
    canonical_roots.sort_unstable();
    let mut pending = VecDeque::new();
    for root in &canonical_roots {
        pending.push_back(root.machine_name().to_owned());
    }
    let mut selected = BTreeSet::new();
    while let Some(name) = pending.pop_front() {
        if !selected.insert(name.clone()) {
            continue;
        }
        let definition = catalogue
            .get(&name)
            .ok_or_else(|| format!("unknown schema root or dependency: {name}"))?;
        for dependency in &definition.dependencies {
            if !selected.contains(dependency) {
                pending.push_back(dependency.clone());
            }
        }
    }
    let definitions = selected
        .into_iter()
        .map(|name| {
            catalogue
                .get(&name)
                .cloned()
                .ok_or_else(|| format!("missing selected schema definition: {name}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((canonical_roots, definitions))
}

fn lookup_named_record<'a>(
    records: &'a [NamedPayloadDescription],
    name: &str,
) -> Result<&'a NamedPayloadDescription, String> {
    records
        .iter()
        .find(|record| record.name == name)
        .ok_or_else(|| format!("missing executable record descriptor: {name}"))
}

fn named_variant_family<'a>(
    variants: &'a [NamedVariantDescription],
    name: &str,
) -> Result<&'a NamedVariantDescription, String> {
    variants
        .iter()
        .find(|variant| variant.name == name)
        .ok_or_else(|| format!("missing executable variant descriptor: {name}"))
}

fn variant_payload_by_name(
    variants: &[VariantPayloadDescription],
    name: &str,
) -> Result<VariantPayloadDescription, String> {
    variants
        .iter()
        .find(|variant| variant.name == name)
        .cloned()
        .ok_or_else(|| format!("missing executable variant payload: {name}"))
}

fn projected_variant_family(
    variants: &[NamedVariantDescription],
    name: &str,
    retained_variants: &[&str],
) -> Result<NamedVariantDescription, String> {
    let source = named_variant_family(variants, name)?;
    let mut selected = Vec::with_capacity(retained_variants.len());
    for retained in retained_variants {
        selected.push(
            source
                .variants
                .iter()
                .find(|variant| variant.name == *retained)
                .cloned()
                .ok_or_else(|| format!("missing executable {name} variant: {retained}"))?,
        );
    }
    Ok(NamedVariantDescription {
        name: source.name.clone(),
        tagging: source.tagging.clone(),
        tag_field: source.tag_field.clone(),
        content_field: source.content_field.clone(),
        variants: selected,
    })
}

pub(crate) fn endpoint_protocol_templates(
    description: &SchemaDescription,
) -> Result<Vec<EndpointProtocolTemplateDescription>, String> {
    let records = |sources: &[(&[NamedPayloadDescription], &str)]| {
        sources
            .iter()
            .map(|(records, name)| lookup_named_record(records, name).cloned())
            .collect::<Result<Vec<_>, _>>()
    };
    let parameter =
        |name: &str, target_variant: &str, semantics: &str| EndpointTemplateParameterDescription {
            name: name.to_owned(),
            target_variant: target_variant.to_owned(),
            semantics: semantics.to_owned(),
        };

    let control = EndpointProtocolTemplateDescription {
        name: "control_endpoint_protocol".to_owned(),
        parameters: vec![
            parameter(
                "request_variant",
                "request",
                "the endpoint binding supplies exactly one top-level request variant and its leaf payload",
            ),
            parameter(
                "success_response_variant",
                "response",
                "the endpoint binding supplies exactly one successful top-level response variant and its leaf payload",
            ),
        ],
        records: records(&[
            (&description.envelopes, "request_envelope"),
            (&description.envelopes, "response_envelope"),
        ])?,
        variants: vec![
            projected_variant_family(&description.identity_variants, "request", &[])?,
            projected_variant_family(
                &description.identity_variants,
                "response",
                &[ResponseCode::Error.machine_name()],
            )?,
        ],
    };
    let query = EndpointProtocolTemplateDescription {
        name: "query_endpoint_protocol".to_owned(),
        parameters: vec![
            parameter(
                "query_variant",
                "query",
                "the endpoint binding supplies exactly one selected inner query variant and its leaf payload",
            ),
            parameter(
                "query_result_variant",
                "query_result",
                "the endpoint binding supplies the matching inner success-result variant and its leaf payload",
            ),
        ],
        records: records(&[
            (&description.envelopes, "request_envelope"),
            (&description.envelopes, "response_envelope"),
            (&description.query_records, "query_batch_request"),
            (&description.query_records, "query_item"),
            (&description.query_records, "query_batch_result"),
            (&description.query_records, "query_item_result"),
        ])?,
        variants: vec![
            projected_variant_family(
                &description.identity_variants,
                "request",
                &[RequestCode::QueryBatch.machine_name()],
            )?,
            projected_variant_family(
                &description.identity_variants,
                "response",
                &[
                    ResponseCode::QueryBatchResult.machine_name(),
                    ResponseCode::Error.machine_name(),
                ],
            )?,
            projected_variant_family(&description.query_variants, "query", &[])?,
            projected_variant_family(&description.query_variants, "query_result", &[])?,
            projected_variant_family(
                &description.query_variants,
                "query_outcome",
                &["success", "error"],
            )?,
        ],
    };
    Ok(vec![control, query])
}

fn endpoint_definition(
    description: &SchemaDescription,
    endpoint_name: &str,
    family: &str,
    template: &str,
    bindings: Vec<EndpointVariantBindingDescription>,
) -> (String, SchemaDefinitionBody) {
    (
        endpoint_name.to_owned(),
        SchemaDefinitionBody::Endpoint(EndpointDescription {
            name: endpoint_name.to_owned(),
            family: family.to_owned(),
            template: template.to_owned(),
            bindings,
            protocol_version: description.protocol_version,
            json_envelope_version: description.json_envelope_version,
            boundary_error_envelope: "boundary_error_envelope".to_owned(),
            typed_error: "error".to_owned(),
            id_formats: "id_formats".to_owned(),
            limits: "limits".to_owned(),
        }),
    )
}

fn endpoint_binding(
    parameter: &str,
    variant: VariantPayloadDescription,
) -> EndpointVariantBindingDescription {
    EndpointVariantBindingDescription {
        parameter: parameter.to_owned(),
        variant,
    }
}

pub(crate) fn schema_definition_catalogue(
    description: &SchemaDescription,
) -> Result<BTreeMap<String, SchemaDefinition>, String> {
    let mut bodies = BTreeMap::<String, SchemaDefinitionBody>::new();
    let mut insert = |name: String, body: SchemaDefinitionBody| -> Result<(), String> {
        if bodies.insert(name.clone(), body).is_some() {
            return Err(format!("duplicate machine schema definition: {name}"));
        }
        Ok(())
    };

    for scalar in &description.scalar_types {
        insert(
            scalar.name.clone(),
            SchemaDefinitionBody::Scalar(scalar.clone()),
        )?;
    }
    for record in description
        .semantic_records
        .iter()
        .chain(description.transaction_records.iter())
        .chain(description.query_records.iter())
        .chain(description.run.records.iter())
        .chain(description.error_records.iter())
        .chain(description.envelopes.iter())
        .chain(description.schema_discovery.records.iter())
    {
        insert(
            record.name.clone(),
            SchemaDefinitionBody::Record(record.clone()),
        )?;
    }
    insert(
        "describe_schema_request".to_owned(),
        SchemaDefinitionBody::Record(NamedPayloadDescription {
            name: "describe_schema_request".to_owned(),
            payload: description.schema_discovery.request.clone(),
        }),
    )?;
    insert(
        "error".to_owned(),
        SchemaDefinitionBody::Record(NamedPayloadDescription {
            name: "error".to_owned(),
            payload: description.error_payload.clone(),
        }),
    )?;
    for variant in description
        .semantic_variants
        .iter()
        .chain(description.transaction_variants.iter())
        .chain(description.query_variants.iter())
        .chain(description.run.variants.iter())
        .chain(description.error_variants.iter())
        .chain(description.identity_variants.iter())
        .chain(description.schema_discovery.variants.iter())
    {
        insert(
            variant.name.clone(),
            SchemaDefinitionBody::Variant(variant.clone()),
        )?;
    }
    for template in endpoint_protocol_templates(description)? {
        insert(
            template.name.clone(),
            SchemaDefinitionBody::EndpointTemplate(template),
        )?;
    }
    for (request_code, response_code) in [
        (RequestCode::CreateWorkspace, ResponseCode::WorkspaceCreated),
        (
            RequestCode::ApplyTransaction,
            ResponseCode::TransactionReceipt,
        ),
        (RequestCode::Run, ResponseCode::Run),
        (RequestCode::Shutdown, ResponseCode::Acknowledged),
        (RequestCode::DescribeSchema, ResponseCode::DescribeSchema),
    ] {
        let (name, body) = endpoint_definition(
            description,
            request_code.machine_name(),
            "control",
            "control_endpoint_protocol",
            vec![
                endpoint_binding(
                    "request_variant",
                    variant_payload_by_name(
                        &description.request_payloads,
                        request_code.machine_name(),
                    )?,
                ),
                endpoint_binding(
                    "success_response_variant",
                    variant_payload_by_name(
                        &description.response_payloads,
                        response_code.machine_name(),
                    )?,
                ),
            ],
        );
        insert(name, body)?;
    }
    for query_code in QueryCode::ALL {
        let endpoint_name = format!("query_{}", query_code.machine_name());
        let (name, body) = endpoint_definition(
            description,
            &endpoint_name,
            "query",
            "query_endpoint_protocol",
            vec![
                endpoint_binding(
                    "query_variant",
                    variant_payload_by_name(
                        &description.query_payloads,
                        query_code.machine_name(),
                    )?,
                ),
                endpoint_binding(
                    "query_result_variant",
                    variant_payload_by_name(
                        &description.query_result_payloads,
                        query_code.machine_name(),
                    )?,
                ),
            ],
        );
        insert(name, body)?;
    }
    for record in &description.structured_authoring.records {
        insert(
            record.name.clone(),
            SchemaDefinitionBody::DraftRecord(record.clone()),
        )?;
    }
    for family in [
        DraftVariantFamilyDescription {
            name: "expression_kind_draft".to_owned(),
            tagging: description.structured_authoring.expression_tagging.clone(),
            variants: description.structured_authoring.expression_variants.clone(),
        },
        DraftVariantFamilyDescription {
            name: "operation_draft".to_owned(),
            tagging: description.structured_authoring.operation_tagging.clone(),
            variants: description.structured_authoring.operation_variants.clone(),
        },
        DraftVariantFamilyDescription {
            name: "value_draft".to_owned(),
            tagging: description.structured_authoring.value_tagging.clone(),
            variants: description.structured_authoring.value_variants.clone(),
        },
        DraftVariantFamilyDescription {
            name: "type_draft".to_owned(),
            tagging: description.structured_authoring.type_tagging.clone(),
            variants: description.structured_authoring.type_variants.clone(),
        },
    ] {
        insert(
            family.name.clone(),
            SchemaDefinitionBody::DraftVariant(family),
        )?;
    }

    let code_family = |name: &str, codes: &[CodeDescription]| CodeFamilyDescription {
        name: name.to_owned(),
        members: codes.iter().map(|code| code.name.clone()).collect(),
    };
    for family in [
        code_family("node_kind", &description.node_kinds),
        CodeFamilyDescription {
            name: "node_identity_class".to_owned(),
            members: vec!["durable".to_owned(), "function_local".to_owned()],
        },
        CodeFamilyDescription {
            name: "operation_code".to_owned(),
            members: description
                .operations
                .iter()
                .map(|operation| operation.name.clone())
                .collect(),
        },
        code_family(
            "transaction_operation_code",
            &description.transaction_operations,
        ),
        code_family("error_code", &description.errors),
    ] {
        insert(family.name.clone(), SchemaDefinitionBody::Codes(family))?;
    }
    insert(
        "operations".to_owned(),
        SchemaDefinitionBody::Operations(description.operations.clone()),
    )?;
    insert(
        "structured_authoring".to_owned(),
        SchemaDefinitionBody::StructuredAuthoring(StructuredAuthoringPolicyDescription {
            allocation_order: description.structured_authoring.allocation_order.clone(),
            inline_expression_variants: description
                .structured_authoring
                .inline_expression_variants
                .clone(),
            inline_holes_allowed: description.structured_authoring.inline_holes_allowed,
            inline_region_operations_allowed: description
                .structured_authoring
                .inline_region_operations_allowed,
            maintenance_accepts_inline_values: description
                .structured_authoring
                .maintenance_accepts_inline_values,
            nesting_metric: description.structured_authoring.nesting_metric.clone(),
            explicit_symbols_are_selectable: description
                .structured_authoring
                .explicit_symbols_are_selectable,
            implicit_symbols_are_selectable: description
                .structured_authoring
                .implicit_symbols_are_selectable,
            implicit_node_kinds: description.structured_authoring.implicit_node_kinds.clone(),
            maximum_request_depth: description.structured_authoring.maximum_request_depth,
            maximum_request_items: description.structured_authoring.maximum_request_items,
            counted_item_categories: description
                .structured_authoring
                .counted_item_categories
                .clone(),
        }),
    )?;
    insert(
        "name_contract".to_owned(),
        SchemaDefinitionBody::NameContract(description.name_contract.clone()),
    )?;
    insert(
        "nominal_declarations".to_owned(),
        SchemaDefinitionBody::NominalDeclarations(description.nominal_declarations.clone()),
    )?;
    insert(
        "id_formats".to_owned(),
        SchemaDefinitionBody::IdFormats(description.id_formats.clone()),
    )?;
    insert(
        "limits".to_owned(),
        SchemaDefinitionBody::Limits(description.limits.clone()),
    )?;

    for (name, body) in &bodies {
        let SchemaDefinitionBody::Endpoint(endpoint) = body else {
            continue;
        };
        let Some(SchemaDefinitionBody::EndpointTemplate(template)) = bodies.get(&endpoint.template)
        else {
            return Err(format!(
                "endpoint {name} references unknown template {}",
                endpoint.template
            ));
        };
        let expected = template
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<BTreeSet<_>>();
        let actual = endpoint
            .bindings
            .iter()
            .map(|binding| binding.parameter.as_str())
            .collect::<BTreeSet<_>>();
        if actual.len() != endpoint.bindings.len() || actual != expected {
            return Err(format!(
                "endpoint {name} must bind every template parameter exactly once"
            ));
        }
    }

    let names = bodies.keys().cloned().collect::<BTreeSet<_>>();
    let mut catalogue = BTreeMap::new();
    for (name, body) in bodies {
        let dependencies = definition_dependencies(&body)?;
        for dependency in &dependencies {
            if !names.contains(dependency) {
                return Err(format!(
                    "machine schema definition {name} references unknown definition {dependency}"
                ));
            }
        }
        catalogue.insert(
            name.clone(),
            SchemaDefinition {
                name,
                dependencies: dependencies.into_iter().collect(),
                body,
            },
        );
    }
    for root in SchemaRoot::ALL {
        if !catalogue.contains_key(root.machine_name()) {
            return Err(format!(
                "machine schema root {} has no definition",
                root.machine_name()
            ));
        }
    }
    Ok(catalogue)
}

pub(crate) fn definition_dependencies(
    body: &SchemaDefinitionBody,
) -> Result<BTreeSet<String>, String> {
    let mut dependencies = BTreeSet::new();
    match body {
        SchemaDefinitionBody::Scalar(_)
        | SchemaDefinitionBody::Codes(_)
        | SchemaDefinitionBody::IdFormats(_)
        | SchemaDefinitionBody::Limits(_) => {}
        SchemaDefinitionBody::Record(record) => {
            payload_dependencies(&record.payload, &mut dependencies)?;
        }
        SchemaDefinitionBody::Variant(variant) => {
            for payload in &variant.variants {
                payload_dependencies(&payload.payload, &mut dependencies)?;
            }
        }
        SchemaDefinitionBody::DraftRecord(record) => {
            for field in &record.fields {
                dependencies.extend(type_expression_dependencies(
                    field.field_type.type_expression(),
                )?);
            }
        }
        SchemaDefinitionBody::Endpoint(endpoint) => {
            dependencies.extend(
                [
                    &endpoint.template,
                    &endpoint.boundary_error_envelope,
                    &endpoint.typed_error,
                    &endpoint.id_formats,
                    &endpoint.limits,
                ]
                .into_iter()
                .cloned(),
            );
            for binding in &endpoint.bindings {
                payload_dependencies(&binding.variant.payload, &mut dependencies)?;
            }
        }
        SchemaDefinitionBody::EndpointTemplate(template) => {
            let mut local_names = BTreeSet::new();
            for name in template
                .records
                .iter()
                .map(|record| &record.name)
                .chain(template.variants.iter().map(|variant| &variant.name))
            {
                if !local_names.insert(name.clone()) {
                    return Err(format!(
                        "duplicate endpoint template local definition: {name}"
                    ));
                }
            }
            let mut parameter_names = BTreeSet::new();
            for parameter in &template.parameters {
                if !parameter_names.insert(parameter.name.clone()) {
                    return Err(format!(
                        "duplicate endpoint template parameter: {}",
                        parameter.name
                    ));
                }
                if !template
                    .variants
                    .iter()
                    .any(|variant| variant.name == parameter.target_variant)
                {
                    return Err(format!(
                        "endpoint template parameter {} targets unknown local variant {}",
                        parameter.name, parameter.target_variant
                    ));
                }
            }
            for record in &template.records {
                payload_dependencies(&record.payload, &mut dependencies)?;
            }
            for variant in &template.variants {
                for payload in &variant.variants {
                    payload_dependencies(&payload.payload, &mut dependencies)?;
                }
            }
            dependencies.retain(|dependency| !local_names.contains(dependency));
        }
        SchemaDefinitionBody::DraftVariant(family) => {
            for variant in &family.variants {
                if let Some(field_type) = variant.newtype {
                    dependencies
                        .extend(type_expression_dependencies(field_type.type_expression())?);
                }
                for field in &variant.fields {
                    dependencies.extend(type_expression_dependencies(
                        field.field_type.type_expression(),
                    )?);
                }
            }
            dependencies.insert("structured_authoring".to_owned());
        }
        SchemaDefinitionBody::Operations(_) => {
            dependencies.extend(
                [
                    "operation_code",
                    "operand_arity",
                    "operand_use",
                    "literal_field",
                    "region_arity",
                    "region_role",
                    "block_argument_role",
                    "type_rule",
                ]
                .into_iter()
                .map(str::to_owned),
            );
        }
        SchemaDefinitionBody::StructuredAuthoring(_) => {
            dependencies.insert("node_kind".to_owned());
        }
        SchemaDefinitionBody::NameContract(_) | SchemaDefinitionBody::NominalDeclarations(_) => {
            dependencies.insert("node_kind".to_owned());
        }
    }
    Ok(dependencies)
}

fn payload_dependencies(
    payload: &PayloadShapeDescription,
    dependencies: &mut BTreeSet<String>,
) -> Result<(), String> {
    if let Some(newtype) = &payload.newtype {
        dependencies.extend(type_expression_dependencies(newtype)?);
    }
    for field in &payload.fields {
        dependencies.extend(type_expression_dependencies(&field.type_expression)?);
    }
    Ok(())
}

pub(crate) fn type_expression_dependencies(expression: &str) -> Result<BTreeSet<String>, String> {
    #[derive(Clone, Copy)]
    struct Frame {
        constructor: &'static str,
        items: usize,
    }
    fn constructor(name: &str) -> Option<&'static str> {
        match name {
            "list" => Some("list"),
            "optional" => Some("optional"),
            "tuple" => Some("tuple"),
            "page" => Some("page"),
            _ => None,
        }
    }

    let bytes = expression.as_bytes();
    if bytes.is_empty() || !bytes.is_ascii() {
        return Err(format!("invalid machine type expression: {expression}"));
    }
    let mut dependencies = BTreeSet::new();
    let mut stack = Vec::<Frame>::new();
    let mut index = 0;
    let mut expect_type = true;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_lowercase() {
            if !expect_type {
                return Err(format!("invalid machine type expression: {expression}"));
            }
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_lowercase()
                    || bytes[index].is_ascii_digit()
                    || bytes[index] == b'_')
            {
                index += 1;
            }
            let name = &expression[start..index];
            if let Some(constructor) = constructor(name) {
                if index >= bytes.len() || bytes[index] != b'<' {
                    return Err(format!("invalid machine type expression: {expression}"));
                }
                if constructor == "page" {
                    dependencies.insert("page".to_owned());
                }
                stack.push(Frame {
                    constructor,
                    items: 0,
                });
                index += 1;
                expect_type = true;
            } else {
                if name != "type_parameter" {
                    dependencies.insert(name.to_owned());
                }
                expect_type = false;
            }
            continue;
        }
        match byte {
            b',' if !expect_type && !stack.is_empty() => {
                let Some(frame) = stack.last_mut() else {
                    return Err(format!("invalid machine type expression: {expression}"));
                };
                frame.items += 1;
                if frame.constructor != "tuple" {
                    return Err(format!("invalid machine type expression: {expression}"));
                }
                expect_type = true;
                index += 1;
            }
            b'>' if !expect_type && !stack.is_empty() => {
                let Some(mut frame) = stack.pop() else {
                    return Err(format!("invalid machine type expression: {expression}"));
                };
                frame.items += 1;
                if frame.constructor != "tuple" && frame.items != 1 {
                    return Err(format!("invalid machine type expression: {expression}"));
                }
                expect_type = false;
                index += 1;
            }
            _ => return Err(format!("invalid machine type expression: {expression}")),
        }
    }
    if expect_type || !stack.is_empty() {
        return Err(format!("invalid machine type expression: {expression}"));
    }
    Ok(dependencies)
}

fn described(name: &str) -> CodeDescription {
    CodeDescription {
        name: name.to_owned(),
    }
}
