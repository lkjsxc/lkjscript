use crate::schema::{
    BlockArgumentRole, LiteralField, NodeKind, OperandArity, OperandUse, OperationCode,
    RegionArity, RegionRole, TypeRule,
};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

pub const MAX_SCHEMA_ROOTS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MachineSchemaDigest([u8; 32]);

impl MachineSchemaDigest {
    pub const BYTE_LEN: usize = 32;

    pub const fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; Self::BYTE_LEN] {
        self.0
    }
}

impl fmt::Display for MachineSchemaDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = [0_u8; 64];
        for (index, byte) in self.0.iter().copied().enumerate() {
            output[index * 2] = HEX[usize::from(byte >> 4)];
            output[index * 2 + 1] = HEX[usize::from(byte & 0x0f)];
        }
        formatter.write_str(std::str::from_utf8(&output).map_err(|_| fmt::Error)?)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachineSchemaDigestParseError;

impl fmt::Display for MachineSchemaDigestParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("machine schema digest must be exactly 64 lowercase hexadecimal characters")
    }
}

impl std::error::Error for MachineSchemaDigestParseError {}

impl FromStr for MachineSchemaDigest {
    type Err = MachineSchemaDigestParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64 {
            return Err(MachineSchemaDigestParseError);
        }
        let mut bytes = [0_u8; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            let digit = |value: u8| match value {
                b'0'..=b'9' => Some(value - b'0'),
                b'a'..=b'f' => Some(value - b'a' + 10),
                _ => None,
            };
            let high = digit(pair[0]).ok_or(MachineSchemaDigestParseError)?;
            let low = digit(pair[1]).ok_or(MachineSchemaDigestParseError)?;
            bytes[index] = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

impl Serialize for MachineSchemaDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MachineSchemaDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DigestVisitor;
        impl Visitor<'_> for DigestVisitor {
            type Value = MachineSchemaDigest;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a canonical lowercase machine schema digest")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value.parse().map_err(E::custom)
            }
        }
        deserializer.deserialize_str(DigestVisitor)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaRoot {
    Request,
    Response,
    CreateWorkspace,
    ApplyTransaction,
    Run,
    Shutdown,
    DescribeSchema,
    ApplyTransactionRequest,
    TransactionReceipt,
    TransactionOperation,
    ExpressionKindDraft,
    OperationDraft,
    ValueDraft,
    TypeDraft,
    Query,
    QueryResult,
    QueryWorkspaceSummary,
    QueryNode,
    QueryBlockers,
    QueryOwnerChain,
    QueryBody,
    QueryIncomingUses,
    QueryDefinitionReferences,
    QueryDependencies,
    QueryVisibleValues,
    QueryLegalConstructors,
    QueryRepairContext,
    QuerySemanticDiff,
    QueryNominalType,
    RuntimeValue,
    Error,
    Node,
    Operations,
    NominalDeclarations,
    IdFormats,
    Limits,
    DescribeSchemaRequest,
    DescribeSchemaResult,
}

impl SchemaRoot {
    pub const ALL: [Self; 38] = [
        Self::Request,
        Self::Response,
        Self::CreateWorkspace,
        Self::ApplyTransaction,
        Self::Run,
        Self::Shutdown,
        Self::DescribeSchema,
        Self::ApplyTransactionRequest,
        Self::TransactionReceipt,
        Self::TransactionOperation,
        Self::ExpressionKindDraft,
        Self::OperationDraft,
        Self::ValueDraft,
        Self::TypeDraft,
        Self::Query,
        Self::QueryResult,
        Self::QueryWorkspaceSummary,
        Self::QueryNode,
        Self::QueryBlockers,
        Self::QueryOwnerChain,
        Self::QueryBody,
        Self::QueryIncomingUses,
        Self::QueryDefinitionReferences,
        Self::QueryDependencies,
        Self::QueryVisibleValues,
        Self::QueryLegalConstructors,
        Self::QueryRepairContext,
        Self::QuerySemanticDiff,
        Self::QueryNominalType,
        Self::RuntimeValue,
        Self::Error,
        Self::Node,
        Self::Operations,
        Self::NominalDeclarations,
        Self::IdFormats,
        Self::Limits,
        Self::DescribeSchemaRequest,
        Self::DescribeSchemaResult,
    ];

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Response => "response",
            Self::CreateWorkspace => "create_workspace",
            Self::ApplyTransaction => "apply_transaction",
            Self::Run => "run",
            Self::Shutdown => "shutdown",
            Self::DescribeSchema => "describe_schema",
            Self::ApplyTransactionRequest => "apply_transaction_request",
            Self::TransactionReceipt => "transaction_receipt",
            Self::TransactionOperation => "transaction_operation",
            Self::ExpressionKindDraft => "expression_kind_draft",
            Self::OperationDraft => "operation_draft",
            Self::ValueDraft => "value_draft",
            Self::TypeDraft => "type_draft",
            Self::Query => "query",
            Self::QueryResult => "query_result",
            Self::QueryWorkspaceSummary => "query_workspace_summary",
            Self::QueryNode => "query_node",
            Self::QueryBlockers => "query_blockers",
            Self::QueryOwnerChain => "query_owner_chain",
            Self::QueryBody => "query_body",
            Self::QueryIncomingUses => "query_incoming_uses",
            Self::QueryDefinitionReferences => "query_definition_references",
            Self::QueryDependencies => "query_dependencies",
            Self::QueryVisibleValues => "query_visible_values",
            Self::QueryLegalConstructors => "query_legal_constructors",
            Self::QueryRepairContext => "query_repair_context",
            Self::QuerySemanticDiff => "query_semantic_diff",
            Self::QueryNominalType => "query_nominal_type",
            Self::RuntimeValue => "runtime_value",
            Self::Error => "error",
            Self::Node => "node",
            Self::Operations => "operations",
            Self::NominalDeclarations => "nominal_declarations",
            Self::IdFormats => "id_formats",
            Self::Limits => "limits",
            Self::DescribeSchemaRequest => "describe_schema_request",
            Self::DescribeSchemaResult => "describe_schema_result",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SchemaProjection {
    Manifest,
    Roots { roots: Vec<SchemaRoot> },
    Full,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DescribeSchemaRequest {
    pub projection: SchemaProjection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_digest: Option<MachineSchemaDigest>,
}

impl DescribeSchemaRequest {
    pub fn manifest() -> Self {
        Self {
            projection: SchemaProjection::Manifest,
            known_digest: None,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if let SchemaProjection::Roots { roots } = &self.projection {
            if roots.is_empty() {
                return Err("schema root list must not be empty");
            }
            if roots.len() > MAX_SCHEMA_ROOTS {
                return Err("schema root count exceeds policy");
            }
            let mut canonical = roots.clone();
            canonical.sort_unstable();
            if canonical.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err("schema root list contains a duplicate");
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DescribeSchemaResult {
    Unchanged {
        digest: MachineSchemaDigest,
    },
    Manifest(SchemaManifest),
    Roots(SchemaDefinitions),
    Full {
        digest: MachineSchemaDigest,
        description: Box<SchemaDescription>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaManifest {
    pub schema_identity: String,
    pub digest: MachineSchemaDigest,
    pub protocol_version: u16,
    pub json_envelope_version: u16,
    pub artifact_format_version: u16,
    pub artifact_magic_hex: String,
    pub semantic_schema_identity: String,
    pub roots: Vec<String>,
    pub type_constructors: Vec<String>,
    pub maximum_roots_per_request: u8,
    pub full_available: bool,
    pub maximum_request_frame_bytes: u64,
    pub maximum_response_frame_bytes: u64,
    pub maximum_json_output_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaDefinitions {
    pub digest: MachineSchemaDigest,
    pub roots: Vec<SchemaRoot>,
    pub type_constructors: Vec<String>,
    pub definitions: Vec<SchemaDefinition>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaDefinition {
    pub name: String,
    pub dependencies: Vec<String>,
    pub body: SchemaDefinitionBody,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SchemaDefinitionBody {
    Scalar(MachineScalarDescription),
    Record(NamedPayloadDescription),
    Variant(NamedVariantDescription),
    DraftRecord(DraftRecordDescription),
    DraftVariant(DraftVariantFamilyDescription),
    Endpoint(EndpointDescription),
    EndpointTemplate(EndpointProtocolTemplateDescription),
    Codes(CodeFamilyDescription),
    Operations(Vec<OperationDescription>),
    StructuredAuthoring(StructuredAuthoringPolicyDescription),
    NameContract(NameContractDescription),
    NominalDeclarations(NominalDeclarationsDescription),
    IdFormats(IdFormats),
    Limits(BoundaryLimits),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftVariantFamilyDescription {
    pub name: String,
    pub tagging: String,
    pub variants: Vec<DraftVariantDescription>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointDescription {
    pub name: String,
    pub family: String,
    pub template: String,
    pub bindings: Vec<EndpointVariantBindingDescription>,
    pub protocol_version: u16,
    pub json_envelope_version: u16,
    pub boundary_error_envelope: String,
    pub typed_error: String,
    pub id_formats: String,
    pub limits: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointVariantBindingDescription {
    pub parameter: String,
    pub variant: VariantPayloadDescription,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointProtocolTemplateDescription {
    pub name: String,
    pub parameters: Vec<EndpointTemplateParameterDescription>,
    pub records: Vec<NamedPayloadDescription>,
    pub variants: Vec<NamedVariantDescription>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointTemplateParameterDescription {
    pub name: String,
    pub target_variant: String,
    pub semantics: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodeFamilyDescription {
    pub name: String,
    pub members: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredAuthoringPolicyDescription {
    pub allocation_order: String,
    pub inline_expression_variants: Vec<String>,
    pub inline_holes_allowed: bool,
    pub inline_region_operations_allowed: bool,
    pub maintenance_accepts_inline_values: bool,
    pub nesting_metric: String,
    pub explicit_symbols_are_selectable: bool,
    pub implicit_symbols_are_selectable: bool,
    pub implicit_node_kinds: Vec<NodeKind>,
    pub maximum_request_depth: u32,
    pub maximum_request_items: u64,
    pub counted_item_categories: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaDescription {
    pub machine_schema_identity: String,
    pub protocol_version: u16,
    pub json_envelope_version: u16,
    pub artifact_format_version: u16,
    pub artifact_magic_hex: String,
    pub semantic_schema_identity: String,
    pub schema_discovery: SchemaDiscoveryDescription,
    pub scalar_types: Vec<MachineScalarDescription>,
    pub semantic_types: Vec<CodeDescription>,
    pub node_kinds: Vec<CodeDescription>,
    pub name_contract: NameContractDescription,
    pub operations: Vec<OperationDescription>,
    pub semantic_records: Vec<NamedPayloadDescription>,
    pub semantic_variants: Vec<NamedVariantDescription>,
    pub transaction_operations: Vec<CodeDescription>,
    pub transaction_operation_payloads: Vec<VariantPayloadDescription>,
    pub transaction_records: Vec<NamedPayloadDescription>,
    pub transaction_variants: Vec<NamedVariantDescription>,
    pub structured_authoring: StructuredAuthoringDescription,
    pub run: RunDescription,
    pub queries: Vec<CodeDescription>,
    pub query_payloads: Vec<VariantPayloadDescription>,
    pub query_result_payloads: Vec<VariantPayloadDescription>,
    pub query_records: Vec<NamedPayloadDescription>,
    pub query_variants: Vec<NamedVariantDescription>,
    pub query_member_payloads: Vec<VariantPayloadDescription>,
    pub query_cursor_payloads: Vec<VariantPayloadDescription>,
    pub errors: Vec<CodeDescription>,
    pub error_payload: PayloadShapeDescription,
    pub error_records: Vec<NamedPayloadDescription>,
    pub error_variants: Vec<NamedVariantDescription>,
    pub requests: Vec<CodeDescription>,
    pub request_payloads: Vec<VariantPayloadDescription>,
    pub responses: Vec<CodeDescription>,
    pub response_payloads: Vec<VariantPayloadDescription>,
    pub identity_variants: Vec<NamedVariantDescription>,
    pub envelopes: Vec<NamedPayloadDescription>,
    pub boundary_error_kinds: Vec<String>,
    pub limits: BoundaryLimits,
    pub id_formats: IdFormats,
    pub nominal_declarations: NominalDeclarationsDescription,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaDiscoveryDescription {
    pub digest_format: String,
    pub digest_domain: String,
    pub request: PayloadShapeDescription,
    pub records: Vec<NamedPayloadDescription>,
    pub variants: Vec<NamedVariantDescription>,
    pub projection_payloads: Vec<VariantPayloadDescription>,
    pub result_payloads: Vec<VariantPayloadDescription>,
    pub roots: Vec<String>,
    pub type_constructors: Vec<String>,
    pub maximum_roots_per_request: u8,
    pub full_available: bool,
    pub known_digest_match_follows_root_validation: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NominalDeclarationsDescription {
    pub declaration_kinds: Vec<NodeKind>,
    pub member_kinds: Vec<NodeKind>,
    pub shape_invariants: Vec<String>,
    pub layout_invariants: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodeDescription {
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NameContractDescription {
    pub named_node_kinds: Vec<NodeKind>,
    pub minimum_utf8_bytes: u64,
    pub maximum_utf8_bytes: u64,
    pub sibling_uniqueness_groups: Vec<NameUniquenessGroupDescription>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NameUniquenessGroupDescription {
    pub name: String,
    pub owner_kind: NodeKind,
    pub member_kinds: Vec<NodeKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonScalarKind {
    Boolean,
    Number,
    String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MachineScalarDomain {
    Boolean,
    Utf8String,
    CanonicalIdentifier {
        grammar: String,
        minimum_utf8_bytes: u64,
        maximum_utf8_bytes: u64,
    },
    SignedInteger {
        minimum: i64,
        maximum: i64,
    },
    UnsignedInteger {
        minimum: u64,
        maximum: u64,
    },
    LowercaseHex {
        encoded_bytes: u8,
    },
    NodeId {
        workspace_bytes: u8,
        minimum_serial: u64,
        maximum_serial: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MachineScalarDescription {
    pub name: String,
    pub json_kind: JsonScalarKind,
    pub domain: MachineScalarDomain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadShapeKind {
    Unit,
    Newtype,
    Record,
}
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MachineFieldDescription {
    pub name: String,
    pub type_expression: String,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadShapeDescription {
    pub shape: PayloadShapeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub newtype: Option<String>,
    pub fields: Vec<MachineFieldDescription>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VariantPayloadDescription {
    pub name: String,
    pub payload: PayloadShapeDescription,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NamedPayloadDescription {
    pub name: String,
    pub payload: PayloadShapeDescription,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NamedVariantDescription {
    pub name: String,
    pub tagging: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_field: Option<String>,
    pub variants: Vec<VariantPayloadDescription>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunFieldType {
    Workspace,
    Revision,
    Node,
    RuntimeValueList,
    RunPolicy,
    U64,
    U32,
}
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunFieldDescription {
    pub name: String,
    pub field_type: RunFieldType,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunDescription {
    pub fields: Vec<RunFieldDescription>,
    pub policy_fields: Vec<RunFieldDescription>,
    pub runtime_values: Vec<RuntimeValueDescription>,
    pub records: Vec<NamedPayloadDescription>,
    pub variants: Vec<NamedVariantDescription>,
    pub limit_scope: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeValuePayload {
    None,
    Bool,
    I64,
    Product,
    Sum,
}
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeValueDescription {
    pub name: String,
    pub payload: RuntimeValuePayload,
    pub fields: Vec<MachineFieldDescription>,
    pub invariants: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredAuthoringDescription {
    pub draft_field_types: Vec<DraftFieldTypeDescription>,
    pub records: Vec<DraftRecordDescription>,
    pub expression_variants: Vec<DraftVariantDescription>,
    pub operation_variants: Vec<DraftVariantDescription>,
    pub value_variants: Vec<DraftVariantDescription>,
    pub type_variants: Vec<DraftVariantDescription>,
    pub expression_tagging: String,
    pub operation_tagging: String,
    pub value_tagging: String,
    pub type_tagging: String,
    pub allocation_order: String,
    pub inline_expression_variants: Vec<String>,
    pub inline_holes_allowed: bool,
    pub inline_region_operations_allowed: bool,
    pub maintenance_accepts_inline_values: bool,
    pub nesting_metric: String,
    pub explicit_symbols_are_selectable: bool,
    pub implicit_symbols_are_selectable: bool,
    pub implicit_node_kinds: Vec<NodeKind>,
    pub maximum_request_depth: u32,
    pub maximum_request_items: u64,
    pub counted_item_categories: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftFieldType {
    DraftSymbol,
    NodeTarget,
    NodeId,
    String,
    SemanticType,
    I64,
    U8,
    Value,
    ValueList,
    ExpressionKind,
    ExpressionList,
    ParameterList,
    FunctionBody,
    YieldingBody,
    Bool,
    Expression,
    TypeDraft,
    ProductFieldList,
    SumVariantList,
    ProductFieldValueList,
    MatchArmList,
    OperationMatchArmList,
}
impl DraftFieldType {
    pub const ALL: [Self; 22] = [
        Self::DraftSymbol,
        Self::NodeTarget,
        Self::NodeId,
        Self::String,
        Self::SemanticType,
        Self::I64,
        Self::U8,
        Self::Value,
        Self::ValueList,
        Self::ExpressionKind,
        Self::ExpressionList,
        Self::ParameterList,
        Self::FunctionBody,
        Self::YieldingBody,
        Self::Bool,
        Self::Expression,
        Self::TypeDraft,
        Self::ProductFieldList,
        Self::SumVariantList,
        Self::ProductFieldValueList,
        Self::MatchArmList,
        Self::OperationMatchArmList,
    ];

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::DraftSymbol => "draft_symbol",
            Self::NodeTarget => "node_target",
            Self::NodeId => "node_id",
            Self::String => "string",
            Self::SemanticType => "semantic_type",
            Self::I64 => "i64",
            Self::U8 => "u8",
            Self::Value => "value",
            Self::ValueList => "value_list",
            Self::ExpressionKind => "expression_kind",
            Self::ExpressionList => "expression_list",
            Self::ParameterList => "parameter_list",
            Self::FunctionBody => "function_body",
            Self::YieldingBody => "yielding_body",
            Self::Bool => "bool",
            Self::Expression => "expression",
            Self::TypeDraft => "type_draft",
            Self::ProductFieldList => "product_field_list",
            Self::SumVariantList => "sum_variant_list",
            Self::ProductFieldValueList => "product_field_value_list",
            Self::MatchArmList => "match_arm_list",
            Self::OperationMatchArmList => "operation_match_arm_list",
        }
    }

    pub const fn type_expression(self) -> &'static str {
        match self {
            Self::DraftSymbol => "draft_symbol",
            Self::NodeTarget => "node_target",
            Self::NodeId => "node_id",
            Self::String => "string",
            Self::SemanticType => "semantic_type",
            Self::I64 => "i64",
            Self::U8 => "u8",
            Self::Value => "value_draft",
            Self::ValueList => "list<value_draft>",
            Self::ExpressionKind => "expression_kind_draft",
            Self::ExpressionList => "list<expression>",
            Self::ParameterList => "list<function_parameter>",
            Self::FunctionBody => "function_body",
            Self::YieldingBody => "yielding_body",
            Self::Bool => "bool",
            Self::Expression => "expression",
            Self::TypeDraft => "type_draft",
            Self::ProductFieldList => "list<product_field>",
            Self::SumVariantList => "list<sum_variant>",
            Self::ProductFieldValueList => "list<product_field_value>",
            Self::MatchArmList => "list<match_arm>",
            Self::OperationMatchArmList => "list<operation_match_arm>",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftFieldTypeDescription {
    pub name: String,
    pub type_expression: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftFieldDescription {
    pub name: String,
    pub field_type: DraftFieldType,
    pub required: bool,
    pub nullable: bool,
    pub declares_symbol: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftRecordDescription {
    pub name: String,
    pub fields: Vec<DraftFieldDescription>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftVariantDescription {
    pub name: String,
    pub shape: PayloadShapeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub newtype: Option<DraftFieldType>,
    pub fields: Vec<DraftFieldDescription>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationDescription {
    pub name: String,
    pub operand_arity: OperandArity,
    pub operands: Vec<OperandDescription>,
    pub results: Vec<TypeRule>,
    pub literal_fields: Vec<LiteralField>,
    pub region_arity: RegionArity,
    pub regions: Vec<RegionDescription>,
    pub complete: bool,
    pub terminator: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionDescription {
    pub role: RegionRole,
    pub block_arguments: Vec<BlockArgumentDescription>,
    pub terminator: OperationCode,
    pub yield_type: TypeRule,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BlockArgumentDescription {
    pub role: BlockArgumentRole,
    pub ty: TypeRule,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperandDescription {
    pub ty: TypeRule,
    pub use_mode: OperandUse,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryLimits {
    pub maximum_request_frame_bytes: u64,
    pub maximum_response_frame_bytes: u64,
    pub maximum_artifact_bytes: u64,
    pub maximum_artifact_name_bytes: u64,
    pub maximum_json_input_bytes: u64,
    pub maximum_json_output_bytes: u64,
    pub maximum_page_items: u32,
    pub maximum_batch_queries: u32,
    pub maximum_batch_items: u32,
    pub maximum_context_items_per_category: u32,
    pub maximum_returned_bindings: u32,
    pub maximum_run_arguments: u32,
    pub maximum_run_fuel: u64,
    pub maximum_run_frames: u32,
    pub maximum_run_live_cells: u64,
    pub maximum_runtime_value_depth: u32,
    pub maximum_runtime_value_items: u64,
    pub maximum_runtime_value_bytes: u64,
    pub maximum_error_related_ids: u32,
    pub maximum_boundary_error_message_bytes: u64,
    pub maximum_persistence_head_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdFormats {
    pub workspace: String,
    pub idempotency_key: String,
    pub node: String,
    pub snapshot_hash: String,
    pub change_digest: String,
    pub revision: String,
    pub request_id: String,
    pub query_id: String,
    pub draft_symbol: String,
    pub machine_schema_digest: String,
}
