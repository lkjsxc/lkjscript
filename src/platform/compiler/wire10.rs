//! Test-only compiler unit 10 / bytecode 6 decoder frozen from 4306ef64.
//! Conversion preserves historical meaning for independent admission tests.
//! Production requires rebuilding derived units from supported canonical owners.
use super::unit::*;
use crate::platform::kernel::*;
use crate::platform::package::RunnerKind;
use crate::platform::semantic_id::{ParameterId, TypeParameterId};
use bincode::Decode;

#[derive(Clone, Debug, Decode, Eq, PartialEq)]
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

#[derive(Clone, Debug, Decode, Eq, PartialEq)]
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

#[derive(Clone, Debug, Decode, Eq, PartialEq)]
pub struct CompiledSignature10 {
    pub effect_parameters: Vec<crate::platform::semantic_id::EffectParameterId>,
    pub effect: crate::platform::kernel::wire14::FunctionEffect14,
    pub type_parameters: Vec<TypeParameterId>,
    pub type_parameter_constraints: Vec<crate::platform::kernel::TypeParameterConstraints>,
    pub parameters: Vec<CompiledParameter>,
    pub result: u32,
    pub task_requirements: Vec<u32>,
}

#[derive(Clone, Debug, Decode, Eq, PartialEq)]
pub struct CompiledCode10 {
    pub parameter_count: u32,
    pub local_count: u32,
    pub instructions: Vec<CompiledInstruction10>,
}

#[derive(Clone, Debug, Decode, Eq, PartialEq)]
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

#[derive(Clone, Debug, Decode, Eq, PartialEq)]
pub struct CompiledPort10 {
    pub port: u32,
    pub function_type: u32,
    pub implementation: CompiledPortImplementation10,
}

#[derive(Clone, Debug, Decode, Eq, PartialEq)]
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
            implementation_parameters: Vec::new(),
            requirement_parameters: Vec::new(),
        }
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

impl From<CompiledPortImplementation10> for CompiledPortImplementation {
    fn from(value: CompiledPortImplementation10) -> Self {
        match value {
            CompiledPortImplementation10::Function(value) => Self::Function(value),
            CompiledPortImplementation10::Expression(value) => Self::Expression(value.into()),
        }
    }
}
