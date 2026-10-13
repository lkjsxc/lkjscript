//! Frozen ordinary .95 compiler33/bytecode27 DTOs. Original encodings/profile identities
//! are retained; conversion supplies only empty new inventories and old Value/ReadFrom.
//! Converted data still requires complete current independent artifact admission.

use super::unit::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::*;
use crate::platform::package::RunnerKind;
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, MethodId, ParameterId, TypeParameterId,
};
use bincode::{Decode, Encode};

fn legacy_content() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Corrupt,
        "compiler_unit_frozen_content",
        "predecessor compiler schema cannot carry new result, scope or view meaning",
    )
}
fn require_empty<T>(values: &[T]) -> Result<(), Diagnostic> {
    if values.is_empty() {
        Ok(())
    } else {
        Err(legacy_content())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Bound33 {
    Owned,
}
impl Encode for Bound33 {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        encoder: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        1_u8.encode(encoder)
    }
}
impl<C> Decode<C> for Bound33 {
    fn decode<D: bincode::de::Decoder<Context = C>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        match u8::decode(decoder)? {
            1 => Ok(Self::Owned),
            _ => Err(bincode::error::DecodeError::Other(
                "foreign predecessor associated bound",
            )),
        }
    }
}
bincode::impl_borrow_decode!(Bound33);

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
struct Unit33 {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub bytecode_contract_version: u16,
    pub key: CompilationUnitKey,
    pub source: CompilationSource,
    pub optimization: OptimizationPolicy,
    pub tables: Tables33,
    pub payload: Payload33,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub(super) struct Tables33 {
    pub declarations: Vec<DeclarationReference>,
    pub fields: Vec<FieldReference>,
    pub cases: Vec<CaseReference>,
    pub requirements: Vec<RequirementReference>,
    pub operations: Vec<OperationReference>,
    pub ports: Vec<PortReference>,
    pub types: Vec<TypeObjectDigest>,
    pub(super) structural_names: Vec<Name>,
    pub texts: Vec<CompiledText>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
enum Payload33 {
    Contract33(Contract33),
    Implementation33(Implementation33),
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
        signature: Signature33,
        implementation: ImplementationName,
    },
    Function {
        signature: Signature33,
        code: Code33,
    },
    Constant {
        ty: u32,
        code: Code33,
    },
    Component {
        requirements: Vec<CompiledRequirement>,
        ports: Vec<Port33>,
    },
    Test {
        actual: Code33,
        expected: Code33,
        comparison: ComparisonPolicy,
    },
    Target {
        component: u32,
        port: Option<u32>,
        routes: Vec<CompiledHttpRoute>,
        runner: RunnerKind,
    },
    OwnedRecord {
        type_parameters: Vec<TypeParameterId>,
        type_parameter_constraints: Vec<crate::platform::kernel::TypeParameterConstraints>,
        fields: Vec<CompiledFieldLayout>,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub(super) struct Signature33 {
    pub implementation_parameters: Vec<crate::platform::kernel::ImplementationParameter>,
    pub requirement_parameters: Vec<crate::platform::semantic_id::RequirementParameterId>,
    pub effect_parameters: Vec<crate::platform::semantic_id::EffectParameterId>,
    pub effect: crate::platform::kernel::FunctionEffect,
    pub type_parameters: Vec<TypeParameterId>,
    pub type_parameter_constraints: Vec<crate::platform::kernel::TypeParameterConstraints>,
    pub parameters: Vec<CompiledParameter>,
    pub result: u32,
    pub result_borrow: Option<ParameterId>,
    pub task_requirements: Vec<u32>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub(super) struct Code33 {
    pub parameter_count: u32,
    pub local_count: u32,
    pub instructions: Vec<Instruction33>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub(super) enum Instruction33 {
    ImplementationCall {
        requirement_arguments: Vec<crate::platform::kernel::RequirementOperand>,
        effect_arguments: Vec<crate::platform::kernel::EffectRow>,
        function: u32,
        type_arguments: Vec<u32>,
        implementations: Vec<crate::platform::kernel::ImplementationOperand>,
        arguments: u32,
    },
    MethodCall {
        witness: crate::platform::kernel::ImplementationOperand,
        contract: crate::platform::kernel::DeclarationReference,
        method: crate::platform::semantic_id::MethodId,
        arguments: u32,
    },
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
        requirement_arguments: Vec<crate::platform::kernel::RequirementOperand>,
        effect_arguments: Vec<crate::platform::kernel::EffectRow>,
        function: u32,
        type_arguments: Vec<u32>,
        arguments: u32,
    },
    FunctionValue {
        requirement_arguments: Vec<crate::platform::kernel::RequirementOperand>,
        effect_arguments: Vec<crate::platform::kernel::EffectRow>,
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
    PerformParameter {
        parameter: crate::platform::kernel::RequirementParameterReference,
        operation: u32,
        arguments: u32,
    },
    BeginParameterTransaction {
        parameter: crate::platform::kernel::RequirementParameterReference,
        binding: u32,
    },
    CommitParameterTransaction {
        parameter: crate::platform::kernel::RequirementParameterReference,
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
    BeginTransactionOutcome {
        requirement: CompiledTransactionRequirement,
        binding: u32,
        outcome: CompiledTransactionOutcome,
    },
    CommitTransactionOutcome {
        requirement: CompiledTransactionRequirement,
        binding: u32,
        outcome: CompiledTransactionOutcome,
    },
    /// Fixed little-endian scalar bytes; predecessor instruction ordinals remain unchanged.
    F64(crate::platform::binary64::Binary64),
    PackOwned {
        product_type: u32,
        fields: Vec<u32>,
    },
    UnpackOwned {
        product_type: u32,
        locals: Vec<u32>,
    },
    ChooseOwned {
        choice_type: u32,
        case: u32,
    },
    MatchOwned {
        choice_type: u32,
        cases: Vec<CompiledOwnedChoiceJump>,
    },
    Parallel {
        left_types: Vec<u32>,
        left_implementations: Vec<crate::platform::kernel::ImplementationOperand>,
        right_types: Vec<u32>,
        right_implementations: Vec<crate::platform::kernel::ImplementationOperand>,
        left: u32,
        left_arguments: u32,
        right: u32,
        right_arguments: u32,
        result_type: u32,
    },
    /// Enter a lexical read loan without removing the source local's custody.
    BorrowOwnedField {
        product_type: u32,
        source_local: u32,
        field: u32,
        binding_local: u32,
        binding_type: u32,
    },
    MatchBorrowedOwned {
        choice_type: u32,
        source_local: u32,
        cases: Vec<CompiledBorrowedOwnedChoiceJump>,
    },
    /// Clear the scoped child before releasing its parent loan; preserve the body result.
    EndOwnedBorrow {
        binding_local: u32,
    },
    SequenceEmpty {
        sequence_type: u32,
    },
    SequenceLength {
        sequence_type: u32,
        source_local: u32,
    },
    SequencePush {
        sequence_type: u32,
        source_local: u32,
    },
    SequencePop {
        sequence_type: u32,
        result_type: u32,
        source_local: u32,
    },
    /// Pop the authored index before entering a lexical item loan.
    BorrowOwnedItem {
        sequence_type: u32,
        source_local: u32,
        binding_local: u32,
        binding_type: u32,
    },
    /// Bind the immediately following exact call to its selected live source local.
    BeginBorrowCall {
        source_local: u32,
        source_position: u32,
    },
    /// Adopt the immediately preceding borrowed call result into a lexical read scope.
    AdoptBorrowResult {
        source_local: u32,
        binding_local: u32,
        binding_type: u32,
    },
    ReturnBorrowed,
    /// Pop an evaluated index, read the ordinary item, and release the short source loan.
    SequenceGet {
        sequence_type: u32,
        source_local: u32,
    },
    /// Pop replacement then index, and consume the source into a rest/value product.
    SequenceReplace {
        sequence_type: u32,
        result_type: u32,
        source_local: u32,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub(super) struct Port33 {
    pub port: u32,
    pub function_type: u32,
    pub implementation: PortImplementation33,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub(super) enum PortImplementation33 {
    Function(u32),
    Expression(Code33),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
struct Contract33 {
    pub self_parameter: TypeParameterId,
    pub type_parameters: Vec<TypeParameterId>,
    pub methods: Vec<Method33>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
struct Implementation33 {
    pub type_parameters: Vec<TypeParameterId>,
    pub implementation_parameters: Vec<ImplementationParameter>,
    pub contract: DeclarationReference,
    pub self_type: TypeObjectDigest,
    pub type_arguments: Vec<TypeObjectDigest>,
    pub methods: Vec<Mapping33>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
struct Method33 {
    pub id: MethodId,
    pub name: Name,
    pub parameters: Vec<OwnedMethodParameter>,
    pub result: TypeObjectDigest,
    pub result_borrow: Option<u32>,
    pub effect: FunctionEffect,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
struct Mapping33 {
    pub method: MethodId,
    pub function: DeclarationReference,
    pub type_arguments: Vec<TypeObjectDigest>,
    pub implementations: Vec<ImplementationOperand>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
struct Member33 {
    pub id: AssociatedTypeMemberId,
    pub name: Name,
    pub bound: Bound33,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
struct Binding33 {
    pub member: AssociatedTypeMemberId,
    pub ty: TypeObjectDigest,
}

impl From<Method33> for OwnedMethod {
    fn from(value: Method33) -> Self {
        Self {
            id: value.id,
            name: value.name,
            parameters: value.parameters,
            result: CallableResult {
                ty: value.result,
                relation: value.result_borrow.map_or(ResultRelation::Value, |source| {
                    ResultRelation::ReadFrom { source }
                }),
            },
            effect: value.effect,
            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
        }
    }
}
impl TryFrom<&OwnedMethod> for Method33 {
    type Error = Diagnostic;
    fn try_from(value: &OwnedMethod) -> Result<Self, Diagnostic> {
        require_empty(&value.read_scope_parameters)?;
        require_empty(&value.read_scope_sources)?;
        let result_borrow = match value.result.relation {
            ResultRelation::Value => None,
            ResultRelation::ReadFrom { source } => Some(source),
            ResultRelation::ViewFrom { .. } => return Err(legacy_content()),
        };
        Ok(Self {
            id: value.id,
            name: value.name.clone(),
            parameters: value.parameters.clone(),
            result: value.result.ty,
            result_borrow,
            effect: value.effect.clone(),
        })
    }
}
impl From<Mapping33> for OwnedMethodImplementation {
    fn from(value: Mapping33) -> Self {
        Self {
            method: value.method,
            function: value.function,
            type_arguments: value.type_arguments,
            implementations: value.implementations,
            read_scope_arguments: Vec::new(),
        }
    }
}
impl TryFrom<&OwnedMethodImplementation> for Mapping33 {
    type Error = Diagnostic;
    fn try_from(value: &OwnedMethodImplementation) -> Result<Self, Diagnostic> {
        require_empty(&value.read_scope_arguments)?;
        Ok(Self {
            method: value.method,
            function: value.function,
            type_arguments: value.type_arguments.clone(),
            implementations: value.implementations.clone(),
        })
    }
}
impl From<Contract33> for OwnedContract {
    fn from(value: Contract33) -> Self {
        Self {
            self_parameter: value.self_parameter,
            type_parameters: value.type_parameters,
            methods: value.methods.into_iter().map(Into::into).collect(),
            associated_types: Vec::new(),
        }
    }
}
impl TryFrom<&OwnedContract> for Contract33 {
    type Error = Diagnostic;
    fn try_from(value: &OwnedContract) -> Result<Self, Diagnostic> {
        Ok(Self {
            self_parameter: value.self_parameter,
            type_parameters: value.type_parameters.clone(),
            methods: value
                .methods
                .iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}
impl From<Implementation33> for OwnedImplementation {
    fn from(value: Implementation33) -> Self {
        Self {
            type_parameters: value.type_parameters,
            implementation_parameters: value.implementation_parameters,
            contract: value.contract,
            self_type: value.self_type,
            type_arguments: value.type_arguments,
            methods: value.methods.into_iter().map(Into::into).collect(),
            associated_types: Vec::new(),
        }
    }
}
impl TryFrom<&OwnedImplementation> for Implementation33 {
    type Error = Diagnostic;
    fn try_from(value: &OwnedImplementation) -> Result<Self, Diagnostic> {
        Ok(Self {
            type_parameters: value.type_parameters.clone(),
            implementation_parameters: value.implementation_parameters.clone(),
            contract: value.contract,
            self_type: value.self_type,
            type_arguments: value.type_arguments.clone(),
            methods: value
                .methods
                .iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}
impl From<Member33> for AssociatedTypeMember {
    fn from(value: Member33) -> Self {
        Self {
            id: value.id,
            name: value.name,
            bound: AssociatedTypeBound::Owned,
        }
    }
}
impl TryFrom<&AssociatedTypeMember> for Member33 {
    type Error = Diagnostic;
    fn try_from(value: &AssociatedTypeMember) -> Result<Self, Diagnostic> {
        if !matches!(value.bound, AssociatedTypeBound::Owned) {
            return Err(legacy_content());
        }
        Ok(Self {
            id: value.id,
            name: value.name.clone(),
            bound: Bound33::Owned,
        })
    }
}
impl From<Binding33> for AssociatedTypeBinding {
    fn from(value: Binding33) -> Self {
        Self {
            member: value.member,
            value: AssociatedTypeBindingValue::Owned { ty: value.ty },
        }
    }
}
impl TryFrom<&AssociatedTypeBinding> for Binding33 {
    type Error = Diagnostic;
    fn try_from(value: &AssociatedTypeBinding) -> Result<Self, Diagnostic> {
        match value.value {
            AssociatedTypeBindingValue::Owned { ty } => Ok(Self {
                member: value.member,
                ty,
            }),
            _ => Err(legacy_content()),
        }
    }
}
impl From<Signature33> for CompiledSignature {
    fn from(value: Signature33) -> Self {
        Self {
            implementation_parameters: value.implementation_parameters,
            requirement_parameters: value.requirement_parameters,
            effect_parameters: value.effect_parameters,
            effect: value.effect,
            type_parameters: value.type_parameters,
            type_parameter_constraints: value.type_parameter_constraints,
            parameters: value.parameters,
            result: CallableResult {
                ty: value.result,
                relation: value.result_borrow.map_or(ResultRelation::Value, |source| {
                    ResultRelation::ReadFrom { source }
                }),
            },
            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
            task_requirements: value.task_requirements,
        }
    }
}
impl TryFrom<&CompiledSignature> for Signature33 {
    type Error = Diagnostic;
    fn try_from(value: &CompiledSignature) -> Result<Self, Diagnostic> {
        require_empty(&value.read_scope_parameters)?;
        require_empty(&value.read_scope_sources)?;
        let result_borrow = match value.result.relation {
            ResultRelation::Value => None,
            ResultRelation::ReadFrom { source } => Some(source),
            ResultRelation::ViewFrom { .. } => return Err(legacy_content()),
        };
        Ok(Self {
            implementation_parameters: value.implementation_parameters.clone(),
            requirement_parameters: value.requirement_parameters.clone(),
            effect_parameters: value.effect_parameters.clone(),
            effect: value.effect.clone(),
            type_parameters: value.type_parameters.clone(),
            type_parameter_constraints: value.type_parameter_constraints.clone(),
            parameters: value.parameters.clone(),
            result: value.result.ty,
            result_borrow,
            task_requirements: value.task_requirements.clone(),
        })
    }
}
impl From<Code33> for CompiledCode {
    fn from(value: Code33) -> Self {
        Self {
            parameter_count: value.parameter_count,
            local_count: value.local_count,
            instructions: value.instructions.into_iter().map(Into::into).collect(),
        }
    }
}
impl TryFrom<&CompiledCode> for Code33 {
    type Error = Diagnostic;
    fn try_from(value: &CompiledCode) -> Result<Self, Diagnostic> {
        Ok(Self {
            parameter_count: value.parameter_count,
            local_count: value.local_count,
            instructions: value
                .instructions
                .iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}
impl From<PortImplementation33> for CompiledPortImplementation {
    fn from(value: PortImplementation33) -> Self {
        match value {
            PortImplementation33::Function(i) => Self::Function(i),
            PortImplementation33::Expression(code) => Self::Expression(code.into()),
        }
    }
}
impl TryFrom<&CompiledPortImplementation> for PortImplementation33 {
    type Error = Diagnostic;
    fn try_from(value: &CompiledPortImplementation) -> Result<Self, Diagnostic> {
        Ok(match value {
            CompiledPortImplementation::Function(i) => Self::Function(*i),
            CompiledPortImplementation::Expression(code) => Self::Expression(code.try_into()?),
        })
    }
}
impl From<Port33> for CompiledPort {
    fn from(value: Port33) -> Self {
        Self {
            port: value.port,
            function_type: value.function_type,
            implementation: value.implementation.into(),
        }
    }
}
impl TryFrom<&CompiledPort> for Port33 {
    type Error = Diagnostic;
    fn try_from(value: &CompiledPort) -> Result<Self, Diagnostic> {
        Ok(Self {
            port: value.port,
            function_type: value.function_type,
            implementation: (&value.implementation).try_into()?,
        })
    }
}
impl From<Tables33> for CompilationTables {
    fn from(value: Tables33) -> Self {
        Self {
            declarations: value.declarations,
            fields: value.fields,
            cases: value.cases,
            requirements: value.requirements,
            operations: value.operations,
            ports: value.ports,
            types: value.types,
            structural_names: value.structural_names,
            texts: value.texts,
            read_view_fields: Vec::new(),
        }
    }
}
impl TryFrom<&CompilationTables> for Tables33 {
    type Error = Diagnostic;
    fn try_from(value: &CompilationTables) -> Result<Self, Diagnostic> {
        require_empty(&value.read_view_fields)?;
        Ok(Self {
            declarations: value.declarations.clone(),
            fields: value.fields.clone(),
            cases: value.cases.clone(),
            requirements: value.requirements.clone(),
            operations: value.operations.clone(),
            ports: value.ports.clone(),
            types: value.types.clone(),
            structural_names: value.structural_names.clone(),
            texts: value.texts.clone(),
        })
    }
}

impl From<Instruction33> for CompiledInstruction {
    fn from(value: Instruction33) -> Self {
        match value {
            Instruction33::ImplementationCall {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                implementations,
                arguments,
            } => CompiledInstruction::ImplementationCall {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                implementations,
                arguments,
                read_scope_arguments: Vec::new(),
            },
            Instruction33::MethodCall {
                witness,
                contract,
                method,
                arguments,
            } => CompiledInstruction::MethodCall {
                witness,
                contract,
                method,
                arguments,
                read_scope_arguments: Vec::new(),
            },
            Instruction33::Unit => CompiledInstruction::Unit,
            Instruction33::Bool(value) => CompiledInstruction::Bool(value),
            Instruction33::I64(value) => CompiledInstruction::I64(value),
            Instruction33::Text(value) => CompiledInstruction::Text(value),
            Instruction33::StaticText(value) => CompiledInstruction::StaticText(value),
            Instruction33::LoadLocal { local, use_mode } => {
                CompiledInstruction::LoadLocal { local, use_mode }
            }
            Instruction33::StoreLocal(value) => CompiledInstruction::StoreLocal(value),
            Instruction33::Drop => CompiledInstruction::Drop,
            Instruction33::JumpIfFalse(value) => CompiledInstruction::JumpIfFalse(value),
            Instruction33::Jump(value) => CompiledInstruction::Jump(value),
            Instruction33::Call {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
            } => CompiledInstruction::Call {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
                read_scope_arguments: Vec::new(),
            },
            Instruction33::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
            } => CompiledInstruction::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                read_scope_arguments: Vec::new(),
            },
            Instruction33::Invoke { arguments } => CompiledInstruction::Invoke { arguments },
            Instruction33::Record {
                nominal_type,
                type_arguments,
                fields,
            } => CompiledInstruction::Record {
                nominal_type,
                type_arguments,
                fields,
            },
            Instruction33::Variant {
                case,
                type_arguments,
                has_payload,
            } => CompiledInstruction::Variant {
                case,
                type_arguments,
                has_payload,
            },
            Instruction33::Field(value) => CompiledInstruction::Field(value),
            Instruction33::List { item_type, items } => {
                CompiledInstruction::List { item_type, items }
            }
            Instruction33::Map {
                key_type,
                value_type,
                entries,
            } => CompiledInstruction::Map {
                key_type,
                value_type,
                entries,
            },
            Instruction33::SwitchVariant(value) => CompiledInstruction::SwitchVariant(value),
            Instruction33::Perform {
                requirement,
                operation,
                arguments,
            } => CompiledInstruction::Perform {
                requirement,
                operation,
                arguments,
            },
            Instruction33::BeginTransaction {
                requirement,
                binding,
            } => CompiledInstruction::BeginTransaction {
                requirement,
                binding,
            },
            Instruction33::CommitTransaction {
                requirement,
                binding,
            } => CompiledInstruction::CommitTransaction {
                requirement,
                binding,
            },
            Instruction33::PerformParameter {
                parameter,
                operation,
                arguments,
            } => CompiledInstruction::PerformParameter {
                parameter,
                operation,
                arguments,
            },
            Instruction33::BeginParameterTransaction { parameter, binding } => {
                CompiledInstruction::BeginParameterTransaction { parameter, binding }
            }
            Instruction33::CommitParameterTransaction { parameter, binding } => {
                CompiledInstruction::CommitParameterTransaction { parameter, binding }
            }
            Instruction33::Return => CompiledInstruction::Return,
            Instruction33::Bind { arguments } => CompiledInstruction::Bind { arguments },
            Instruction33::BeginBind { arguments } => CompiledInstruction::BeginBind { arguments },
            Instruction33::Capture { index } => CompiledInstruction::Capture { index },
            Instruction33::BeginTransactionOutcome {
                requirement,
                binding,
                outcome,
            } => CompiledInstruction::BeginTransactionOutcome {
                requirement,
                binding,
                outcome,
            },
            Instruction33::CommitTransactionOutcome {
                requirement,
                binding,
                outcome,
            } => CompiledInstruction::CommitTransactionOutcome {
                requirement,
                binding,
                outcome,
            },
            Instruction33::F64(value) => CompiledInstruction::F64(value),
            Instruction33::PackOwned {
                product_type,
                fields,
            } => CompiledInstruction::PackOwned {
                product_type,
                fields,
            },
            Instruction33::UnpackOwned {
                product_type,
                locals,
            } => CompiledInstruction::UnpackOwned {
                product_type,
                locals,
            },
            Instruction33::ChooseOwned { choice_type, case } => {
                CompiledInstruction::ChooseOwned { choice_type, case }
            }
            Instruction33::MatchOwned { choice_type, cases } => {
                CompiledInstruction::MatchOwned { choice_type, cases }
            }
            Instruction33::Parallel {
                left_types,
                left_implementations,
                right_types,
                right_implementations,
                left,
                left_arguments,
                right,
                right_arguments,
                result_type,
            } => CompiledInstruction::Parallel {
                left_types,
                left_implementations,
                right_types,
                right_implementations,
                left,
                left_arguments,
                right,
                right_arguments,
                result_type,
                left_read_scope_arguments: Vec::new(),
                right_read_scope_arguments: Vec::new(),
            },
            Instruction33::BorrowOwnedField {
                product_type,
                source_local,
                field,
                binding_local,
                binding_type,
            } => CompiledInstruction::BorrowOwnedField {
                product_type,
                source_local,
                field,
                binding_local,
                binding_type,
            },
            Instruction33::MatchBorrowedOwned {
                choice_type,
                source_local,
                cases,
            } => CompiledInstruction::MatchBorrowedOwned {
                choice_type,
                source_local,
                cases,
            },
            Instruction33::EndOwnedBorrow { binding_local } => {
                CompiledInstruction::EndOwnedBorrow { binding_local }
            }
            Instruction33::SequenceEmpty { sequence_type } => {
                CompiledInstruction::SequenceEmpty { sequence_type }
            }
            Instruction33::SequenceLength {
                sequence_type,
                source_local,
            } => CompiledInstruction::SequenceLength {
                sequence_type,
                source_local,
            },
            Instruction33::SequencePush {
                sequence_type,
                source_local,
            } => CompiledInstruction::SequencePush {
                sequence_type,
                source_local,
            },
            Instruction33::SequencePop {
                sequence_type,
                result_type,
                source_local,
            } => CompiledInstruction::SequencePop {
                sequence_type,
                result_type,
                source_local,
            },
            Instruction33::BorrowOwnedItem {
                sequence_type,
                source_local,
                binding_local,
                binding_type,
            } => CompiledInstruction::BorrowOwnedItem {
                sequence_type,
                source_local,
                binding_local,
                binding_type,
            },
            Instruction33::BeginBorrowCall {
                source_local,
                source_position,
            } => CompiledInstruction::BeginBorrowCall {
                source_local,
                source_position,
            },
            Instruction33::AdoptBorrowResult {
                source_local,
                binding_local,
                binding_type,
            } => CompiledInstruction::AdoptBorrowResult {
                source_local,
                binding_local,
                binding_type,
            },
            Instruction33::ReturnBorrowed => CompiledInstruction::ReturnBorrowed,
            Instruction33::SequenceGet {
                sequence_type,
                source_local,
            } => CompiledInstruction::SequenceGet {
                sequence_type,
                source_local,
            },
            Instruction33::SequenceReplace {
                sequence_type,
                result_type,
                source_local,
            } => CompiledInstruction::SequenceReplace {
                sequence_type,
                result_type,
                source_local,
            },
        }
    }
}

impl TryFrom<&CompiledInstruction> for Instruction33 {
    type Error = Diagnostic;
    fn try_from(value: &CompiledInstruction) -> Result<Self, Diagnostic> {
        Ok(match value {
            CompiledInstruction::ImplementationCall {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                implementations,
                arguments,
                read_scope_arguments,
            } => {
                require_empty(read_scope_arguments)?;
                Instruction33::ImplementationCall {
                    requirement_arguments: requirement_arguments.clone(),
                    effect_arguments: effect_arguments.clone(),
                    function: *function,
                    type_arguments: type_arguments.clone(),
                    implementations: implementations.clone(),
                    arguments: *arguments,
                }
            }
            CompiledInstruction::MethodCall {
                witness,
                contract,
                method,
                arguments,
                read_scope_arguments,
            } => {
                require_empty(read_scope_arguments)?;
                Instruction33::MethodCall {
                    witness: witness.clone(),
                    contract: *contract,
                    method: *method,
                    arguments: *arguments,
                }
            }
            CompiledInstruction::Unit => Instruction33::Unit,
            CompiledInstruction::Bool(value) => Instruction33::Bool(*value),
            CompiledInstruction::I64(value) => Instruction33::I64(*value),
            CompiledInstruction::Text(value) => Instruction33::Text(*value),
            CompiledInstruction::StaticText(value) => Instruction33::StaticText(*value),
            CompiledInstruction::LoadLocal { local, use_mode } => Instruction33::LoadLocal {
                local: *local,
                use_mode: *use_mode,
            },
            CompiledInstruction::StoreLocal(value) => Instruction33::StoreLocal(*value),
            CompiledInstruction::Drop => Instruction33::Drop,
            CompiledInstruction::JumpIfFalse(value) => Instruction33::JumpIfFalse(*value),
            CompiledInstruction::Jump(value) => Instruction33::Jump(*value),
            CompiledInstruction::Call {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                arguments,
                read_scope_arguments,
            } => {
                require_empty(read_scope_arguments)?;
                Instruction33::Call {
                    requirement_arguments: requirement_arguments.clone(),
                    effect_arguments: effect_arguments.clone(),
                    function: *function,
                    type_arguments: type_arguments.clone(),
                    arguments: *arguments,
                }
            }
            CompiledInstruction::FunctionValue {
                requirement_arguments,
                effect_arguments,
                function,
                type_arguments,
                read_scope_arguments,
            } => {
                require_empty(read_scope_arguments)?;
                Instruction33::FunctionValue {
                    requirement_arguments: requirement_arguments.clone(),
                    effect_arguments: effect_arguments.clone(),
                    function: *function,
                    type_arguments: type_arguments.clone(),
                }
            }
            CompiledInstruction::Invoke { arguments } => Instruction33::Invoke {
                arguments: *arguments,
            },
            CompiledInstruction::Record {
                nominal_type,
                type_arguments,
                fields,
            } => Instruction33::Record {
                nominal_type: *nominal_type,
                type_arguments: type_arguments.clone(),
                fields: fields.clone(),
            },
            CompiledInstruction::Variant {
                case,
                type_arguments,
                has_payload,
            } => Instruction33::Variant {
                case: *case,
                type_arguments: type_arguments.clone(),
                has_payload: *has_payload,
            },
            CompiledInstruction::Field(value) => Instruction33::Field(*value),
            CompiledInstruction::List { item_type, items } => Instruction33::List {
                item_type: *item_type,
                items: *items,
            },
            CompiledInstruction::Map {
                key_type,
                value_type,
                entries,
            } => Instruction33::Map {
                key_type: *key_type,
                value_type: *value_type,
                entries: *entries,
            },
            CompiledInstruction::SwitchVariant(value) => {
                Instruction33::SwitchVariant(value.clone())
            }
            CompiledInstruction::Perform {
                requirement,
                operation,
                arguments,
            } => Instruction33::Perform {
                requirement: *requirement,
                operation: *operation,
                arguments: *arguments,
            },
            CompiledInstruction::BeginTransaction {
                requirement,
                binding,
            } => Instruction33::BeginTransaction {
                requirement: *requirement,
                binding: *binding,
            },
            CompiledInstruction::CommitTransaction {
                requirement,
                binding,
            } => Instruction33::CommitTransaction {
                requirement: *requirement,
                binding: *binding,
            },
            CompiledInstruction::PerformParameter {
                parameter,
                operation,
                arguments,
            } => Instruction33::PerformParameter {
                parameter: *parameter,
                operation: *operation,
                arguments: *arguments,
            },
            CompiledInstruction::BeginParameterTransaction { parameter, binding } => {
                Instruction33::BeginParameterTransaction {
                    parameter: *parameter,
                    binding: *binding,
                }
            }
            CompiledInstruction::CommitParameterTransaction { parameter, binding } => {
                Instruction33::CommitParameterTransaction {
                    parameter: *parameter,
                    binding: *binding,
                }
            }
            CompiledInstruction::Return => Instruction33::Return,
            CompiledInstruction::Bind { arguments } => Instruction33::Bind {
                arguments: *arguments,
            },
            CompiledInstruction::BeginBind { arguments } => Instruction33::BeginBind {
                arguments: *arguments,
            },
            CompiledInstruction::Capture { index } => Instruction33::Capture { index: *index },
            CompiledInstruction::BeginTransactionOutcome {
                requirement,
                binding,
                outcome,
            } => Instruction33::BeginTransactionOutcome {
                requirement: *requirement,
                binding: *binding,
                outcome: *outcome,
            },
            CompiledInstruction::CommitTransactionOutcome {
                requirement,
                binding,
                outcome,
            } => Instruction33::CommitTransactionOutcome {
                requirement: *requirement,
                binding: *binding,
                outcome: *outcome,
            },
            CompiledInstruction::F64(value) => Instruction33::F64(*value),
            CompiledInstruction::PackOwned {
                product_type,
                fields,
            } => Instruction33::PackOwned {
                product_type: *product_type,
                fields: fields.clone(),
            },
            CompiledInstruction::UnpackOwned {
                product_type,
                locals,
            } => Instruction33::UnpackOwned {
                product_type: *product_type,
                locals: locals.clone(),
            },
            CompiledInstruction::ChooseOwned { choice_type, case } => Instruction33::ChooseOwned {
                choice_type: *choice_type,
                case: *case,
            },
            CompiledInstruction::MatchOwned { choice_type, cases } => Instruction33::MatchOwned {
                choice_type: *choice_type,
                cases: cases.clone(),
            },
            CompiledInstruction::Parallel {
                left_types,
                left_implementations,
                right_types,
                right_implementations,
                left,
                left_arguments,
                right,
                right_arguments,
                result_type,
                left_read_scope_arguments,
                right_read_scope_arguments,
            } => {
                require_empty(left_read_scope_arguments)?;
                require_empty(right_read_scope_arguments)?;
                Instruction33::Parallel {
                    left_types: left_types.clone(),
                    left_implementations: left_implementations.clone(),
                    right_types: right_types.clone(),
                    right_implementations: right_implementations.clone(),
                    left: *left,
                    left_arguments: *left_arguments,
                    right: *right,
                    right_arguments: *right_arguments,
                    result_type: *result_type,
                }
            }
            CompiledInstruction::BorrowOwnedField {
                product_type,
                source_local,
                field,
                binding_local,
                binding_type,
            } => Instruction33::BorrowOwnedField {
                product_type: *product_type,
                source_local: *source_local,
                field: *field,
                binding_local: *binding_local,
                binding_type: *binding_type,
            },
            CompiledInstruction::MatchBorrowedOwned {
                choice_type,
                source_local,
                cases,
            } => Instruction33::MatchBorrowedOwned {
                choice_type: *choice_type,
                source_local: *source_local,
                cases: cases.clone(),
            },
            CompiledInstruction::EndOwnedBorrow { binding_local } => {
                Instruction33::EndOwnedBorrow {
                    binding_local: *binding_local,
                }
            }
            CompiledInstruction::SequenceEmpty { sequence_type } => Instruction33::SequenceEmpty {
                sequence_type: *sequence_type,
            },
            CompiledInstruction::SequenceLength {
                sequence_type,
                source_local,
            } => Instruction33::SequenceLength {
                sequence_type: *sequence_type,
                source_local: *source_local,
            },
            CompiledInstruction::SequencePush {
                sequence_type,
                source_local,
            } => Instruction33::SequencePush {
                sequence_type: *sequence_type,
                source_local: *source_local,
            },
            CompiledInstruction::SequencePop {
                sequence_type,
                result_type,
                source_local,
            } => Instruction33::SequencePop {
                sequence_type: *sequence_type,
                result_type: *result_type,
                source_local: *source_local,
            },
            CompiledInstruction::BorrowOwnedItem {
                sequence_type,
                source_local,
                binding_local,
                binding_type,
            } => Instruction33::BorrowOwnedItem {
                sequence_type: *sequence_type,
                source_local: *source_local,
                binding_local: *binding_local,
                binding_type: *binding_type,
            },
            CompiledInstruction::BeginBorrowCall {
                source_local,
                source_position,
            } => Instruction33::BeginBorrowCall {
                source_local: *source_local,
                source_position: *source_position,
            },
            CompiledInstruction::AdoptBorrowResult {
                source_local,
                binding_local,
                binding_type,
            } => Instruction33::AdoptBorrowResult {
                source_local: *source_local,
                binding_local: *binding_local,
                binding_type: *binding_type,
            },
            CompiledInstruction::ReturnBorrowed => Instruction33::ReturnBorrowed,
            CompiledInstruction::SequenceGet {
                sequence_type,
                source_local,
            } => Instruction33::SequenceGet {
                sequence_type: *sequence_type,
                source_local: *source_local,
            },
            CompiledInstruction::SequenceReplace {
                sequence_type,
                result_type,
                source_local,
            } => Instruction33::SequenceReplace {
                sequence_type: *sequence_type,
                result_type: *result_type,
                source_local: *source_local,
            },
            _ => return Err(legacy_content()),
        })
    }
}

impl From<Payload33> for CompilationPayload {
    fn from(value: Payload33) -> Self {
        match value {
            Payload33::Contract33(value) => CompilationPayload::OwnedContract(value.into()),
            Payload33::Implementation33(value) => {
                CompilationPayload::OwnedImplementation(value.into())
            }
            Payload33::Record {
                type_parameters,
                type_parameter_constraints,
                fields,
            } => CompilationPayload::Record {
                type_parameters,
                type_parameter_constraints,
                fields,
            },
            Payload33::Variant {
                type_parameters,
                type_parameter_constraints,
                cases,
            } => CompilationPayload::Variant {
                type_parameters,
                type_parameter_constraints,
                cases,
            },
            Payload33::Interface { operations } => CompilationPayload::Interface { operations },
            Payload33::External {
                signature,
                implementation,
            } => CompilationPayload::External {
                signature: signature.into(),
                implementation,
            },
            Payload33::Function { signature, code } => CompilationPayload::Function {
                signature: signature.into(),
                code: code.into(),
            },
            Payload33::Constant { ty, code } => CompilationPayload::Constant {
                ty,
                code: code.into(),
            },
            Payload33::Component {
                requirements,
                ports,
            } => CompilationPayload::Component {
                requirements,
                ports: ports.into_iter().map(Into::into).collect(),
            },
            Payload33::Test {
                actual,
                expected,
                comparison,
            } => CompilationPayload::Test {
                actual: actual.into(),
                expected: expected.into(),
                comparison,
            },
            Payload33::Target {
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
            Payload33::OwnedRecord {
                type_parameters,
                type_parameter_constraints,
                fields,
            } => CompilationPayload::OwnedRecord {
                type_parameters,
                type_parameter_constraints,
                fields,
            },
        }
    }
}

impl TryFrom<&CompilationPayload> for Payload33 {
    type Error = Diagnostic;
    fn try_from(value: &CompilationPayload) -> Result<Self, Diagnostic> {
        Ok(match value {
            CompilationPayload::OwnedContract(value) => Payload33::Contract33(value.try_into()?),
            CompilationPayload::OwnedImplementation(value) => {
                Payload33::Implementation33(value.try_into()?)
            }
            CompilationPayload::Record {
                type_parameters,
                type_parameter_constraints,
                fields,
            } => Payload33::Record {
                type_parameters: type_parameters.clone(),
                type_parameter_constraints: type_parameter_constraints.clone(),
                fields: fields.clone(),
            },
            CompilationPayload::Variant {
                type_parameters,
                type_parameter_constraints,
                cases,
            } => Payload33::Variant {
                type_parameters: type_parameters.clone(),
                type_parameter_constraints: type_parameter_constraints.clone(),
                cases: cases.clone(),
            },
            CompilationPayload::Interface { operations } => Payload33::Interface {
                operations: operations.clone(),
            },
            CompilationPayload::External {
                signature,
                implementation,
            } => Payload33::External {
                signature: signature.try_into()?,
                implementation: implementation.clone(),
            },
            CompilationPayload::Function { signature, code } => Payload33::Function {
                signature: signature.try_into()?,
                code: code.try_into()?,
            },
            CompilationPayload::Constant { ty, code } => Payload33::Constant {
                ty: *ty,
                code: code.try_into()?,
            },
            CompilationPayload::Component {
                requirements,
                ports,
            } => Payload33::Component {
                requirements: requirements.clone(),
                ports: ports
                    .iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()?,
            },
            CompilationPayload::Test {
                actual,
                expected,
                comparison,
            } => Payload33::Test {
                actual: actual.try_into()?,
                expected: expected.try_into()?,
                comparison: *comparison,
            },
            CompilationPayload::Target {
                component,
                port,
                routes,
                runner,
            } => Payload33::Target {
                component: *component,
                port: *port,
                routes: routes.clone(),
                runner: *runner,
            },
            CompilationPayload::OwnedRecord {
                type_parameters,
                type_parameter_constraints,
                fields,
            } => Payload33::OwnedRecord {
                type_parameters: type_parameters.clone(),
                type_parameter_constraints: type_parameter_constraints.clone(),
                fields: fields.clone(),
            },
            _ => return Err(legacy_content()),
        })
    }
}

impl From<Unit33> for CompilationUnit {
    fn from(value: Unit33) -> Self {
        Self {
            contract_version: value.contract_version,
            graph_contract_version: value.graph_contract_version,
            bytecode_contract_version: value.bytecode_contract_version,
            key: value.key,
            source: value.source,
            optimization: value.optimization,
            tables: value.tables.into(),
            payload: value.payload.into(),
        }
    }
}
impl TryFrom<&CompilationUnit> for Unit33 {
    type Error = Diagnostic;
    fn try_from(value: &CompilationUnit) -> Result<Self, Diagnostic> {
        if (
            value.contract_version,
            value.graph_contract_version,
            value.bytecode_contract_version,
        ) != (33, 33, 27)
        {
            return Err(legacy_content());
        }
        Ok(Self {
            contract_version: value.contract_version,
            graph_contract_version: value.graph_contract_version,
            bytecode_contract_version: value.bytecode_contract_version,
            key: value.key,
            source: value.source.clone(),
            optimization: value.optimization,
            tables: (&value.tables).try_into()?,
            payload: (&value.payload).try_into()?,
        })
    }
}

enum Inventory33 {
    Contract(Vec<Member33>),
    Implementation(Vec<Binding33>),
}
struct Associated33 {
    unit: Unit33,
    inventory: Inventory33,
}
impl Encode for Associated33 {
    fn encode<E: bincode::enc::Encoder>(
        &self,
        e: &mut E,
    ) -> Result<(), bincode::error::EncodeError> {
        1_u16.encode(e)?;
        let tag = match &self.inventory {
            Inventory33::Contract(_) => 1_u8,
            Inventory33::Implementation(_) => 2_u8,
        };
        tag.encode(e)?;
        self.unit.contract_version.encode(e)?;
        self.unit.graph_contract_version.encode(e)?;
        self.unit.bytecode_contract_version.encode(e)?;
        self.unit.key.encode(e)?;
        self.unit.source.encode(e)?;
        self.unit.optimization.encode(e)?;
        self.unit.tables.encode(e)?;
        match (&self.unit.payload, &self.inventory) {
            (Payload33::Contract33(body), Inventory33::Contract(items)) => {
                body.encode(e)?;
                items.encode(e)
            }
            (Payload33::Implementation33(body), Inventory33::Implementation(items)) => {
                body.encode(e)?;
                items.encode(e)
            }
            _ => Err(bincode::error::EncodeError::Other(
                "predecessor associated unit body/tag mismatch",
            )),
        }
    }
}
impl<C> Decode<C> for Associated33 {
    fn decode<D: bincode::de::Decoder<Context = C>>(
        d: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        if u16::decode(d)? != 1 {
            return Err(bincode::error::DecodeError::Other(
                "foreign predecessor associated profile",
            ));
        }
        let tag = u8::decode(d)?;
        let contract_version = u16::decode(d)?;
        let graph_contract_version = u16::decode(d)?;
        let bytecode_contract_version = u16::decode(d)?;
        let key = CompilationUnitKey::decode(d)?;
        let source = CompilationSource::decode(d)?;
        let optimization = OptimizationPolicy::decode(d)?;
        let tables = Tables33::decode(d)?;
        let (payload, inventory) = match tag {
            1 => (
                Payload33::Contract33(Contract33::decode(d)?),
                Inventory33::Contract(Vec::<Member33>::decode(d)?),
            ),
            2 => (
                Payload33::Implementation33(Implementation33::decode(d)?),
                Inventory33::Implementation(Vec::<Binding33>::decode(d)?),
            ),
            _ => {
                return Err(bincode::error::DecodeError::Other(
                    "foreign predecessor associated body",
                ));
            }
        };
        Ok(Self {
            unit: Unit33 {
                contract_version,
                graph_contract_version,
                bytecode_contract_version,
                key,
                source,
                optimization,
                tables,
                payload,
            },
            inventory,
        })
    }
}
bincode::impl_borrow_decode!(Associated33);

/// Test-only adversarial compiler29 fixtures used this pre-view payload layout.
/// This is not an additional production reader or a source-profile upgrade.
#[cfg(test)]
pub(super) fn decode_ordinary29_for_test(bytes: &[u8]) -> Result<CompilationUnit, Diagnostic> {
    let raw: Unit33 = crate::platform::packed::decode(
        bytes,
        *b"LKJCUN29",
        "lkjscript.compiler-unit-envelope.v29",
        MAXIMUM_COMPILER_UNIT_BYTES,
    )?;
    if (
        raw.contract_version,
        raw.graph_contract_version,
        raw.bytecode_contract_version,
    ) != (29, 29, 24)
    {
        return Err(legacy_content());
    }
    Ok(raw.into())
}

pub(super) fn decode(bytes: &[u8]) -> Result<CompilationUnit, Diagnostic> {
    let value = if bytes.starts_with(&FROZEN_ASSOCIATED_UNIT_MAGIC) {
        let raw: Associated33 = crate::platform::packed::decode(
            bytes,
            FROZEN_ASSOCIATED_UNIT_MAGIC,
            FROZEN_ASSOCIATED_UNIT_ENVELOPE_DOMAIN,
            MAXIMUM_COMPILER_UNIT_BYTES,
        )?;
        let mut unit: CompilationUnit = raw.unit.into();
        match (&mut unit.payload, raw.inventory) {
            (CompilationPayload::OwnedContract(body), Inventory33::Contract(items)) => {
                body.associated_types = items.into_iter().map(Into::into).collect()
            }
            (CompilationPayload::OwnedImplementation(body), Inventory33::Implementation(items)) => {
                body.associated_types = items.into_iter().map(Into::into).collect()
            }
            _ => return Err(legacy_content()),
        }
        unit
    } else {
        let raw: Unit33 = crate::platform::packed::decode(
            bytes,
            FROZEN_COMPILER_UNIT_MAGIC,
            FROZEN_COMPILER_UNIT_ENVELOPE_DOMAIN,
            MAXIMUM_COMPILER_UNIT_BYTES,
        )?;
        raw.into()
    };
    if (
        value.contract_version,
        value.graph_contract_version,
        value.bytecode_contract_version,
    ) != (33, 33, 27)
    {
        return Err(legacy_content());
    }
    Ok(value)
}

pub(super) fn encode(value: &CompilationUnit) -> Result<Vec<u8>, Diagnostic> {
    let unit = Unit33::try_from(value)?;
    let inventory = match &value.payload {
        CompilationPayload::OwnedContract(body) if !body.associated_types.is_empty() => {
            Some(Inventory33::Contract(
                body.associated_types
                    .iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()?,
            ))
        }
        CompilationPayload::OwnedImplementation(body) if !body.associated_types.is_empty() => {
            Some(Inventory33::Implementation(
                body.associated_types
                    .iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()?,
            ))
        }
        _ => None,
    };
    if let Some(inventory) = inventory {
        crate::platform::packed::encode(
            FROZEN_ASSOCIATED_UNIT_MAGIC,
            FROZEN_ASSOCIATED_UNIT_ENVELOPE_DOMAIN,
            &Associated33 { unit, inventory },
            MAXIMUM_COMPILER_UNIT_BYTES,
        )
    } else {
        crate::platform::packed::encode(
            FROZEN_COMPILER_UNIT_MAGIC,
            FROZEN_COMPILER_UNIT_ENVELOPE_DOMAIN,
            &unit,
            MAXIMUM_COMPILER_UNIT_BYTES,
        )
    }
}
