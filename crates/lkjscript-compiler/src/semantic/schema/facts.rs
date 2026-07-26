use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FactSchema {
    Binding,
    Hir,
    StaticType,
    SemanticEffects,
    OwnershipPlaceLoan,
    ControlFlowGraph,
    SsaValuesBlocks,
    FrameStatesSafepointsRoots,
    Layout,
    Proof,
    Bytecode,
    NativeLocation,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FactCertainty {
    Guaranteed,
    Conditional,
    Informational,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RelationCardinality {
    Zero,
    One,
    Many,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProducerStage {
    SourceResolution,
    Hir,
    Ownership,
    Ssa,
    Runtime,
    ProofChecker,
    Bytecode,
    Native,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProducerRecord {
    pub component: String,
    pub stage: ProducerStage,
    pub build: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SemanticEffect {
    Allocates,
    ReadsMemory,
    WritesMemory,
    MutatesLocal,
    HostIo,
    MayTrap,
    MayExit,
    MayDiverge,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum FactValue {
    StaticType { canonical: String },
    SemanticEffects { effects: Vec<SemanticEffect> },
    OwnershipState { state: String },
    LayoutSizeAlign { size: u64, align: u64 },
    NativeLocation { section: String, offset: u64 },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum FactReference {
    Declaration { key: String },
    HirExpression { function: String, expression: u32 },
    Place { function: String, place: u32 },
    Loan { function: String, loan: u32 },
    CfgBlock { function: String, block: u32 },
    SsaValue { function: String, value: u32 },
    SsaBlock { function: String, block: u32 },
    FrameState { function: String, state: u32 },
    Safepoint { function: String, safepoint: u32 },
    Root { function: String, root: u32 },
    Layout { identity: String },
    Proof { identity: String },
    Bytecode { function: String, instruction: u32 },
    NativeCode { artifact: String, offset: u64 },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UnavailableReason {
    NotProduced,
    NoExactSourceCorrelation,
    NotApplicable,
    UnresolvedBinding,
    StructuralPosition,
    LexicalBindingMayShadow,
    DerivedArtifactUnavailable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "availability", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum FactRecord {
    Available {
        producer: ProducerRecord,
        fact_schema: FactSchema,
        fact_contract: String,
        source_revision: String,
        derived_artifact_identity: String,
        certainty: FactCertainty,
        cardinality: RelationCardinality,
        values: Vec<FactValue>,
        references: Vec<FactReference>,
    },
    Unavailable {
        producer: ProducerRecord,
        fact_schema: FactSchema,
        fact_contract: String,
        source_revision: String,
        derived_artifact_identity: Option<String>,
        certainty: FactCertainty,
        cardinality: RelationCardinality,
        reason: UnavailableReason,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeFacts {
    pub binding: FactRecord,
    pub hir: FactRecord,
    pub static_type: FactRecord,
    pub semantic_effects: FactRecord,
    pub ownership_place_loan: FactRecord,
    pub control_flow_graph: FactRecord,
    pub ssa_values_blocks: FactRecord,
    pub frame_states_safepoints_roots: FactRecord,
    pub layout: FactRecord,
    pub proof: FactRecord,
    pub bytecode: FactRecord,
    pub native_location: FactRecord,
}
