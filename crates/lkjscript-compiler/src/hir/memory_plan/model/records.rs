#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MemoryOrigin {
    pub source: u32,
    pub expression: Option<MemoryExpressionId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemorySubject {
    Expression {
        expression: MemoryExpressionId,
        parent: Option<MemoryExpressionId>,
        child_index: u32,
        kind: MemoryExpressionKind,
    },
    Parameter {
        function: MemoryFunctionId,
        index: u32,
        binding: u32,
        place: u32,
    },
    Result {
        function: MemoryFunctionId,
    },
    Place {
        function: MemoryFunctionId,
        place: u32,
        binding: u32,
    },
    Loan {
        function: MemoryFunctionId,
        place: u32,
        loan: u32,
        expression: MemoryExpressionId,
    },
    Constant {
        constant: MemoryConstantId,
        expression: MemoryExpressionId,
    },
    Call {
        call: MemoryCallId,
        expression: MemoryExpressionId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryPlanEntry {
    pub id: MemoryEntryId,
    pub subject: MemorySubject,
    pub ty: MemoryType,
    pub effects: u16,
    pub mode: MemoryMode,
    pub type_fact: MemoryTypeFactId,
    pub root_projection: MemoryRootProjection,
    pub destination: Option<MemoryDestinationId>,
    pub copy_share: MemoryCopySharePlan,
    pub borrow_scope: Option<MemoryBorrowScopeId>,
    pub drop_path: Option<MemoryDropPathId>,
    pub execution: MemoryExecution,
    pub execution_cutover: Option<MemoryExecutionCutover>,
    pub origin: MemoryOrigin,
    pub drop_glue: Option<MemoryDropGlueId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryWitnessOperation {
    Transport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryWitnessParameter {
    pub parameter: String,
    pub operations: Vec<MemoryWitnessOperation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryWitnessArgument {
    pub parameter: String,
    pub witness: MemoryWitnessId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionMemorySignature {
    pub function: MemoryFunctionId,
    pub witness_parameters: Vec<MemoryWitnessParameter>,
    pub parameters: Vec<MemoryParameterMode>,
    pub result: MemoryResultMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionMemoryPlan {
    pub id: MemoryFunctionId,
    pub name: String,
    pub binding: Option<u32>,
    pub source: u32,
    pub signature: FunctionMemorySignature,
    pub parameter_entries: Vec<MemoryEntryId>,
    pub result_entry: MemoryEntryId,
    pub body: MemoryExpressionId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryUseKind {
    Load,
    Move,
    BorrowSource,
    DirectCallTarget,
    IndirectCallTarget,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryUse {
    pub id: MemoryUseId,
    pub function: MemoryFunctionId,
    pub expression: MemoryExpressionId,
    pub binding: u32,
    pub kind: MemoryUseKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryConstantValue {
    I64(i64),
    F64(u64),
    Bool(bool),
    Unit,
    EmptyList,
    String(String),
    Bytes(Vec<u8>),
    Symbol(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryConstantPlan {
    pub id: MemoryConstantId,
    pub function: MemoryFunctionId,
    pub expression: MemoryExpressionId,
    pub value: MemoryConstantValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryCallTarget {
    Direct(MemoryFunctionId),
    Indirect(u32),
    Operation(u16),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryCallPlan {
    pub id: MemoryCallId,
    pub function: MemoryFunctionId,
    pub expression: MemoryExpressionId,
    pub target: MemoryCallTarget,
    pub witness_arguments: Vec<MemoryWitnessArgument>,
    pub parameters: Vec<MemoryParameterMode>,
    pub result: MemoryResultMode,
    pub borrow_scopes: Vec<Option<MemoryBorrowScopeId>>,
}
