//! Canonical normalized compiler-unit and typed-bytecode records.

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    BlobObjectDigest, CaseReference, ComparisonPolicy, DeclarationReference, ExternalVisibility,
    FieldReference, Idempotency, ImplementationName, Name, OperationReference, OwnerKey, OwnerKind,
    PackageId, ParameterUse, PortReference, RequirementReference, ResourceLimit, TypeObjectDigest,
};
use crate::platform::package::RunnerKind;
use crate::platform::semantic_id::{HttpRouteId, ParameterId, TypeParameterId};
use crate::platform::storage::object::{ObjectDomain, ObjectKey};
use crate::platform::witness::SemanticDigest;
use bincode::{Decode, Encode};
use std::collections::BTreeSet;
use std::fmt;

pub const COMPILER_UNIT_CONTRACT_IDENTITY: &str = "lkjscript-compiler-unit-30";
pub const COMPILER_UNIT_CONTRACT_VERSION: u16 = 30;
pub const BYTECODE_CONTRACT_IDENTITY: &str = "lkjscript-bytecode-25";
pub const BYTECODE_CONTRACT_VERSION: u16 = 25;
pub(crate) const COMPILER_UNIT_MAGIC: [u8; 8] = *b"LKJCUN30";
pub(crate) const COMPILER_UNIT_ENVELOPE_DOMAIN: &str = "lkjscript.compiler-unit-envelope.v30";
pub(crate) const COMPILER_UNIT_KEY_DOMAIN: &str = "lkjscript.compiler-unit-key.v30";
pub(crate) const MAXIMUM_COMPILER_UNIT_BYTES: usize = 8 * 1024 * 1024;
pub(crate) const MAXIMUM_COMPILER_UNIT_ITEMS: usize = 1_000_000;

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompilationUnitKey([u8; 32]);

impl CompilationUnitKey {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    pub(crate) fn derive(
        source: &CompilationSource,
        optimization: OptimizationPolicy,
    ) -> Result<Self, Diagnostic> {
        let core = CompilationKeyCore {
            compiler_contract_version: COMPILER_UNIT_CONTRACT_VERSION,
            bytecode_contract_version: BYTECODE_CONTRACT_VERSION,
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            source: source.clone(),
            optimization,
        };
        let bytes = bincode::encode_to_vec(core, bincode::config::standard()).map_err(|error| {
            unit_error(
                DiagnosticClass::Infrastructure,
                "compiler_unit_key_encode",
                format!("failed to encode compiler-unit key: {error}"),
            )
        })?;
        let mut hasher = blake3::Hasher::new_derive_key(COMPILER_UNIT_KEY_DOMAIN);
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
        Ok(Self(*hasher.finalize().as_bytes()))
    }

    #[cfg(test)]
    pub(crate) fn derive_generation(
        source: &CompilationSource,
        optimization: OptimizationPolicy,
        compiler_contract_version: u16,
        bytecode_contract_version: u16,
        graph_contract_version: u16,
    ) -> Result<Self, Diagnostic> {
        let core = CompilationKeyCore {
            compiler_contract_version,
            bytecode_contract_version,
            graph_contract_version,
            source: source.clone(),
            optimization,
        };
        let bytes = bincode::encode_to_vec(core, bincode::config::standard()).map_err(|error| {
            unit_error(
                DiagnosticClass::Infrastructure,
                "compiler_unit_key_encode",
                format!("failed to encode compiler-unit key: {error}"),
            )
        })?;
        let domain = format!("lkjscript.compiler-unit-key.v{compiler_contract_version}");
        let mut hasher = blake3::Hasher::new_derive_key(&domain);
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
        Ok(Self(*hasher.finalize().as_bytes()))
    }
}

impl fmt::Display for CompilationUnitKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("compiler_unit_key_")?;
        formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
    }
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
struct CompilationKeyCore {
    compiler_contract_version: u16,
    bytecode_contract_version: u16,
    graph_contract_version: u16,
    source: CompilationSource,
    optimization: OptimizationPolicy,
}

/// Summary dimensions that fully bind reusable executable lowering while deliberately excluding
/// mutable presentation and namespace state.
#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompilationSource {
    pub package: PackageId,
    pub owner: OwnerKey,
    pub kind: OwnerKind,
    pub semantic_interface: SemanticDigest,
    pub implementation: SemanticDigest,
    pub type_digest: SemanticDigest,
    pub effect: SemanticDigest,
    pub capability: SemanticDigest,
    pub test: Option<SemanticDigest>,
    pub validation_dependencies: SemanticDigest,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub enum OptimizationPolicy {
    DeterministicBaseline,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompilationUnit {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub bytecode_contract_version: u16,
    pub key: CompilationUnitKey,
    pub source: CompilationSource,
    pub optimization: OptimizationPolicy,
    pub tables: CompilationTables,
    pub payload: CompilationPayload,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompilationTables {
    pub declarations: Vec<DeclarationReference>,
    pub fields: Vec<FieldReference>,
    pub cases: Vec<CaseReference>,
    pub requirements: Vec<RequirementReference>,
    pub operations: Vec<OperationReference>,
    pub ports: Vec<PortReference>,
    pub types: Vec<TypeObjectDigest>,
    pub structural_names: Vec<Name>,
    pub texts: Vec<CompiledText>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, Ord, PartialEq, PartialOrd)]
pub enum CompiledText {
    Inline(String),
    Blob {
        digest: BlobObjectDigest,
        bytes: u64,
    },
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum CompilationPayload {
    OwnedContract(crate::platform::kernel::OwnedContract),
    OwnedImplementation(crate::platform::kernel::OwnedImplementation),
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
        signature: CompiledSignature,
        implementation: ImplementationName,
    },
    Function {
        signature: CompiledSignature,
        code: CompiledCode,
    },
    Constant {
        ty: u32,
        code: CompiledCode,
    },
    Component {
        requirements: Vec<CompiledRequirement>,
        ports: Vec<CompiledPort>,
    },
    Test {
        actual: CompiledCode,
        expected: CompiledCode,
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
pub struct CompiledHttpRoute {
    pub route: HttpRouteId,
    pub method: String,
    pub selector: crate::platform::kernel::HttpRouteSelector,
    pub port: u32,
    pub capture_parameters: Vec<ParameterId>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledSignature {
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
pub struct CompiledParameter {
    pub parameter: ParameterId,
    pub name: Name,
    pub ty: u32,
    pub use_mode: ParameterUse,
    pub resource_requirement: Option<u32>,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledFieldLayout {
    pub field: u32,
    pub ty: u32,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledCaseLayout {
    pub case: u32,
    pub payload: Option<u32>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledOperationLayout {
    pub operation: u32,
    pub parameters: Vec<CompiledParameter>,
    pub result: u32,
    pub idempotency: Idempotency,
    pub external_visibility: ExternalVisibility,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledRequirement {
    pub requirement: u32,
    pub interface: u32,
    pub operations: Vec<u32>,
    pub limits: Vec<ResourceLimit>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledPort {
    pub port: u32,
    pub function_type: u32,
    pub implementation: CompiledPortImplementation,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum CompiledPortImplementation {
    Function(u32),
    Expression(CompiledCode),
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledCode {
    pub parameter_count: u32,
    pub local_count: u32,
    pub instructions: Vec<CompiledInstruction>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub enum CompiledInstruction {
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
        value_local: u32,
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
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledBorrowedOwnedChoiceJump {
    pub target: u32,
    pub binding_local: u32,
    pub binding_type: u32,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledOwnedChoiceJump {
    pub target: u32,
    pub binding_local: u32,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub enum CompiledTransactionRequirement {
    Concrete(u32),
    Parameter(crate::platform::kernel::RequirementParameterReference),
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledTransactionOutcome {
    pub outcome: u32,
    pub abort_reason: u32,
    pub committed: u32,
    pub aborted: u32,
    pub condition_failed: u32,
    pub conflict: u32,
    pub type_argument: u32,
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub enum CompiledFieldSelector {
    Nominal(u32),
    Structural(u32),
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct CompiledVariantJump {
    pub case: u32,
    pub target: u32,
    pub binding_local: Option<u32>,
}

impl CompilationUnit {
    pub fn encode(&self) -> Result<(ObjectKey, Vec<u8>), Diagnostic> {
        self.validate()?;
        let bytes = crate::platform::packed::encode(
            COMPILER_UNIT_MAGIC,
            COMPILER_UNIT_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_COMPILER_UNIT_BYTES,
        )?;
        Ok((
            ObjectKey::for_bytes(ObjectDomain::CompilerUnit, &bytes),
            bytes,
        ))
    }

    pub fn decode(bytes: &[u8], expected: ObjectKey) -> Result<Self, Diagnostic> {
        if expected.domain != ObjectDomain::CompilerUnit
            || ObjectKey::for_bytes(ObjectDomain::CompilerUnit, bytes) != expected
        {
            return Err(unit_error(
                DiagnosticClass::Corrupt,
                "compiler_unit_digest",
                "compiler-unit bytes disagree with their exact object-domain digest",
            ));
        }
        // Derived generations 10–29 require a rebuild from supported canonical owners.
        // Refuse before decoding; old bytes never acquire current instruction meaning.
        if [
            b"LKJCUN10",
            b"LKJCUN11",
            b"LKJCUN12",
            b"LKJCUN13",
            b"LKJCUN14",
            b"LKJCUN15",
            b"LKJCUN16",
            b"LKJCUN17",
            b"LKJCUN18",
            b"LKJCUN19",
            b"LKJCUN20",
            b"LKJCUN21",
            b"LKJCUN22",
            b"LKJCUN23",
            b"LKJCUN24",
            b"LKJCUN25",
            b"LKJCUN26",
            b"LKJCUN27",
            b"LKJCUN28",
            b"LKJCUN29",
        ]
        .iter()
        .any(|magic| bytes.starts_with(*magic))
        {
            return Err(unit_error(
                DiagnosticClass::Source,
                "compiler_unit_contract",
                "predecessor compiler units require rebuilding from canonical meaning",
            ));
        }
        let unit: Self = crate::platform::packed::decode(
            bytes,
            COMPILER_UNIT_MAGIC,
            COMPILER_UNIT_ENVELOPE_DOMAIN,
            MAXIMUM_COMPILER_UNIT_BYTES,
        )?;
        unit.validate()?;
        let (actual, canonical) = unit.encode()?;
        if actual != expected || canonical != bytes {
            return Err(unit_error(
                DiagnosticClass::Corrupt,
                "compiler_unit_canonical",
                "compiler unit does not use its canonical current encoding",
            ));
        }
        Ok(unit)
    }

    pub(crate) fn validate(&self) -> Result<(), Diagnostic> {
        if (
            self.contract_version,
            self.bytecode_contract_version,
            self.graph_contract_version,
        ) != (
            COMPILER_UNIT_CONTRACT_VERSION,
            BYTECODE_CONTRACT_VERSION,
            crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        ) {
            return Err(unit_error(
                DiagnosticClass::Source,
                "compiler_unit_contract",
                "compiler unit uses a predecessor or foreign contract",
            ));
        }
        if self.source.package.bytes() == [0; 16] {
            return Err(unit_error(
                DiagnosticClass::Corrupt,
                "compiler_unit_package",
                "compiler unit has the reserved zero package identity",
            ));
        }
        if !self.source.kind.accepts_owner(self.source.owner) {
            return Err(unit_error(
                DiagnosticClass::Corrupt,
                "compiler_unit_owner_kind",
                "compiler-unit source kind disagrees with its stable owner domain",
            ));
        }
        if matches!(self.source.kind, OwnerKind::Test) != self.source.test.is_some() {
            return Err(unit_error(
                DiagnosticClass::Corrupt,
                "compiler_unit_test_digest",
                "compiler-unit source test digest presence disagrees with its owner kind",
            ));
        }
        let expected_key = CompilationUnitKey::derive(&self.source, self.optimization)?;
        if self.key != expected_key {
            return Err(unit_error(
                DiagnosticClass::Corrupt,
                "compiler_unit_key",
                "compiler-unit key disagrees with its exact source summary dimensions",
            ));
        }
        self.tables.validate()?;
        self.payload.validate(&self.source, &self.tables)
    }
}

impl CompilationTables {
    fn validate(&self) -> Result<(), Diagnostic> {
        for (label, length) in [
            ("declarations", self.declarations.len()),
            ("fields", self.fields.len()),
            ("cases", self.cases.len()),
            ("requirements", self.requirements.len()),
            ("operations", self.operations.len()),
            ("ports", self.ports.len()),
            ("types", self.types.len()),
            ("structural names", self.structural_names.len()),
            ("texts", self.texts.len()),
        ] {
            require_item_count(label, length, true)?;
        }
        require_unique("declaration relocation", &self.declarations)?;
        require_unique("field relocation", &self.fields)?;
        require_unique("case relocation", &self.cases)?;
        require_unique("requirement relocation", &self.requirements)?;
        require_unique("operation relocation", &self.operations)?;
        require_unique("port relocation", &self.ports)?;
        require_unique("type relocation", &self.types)?;
        require_unique("structural field name", &self.structural_names)?;
        require_unique("text constant", &self.texts)?;
        for text in &self.texts {
            match text {
                CompiledText::Inline(value)
                    if value.len()
                        <= crate::platform::kernel::contract::MAXIMUM_INLINE_TEXT_BYTES => {}
                CompiledText::Inline(_) => {
                    return Err(unit_corrupt(
                        "compiler_unit_text_length",
                        "compiled inline text exceeds the Graph 14 inline bound",
                    ));
                }
                CompiledText::Blob { bytes, .. }
                    if *bytes > 0
                        && *bytes
                            <= crate::platform::storage::object::ObjectDomain::Blob.maximum_bytes()
                                as u64 => {}
                CompiledText::Blob { .. } => {
                    return Err(unit_corrupt(
                        "compiler_unit_blob_length",
                        "compiled blob text length is outside the object-domain bound",
                    ));
                }
            }
        }
        Ok(())
    }
}

impl CompilationPayload {
    fn validate(
        &self,
        source: &CompilationSource,
        tables: &CompilationTables,
    ) -> Result<(), Diagnostic> {
        match self {
            Self::OwnedContract(c) => {
                require_kind(source, OwnerKind::OwnedContract)?;
                c.validate_local()?;
                let types = tables.types.iter().copied().collect::<BTreeSet<_>>();
                for ty in c.type_roots() {
                    if !types.contains(&ty) {
                        return Err(unit_corrupt(
                            "compiler_unit_owned_contract_type",
                            "owned contract signature references a type absent from its exact table",
                        ));
                    }
                }
            }
            Self::OwnedImplementation(i) => {
                require_kind(source, OwnerKind::OwnedImplementation)?;
                i.validate_local()?;
                let declarations = tables.declarations.iter().copied().collect::<BTreeSet<_>>();
                for declaration in std::iter::once(i.contract)
                    .chain(i.implementation_parameters.iter().map(|p| p.contract))
                    .chain(i.methods.iter().map(|method| method.function))
                {
                    if !declarations.contains(&declaration) {
                        return Err(unit_corrupt(
                            "compiler_unit_owned_implementation_declaration",
                            "owned implementation references a declaration absent from its exact table",
                        ));
                    }
                }
                let types = tables.types.iter().copied().collect::<BTreeSet<_>>();
                for ty in i.type_roots() {
                    if !types.contains(&ty) {
                        return Err(unit_corrupt(
                            "compiler_unit_owned_implementation_type",
                            "owned implementation references a type absent from its exact table",
                        ));
                    }
                }
                let witness_tables = WitnessTables {
                    declarations,
                    types,
                };
                let OwnerKey::Declaration(declaration) = source.owner else {
                    return Err(unit_corrupt(
                        "compiler_unit_witness_scope",
                        "implementation has no declaration scope",
                    ));
                };
                let scope = DeclarationReference {
                    package: source.package,
                    declaration,
                };
                let parameters = i.implementation_parameters.iter().map(|p| p.id).collect();
                for method in &i.methods {
                    for operand in &method.implementations {
                        validate_implementation_operand(operand, &witness_tables)?;
                        validate_witness_scope(operand, scope, &parameters)?;
                    }
                }
            }
            Self::Record {
                fields,
                type_parameters,
                type_parameter_constraints,
            } => {
                validate_nominal_parameters(type_parameters, type_parameter_constraints)?;
                require_kind(source, OwnerKind::Record)?;
                require_item_count("compiled record fields", fields.len(), false)?;
                for field in fields {
                    require_index("record field", field.field, tables.fields.len())?;
                    require_index("record field type", field.ty, tables.types.len())?;
                    if tables.fields[field.field as usize].package != source.package {
                        return Err(unit_corrupt(
                            "compiler_unit_field_identity",
                            "compiled field layout uses a foreign package relocation",
                        ));
                    }
                }
            }
            Self::Variant {
                cases,
                type_parameters,
                type_parameter_constraints,
            } => {
                validate_nominal_parameters(type_parameters, type_parameter_constraints)?;
                require_kind(source, OwnerKind::Variant)?;
                require_item_count("compiled variant cases", cases.len(), false)?;
                for case in cases {
                    require_index("variant case", case.case, tables.cases.len())?;
                    if let Some(payload) = case.payload {
                        require_index("variant case payload", payload, tables.types.len())?;
                    }
                    if tables.cases[case.case as usize].package != source.package {
                        return Err(unit_corrupt(
                            "compiler_unit_case_identity",
                            "compiled case layout uses a foreign package relocation",
                        ));
                    }
                }
            }
            Self::Interface { operations } => {
                require_kind(source, OwnerKind::Interface)?;
                require_item_count("compiled interface operations", operations.len(), false)?;
                for operation in operations {
                    operation.validate(source, tables)?;
                }
            }
            Self::External { signature, .. } => {
                require_kind(source, OwnerKind::External)?;
                signature.validate(tables, source.kind)?;
            }
            Self::Function { signature, code } => {
                if !matches!(
                    source.kind,
                    OwnerKind::PureFunction | OwnerKind::TaskFunction
                ) {
                    return Err(unit_corrupt(
                        "compiler_unit_function_kind",
                        "function payload is bound to another owner kind",
                    ));
                }
                signature.validate(tables, source.kind)?;
                code.validate(tables)?;
                let OwnerKey::Declaration(declaration) = source.owner else {
                    return Err(unit_corrupt(
                        "compiler_unit_witness_scope",
                        "function has no declaration scope",
                    ));
                };
                let scope = DeclarationReference {
                    package: source.package,
                    declaration,
                };
                let parameters = signature
                    .implementation_parameters
                    .iter()
                    .map(|p| p.id)
                    .collect();
                for instruction in &code.instructions {
                    match instruction {
                        CompiledInstruction::ImplementationCall {
                            implementations, ..
                        } => {
                            for operand in implementations {
                                validate_witness_scope(operand, scope, &parameters)?;
                            }
                        }
                        CompiledInstruction::MethodCall { witness, .. } => {
                            validate_witness_scope(witness, scope, &parameters)?
                        }
                        CompiledInstruction::Parallel {
                            left_implementations,
                            right_implementations,
                            ..
                        } => {
                            for operand in left_implementations.iter().chain(right_implementations)
                            {
                                validate_witness_scope(operand, scope, &parameters)?;
                            }
                        }
                        _ => {}
                    }
                }
                if signature.result_borrow.is_some()
                    != matches!(
                        code.instructions.last(),
                        Some(CompiledInstruction::ReturnBorrowed)
                    )
                {
                    return Err(unit_corrupt(
                        "compiler_unit_result_mode",
                        "compiled return path differs from its signature result mode",
                    ));
                }
                if code.parameter_count as usize != signature.parameters.len() {
                    return Err(unit_corrupt(
                        "compiler_unit_parameter_count",
                        "compiled function local parameters disagree with its signature",
                    ));
                }
                if matches!(source.kind, OwnerKind::PureFunction)
                    && !signature.task_requirements.is_empty()
                {
                    return Err(unit_corrupt(
                        "compiler_unit_pure_requirements",
                        "pure compiled function declares task requirements",
                    ));
                }
            }
            Self::Constant { ty, code } => {
                require_kind(source, OwnerKind::Constant)?;
                require_index("constant type", *ty, tables.types.len())?;
                code.validate(tables)?;
                if code.parameter_count != 0 {
                    return Err(unit_corrupt(
                        "compiler_unit_constant_parameters",
                        "compiled constant has parameters",
                    ));
                }
            }
            Self::Component {
                requirements,
                ports,
            } => {
                require_kind(source, OwnerKind::Component)?;
                require_item_count("compiled component ports", ports.len(), false)?;
                require_item_count("compiled component requirements", requirements.len(), true)?;
                for requirement in requirements {
                    requirement.validate(source, tables)?;
                }
                for port in ports {
                    port.validate(source, tables)?;
                }
            }
            Self::Test {
                actual, expected, ..
            } => {
                require_kind(source, OwnerKind::Test)?;
                actual.validate(tables)?;
                expected.validate(tables)?;
                if actual.parameter_count != 0 || expected.parameter_count != 0 {
                    return Err(unit_corrupt(
                        "compiler_unit_test_parameters",
                        "compiled test entries have parameters",
                    ));
                }
            }
            Self::Target {
                component,
                port,
                routes,
                runner,
            } => {
                require_kind(source, OwnerKind::Target)?;
                require_index("target component", *component, tables.declarations.len())?;
                if (*runner == RunnerKind::Http) == port.is_some() {
                    return Err(unit_corrupt(
                        "compiler_unit_target_port_condition",
                        "HTTP target must omit its universal port and non-HTTP target must contain one",
                    ));
                }
                if let Some(port) = port {
                    require_index("target port", *port, tables.ports.len())?;
                }
                if *runner == RunnerKind::Http {
                    validate_compiled_http_routes(routes, tables.ports.len())?;
                } else if !routes.is_empty() {
                    return Err(unit_corrupt(
                        "compiler_unit_non_http_routes",
                        "non-HTTP target contains HTTP routes",
                    ));
                }
            }
        }
        Ok(())
    }
}

fn validate_compiled_http_routes(
    routes: &[CompiledHttpRoute],
    port_count: usize,
) -> Result<(), Diagnostic> {
    use crate::platform::kernel::contract::{
        MAXIMUM_HTTP_PATTERN_SEGMENTS_PER_TARGET, MAXIMUM_HTTP_ROUTE_KEY_BYTES_PER_TARGET,
        MAXIMUM_HTTP_ROUTES_PER_TARGET,
    };
    use crate::platform::kernel::{
        HttpRouteSelector, http_route_pattern_strictly_more_specific, http_route_patterns_overlap,
        http_route_same_pattern_language, http_route_selector_cmp, validate_http_route_method,
    };
    if routes.is_empty() || routes.len() > MAXIMUM_HTTP_ROUTES_PER_TARGET {
        return Err(unit_corrupt(
            "compiler_unit_http_route_count",
            "compiled HTTP target route count is outside the supported bounds",
        ));
    }
    let mut bytes = 0_usize;
    let mut pattern_segments = 0usize;
    let mut identities = BTreeSet::new();
    for (index, route) in routes.iter().enumerate() {
        validate_http_route_method(&route.method).map_err(|_| {
            unit_corrupt(
                "compiler_unit_http_route_key",
                "compiled HTTP route contains an invalid method",
            )
        })?;
        route.selector.validate_local().map_err(|_| {
            unit_corrupt(
                "compiler_unit_http_route_selector",
                "compiled HTTP route contains an invalid selector",
            )
        })?;
        require_index("HTTP route port", route.port, port_count)?;
        if !identities.insert(route.route) {
            return Err(unit_corrupt(
                "compiler_unit_http_route_identity",
                "compiled HTTP target repeats one route identity",
            ));
        }
        bytes = bytes
            .checked_add(route.method.len())
            .and_then(|value| value.checked_add(route.selector.key_bytes()))
            .ok_or_else(|| {
                unit_corrupt(
                    "compiler_unit_http_route_bytes",
                    "compiled HTTP route-key bytes overflowed",
                )
            })?;
        if index > 0 {
            let previous = &routes[index - 1];
            if previous
                .method
                .as_bytes()
                .cmp(route.method.as_bytes())
                .then_with(|| http_route_selector_cmp(&previous.selector, &route.selector))
                != std::cmp::Ordering::Less
            {
                return Err(unit_corrupt(
                    "compiler_unit_http_route_order",
                    "compiled HTTP routes are not in unique canonical key order",
                ));
            }
        }
        if route.capture_parameters.len() != route.selector.capture_count()
            || route
                .capture_parameters
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != route.capture_parameters.len()
        {
            return Err(unit_corrupt(
                "compiler_unit_http_route_capture_parameters",
                "compiled HTTP route capture-parameter identities disagree with its selector",
            ));
        }
        if let HttpRouteSelector::Pattern { segments } = &route.selector {
            pattern_segments = pattern_segments
                .checked_add(segments.len())
                .ok_or_else(|| {
                    unit_corrupt(
                        "compiler_unit_http_route_pattern_segments",
                        "compiled HTTP pattern-segment count overflowed",
                    )
                })?;
            if pattern_segments > MAXIMUM_HTTP_PATTERN_SEGMENTS_PER_TARGET {
                return Err(unit_corrupt(
                    "compiler_unit_http_route_pattern_segments",
                    "compiled HTTP routes exceed the target pattern-segment bound",
                ));
            }
        }
    }
    if bytes > MAXIMUM_HTTP_ROUTE_KEY_BYTES_PER_TARGET {
        return Err(unit_corrupt(
            "compiler_unit_http_route_bytes",
            "compiled HTTP route-key bytes exceed the supported bound",
        ));
    }
    for (index, left) in routes.iter().enumerate() {
        for right in routes.iter().skip(index + 1) {
            if left.port == right.port
                && left.selector.capture_names() != right.selector.capture_names()
            {
                return Err(unit_corrupt(
                    "compiler_unit_http_route_shared_port_signature",
                    "compiled HTTP routes sharing a port disagree on capture names",
                ));
            }
            if left.method != right.method {
                continue;
            }
            match (&left.selector, &right.selector) {
                (
                    HttpRouteSelector::Exact { path: left },
                    HttpRouteSelector::Exact { path: right },
                ) if left == right => {
                    return Err(unit_corrupt(
                        "compiler_unit_http_route_duplicate_language",
                        "compiled HTTP exact routes repeat one match language",
                    ));
                }
                (
                    HttpRouteSelector::Pattern {
                        segments: left_segments,
                    },
                    HttpRouteSelector::Pattern {
                        segments: right_segments,
                    },
                ) if http_route_patterns_overlap(left_segments, right_segments)
                    && !http_route_pattern_strictly_more_specific(
                        left_segments,
                        right_segments,
                    )
                    && !http_route_pattern_strictly_more_specific(
                        right_segments,
                        left_segments,
                    ) =>
                {
                    let message = if http_route_same_pattern_language(left_segments, right_segments)
                    {
                        "compiled HTTP patterns repeat one match language"
                    } else {
                        "compiled HTTP patterns overlap without strict specificity"
                    };
                    return Err(unit_corrupt("compiler_unit_http_route_overlap", message));
                }
                _ => {}
            }
        }
    }
    Ok(())
}

impl CompiledSignature {
    fn validate(&self, tables: &CompilationTables, kind: OwnerKind) -> Result<(), Diagnostic> {
        require_item_count(
            "compiled implementation parameters",
            self.implementation_parameters.len(),
            true,
        )?;
        if !self.implementation_parameters.is_empty() {
            let declarations = tables.declarations.iter().copied().collect::<BTreeSet<_>>();
            let types = tables.types.iter().copied().collect::<BTreeSet<_>>();
            let mut ids = BTreeSet::new();
            let mut names = BTreeSet::new();
            for parameter in &self.implementation_parameters {
                if !ids.insert(parameter.id) || !names.insert(&parameter.name) {
                    return Err(unit_corrupt(
                        "compiler_unit_implementation_parameter",
                        "compiled implementation parameters require distinct identities and names",
                    ));
                }
                if !declarations.contains(&parameter.contract) {
                    return Err(unit_corrupt(
                        "compiler_unit_implementation_parameter_declaration",
                        "compiled implementation parameter references a contract absent from its exact table",
                    ));
                }
                require_item_count(
                    "compiled implementation type arguments",
                    parameter.type_arguments.len(),
                    true,
                )?;
                for ty in std::iter::once(parameter.self_type)
                    .chain(parameter.type_arguments.iter().copied())
                {
                    if !types.contains(&ty) {
                        return Err(unit_corrupt(
                            "compiler_unit_implementation_parameter_type",
                            "compiled implementation parameter references a type absent from its exact table",
                        ));
                    }
                }
            }
        }
        require_item_count(
            "compiled requirement parameters",
            self.requirement_parameters.len(),
            true,
        )?;
        require_unique(
            "compiled requirement parameter",
            &self.requirement_parameters,
        )?;
        require_item_count(
            "compiled effect parameters",
            self.effect_parameters.len(),
            true,
        )?;
        require_unique("compiled effect parameter", &self.effect_parameters)?;
        let row = self.effect.row();
        row.validate()?;
        if matches!(
            self.effect,
            crate::platform::kernel::FunctionEffect::Task { .. }
        ) != (kind == OwnerKind::TaskFunction)
            || (kind == OwnerKind::External
                && (!self.effect_parameters.is_empty() || !self.requirement_parameters.is_empty()))
            || self
                .task_requirements
                .iter()
                .map(|i| tables.requirements.get(*i as usize).copied())
                .collect::<Option<Vec<_>>>()
                != Some(
                    row.requirements
                        .iter()
                        .filter_map(|r| r.concrete())
                        .collect(),
                )
        {
            return Err(unit_corrupt(
                "compiler_effect_signature",
                "compiled callable kind, exact effect row, or requirement relocations disagree",
            ));
        }
        require_item_count("compiled type parameters", self.type_parameters.len(), true)?;
        if self.type_parameter_constraints.len() != self.type_parameters.len()
            || (kind == OwnerKind::External
                && self.type_parameter_constraints.iter().any(|constraints| {
                    constraints.requires_transfer() || constraints.requires_share()
                }))
        {
            return Err(unit_error(
                DiagnosticClass::Corrupt,
                "compiler_type_parameter_constraints",
                "compiled signature must retain exact ordered constraints supported by its callable kind",
            ));
        }
        require_item_count("compiled parameters", self.parameters.len(), true)?;
        require_unique("compiled type parameter", &self.type_parameters)?;
        require_unique(
            "compiled parameter",
            &self
                .parameters
                .iter()
                .map(|parameter| parameter.parameter)
                .collect::<Vec<_>>(),
        )?;
        for parameter in &self.parameters {
            require_index("parameter type", parameter.ty, tables.types.len())?;
            if let Some(requirement) = parameter.resource_requirement {
                require_index(
                    "parameter resource requirement",
                    requirement,
                    tables.requirements.len(),
                )?;
            }
        }
        require_index("signature result", self.result, tables.types.len())?;
        if let Some(source) = self.result_borrow
            && (kind != OwnerKind::PureFunction
                || !matches!(self.effect, crate::platform::kernel::FunctionEffect::Pure)
                || !self.parameters.iter().any(|p| {
                    p.parameter == source
                        && p.use_mode == ParameterUse::Borrow
                        && p.resource_requirement.is_none()
                }))
        {
            return Err(unit_corrupt(
                "compiler_unit_result_borrow",
                "borrowed results require one exact borrowed parameter of a pure graph function",
            ));
        }
        for requirement in &self.task_requirements {
            require_index(
                "signature requirement",
                *requirement,
                tables.requirements.len(),
            )?;
        }
        require_unique("signature requirement", &self.task_requirements)?;
        let bound = self
            .parameters
            .iter()
            .enumerate()
            .filter_map(|(index, parameter)| {
                parameter
                    .resource_requirement
                    .map(|requirement| (index, parameter, requirement))
            })
            .collect::<Vec<_>>();
        if kind == OwnerKind::External {
            let buffer = crate::platform::kernel::encode_type_object(
                &crate::platform::kernel::TypeObject::new(
                    crate::platform::kernel::TypeForm::ByteBuffer,
                )?,
            )?
            .0;
            let cell = crate::platform::kernel::encode_type_object(
                &crate::platform::kernel::TypeObject::new(
                    crate::platform::kernel::TypeForm::OwnedI64Cell,
                )?,
            )?
            .0;
            if self.parameters.iter().any(|parameter| {
                parameter.resource_requirement.is_some()
                    || (parameter.use_mode != ParameterUse::Unrestricted
                        && tables.types[parameter.ty as usize] != buffer
                        && tables.types[parameter.ty as usize] != cell)
            }) {
                return Err(unit_corrupt(
                    "compiler_unit_external_resource_parameter",
                    "compiled external modes require ByteBuffer and cannot bind capability resources",
                ));
            }
        }
        if bound
            .iter()
            .enumerate()
            .any(|(offset, (index, parameter, requirement))| {
                kind != OwnerKind::TaskFunction
                    || index.saturating_add(bound.len() - offset) != self.parameters.len()
                    || parameter.use_mode == ParameterUse::Unrestricted
                    || !self.task_requirements.contains(requirement)
            })
        {
            return Err(unit_corrupt(
                "compiler_unit_resource_parameter_shape",
                "compiled resource parameters must form a borrow/consume suffix bound to their exact task requirements",
            ));
        }
        Ok(())
    }
}

impl CompiledOperationLayout {
    fn validate(
        &self,
        source: &CompilationSource,
        tables: &CompilationTables,
    ) -> Result<(), Diagnostic> {
        require_index(
            "interface operation",
            self.operation,
            tables.operations.len(),
        )?;
        if tables.operations[self.operation as usize].package != source.package {
            return Err(unit_corrupt(
                "compiler_unit_operation_identity",
                "compiled operation layout uses a foreign package relocation",
            ));
        }
        require_item_count("operation parameters", self.parameters.len(), true)?;
        for parameter in &self.parameters {
            require_index("operation parameter type", parameter.ty, tables.types.len())?;
            if parameter.resource_requirement.is_some() {
                return Err(unit_corrupt(
                    "compiler_unit_operation_resource_binding",
                    "compiled operation parameter carries a function resource binding",
                ));
            }
        }
        require_index("operation result", self.result, tables.types.len())
    }
}

impl CompiledRequirement {
    fn validate(
        &self,
        source: &CompilationSource,
        tables: &CompilationTables,
    ) -> Result<(), Diagnostic> {
        require_index(
            "component requirement",
            self.requirement,
            tables.requirements.len(),
        )?;
        if tables.requirements[self.requirement as usize].package != source.package {
            return Err(unit_corrupt(
                "compiler_unit_requirement_identity",
                "compiled requirement uses a foreign package relocation",
            ));
        }
        require_index(
            "requirement interface",
            self.interface,
            tables.declarations.len(),
        )?;
        for operation in &self.operations {
            require_index("requirement operation", *operation, tables.operations.len())?;
        }
        require_unique("requirement operation", &self.operations)
    }
}

impl CompiledPort {
    fn validate(
        &self,
        source: &CompilationSource,
        tables: &CompilationTables,
    ) -> Result<(), Diagnostic> {
        require_index("component port", self.port, tables.ports.len())?;
        if tables.ports[self.port as usize].package != source.package {
            return Err(unit_corrupt(
                "compiler_unit_port_identity",
                "compiled port uses a foreign package relocation",
            ));
        }
        require_index("port function type", self.function_type, tables.types.len())?;
        match &self.implementation {
            CompiledPortImplementation::Function(function) => {
                require_index("port function", *function, tables.declarations.len())
            }
            CompiledPortImplementation::Expression(code) => code.validate(tables),
        }
    }
}

impl CompiledCode {
    fn validate(&self, tables: &CompilationTables) -> Result<(), Diagnostic> {
        let instruction_count = self.instructions.len();
        require_item_count("compiled instructions", instruction_count, false)?;
        if self.local_count < self.parameter_count {
            return Err(unit_corrupt(
                "compiler_unit_local_count",
                "compiled local count is smaller than its parameter count",
            ));
        }
        require_runtime_count("compiled parameters", self.parameter_count)?;
        require_runtime_count("compiled locals", self.local_count)?;
        if !matches!(
            self.instructions.last(),
            Some(CompiledInstruction::Return | CompiledInstruction::ReturnBorrowed)
        ) || self.instructions[..instruction_count - 1]
            .iter()
            .any(|instruction| {
                matches!(
                    instruction,
                    CompiledInstruction::Return | CompiledInstruction::ReturnBorrowed
                )
            })
        {
            return Err(unit_corrupt(
                "compiler_unit_return",
                "compiled code must have exactly one terminal return instruction",
            ));
        }
        // Raw witness operands retain semantic digests rather than dense indexes. Build
        // membership once; per-argument linear scans would multiply hostile table lengths.
        let witness_tables = if self.instructions.iter().any(|instruction| {
            matches!(
                instruction,
                CompiledInstruction::ImplementationCall { .. }
                    | CompiledInstruction::MethodCall { .. }
                    | CompiledInstruction::Parallel { .. }
            )
        }) {
            WitnessTables {
                declarations: tables.declarations.iter().copied().collect(),
                types: tables.types.iter().copied().collect(),
            }
        } else {
            WitnessTables::default()
        };
        for instruction in &self.instructions {
            instruction.validate(self, tables, &witness_tables)?;
        }
        verify_borrow_call_protocol(self)?;
        let depths = verify_stack(self)?;
        verify_owned_borrow_scopes(self, &depths)
    }
}

impl CompiledInstruction {
    fn validate(
        &self,
        code: &CompiledCode,
        tables: &CompilationTables,
        witness_tables: &WitnessTables,
    ) -> Result<(), Diagnostic> {
        if let Self::ImplementationCall {
            effect_arguments, ..
        }
        | Self::Call {
            effect_arguments, ..
        }
        | Self::FunctionValue {
            effect_arguments, ..
        } = self
        {
            require_item_count("effect arguments", effect_arguments.len(), true)?;
            for row in effect_arguments {
                row.validate()?;
            }
        }
        match self {
            Self::SequenceEmpty { sequence_type } => {
                require_index("owned sequence type", *sequence_type, tables.types.len())
            }
            Self::SequenceLength {
                sequence_type,
                source_local,
            }
            | Self::SequencePop {
                sequence_type,
                source_local,
                ..
            } => {
                require_index("owned sequence type", *sequence_type, tables.types.len())?;
                require_index(
                    "sequence source local",
                    *source_local,
                    code.local_count as usize,
                )?;
                if let Self::SequencePop { result_type, .. } = self {
                    require_index("sequence pop result type", *result_type, tables.types.len())?;
                }
                Ok(())
            }
            Self::SequencePush {
                sequence_type,
                value_local,
                source_local,
            } => {
                require_index("owned sequence type", *sequence_type, tables.types.len())?;
                require_index(
                    "sequence value local",
                    *value_local,
                    code.local_count as usize,
                )?;
                require_index(
                    "sequence source local",
                    *source_local,
                    code.local_count as usize,
                )?;
                if value_local == source_local {
                    return Err(unit_corrupt(
                        "compiler_unit_sequence_local",
                        "sequence push requires distinct value and sequence custody",
                    ));
                }
                Ok(())
            }
            Self::BorrowOwnedItem {
                sequence_type,
                source_local,
                binding_local,
                binding_type,
            } => {
                require_index("borrowed sequence type", *sequence_type, tables.types.len())?;
                require_index(
                    "borrowed source local",
                    *source_local,
                    code.local_count as usize,
                )?;
                require_index(
                    "borrowed binding local",
                    *binding_local,
                    code.local_count as usize,
                )?;
                require_index("borrowed binding type", *binding_type, tables.types.len())?;
                if source_local == binding_local || *binding_local < code.parameter_count {
                    return Err(unit_corrupt(
                        "compiler_unit_borrow_local",
                        "scoped read binding must have a distinct lexical destination",
                    ));
                }
                Ok(())
            }
            Self::BorrowOwnedField {
                product_type,
                source_local,
                field,
                binding_local,
                binding_type,
            } => {
                require_index("borrowed product type", *product_type, tables.types.len())?;
                require_index(
                    "borrowed source local",
                    *source_local,
                    code.local_count as usize,
                )?;
                require_index(
                    "borrowed field",
                    *field,
                    crate::platform::kernel::contract::MAXIMUM_CHILDREN,
                )?;
                require_index(
                    "borrowed binding local",
                    *binding_local,
                    code.local_count as usize,
                )?;
                require_index("borrowed binding type", *binding_type, tables.types.len())?;
                if source_local == binding_local || *binding_local < code.parameter_count {
                    return Err(unit_corrupt(
                        "compiler_unit_borrow_local",
                        "scoped read binding must have a distinct lexical destination",
                    ));
                }
                Ok(())
            }
            Self::MatchBorrowedOwned {
                choice_type,
                source_local,
                cases,
            } => {
                require_index("borrowed choice type", *choice_type, tables.types.len())?;
                require_index(
                    "borrowed source local",
                    *source_local,
                    code.local_count as usize,
                )?;
                require_item_count("borrowed choice cases", cases.len(), false)?;
                let mut locals = BTreeSet::new();
                for case in cases {
                    require_index(
                        "borrowed choice target",
                        case.target,
                        code.instructions.len(),
                    )?;
                    require_index(
                        "borrowed choice binding",
                        case.binding_local,
                        code.local_count as usize,
                    )?;
                    require_index(
                        "borrowed choice binding type",
                        case.binding_type,
                        tables.types.len(),
                    )?;
                    if case.binding_local == *source_local
                        || case.binding_local < code.parameter_count
                        || !locals.insert(case.binding_local)
                    {
                        return Err(unit_corrupt(
                            "compiler_unit_borrow_local",
                            "borrowed choice arms require distinct lexical destinations",
                        ));
                    }
                }
                Ok(())
            }
            Self::EndOwnedBorrow { binding_local } => require_index(
                "borrowed binding local",
                *binding_local,
                code.local_count as usize,
            ),
            Self::BeginBorrowCall {
                source_local,
                source_position,
            } => {
                require_index(
                    "borrowed call source",
                    *source_local,
                    code.local_count as usize,
                )?;
                require_runtime_count("borrowed source position", *source_position)
            }
            Self::AdoptBorrowResult {
                source_local,
                binding_local,
                binding_type,
            } => {
                require_index(
                    "borrowed result source",
                    *source_local,
                    code.local_count as usize,
                )?;
                require_index(
                    "borrowed result binding",
                    *binding_local,
                    code.local_count as usize,
                )?;
                require_index("borrowed result type", *binding_type, tables.types.len())?;
                if source_local == binding_local || *binding_local < code.parameter_count {
                    return Err(unit_corrupt(
                        "compiler_unit_borrow_local",
                        "borrowed result requires a distinct lexical destination",
                    ));
                }
                Ok(())
            }
            Self::ChooseOwned { choice_type, case } => {
                require_index("owned choice type", *choice_type, tables.types.len())?;
                require_index(
                    "owned choice case",
                    *case,
                    crate::platform::kernel::contract::MAXIMUM_CHILDREN,
                )
            }
            Self::MatchOwned { choice_type, cases } => {
                require_index("owned choice type", *choice_type, tables.types.len())?;
                require_item_count("owned choice cases", cases.len(), false)?;
                let mut locals = BTreeSet::new();
                for case in cases {
                    require_index("owned choice target", case.target, code.instructions.len())?;
                    require_index(
                        "owned choice payload local",
                        case.binding_local,
                        code.local_count as usize,
                    )?;
                    if !locals.insert(case.binding_local) {
                        return Err(unit_corrupt(
                            "compiler_unit_choice_local",
                            "owned choice arms repeat a payload local",
                        ));
                    }
                }
                Ok(())
            }
            Self::PackOwned {
                product_type,
                fields,
            } => {
                require_index("owned product type", *product_type, tables.types.len())?;
                require_item_count("owned product fields", fields.len(), false)?;
                let unique: BTreeSet<_> = fields.iter().copied().collect();
                if unique.len() != fields.len() || unique.iter().copied().ne(0..fields.len() as u32)
                {
                    return Err(unit_error(
                        DiagnosticClass::Corrupt,
                        "compiler_product_fields",
                        "product field permutation is incomplete",
                    ));
                }
                Ok(())
            }
            Self::UnpackOwned {
                product_type,
                locals,
            } => {
                require_index("owned product type", *product_type, tables.types.len())?;
                require_item_count("owned product locals", locals.len(), false)?;
                for local in locals {
                    require_index("owned product local", *local, code.local_count as usize)?;
                }
                if locals.iter().collect::<BTreeSet<_>>().len() != locals.len() {
                    return Err(unit_error(
                        DiagnosticClass::Corrupt,
                        "compiler_product_locals",
                        "product local destinations repeat",
                    ));
                }
                Ok(())
            }
            Self::Text(index) | Self::StaticText(index) => {
                require_index("text constant", *index, tables.texts.len())
            }
            Self::LoadLocal { local, .. } | Self::StoreLocal(local) => {
                require_index("local", *local, code.local_count as usize)
            }
            Self::JumpIfFalse(target) | Self::Jump(target) => {
                require_index("jump target", *target, code.instructions.len())
            }
            Self::ImplementationCall {
                requirement_arguments,
                effect_arguments: _,
                function,
                type_arguments,
                implementations,
                arguments,
            } => {
                require_item_count("requirement arguments", requirement_arguments.len(), true)?;
                require_index(
                    "implementation call target",
                    *function,
                    tables.declarations.len(),
                )?;
                require_runtime_count("implementation arguments", *arguments)?;
                require_item_count("implementation witnesses", implementations.len(), true)?;
                require_item_count("implementation types", type_arguments.len(), true)?;
                for ty in type_arguments {
                    require_index("implementation type", *ty, tables.types.len())?;
                }
                for operand in implementations {
                    validate_implementation_operand(operand, witness_tables)?;
                }
                Ok(())
            }
            Self::MethodCall {
                witness,
                contract,
                arguments,
                ..
            } => {
                validate_implementation_operand(witness, witness_tables)?;
                if !witness_tables.declarations.contains(contract) {
                    return Err(unit_corrupt(
                        "compiler_unit_method_contract",
                        "owned method contract is absent from its exact declaration table",
                    ));
                }
                require_runtime_count("method arguments", *arguments)
            }
            Self::Parallel {
                left_types,
                left_implementations,
                right_types,
                right_implementations,
                left,
                left_arguments,
                right,
                right_arguments,
                result_type,
            } => {
                for types in [left_types, right_types] {
                    require_item_count("parallel types", types.len(), true)?;
                    for ty in types {
                        require_index("parallel type", *ty, tables.types.len())?;
                    }
                }
                for operands in [left_implementations, right_implementations] {
                    require_item_count("parallel witnesses", operands.len(), true)?;
                    for operand in operands {
                        validate_implementation_operand(operand, witness_tables)?;
                    }
                }
                require_index("left parallel task", *left, tables.declarations.len())?;
                require_index("right parallel task", *right, tables.declarations.len())?;
                require_index("parallel result type", *result_type, tables.types.len())?;
                require_runtime_count("left parallel arguments", *left_arguments)?;
                require_runtime_count("right parallel arguments", *right_arguments)
            }
            Self::Call {
                requirement_arguments,
                effect_arguments: _,
                function,
                type_arguments,
                arguments,
            } => {
                require_item_count("requirement arguments", requirement_arguments.len(), true)?;
                require_runtime_count("call arguments", *arguments)?;
                require_item_count("call type arguments", type_arguments.len(), true)?;
                require_index("function relocation", *function, tables.declarations.len())?;
                for ty in type_arguments {
                    require_index("type argument", *ty, tables.types.len())?;
                }
                Ok(())
            }
            Self::FunctionValue {
                requirement_arguments,
                effect_arguments: _,
                function,
                type_arguments,
            } => {
                require_item_count("requirement arguments", requirement_arguments.len(), true)?;
                require_item_count("function type arguments", type_arguments.len(), true)?;
                require_index("function relocation", *function, tables.declarations.len())?;
                for ty in type_arguments {
                    require_index("type argument", *ty, tables.types.len())?;
                }
                Ok(())
            }
            Self::Record {
                nominal_type,
                type_arguments,
                fields,
            } => {
                require_item_count("nominal type arguments", type_arguments.len(), true)?;
                if nominal_type.is_none() && !type_arguments.is_empty() {
                    return Err(unit_corrupt(
                        "compiler_nominal_arguments",
                        "structural record has nominal arguments",
                    ));
                }
                for ty in type_arguments {
                    require_index("nominal type argument", *ty, tables.types.len())?;
                }
                if let Some(declaration) = nominal_type {
                    require_index(
                        "nominal record declaration",
                        *declaration,
                        tables.declarations.len(),
                    )?;
                }
                require_item_count("record expression fields", fields.len(), false)?;
                for field in fields {
                    field.validate(tables)?;
                }
                Ok(())
            }
            Self::Variant {
                case,
                type_arguments,
                ..
            } => {
                require_item_count("nominal type arguments", type_arguments.len(), true)?;
                for ty in type_arguments {
                    require_index("nominal type argument", *ty, tables.types.len())?;
                }
                require_index("variant case", *case, tables.cases.len())
            }
            Self::Field(selector) => selector.validate(tables),
            Self::List { item_type, items } => {
                require_runtime_count("list items", *items)?;
                require_index("list item type", *item_type, tables.types.len())
            }
            Self::Map {
                key_type,
                value_type,
                entries,
            } => {
                require_runtime_count("map entries", *entries)?;
                require_index("map key type", *key_type, tables.types.len())?;
                require_index("map value type", *value_type, tables.types.len())
            }
            Self::SwitchVariant(arms) => {
                require_item_count("variant switch arms", arms.len(), false)?;
                let mut cases = BTreeSet::new();
                for arm in arms {
                    require_index("match case", arm.case, tables.cases.len())?;
                    require_index("match target", arm.target, code.instructions.len())?;
                    if let Some(local) = arm.binding_local {
                        require_index("match payload local", local, code.local_count as usize)?;
                    }
                    if !cases.insert(arm.case) {
                        return Err(unit_corrupt(
                            "compiler_unit_match_case",
                            "compiled variant switch repeats one exact case",
                        ));
                    }
                }
                Ok(())
            }
            Self::PerformParameter {
                operation,
                arguments,
                ..
            } => {
                require_runtime_count("capability arguments", *arguments)?;
                require_index("capability operation", *operation, tables.operations.len())
            }
            Self::BeginParameterTransaction { binding, .. }
            | Self::CommitParameterTransaction { binding, .. } => {
                require_index("transaction local", *binding, code.local_count as usize)
            }
            Self::BeginTransactionOutcome {
                requirement,
                binding,
                outcome,
            }
            | Self::CommitTransactionOutcome {
                requirement,
                binding,
                outcome,
            } => {
                if let CompiledTransactionRequirement::Concrete(requirement) = requirement {
                    require_index(
                        "transaction requirement",
                        *requirement,
                        tables.requirements.len(),
                    )?;
                }
                require_index("transaction local", *binding, code.local_count as usize)?;
                require_index(
                    "transaction outcome declaration",
                    outcome.outcome,
                    tables.declarations.len(),
                )?;
                require_index(
                    "transaction abort declaration",
                    outcome.abort_reason,
                    tables.declarations.len(),
                )?;
                require_index(
                    "transaction body type",
                    outcome.type_argument,
                    tables.types.len(),
                )?;
                for case in [
                    outcome.committed,
                    outcome.aborted,
                    outcome.condition_failed,
                    outcome.conflict,
                ] {
                    require_index("transaction outcome case", case, tables.cases.len())?;
                }
                Ok(())
            }
            Self::Perform {
                requirement,
                operation,
                arguments,
            } => {
                require_runtime_count("capability arguments", *arguments)?;
                require_index(
                    "capability requirement",
                    *requirement,
                    tables.requirements.len(),
                )?;
                require_index("capability operation", *operation, tables.operations.len())
            }
            Self::BeginTransaction {
                requirement,
                binding,
            }
            | Self::CommitTransaction {
                requirement,
                binding,
            } => {
                require_index(
                    "transaction requirement",
                    *requirement,
                    tables.requirements.len(),
                )?;
                require_index("transaction binding", *binding, code.local_count as usize)
            }
            Self::Invoke { arguments } => require_runtime_count("invoke arguments", *arguments),
            Self::Bind { arguments } | Self::BeginBind { arguments } => {
                if *arguments as usize > crate::platform::kernel::contract::MAXIMUM_CHILDREN {
                    return Err(unit_error(
                        DiagnosticClass::Corrupt,
                        "compiler_unit_bind_protocol",
                        "bind prefix exceeds the canonical child bound",
                    ));
                }
                Ok(())
            }
            Self::Capture { index } => {
                if *index as usize >= crate::platform::kernel::contract::MAXIMUM_CHILDREN {
                    return Err(unit_error(
                        DiagnosticClass::Corrupt,
                        "compiler_unit_bind_protocol",
                        "capture index exceeds the canonical child bound",
                    ));
                }
                Ok(())
            }
            Self::Unit
            | Self::Bool(_)
            | Self::I64(_)
            | Self::F64(_)
            | Self::Drop
            | Self::Return
            | Self::ReturnBorrowed => Ok(()),
        }
    }
}

impl CompiledFieldSelector {
    fn validate(self, tables: &CompilationTables) -> Result<(), Diagnostic> {
        match self {
            Self::Nominal(field) => require_index("nominal field", field, tables.fields.len()),
            Self::Structural(name) => {
                require_index("structural field name", name, tables.structural_names.len())
            }
        }
    }
}

#[derive(Default)]
struct WitnessTables {
    declarations: BTreeSet<DeclarationReference>,
    types: BTreeSet<TypeObjectDigest>,
}

fn validate_implementation_operand(
    operand: &crate::platform::kernel::ImplementationOperand,
    tables: &WitnessTables,
) -> Result<(), Diagnostic> {
    operand.validate_local()?;
    for operand in operand.walk() {
        let declaration = match operand {
            crate::platform::kernel::ImplementationOperand::Concrete {
                implementation,
                type_arguments,
                ..
            } => {
                require_item_count("witness type arguments", type_arguments.len(), true)?;
                for ty in type_arguments {
                    if !tables.types.contains(ty) {
                        return Err(unit_corrupt(
                            "compiler_unit_witness_type",
                            "applied witness references a type absent from its exact table",
                        ));
                    }
                }
                implementation
            }
            crate::platform::kernel::ImplementationOperand::Parameter { scope, .. } => scope,
        };
        if !tables.declarations.contains(declaration) {
            return Err(unit_corrupt(
                "compiler_unit_witness_declaration",
                "implementation witness references a declaration absent from its exact table",
            ));
        }
    }
    Ok(())
}

fn validate_witness_scope(
    operand: &crate::platform::kernel::ImplementationOperand,
    scope: DeclarationReference,
    parameters: &BTreeSet<crate::platform::semantic_id::ImplementationParameterId>,
) -> Result<(), Diagnostic> {
    for operand in operand.walk() {
        if let crate::platform::kernel::ImplementationOperand::Parameter {
            scope: actual,
            parameter,
        } = operand
            && (*actual != scope || !parameters.contains(parameter))
        {
            return Err(unit_corrupt(
                "compiler_unit_witness_scope",
                "implementation parameter is outside the exact compiled declaration scope",
            ));
        }
    }
    Ok(())
}

fn require_kind(source: &CompilationSource, expected: OwnerKind) -> Result<(), Diagnostic> {
    if source.kind != expected {
        return Err(unit_corrupt(
            "compiler_unit_payload_kind",
            format!(
                "compiled payload requires {expected:?}, but source is {:?}",
                source.kind
            ),
        ));
    }
    Ok(())
}

fn require_index(label: &str, index: u32, length: usize) -> Result<(), Diagnostic> {
    if index as usize >= length {
        return Err(unit_corrupt(
            "compiler_unit_index",
            format!("{label} index {index} is outside table length {length}"),
        ));
    }
    Ok(())
}

fn validate_nominal_parameters(
    parameters: &[TypeParameterId],
    constraints: &[crate::platform::kernel::TypeParameterConstraints],
) -> Result<(), Diagnostic> {
    require_item_count("nominal type parameters", parameters.len(), true)?;
    if parameters.len() != constraints.len()
        || parameters.iter().collect::<BTreeSet<_>>().len() != parameters.len()
        || constraints
            .iter()
            .any(|constraint| constraint.requires_transfer() || constraint.requires_share())
    {
        return Err(unit_corrupt(
            "compiler_nominal_parameters",
            "nominal parameters and constraints must form one exact ordered unique vector",
        ));
    }
    Ok(())
}

fn require_item_count(label: &str, count: usize, allow_zero: bool) -> Result<(), Diagnostic> {
    if (!allow_zero && count == 0) || count > MAXIMUM_COMPILER_UNIT_ITEMS {
        return Err(unit_error(
            DiagnosticClass::Resource,
            "compiler_unit_item_count",
            format!("{label} count {count} is outside the compiler-unit bound"),
        ));
    }
    Ok(())
}

fn require_runtime_count(label: &str, count: u32) -> Result<(), Diagnostic> {
    if count as usize > MAXIMUM_COMPILER_UNIT_ITEMS {
        return Err(unit_error(
            DiagnosticClass::Resource,
            "compiler_unit_runtime_count",
            format!("{label} count {count} exceeds the compiler-unit bound"),
        ));
    }
    Ok(())
}

fn verify_borrow_call_protocol(code: &CompiledCode) -> Result<(), Diagnostic> {
    let invalid = || {
        unit_corrupt(
            "compiler_unit_borrow_call_protocol",
            "borrowed call handoff requires an uninterrupted exact begin, call and adoption",
        )
    };
    let interior = |target: u32| {
        let target = target as usize;
        matches!(
            code.instructions.get(target),
            Some(CompiledInstruction::AdoptBorrowResult { .. })
        ) || target.checked_sub(1).is_some_and(|previous| {
            matches!(
                code.instructions.get(previous),
                Some(CompiledInstruction::BeginBorrowCall { .. })
            )
        })
    };
    for (index, instruction) in code.instructions.iter().enumerate() {
        match instruction {
            CompiledInstruction::BeginBorrowCall {
                source_local,
                source_position,
            } => {
                let arguments = match code.instructions.get(index + 1) {
                    Some(
                        CompiledInstruction::Call { arguments, .. }
                        | CompiledInstruction::ImplementationCall { arguments, .. }
                        | CompiledInstruction::MethodCall { arguments, .. },
                    ) => *arguments,
                    _ => return Err(invalid()),
                };
                if *source_position >= arguments
                    || !matches!(code.instructions.get(index + 2), Some(CompiledInstruction::AdoptBorrowResult { source_local: actual, .. }) if actual == source_local)
                {
                    return Err(invalid());
                }
            }
            CompiledInstruction::AdoptBorrowResult { .. } => {
                if !index.checked_sub(2).is_some_and(|previous| {
                    matches!(
                        code.instructions.get(previous),
                        Some(CompiledInstruction::BeginBorrowCall { .. })
                    )
                }) {
                    return Err(invalid());
                }
            }
            CompiledInstruction::Jump(target) | CompiledInstruction::JumpIfFalse(target) => {
                if interior(*target) {
                    return Err(invalid());
                }
            }
            CompiledInstruction::MatchOwned { cases, .. } => {
                if cases.iter().any(|case| interior(case.target)) {
                    return Err(invalid());
                }
            }
            CompiledInstruction::MatchBorrowedOwned { cases, .. } => {
                if cases.iter().any(|case| interior(case.target)) {
                    return Err(invalid());
                }
            }
            CompiledInstruction::SwitchVariant(cases)
                if cases.iter().any(|case| interior(case.target)) =>
            {
                return Err(invalid());
            }
            _ => {}
        }
    }
    Ok(())
}

fn verify_stack(code: &CompiledCode) -> Result<Vec<usize>, Diagnostic> {
    // Persistent, bounded binding states avoid copying an operand bitmap at every instruction.
    // A live binding protects its callee and admitted prefix from ordinary stack consumers.
    #[derive(Clone, Copy)]
    struct Binding {
        parent: Option<usize>,
        position: usize,
        arguments: usize,
        captured: usize,
    }
    let mut bindings = Vec::<Binding>::new();
    let mut pending = vec![(0_usize, 0_usize, None::<usize>)];
    let mut depths = vec![None; code.instructions.len()];
    while let Some((instruction_index, depth, binding)) = pending.pop() {
        let slot = depths.get_mut(instruction_index).ok_or_else(|| {
            unit_corrupt(
                "compiler_unit_control_flow",
                "compiled control flow reaches beyond the instruction stream",
            )
        })?;
        if let Some(previous) = *slot {
            if previous != (depth, binding) {
                return Err(unit_corrupt(
                    "compiler_unit_stack_merge",
                    "compiled control-flow paths merge with different stack depths or unfinished bindings",
                ));
            }
            continue;
        }
        *slot = Some((depth, binding));
        let instruction = &code.instructions[instruction_index];
        let (consumed, produced) = stack_effect(instruction)?;
        let next_depth = depth
            .checked_sub(consumed)
            .and_then(|depth| depth.checked_add(produced))
            .ok_or_else(|| {
                unit_corrupt(
                    "compiler_unit_stack_underflow",
                    "compiled instruction consumes beneath its operand stack",
                )
            })?;
        if next_depth > MAXIMUM_COMPILER_UNIT_ITEMS {
            return Err(unit_error(
                DiagnosticClass::Resource,
                "compiler_unit_stack_depth",
                "compiled operand stack exceeds the compiler-unit bound",
            ));
        }
        let active = binding.map(|index| bindings[index]);
        let protected = active.map_or(0, |value| value.position + 1 + value.captured);
        let mut next_binding = binding;
        let next_state =
            match instruction {
                CompiledInstruction::BeginBind { arguments } if depth > protected => {
                    Some(Binding {
                        parent: binding,
                        position: depth - 1,
                        arguments: *arguments as usize,
                        captured: 0,
                    })
                }
                CompiledInstruction::Capture { index } => {
                    let value = active.filter(|value| {
                    value.captured == *index as usize
                        && value.captured < value.arguments
                        && depth == protected + 1
                }).ok_or_else(|| unit_corrupt(
                    "compiler_unit_bind_protocol",
                    "capture does not admit the next value of its exact unfinished binding",
                ))?;
                    Some(Binding {
                        captured: value.captured + 1,
                        ..value
                    })
                }
                CompiledInstruction::Bind { arguments } => {
                    let value = active
                        .filter(|value| {
                            value.arguments == *arguments as usize
                                && value.captured == value.arguments
                                && depth == protected
                        })
                        .ok_or_else(|| {
                            unit_corrupt(
                                "compiler_unit_bind_protocol",
                                "bind does not complete one fully admitted ordered prefix",
                            )
                        })?;
                    next_binding = value.parent;
                    None
                }
                CompiledInstruction::BeginBind { .. } => {
                    return Err(unit_corrupt(
                        "compiler_unit_bind_protocol",
                        "bind preparation consumes an unfinished binding or capture",
                    ));
                }
                _ if depth - consumed < protected => {
                    return Err(unit_corrupt(
                        "compiler_unit_bind_protocol",
                        "ordinary instruction consumes an unfinished binding or admitted capture",
                    ));
                }
                _ => None,
            };
        if let Some(state) = next_state {
            if bindings.len() >= MAXIMUM_COMPILER_UNIT_ITEMS {
                return Err(unit_error(
                    DiagnosticClass::Resource,
                    "compiler_unit_item_count",
                    "binding verification exceeds the compiler-unit bound",
                ));
            }
            next_binding = Some(bindings.len());
            bindings.push(state);
        }
        match instruction {
            CompiledInstruction::Return | CompiledInstruction::ReturnBorrowed => {
                if depth != 1 {
                    return Err(unit_corrupt(
                        "compiler_unit_return_stack",
                        "compiled return does not consume exactly one result value",
                    ));
                }
            }
            CompiledInstruction::Jump(target) => {
                pending.push((*target as usize, next_depth, next_binding));
            }
            CompiledInstruction::JumpIfFalse(target) => {
                pending.push((*target as usize, next_depth, next_binding));
                pending.push((instruction_index + 1, next_depth, next_binding));
            }
            CompiledInstruction::MatchOwned { cases, .. } => {
                pending.extend(
                    cases
                        .iter()
                        .map(|case| (case.target as usize, next_depth, next_binding)),
                );
            }
            CompiledInstruction::MatchBorrowedOwned { cases, .. } => {
                pending.extend(
                    cases
                        .iter()
                        .map(|case| (case.target as usize, next_depth, next_binding)),
                );
            }
            CompiledInstruction::SwitchVariant(arms) => {
                pending.extend(
                    arms.iter()
                        .map(|arm| (arm.target as usize, next_depth, next_binding)),
                );
            }
            _ => pending.push((instruction_index + 1, next_depth, next_binding)),
        }
    }
    if depths.iter().any(Option::is_none) {
        return Err(unit_corrupt(
            "compiler_unit_unreachable_instruction",
            "compiled code contains an unreachable instruction",
        ));
    }
    Ok(depths
        .into_iter()
        .flatten()
        .map(|(depth, _)| depth)
        .collect())
}

/// Admit lexical loan custody independently of operand-stack shape. Persistent scope nodes
/// keep branch states small; every traversal, including rejected input, shares a finite bound.
fn verify_owned_borrow_scopes(code: &CompiledCode, depths: &[usize]) -> Result<(), Diagnostic> {
    verify_owned_borrow_scopes_with_limit(
        code,
        depths,
        crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK,
    )
}

fn verify_owned_borrow_scopes_with_limit(
    code: &CompiledCode,
    depths: &[usize],
    mut remaining: usize,
) -> Result<(), Diagnostic> {
    #[derive(Clone, Copy)]
    struct Scope {
        parent: Option<usize>,
        source: u32,
        binding: u32,
        entry_depth: usize,
    }
    let mut reserve = || {
        remaining = remaining.checked_sub(1).ok_or_else(|| {
            unit_error(
                DiagnosticClass::Resource,
                "compiler_unit_borrow_work",
                "lexical loan admission exceeds the validation work bound",
            )
        })?;
        Ok::<_, Diagnostic>(())
    };
    let mut scopes = Vec::<Scope>::new();
    let mut pending = vec![(0_usize, None::<usize>)];
    let mut states = vec![None::<Option<usize>>; code.instructions.len()];
    while let Some((index, active)) = pending.pop() {
        reserve()?;
        let slot = states.get_mut(index).ok_or_else(|| {
            unit_corrupt(
                "compiler_unit_borrow_control",
                "lexical loan reaches beyond the instruction stream",
            )
        })?;
        if let Some(previous) = *slot {
            if previous != active {
                return Err(unit_corrupt(
                    "compiler_unit_borrow_merge",
                    "control-flow paths merge with different active lexical loans",
                ));
            }
            continue;
        }
        *slot = Some(active);
        let instruction = &code.instructions[index];
        let depth = depths[index];
        if let Some(scope) = active.map(|node| scopes[node]) {
            let (consumed, _) = stack_effect(instruction)?;
            if depth - consumed < scope.entry_depth {
                return Err(unit_corrupt(
                    "compiler_unit_borrow_stack_prefix",
                    "lexical read body consumes its enclosing operand prefix",
                ));
            }
        }
        let mut next = active;
        let enters = match instruction {
            CompiledInstruction::BorrowOwnedField {
                source_local,
                binding_local,
                ..
            }
            | CompiledInstruction::BorrowOwnedItem {
                source_local,
                binding_local,
                ..
            }
            | CompiledInstruction::AdoptBorrowResult {
                source_local,
                binding_local,
                ..
            } => Some((*source_local, *binding_local)),
            _ => None,
        };
        // A forged assignment or consuming load cannot destroy any ancestor custody. The
        // canonical correspondence check additionally verifies exact types and view uses.
        let destinations: &[u32] = match instruction {
            CompiledInstruction::UnpackOwned { locals, .. } => locals,
            _ => &[],
        };
        let sequence_consumes = match instruction {
            CompiledInstruction::SequencePush {
                value_local,
                source_local,
                ..
            } => Some([*value_local, *source_local]),
            CompiledInstruction::SequencePop { source_local, .. } => {
                Some([*source_local, *source_local])
            }
            _ => None,
        };
        let changed = match instruction {
            CompiledInstruction::StoreLocal(local)
            | CompiledInstruction::LoadLocal {
                local,
                use_mode: ParameterUse::Consume,
            }
            | CompiledInstruction::BeginTransaction { binding: local, .. }
            | CompiledInstruction::BeginParameterTransaction { binding: local, .. }
            | CompiledInstruction::BeginTransactionOutcome { binding: local, .. }
            | CompiledInstruction::CommitTransaction { binding: local, .. }
            | CompiledInstruction::CommitParameterTransaction { binding: local, .. }
            | CompiledInstruction::CommitTransactionOutcome { binding: local, .. } => Some(*local),
            _ => None,
        };
        if changed.is_some()
            || !destinations.is_empty()
            || enters.is_some()
            || sequence_consumes.is_some()
        {
            let mut ancestor = active;
            while let Some(scope) = ancestor.map(|node| scopes[node]) {
                reserve()?;
                let mut writes_custody = false;
                for local in destinations {
                    reserve()?;
                    if *local == scope.source || *local == scope.binding {
                        writes_custody = true;
                        break;
                    }
                }
                if changed.is_some_and(|local| local == scope.source || local == scope.binding)
                    || sequence_consumes.is_some_and(|locals| {
                        locals
                            .iter()
                            .any(|local| *local == scope.source || *local == scope.binding)
                    })
                    || writes_custody
                    || enters.is_some_and(|(_, binding)| {
                        binding == scope.source || binding == scope.binding
                    })
                {
                    return Err(unit_corrupt(
                        "compiler_unit_borrow_custody",
                        "compiled instruction overwrites or consumes an active lexical loan",
                    ));
                }
                ancestor = scope.parent;
            }
        }
        if let Some((source, binding)) = enters {
            reserve()?;
            require_item_count("lexical loan scope states", scopes.len() + 1, false)?;
            next = Some(scopes.len());
            scopes.push(Scope {
                parent: active,
                source,
                binding,
                entry_depth: depth - stack_effect(instruction)?.0,
            });
        }
        match instruction {
            CompiledInstruction::EndOwnedBorrow { binding_local } => {
                let scope = active
                    .map(|node| scopes[node])
                    .filter(|scope| scope.binding == *binding_local)
                    .ok_or_else(|| {
                        unit_corrupt(
                            "compiler_unit_borrow_end",
                            "lexical loan end must release its exact innermost binding",
                        )
                    })?;
                if depth != scope.entry_depth + 1 {
                    return Err(unit_corrupt(
                        "compiler_unit_borrow_result_stack",
                        "lexical read body must leave exactly one independent result above its enclosing prefix",
                    ));
                }
                pending.push((index + 1, scope.parent));
            }
            CompiledInstruction::MatchBorrowedOwned {
                source_local,
                cases,
                ..
            } => {
                for case in cases {
                    reserve()?;
                    let mut ancestor = active;
                    while let Some(scope) = ancestor.map(|node| scopes[node]) {
                        reserve()?;
                        if case.binding_local == scope.source || case.binding_local == scope.binding
                        {
                            return Err(unit_corrupt(
                                "compiler_unit_borrow_custody",
                                "borrowed choice binding overwrites active loan custody",
                            ));
                        }
                        ancestor = scope.parent;
                    }
                    let scope = scopes.len();
                    require_item_count("lexical loan scope states", scope + 1, false)?;
                    scopes.push(Scope {
                        parent: active,
                        source: *source_local,
                        binding: case.binding_local,
                        entry_depth: depth,
                    });
                    pending.push((case.target as usize, Some(scope)));
                }
            }
            CompiledInstruction::Return | CompiledInstruction::ReturnBorrowed
                if active.is_some() =>
            {
                return Err(unit_corrupt(
                    "compiler_unit_borrow_escape",
                    "compiled return leaves an active lexical loan",
                ));
            }
            CompiledInstruction::Return | CompiledInstruction::ReturnBorrowed => {}
            CompiledInstruction::Jump(target) => pending.push((*target as usize, next)),
            CompiledInstruction::JumpIfFalse(target) => {
                pending.push((*target as usize, next));
                pending.push((index + 1, next));
            }
            CompiledInstruction::MatchOwned { cases, .. } => {
                for case in cases {
                    reserve()?;
                    let mut ancestor = active;
                    while let Some(scope) = ancestor.map(|node| scopes[node]) {
                        reserve()?;
                        if case.binding_local == scope.source || case.binding_local == scope.binding
                        {
                            return Err(unit_corrupt(
                                "compiler_unit_borrow_custody",
                                "owned choice binding overwrites active loan custody",
                            ));
                        }
                        ancestor = scope.parent;
                    }
                    pending.push((case.target as usize, next));
                }
            }
            CompiledInstruction::SwitchVariant(arms) => {
                for arm in arms {
                    reserve()?;
                    if let Some(binding) = arm.binding_local {
                        let mut ancestor = active;
                        while let Some(scope) = ancestor.map(|node| scopes[node]) {
                            reserve()?;
                            if binding == scope.source || binding == scope.binding {
                                return Err(unit_corrupt(
                                    "compiler_unit_borrow_custody",
                                    "variant payload binding overwrites active loan custody",
                                ));
                            }
                            ancestor = scope.parent;
                        }
                    }
                    pending.push((arm.target as usize, next));
                }
            }
            _ => pending.push((index + 1, next)),
        }
    }
    Ok(())
}

fn stack_effect(instruction: &CompiledInstruction) -> Result<(usize, usize), Diagnostic> {
    let count = |value: u32| {
        usize::try_from(value).map_err(|_| {
            unit_error(
                DiagnosticClass::Resource,
                "compiler_unit_stack_count",
                "compiled operand count does not fit this platform",
            )
        })
    };
    Ok(match instruction {
        CompiledInstruction::BorrowOwnedItem { .. }
        | CompiledInstruction::AdoptBorrowResult { .. } => (1, 0),
        CompiledInstruction::SequenceEmpty { .. }
        | CompiledInstruction::SequenceLength { .. }
        | CompiledInstruction::SequencePush { .. }
        | CompiledInstruction::SequencePop { .. } => (0, 1),
        CompiledInstruction::BorrowOwnedField { .. }
        | CompiledInstruction::MatchBorrowedOwned { .. }
        | CompiledInstruction::EndOwnedBorrow { .. }
        | CompiledInstruction::BeginBorrowCall { .. } => (0, 0),
        CompiledInstruction::ChooseOwned { .. } => (1, 1),
        CompiledInstruction::MatchOwned { .. } => (1, 0),
        CompiledInstruction::PackOwned { fields, .. } => (fields.len(), 1),
        CompiledInstruction::UnpackOwned { .. } => (1, 0),
        CompiledInstruction::Unit
        | CompiledInstruction::Bool(_)
        | CompiledInstruction::I64(_)
        | CompiledInstruction::F64(_)
        | CompiledInstruction::Text(_)
        | CompiledInstruction::StaticText(_)
        | CompiledInstruction::LoadLocal { .. }
        | CompiledInstruction::FunctionValue { .. } => (0, 1),
        CompiledInstruction::StoreLocal(_) | CompiledInstruction::Drop => (1, 0),
        CompiledInstruction::JumpIfFalse(_) => (1, 0),
        CompiledInstruction::Jump(_)
        | CompiledInstruction::BeginParameterTransaction { .. }
        | CompiledInstruction::CommitParameterTransaction { .. }
        | CompiledInstruction::BeginTransaction { .. }
        | CompiledInstruction::BeginTransactionOutcome { .. }
        | CompiledInstruction::CommitTransaction { .. } => (0, 0),
        CompiledInstruction::CommitTransactionOutcome { .. } => (1, 1),
        CompiledInstruction::ImplementationCall { arguments, .. }
        | CompiledInstruction::MethodCall { arguments, .. }
        | CompiledInstruction::Call { arguments, .. } => (count(*arguments)?, 1),
        CompiledInstruction::Parallel {
            left_arguments,
            right_arguments,
            ..
        } => {
            let arguments = count(*left_arguments)?
                .checked_add(count(*right_arguments)?)
                .ok_or_else(|| {
                    unit_error(
                        DiagnosticClass::Resource,
                        "compiler_unit_stack_count",
                        "parallel operand count overflows",
                    )
                })?;
            (arguments, 1)
        }
        CompiledInstruction::BeginBind { .. } | CompiledInstruction::Capture { .. } => (1, 1),
        CompiledInstruction::Invoke { arguments } | CompiledInstruction::Bind { arguments } => {
            let consumed = count(*arguments)?.checked_add(1).ok_or_else(|| {
                unit_error(
                    DiagnosticClass::Resource,
                    "compiler_unit_stack_count",
                    "invoke operand count overflows",
                )
            })?;
            (consumed, 1)
        }
        CompiledInstruction::Record { fields, .. } => (fields.len(), 1),
        CompiledInstruction::Variant { has_payload, .. } => (usize::from(*has_payload), 1),
        CompiledInstruction::Field(_) => (1, 1),
        CompiledInstruction::List { items, .. } => (count(*items)?, 1),
        CompiledInstruction::Map { entries, .. } => {
            let consumed = count(*entries)?.checked_mul(2).ok_or_else(|| {
                unit_error(
                    DiagnosticClass::Resource,
                    "compiler_unit_stack_count",
                    "map operand count overflows",
                )
            })?;
            (consumed, 1)
        }
        CompiledInstruction::SwitchVariant(_) => (1, 0),
        CompiledInstruction::Perform { arguments, .. }
        | CompiledInstruction::PerformParameter { arguments, .. } => (count(*arguments)?, 1),
        CompiledInstruction::Return | CompiledInstruction::ReturnBorrowed => (1, 0),
    })
}

fn require_unique<T: Ord + Clone>(label: &str, values: &[T]) -> Result<(), Diagnostic> {
    let mut observed = BTreeSet::new();
    for value in values {
        if !observed.insert(value.clone()) {
            return Err(unit_corrupt(
                "compiler_unit_duplicate",
                format!("compiled {label} is duplicated"),
            ));
        }
    }
    Ok(())
}

fn unit_corrupt(code: &'static str, message: impl Into<String>) -> Diagnostic {
    unit_error(DiagnosticClass::Corrupt, code, message)
}

fn unit_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod binding_tests {
    use super::*;

    #[test]
    fn stack_verifier_requires_ordered_capture_admission_and_protects_live_prefixes() {
        use CompiledInstruction::*;
        let code = |instructions| CompiledCode {
            parameter_count: 0,
            local_count: 1,
            instructions,
        };
        let valid = vec![
            Unit,
            BeginBind { arguments: 2 },
            I64(3),
            Capture { index: 0 },
            Unit,
            BeginBind { arguments: 1 },
            Bool(true),
            JumpIfFalse(10),
            I64(5),
            Jump(11),
            I64(6),
            Capture { index: 0 },
            Bind { arguments: 1 },
            Capture { index: 1 },
            Bind { arguments: 2 },
            Return,
        ];
        assert!(verify_stack(&code(valid)).is_ok());
        for instructions in [
            vec![Unit, Bind { arguments: 0 }, Return],
            vec![
                Unit,
                BeginBind { arguments: 1 },
                I64(3),
                Bind { arguments: 1 },
                Return,
            ],
            vec![
                Unit,
                BeginBind { arguments: 1 },
                I64(3),
                Capture { index: 1 },
                Bind { arguments: 1 },
                Return,
            ],
            vec![
                Unit,
                BeginBind { arguments: 1 },
                I64(3),
                Capture { index: 0 },
                StoreLocal(0),
                Bind { arguments: 0 },
                Return,
            ],
            vec![
                Unit,
                BeginBind { arguments: 0 },
                Invoke { arguments: 0 },
                Return,
            ],
            vec![Unit, BeginBind { arguments: 0 }, Return],
            vec![
                Unit,
                BeginBind { arguments: 1 },
                Bool(true),
                JumpIfFalse(7),
                I64(3),
                Capture { index: 0 },
                Jump(8),
                I64(3),
                Bind { arguments: 1 },
                Return,
            ],
        ] {
            let result = verify_stack(&code(instructions));
            assert!(
                matches!(
                    result,
                    Err(Diagnostic {
                        class: DiagnosticClass::Corrupt,
                        ..
                    })
                ),
                "{result:?}"
            );
        }
    }
}

#[cfg(test)]
mod borrow_scope_tests {
    use super::*;
    use CompiledInstruction::*;

    fn field(source_local: u32, binding_local: u32) -> CompiledInstruction {
        BorrowOwnedField {
            product_type: 0,
            source_local,
            field: 0,
            binding_local,
            binding_type: 0,
        }
    }

    fn check(instructions: Vec<CompiledInstruction>) -> Result<(), Diagnostic> {
        let code = CompiledCode {
            parameter_count: 1,
            local_count: 4,
            instructions,
        };
        let depths = verify_stack(&code)?;
        verify_owned_borrow_scopes(&code, &depths)
    }

    #[test]
    fn lexical_loan_admission_preserves_nested_and_choice_scopes_across_merges() {
        assert!(
            check(vec![
                field(0, 1),
                Bool(true),
                JumpIfFalse(7),
                field(1, 2),
                I64(1),
                EndOwnedBorrow { binding_local: 2 },
                Jump(8),
                I64(2),
                EndOwnedBorrow { binding_local: 1 },
                Return,
            ])
            .is_ok()
        );
        assert!(
            check(vec![
                MatchBorrowedOwned {
                    choice_type: 0,
                    source_local: 0,
                    cases: vec![
                        CompiledBorrowedOwnedChoiceJump {
                            target: 1,
                            binding_local: 1,
                            binding_type: 0
                        },
                        CompiledBorrowedOwnedChoiceJump {
                            target: 4,
                            binding_local: 2,
                            binding_type: 0
                        },
                    ],
                },
                I64(3),
                EndOwnedBorrow { binding_local: 1 },
                Jump(7),
                I64(4),
                EndOwnedBorrow { binding_local: 2 },
                Jump(7),
                Return,
            ])
            .is_ok()
        );
    }

    fn item(source_local: u32, binding_local: u32) -> CompiledInstruction {
        BorrowOwnedItem {
            sequence_type: 0,
            source_local,
            binding_local,
            binding_type: 0,
        }
    }

    #[test]
    fn sequence_item_loan_excludes_consumed_index_from_its_operand_prefix() {
        assert!(
            check(vec![
                I64(9),
                I64(0),
                item(0, 1),
                I64(7),
                EndOwnedBorrow { binding_local: 1 },
                Drop,
                Return,
            ])
            .is_ok()
        );
        assert!(
            check(vec![
                I64(0),
                item(0, 1),
                I64(1),
                item(1, 2),
                I64(7),
                EndOwnedBorrow { binding_local: 2 },
                EndOwnedBorrow { binding_local: 1 },
                Return,
            ])
            .is_ok()
        );
        let error = check(vec![
            I64(9),
            I64(0),
            item(0, 1),
            Drop,
            I64(7),
            I64(8),
            EndOwnedBorrow { binding_local: 1 },
            Drop,
            Return,
        ])
        .unwrap_err();
        assert_eq!(error.code, "compiler_unit_borrow_stack_prefix");
        let error = check(vec![
            I64(0),
            item(0, 1),
            EndOwnedBorrow { binding_local: 1 },
            I64(7),
            Return,
        ])
        .unwrap_err();
        assert_eq!(error.code, "compiler_unit_borrow_result_stack");
    }

    #[test]
    fn sequence_mutation_cannot_consume_an_active_source_or_item_loan() {
        for mutation in [
            SequencePush {
                sequence_type: 0,
                value_local: 2,
                source_local: 0,
            },
            SequencePush {
                sequence_type: 0,
                value_local: 1,
                source_local: 2,
            },
            SequencePop {
                sequence_type: 0,
                result_type: 0,
                source_local: 0,
            },
            SequencePop {
                sequence_type: 0,
                result_type: 0,
                source_local: 1,
            },
        ] {
            let error = check(vec![
                I64(0),
                item(0, 1),
                mutation,
                EndOwnedBorrow { binding_local: 1 },
                Return,
            ])
            .unwrap_err();
            assert_eq!(error.code, "compiler_unit_borrow_custody");
        }
        assert!(
            check(vec![
                I64(0),
                item(0, 1),
                SequenceLength {
                    sequence_type: 0,
                    source_local: 0
                },
                EndOwnedBorrow { binding_local: 1 },
                Return,
            ])
            .is_ok()
        );
    }

    #[test]
    fn lexical_loan_admission_rejects_missing_forged_and_out_of_order_guards() {
        for (instructions, expected) in [
            (
                vec![field(0, 1), Unit, Return],
                "compiler_unit_borrow_escape",
            ),
            (
                vec![
                    field(0, 1),
                    Unit,
                    EndOwnedBorrow { binding_local: 2 },
                    Return,
                ],
                "compiler_unit_borrow_end",
            ),
            (
                vec![Unit, EndOwnedBorrow { binding_local: 1 }, Return],
                "compiler_unit_borrow_end",
            ),
            (
                vec![
                    field(0, 1),
                    field(1, 2),
                    Unit,
                    EndOwnedBorrow { binding_local: 1 },
                    EndOwnedBorrow { binding_local: 2 },
                    Return,
                ],
                "compiler_unit_borrow_end",
            ),
            (
                vec![
                    field(0, 1),
                    LoadLocal {
                        local: 0,
                        use_mode: ParameterUse::Consume,
                    },
                    Drop,
                    Unit,
                    EndOwnedBorrow { binding_local: 1 },
                    Return,
                ],
                "compiler_unit_borrow_custody",
            ),
            (
                vec![
                    field(0, 1),
                    Unit,
                    StoreLocal(1),
                    Unit,
                    EndOwnedBorrow { binding_local: 1 },
                    Return,
                ],
                "compiler_unit_borrow_custody",
            ),
            (
                vec![
                    field(0, 1),
                    field(1, 1),
                    Unit,
                    EndOwnedBorrow { binding_local: 1 },
                    EndOwnedBorrow { binding_local: 1 },
                    Return,
                ],
                "compiler_unit_borrow_custody",
            ),
            (
                vec![
                    Bool(true),
                    JumpIfFalse(4),
                    field(0, 1),
                    Jump(5),
                    field(0, 2),
                    Unit,
                    EndOwnedBorrow { binding_local: 1 },
                    Return,
                ],
                "compiler_unit_borrow_merge",
            ),
        ] {
            let error = check(instructions).unwrap_err();
            assert_eq!(error.code, expected, "{error:?}");
        }
    }

    #[test]
    fn lexical_loan_admission_rejects_branch_payload_writes_to_ancestor_custody() {
        for branch in [
            MatchOwned {
                choice_type: 0,
                cases: vec![CompiledOwnedChoiceJump {
                    target: 3,
                    binding_local: 1,
                }],
            },
            SwitchVariant(vec![CompiledVariantJump {
                case: 0,
                target: 3,
                binding_local: Some(0),
            }]),
        ] {
            let error = check(vec![
                field(0, 1),
                Unit,
                branch,
                Unit,
                EndOwnedBorrow { binding_local: 1 },
                Return,
            ])
            .unwrap_err();
            assert_eq!(error.code, "compiler_unit_borrow_custody", "{error:?}");
        }
    }

    #[test]
    fn lexical_loan_admission_binds_body_result_to_its_entry_operand_prefix() {
        for (instructions, expected) in [
            (
                vec![
                    I64(1),
                    field(0, 1),
                    Drop,
                    I64(2),
                    EndOwnedBorrow { binding_local: 1 },
                    Return,
                ],
                "compiler_unit_borrow_stack_prefix",
            ),
            (
                vec![
                    I64(1),
                    field(0, 1),
                    EndOwnedBorrow { binding_local: 1 },
                    Return,
                ],
                "compiler_unit_borrow_result_stack",
            ),
        ] {
            let error = check(instructions).unwrap_err();
            assert_eq!(error.code, expected, "{error:?}");
        }
    }

    #[test]
    fn lexical_loan_work_bound_distinguishes_exhaustion_from_nested_branch_corruption() {
        let code = |destination| CompiledCode {
            parameter_count: 1,
            local_count: 4,
            instructions: vec![
                field(0, 1),
                field(1, 2),
                Unit,
                MatchOwned {
                    choice_type: 0,
                    cases: vec![CompiledOwnedChoiceJump {
                        target: 4,
                        binding_local: destination,
                    }],
                },
                Unit,
                EndOwnedBorrow { binding_local: 2 },
                EndOwnedBorrow { binding_local: 1 },
                Return,
            ],
        };
        let invalid = code(0);
        let depths = verify_stack(&invalid).expect("the adversary has valid operand flow");
        // The smaller allowance reaches the inner scope but cannot inspect the outer
        // ancestor. One more step exposes the forbidden write instead of exhaustion.
        let exhausted = verify_owned_borrow_scopes_with_limit(&invalid, &depths, 9).unwrap_err();
        assert_eq!(exhausted.class, DiagnosticClass::Resource);
        assert_eq!(exhausted.code, "compiler_unit_borrow_work");
        let corrupt = verify_owned_borrow_scopes_with_limit(&invalid, &depths, 10).unwrap_err();
        assert_eq!(corrupt.class, DiagnosticClass::Corrupt);
        assert_eq!(corrupt.code, "compiler_unit_borrow_custody");

        let valid = code(3);
        let depths = verify_stack(&valid).unwrap();
        assert!(verify_owned_borrow_scopes_with_limit(&valid, &depths, 14).is_ok());
        let exhausted = verify_owned_borrow_scopes_with_limit(&valid, &depths, 13).unwrap_err();
        assert_eq!(exhausted.class, DiagnosticClass::Resource);
        assert_eq!(exhausted.code, "compiler_unit_borrow_work");
    }
}
