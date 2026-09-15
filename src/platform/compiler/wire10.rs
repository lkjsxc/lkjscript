//! Strict compiler unit 10 / bytecode 6 layouts from 4306ef64.
//! Only the wire representation is historical; admission uses the current checker.
use super::unit::*;
use crate::platform::diagnostic::Diagnostic;
use crate::platform::kernel::*;
use crate::platform::package::RunnerKind;
use crate::platform::semantic_id::{ParameterId, TypeParameterId};
use bincode::{Decode, Encode};

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompilationUnit10 {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub bytecode_contract_version: u16,
    pub key: CompilationUnitKey,
    pub source: CompilationSource,
    pub optimization: OptimizationPolicy,
    pub tables: CompilationTables,
    pub payload: CompilationPayload10,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum CompilationPayload10 {
    Record {
        type_parameters: Vec<TypeParameterId>,
        type_parameter_constraints: Vec<crate::platform::kernel::TypeParameterConstraints>,
        fields: Vec<CompiledFieldLayout>,
    },
    Variant {
        type_parameters: Vec<TypeParameterId>,
        type_parameter_constraints: Vec<crate::platform::kernel::TypeParameterConstraints>,
        cases: Vec<CompiledCaseLayout>,
    },
    Interface {
        operations: Vec<CompiledOperationLayout>,
    },
    External {
        signature: CompiledSignature10,
        implementation: ImplementationName,
    },
    Function {
        signature: CompiledSignature10,
        code: CompiledCode10,
    },
    Constant {
        ty: u32,
        code: CompiledCode10,
    },
    Component {
        requirements: Vec<CompiledRequirement>,
        ports: Vec<CompiledPort10>,
    },
    Test {
        actual: CompiledCode10,
        expected: CompiledCode10,
        comparison: ComparisonPolicy,
    },
    Target {
        component: u32,
        port: Option<u32>,
        routes: Vec<CompiledHttpRoute>,
        runner: RunnerKind,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledSignature10 {
    pub effect_parameters: Vec<crate::platform::semantic_id::EffectParameterId>,
    pub effect: crate::platform::kernel::wire14::FunctionEffect14,
    pub type_parameters: Vec<TypeParameterId>,
    pub type_parameter_constraints: Vec<crate::platform::kernel::TypeParameterConstraints>,
    pub parameters: Vec<CompiledParameter>,
    pub result: u32,
    pub task_requirements: Vec<u32>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledCode10 {
    pub parameter_count: u32,
    pub local_count: u32,
    pub instructions: Vec<CompiledInstruction10>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum CompiledInstruction10 {
    Unit,
    Bool(bool),
    I64(i64),
    Text(u32),
    StaticText(u32),
    LoadLocal {
        local: u32,
        use_mode: ParameterUse,
    },
    StoreLocal(u32),
    Drop,
    JumpIfFalse(u32),
    Jump(u32),
    Call {
        effect_arguments: Vec<crate::platform::kernel::wire14::EffectRow14>,
        function: u32,
        type_arguments: Vec<u32>,
        arguments: u32,
    },
    FunctionValue {
        effect_arguments: Vec<crate::platform::kernel::wire14::EffectRow14>,
        function: u32,
        type_arguments: Vec<u32>,
    },
    Invoke {
        arguments: u32,
    },
    Record {
        nominal_type: Option<u32>,
        type_arguments: Vec<u32>,
        fields: Vec<CompiledFieldSelector>,
    },
    Variant {
        case: u32,
        type_arguments: Vec<u32>,
        has_payload: bool,
    },
    Field(CompiledFieldSelector),
    List {
        item_type: u32,
        items: u32,
    },
    Map {
        key_type: u32,
        value_type: u32,
        entries: u32,
    },
    SwitchVariant(Vec<CompiledVariantJump>),
    Perform {
        requirement: u32,
        operation: u32,
        arguments: u32,
    },
    BeginTransaction {
        requirement: u32,
        binding: u32,
    },
    CommitTransaction {
        requirement: u32,
        binding: u32,
    },
    Return,
    Bind {
        arguments: u32,
    },
    BeginBind {
        arguments: u32,
    },
    Capture {
        index: u32,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledPort10 {
    pub port: u32,
    pub function_type: u32,
    pub implementation: CompiledPortImplementation10,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum CompiledPortImplementation10 {
    Function(u32),
    Expression(CompiledCode10),
}

impl From<CompilationUnit10> for CompilationUnit {
    fn from(value: CompilationUnit10) -> Self {
        let CompilationUnit10 {
            contract_version,
            graph_contract_version,
            bytecode_contract_version,
            key,
            source,
            optimization,
            tables,
            payload,
        } = value;
        Self {
            contract_version,
            graph_contract_version,
            bytecode_contract_version,
            key,
            source,
            optimization,
            tables,
            payload: payload.into(),
        }
    }
}

impl TryFrom<CompilationUnit> for CompilationUnit10 {
    type Error = Diagnostic;
    fn try_from(value: CompilationUnit) -> Result<Self, Diagnostic> {
        let CompilationUnit {
            contract_version,
            graph_contract_version,
            bytecode_contract_version,
            key,
            source,
            optimization,
            tables,
            payload,
        } = value;
        Ok(Self {
            contract_version,
            graph_contract_version,
            bytecode_contract_version,
            key,
            source,
            optimization,
            tables,
            payload: payload.try_into()?,
        })
    }
}

impl From<CompilationPayload10> for CompilationPayload {
    fn from(value: CompilationPayload10) -> Self {
        match value {
            CompilationPayload10::Record {
                type_parameters,
                type_parameter_constraints,
                fields,
            } => Self::Record {
                type_parameters,
                type_parameter_constraints,
                fields,
            },
            CompilationPayload10::Variant {
                type_parameters,
                type_parameter_constraints,
                cases,
            } => Self::Variant {
                type_parameters,
                type_parameter_constraints,
                cases,
            },
            CompilationPayload10::Interface { operations } => Self::Interface { operations },
            CompilationPayload10::External {
                signature,
                implementation,
            } => Self::External {
                signature: signature.into(),
                implementation,
            },
            CompilationPayload10::Function { signature, code } => Self::Function {
                signature: signature.into(),
                code: code.into(),
            },
            CompilationPayload10::Constant { ty, code } => Self::Constant {
                ty,
                code: code.into(),
            },
            CompilationPayload10::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports: ports.into_iter().map(Into::into).collect(),
            },
            CompilationPayload10::Test {
                actual,
                expected,
                comparison,
            } => Self::Test {
                actual: actual.into(),
                expected: expected.into(),
                comparison,
            },
            CompilationPayload10::Target {
                component,
                port,
                routes,
                runner,
            } => Self::Target {
                component,
                port,
                routes,
                runner,
            },
        }
    }
}

impl TryFrom<CompilationPayload> for CompilationPayload10 {
    type Error = Diagnostic;
    fn try_from(value: CompilationPayload) -> Result<Self, Diagnostic> {
        Ok(match value {
            CompilationPayload::Record {
                type_parameters,
                type_parameter_constraints,
                fields,
            } => Self::Record {
                type_parameters,
                type_parameter_constraints,
                fields,
            },
            CompilationPayload::Variant {
                type_parameters,
                type_parameter_constraints,
                cases,
            } => Self::Variant {
                type_parameters,
                type_parameter_constraints,
                cases,
            },
            CompilationPayload::Interface { operations } => Self::Interface { operations },
            CompilationPayload::External {
                signature,
                implementation,
            } => Self::External {
                signature: signature.try_into()?,
                implementation,
            },
            CompilationPayload::Function { signature, code } => Self::Function {
                signature: signature.try_into()?,
                code: code.try_into()?,
            },
            CompilationPayload::Constant { ty, code } => Self::Constant {
                ty,
                code: code.try_into()?,
            },
            CompilationPayload::Component {
                requirements,
                ports,
            } => Self::Component {
                requirements,
                ports: ports
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()?,
            },
            CompilationPayload::Test {
                actual,
                expected,
                comparison,
            } => Self::Test {
                actual: actual.try_into()?,
                expected: expected.try_into()?,
                comparison,
            },
            CompilationPayload::Target {
                component,
                port,
                routes,
                runner,
            } => Self::Target {
                component,
                port,
                routes,
                runner,
            },
        })
    }
}

impl From<CompiledSignature10> for CompiledSignature {
    fn from(value: CompiledSignature10) -> Self {
        let CompiledSignature10 {
            effect_parameters,
            effect,
            type_parameters,
            type_parameter_constraints,
            parameters,
            result,
            task_requirements,
        } = value;
        Self {
            effect_parameters,
            effect: effect.into(),
            type_parameters,
            type_parameter_constraints,
            parameters,
            result,
            task_requirements,
            requirement_parameters: Vec::new(),
        }
    }
}

impl TryFrom<CompiledSignature> for CompiledSignature10 {
    type Error = Diagnostic;
    fn try_from(value: CompiledSignature) -> Result<Self, Diagnostic> {
        let CompiledSignature {
            effect_parameters,
            effect,
            type_parameters,
            type_parameter_constraints,
            parameters,
            result,
            task_requirements,
            requirement_parameters,
        } = value;
        if !requirement_parameters.is_empty() {
            return Err(extension());
        }
        Ok(Self {
            effect_parameters,
            effect: effect.try_into()?,
            type_parameters,
            type_parameter_constraints,
            parameters,
            result,
            task_requirements,
        })
    }
}

impl From<CompiledCode10> for CompiledCode {
    fn from(value: CompiledCode10) -> Self {
        let CompiledCode10 {
            parameter_count,
            local_count,
            instructions,
        } = value;
        Self {
            parameter_count,
            local_count,
            instructions: instructions.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<CompiledCode> for CompiledCode10 {
    type Error = Diagnostic;
    fn try_from(value: CompiledCode) -> Result<Self, Diagnostic> {
        let CompiledCode {
            parameter_count,
            local_count,
            instructions,
        } = value;
        Ok(Self {
            parameter_count,
            local_count,
            instructions: instructions
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<CompiledInstruction10> for CompiledInstruction {
    fn from(value: CompiledInstruction10) -> Self {
        match value {
            CompiledInstruction10::Unit => Self::Unit,
            CompiledInstruction10::Bool(value) => Self::Bool(value),
            CompiledInstruction10::I64(value) => Self::I64(value),
            CompiledInstruction10::Text(value) => Self::Text(value),
            CompiledInstruction10::StaticText(value) => Self::StaticText(value),
            CompiledInstruction10::LoadLocal { local, use_mode } => {
                Self::LoadLocal { local, use_mode }
            }
            CompiledInstruction10::StoreLocal(value) => Self::StoreLocal(value),
            CompiledInstruction10::Drop => Self::Drop,
            CompiledInstruction10::JumpIfFalse(value) => Self::JumpIfFalse(value),
            CompiledInstruction10::Jump(value) => Self::Jump(value),
            CompiledInstruction10::Call {
                effect_arguments,
                function,
                type_arguments,
                arguments,
            } => Self::Call {
                effect_arguments: effect_arguments.into_iter().map(Into::into).collect(),
                function,
                type_arguments,
                arguments,
                requirement_arguments: Vec::new(),
            },
            CompiledInstruction10::FunctionValue {
                effect_arguments,
                function,
                type_arguments,
            } => Self::FunctionValue {
                effect_arguments: effect_arguments.into_iter().map(Into::into).collect(),
                function,
                type_arguments,
                requirement_arguments: Vec::new(),
            },
            CompiledInstruction10::Invoke { arguments } => Self::Invoke { arguments },
            CompiledInstruction10::Record {
                nominal_type,
                type_arguments,
                fields,
            } => Self::Record {
                nominal_type,
                type_arguments,
                fields,
            },
            CompiledInstruction10::Variant {
                case,
                type_arguments,
                has_payload,
            } => Self::Variant {
                case,
                type_arguments,
                has_payload,
            },
            CompiledInstruction10::Field(value) => Self::Field(value),
            CompiledInstruction10::List { item_type, items } => Self::List { item_type, items },
            CompiledInstruction10::Map {
                key_type,
                value_type,
                entries,
            } => Self::Map {
                key_type,
                value_type,
                entries,
            },
            CompiledInstruction10::SwitchVariant(value) => Self::SwitchVariant(value),
            CompiledInstruction10::Perform {
                requirement,
                operation,
                arguments,
            } => Self::Perform {
                requirement,
                operation,
                arguments,
            },
            CompiledInstruction10::BeginTransaction {
                requirement,
                binding,
            } => Self::BeginTransaction {
                requirement,
                binding,
            },
            CompiledInstruction10::CommitTransaction {
                requirement,
                binding,
            } => Self::CommitTransaction {
                requirement,
                binding,
            },
            CompiledInstruction10::Return => Self::Return,
            CompiledInstruction10::Bind { arguments } => Self::Bind { arguments },
            CompiledInstruction10::BeginBind { arguments } => Self::BeginBind { arguments },
            CompiledInstruction10::Capture { index } => Self::Capture { index },
        }
    }
}

impl TryFrom<CompiledInstruction> for CompiledInstruction10 {
    type Error = Diagnostic;
    fn try_from(value: CompiledInstruction) -> Result<Self, Diagnostic> {
        Ok(match value {
            CompiledInstruction::Unit => Self::Unit,
            CompiledInstruction::Bool(value) => Self::Bool(value),
            CompiledInstruction::I64(value) => Self::I64(value),
            CompiledInstruction::Text(value) => Self::Text(value),
            CompiledInstruction::StaticText(value) => Self::StaticText(value),
            CompiledInstruction::LoadLocal { local, use_mode } => {
                Self::LoadLocal { local, use_mode }
            }
            CompiledInstruction::StoreLocal(value) => Self::StoreLocal(value),
            CompiledInstruction::Drop => Self::Drop,
            CompiledInstruction::JumpIfFalse(value) => Self::JumpIfFalse(value),
            CompiledInstruction::Jump(value) => Self::Jump(value),
            CompiledInstruction::Call {
                effect_arguments,
                function,
                type_arguments,
                arguments,
                requirement_arguments,
            } => {
                if !requirement_arguments.is_empty() {
                    return Err(extension());
                }
                Self::Call {
                    effect_arguments: effect_arguments
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()?,
                    function,
                    type_arguments,
                    arguments,
                }
            }
            CompiledInstruction::FunctionValue {
                effect_arguments,
                function,
                type_arguments,
                requirement_arguments,
            } => {
                if !requirement_arguments.is_empty() {
                    return Err(extension());
                }
                Self::FunctionValue {
                    effect_arguments: effect_arguments
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()?,
                    function,
                    type_arguments,
                }
            }
            CompiledInstruction::Invoke { arguments } => Self::Invoke { arguments },
            CompiledInstruction::Record {
                nominal_type,
                type_arguments,
                fields,
            } => Self::Record {
                nominal_type,
                type_arguments,
                fields,
            },
            CompiledInstruction::Variant {
                case,
                type_arguments,
                has_payload,
            } => Self::Variant {
                case,
                type_arguments,
                has_payload,
            },
            CompiledInstruction::Field(value) => Self::Field(value),
            CompiledInstruction::List { item_type, items } => Self::List { item_type, items },
            CompiledInstruction::Map {
                key_type,
                value_type,
                entries,
            } => Self::Map {
                key_type,
                value_type,
                entries,
            },
            CompiledInstruction::SwitchVariant(value) => Self::SwitchVariant(value),
            CompiledInstruction::Perform {
                requirement,
                operation,
                arguments,
            } => Self::Perform {
                requirement,
                operation,
                arguments,
            },
            CompiledInstruction::BeginTransaction {
                requirement,
                binding,
            } => Self::BeginTransaction {
                requirement,
                binding,
            },
            CompiledInstruction::CommitTransaction {
                requirement,
                binding,
            } => Self::CommitTransaction {
                requirement,
                binding,
            },
            CompiledInstruction::Return => Self::Return,
            CompiledInstruction::Bind { arguments } => Self::Bind { arguments },
            CompiledInstruction::BeginBind { arguments } => Self::BeginBind { arguments },
            CompiledInstruction::Capture { index } => Self::Capture { index },
            CompiledInstruction::PerformParameter { .. } => return Err(extension()),
            CompiledInstruction::BeginParameterTransaction { .. } => return Err(extension()),
            CompiledInstruction::CommitParameterTransaction { .. } => return Err(extension()),
            CompiledInstruction::BeginTransactionOutcome { .. }
            | CompiledInstruction::CommitTransactionOutcome { .. }
            | CompiledInstruction::F64(_) => return Err(extension()),
        })
    }
}

impl From<CompiledPort10> for CompiledPort {
    fn from(value: CompiledPort10) -> Self {
        let CompiledPort10 {
            port,
            function_type,
            implementation,
        } = value;
        Self {
            port,
            function_type,
            implementation: implementation.into(),
        }
    }
}

impl TryFrom<CompiledPort> for CompiledPort10 {
    type Error = Diagnostic;
    fn try_from(value: CompiledPort) -> Result<Self, Diagnostic> {
        let CompiledPort {
            port,
            function_type,
            implementation,
        } = value;
        Ok(Self {
            port,
            function_type,
            implementation: implementation.try_into()?,
        })
    }
}

impl From<CompiledPortImplementation10> for CompiledPortImplementation {
    fn from(value: CompiledPortImplementation10) -> Self {
        match value {
            CompiledPortImplementation10::Function(value) => Self::Function(value),
            CompiledPortImplementation10::Expression(value) => Self::Expression(value.into()),
        }
    }
}

impl TryFrom<CompiledPortImplementation> for CompiledPortImplementation10 {
    type Error = Diagnostic;
    fn try_from(value: CompiledPortImplementation) -> Result<Self, Diagnostic> {
        Ok(match value {
            CompiledPortImplementation::Function(value) => Self::Function(value),
            CompiledPortImplementation::Expression(value) => Self::Expression(value.try_into()?),
        })
    }
}

fn extension() -> Diagnostic {
    Diagnostic::new(
        crate::platform::diagnostic::DiagnosticClass::Corrupt,
        "compiler_predecessor_extension",
        "requirement extension cannot be encoded in compiler unit 10",
    )
}
