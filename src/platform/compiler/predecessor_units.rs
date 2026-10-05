//! Frozen compiler-unit 11–14 test reader from 40e098d9. Production requires rebuilding.
//! This keeps exact predecessor execution/hostile fixtures exercised after the derived-format cut.
use super::super::unit::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::{EffectParameterId, RequirementParameterId, TypeParameterId};
use bincode::{
    Decode,
    de::{BorrowDecoder, Decoder},
    error::DecodeError,
};

#[derive(Decode)]
pub(super) struct Unit {
    contract_version: u16,
    graph_contract_version: u16,
    bytecode_contract_version: u16,
    key: CompilationUnitKey,
    source: CompilationSource,
    optimization: OptimizationPolicy,
    tables: CompilationTables,
    payload: Payload,
}
#[derive(Decode)]
struct Signature {
    requirement_parameters: Vec<RequirementParameterId>,
    effect_parameters: Vec<EffectParameterId>,
    effect: FunctionEffect,
    type_parameters: Vec<TypeParameterId>,
    type_parameter_constraints: Vec<TypeParameterConstraints>,
    parameters: Vec<CompiledParameter>,
    result: u32,
    task_requirements: Vec<u32>,
}
#[derive(Decode)]
struct Code {
    parameter_count: u32,
    local_count: u32,
    instructions: Vec<Instruction>,
}
#[derive(Decode)]
enum Payload {
    Record {
        type_parameters: Vec<TypeParameterId>,
        type_parameter_constraints: Vec<TypeParameterConstraints>,
        fields: Vec<CompiledFieldLayout>,
    },
    Variant {
        type_parameters: Vec<TypeParameterId>,
        type_parameter_constraints: Vec<TypeParameterConstraints>,
        cases: Vec<CompiledCaseLayout>,
    },
    Interface {
        operations: Vec<CompiledOperationLayout>,
    },
    External {
        signature: Signature,
        implementation: ImplementationName,
    },
    Function {
        signature: Signature,
        code: Code,
    },
    Constant {
        ty: u32,
        code: Code,
    },
    Component {
        requirements: Vec<CompiledRequirement>,
        ports: Vec<Port>,
    },
    Test {
        actual: Code,
        expected: Code,
        comparison: ComparisonPolicy,
    },
    Target {
        component: u32,
        port: Option<u32>,
        routes: Vec<CompiledHttpRoute>,
        runner: crate::platform::package::RunnerKind,
    },
}
#[derive(Decode)]
struct Port {
    port: u32,
    function_type: u32,
    implementation: PortImplementation,
}
#[derive(Decode)]
enum PortImplementation {
    Function(u32),
    Expression(Code),
}
struct Instruction(CompiledInstruction);
impl<C> Decode<C> for Instruction {
    fn decode<D: Decoder<Context = C>>(d: &mut D) -> Result<Self, DecodeError> {
        use CompiledInstruction::*;
        macro_rules! value {
            () => {
                Decode::decode(d)?
            };
        }
        Ok(Self(match u32::decode(d)? {
            0 => Unit,
            1 => Bool(value!()),
            2 => I64(value!()),
            3 => Text(value!()),
            4 => StaticText(value!()),
            5 => LoadLocal {
                local: value!(),
                use_mode: value!(),
            },
            6 => StoreLocal(value!()),
            7 => Drop,
            8 => JumpIfFalse(value!()),
            9 => Jump(value!()),
            10 => Call {
                requirement_arguments: value!(),
                effect_arguments: value!(),
                function: value!(),
                type_arguments: value!(),
                arguments: value!(),
            },
            11 => FunctionValue {
                requirement_arguments: value!(),
                effect_arguments: value!(),
                function: value!(),
                type_arguments: value!(),
            },
            12 => Invoke {
                arguments: value!(),
            },
            13 => Record {
                nominal_type: value!(),
                type_arguments: value!(),
                fields: value!(),
            },
            14 => Variant {
                case: value!(),
                type_arguments: value!(),
                has_payload: value!(),
            },
            15 => Field(value!()),
            16 => List {
                item_type: value!(),
                items: value!(),
            },
            17 => Map {
                key_type: value!(),
                value_type: value!(),
                entries: value!(),
            },
            18 => SwitchVariant(value!()),
            19 => Perform {
                requirement: value!(),
                operation: value!(),
                arguments: value!(),
            },
            20 => BeginTransaction {
                requirement: value!(),
                binding: value!(),
            },
            21 => CommitTransaction {
                requirement: value!(),
                binding: value!(),
            },
            22 => PerformParameter {
                parameter: value!(),
                operation: value!(),
                arguments: value!(),
            },
            23 => BeginParameterTransaction {
                parameter: value!(),
                binding: value!(),
            },
            24 => CommitParameterTransaction {
                parameter: value!(),
                binding: value!(),
            },
            25 => Return,
            26 => Bind {
                arguments: value!(),
            },
            27 => BeginBind {
                arguments: value!(),
            },
            28 => Capture { index: value!() },
            29 => BeginTransactionOutcome {
                requirement: value!(),
                binding: value!(),
                outcome: value!(),
            },
            30 => CommitTransactionOutcome {
                requirement: value!(),
                binding: value!(),
                outcome: value!(),
            },
            31 => F64(value!()),
            _ => return Err(DecodeError::Other("unknown frozen instruction")),
        }))
    }
}
impl<'de, C> bincode::BorrowDecode<'de, C> for Instruction {
    fn borrow_decode<D: BorrowDecoder<'de, Context = C>>(d: &mut D) -> Result<Self, DecodeError> {
        Self::decode(d)
    }
}
impl Signature {
    fn current(self) -> CompiledSignature {
        CompiledSignature {
            implementation_parameters: vec![],
            requirement_parameters: self.requirement_parameters,
            effect_parameters: self.effect_parameters,
            effect: self.effect,
            type_parameters: self.type_parameters,
            type_parameter_constraints: self.type_parameter_constraints,
            parameters: self.parameters,
            result: self.result,
            result_borrow: None,
            task_requirements: self.task_requirements,
        }
    }
}
impl Code {
    fn current(self) -> CompiledCode {
        CompiledCode {
            parameter_count: self.parameter_count,
            local_count: self.local_count,
            instructions: self.instructions.into_iter().map(|v| v.0).collect(),
        }
    }
}
impl Unit {
    pub(super) fn current(self) -> CompilationUnit {
        let payload = match self.payload {
            Payload::Record {
                type_parameters,
                type_parameter_constraints,
                fields,
            } => CompilationPayload::Record {
                type_parameters,
                type_parameter_constraints,
                fields,
            },
            Payload::Variant {
                type_parameters,
                type_parameter_constraints,
                cases,
            } => CompilationPayload::Variant {
                type_parameters,
                type_parameter_constraints,
                cases,
            },
            Payload::Interface { operations } => CompilationPayload::Interface { operations },
            Payload::External {
                signature,
                implementation,
            } => CompilationPayload::External {
                signature: signature.current(),
                implementation,
            },
            Payload::Function { signature, code } => CompilationPayload::Function {
                signature: signature.current(),
                code: code.current(),
            },
            Payload::Constant { ty, code } => CompilationPayload::Constant {
                ty,
                code: code.current(),
            },
            Payload::Component {
                requirements,
                ports,
            } => CompilationPayload::Component {
                requirements,
                ports: ports
                    .into_iter()
                    .map(|p| CompiledPort {
                        port: p.port,
                        function_type: p.function_type,
                        implementation: match p.implementation {
                            PortImplementation::Function(f) => {
                                CompiledPortImplementation::Function(f)
                            }
                            PortImplementation::Expression(c) => {
                                CompiledPortImplementation::Expression(c.current())
                            }
                        },
                    })
                    .collect(),
            },
            Payload::Test {
                actual,
                expected,
                comparison,
            } => CompilationPayload::Test {
                actual: actual.current(),
                expected: expected.current(),
                comparison,
            },
            Payload::Target {
                component,
                port,
                routes,
                runner,
            } => CompilationPayload::Target {
                component,
                port,
                routes,
                runner,
            },
        };
        CompilationUnit {
            contract_version: self.contract_version,
            graph_contract_version: self.graph_contract_version,
            bytecode_contract_version: self.bytecode_contract_version,
            key: self.key,
            source: self.source,
            optimization: self.optimization,
            tables: self.tables,
            payload,
        }
    }
}
